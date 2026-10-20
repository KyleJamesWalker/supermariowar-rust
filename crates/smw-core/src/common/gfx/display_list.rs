//! A record of the screen writes of one gameplay draw, so the main loop can show frames between two ticks. Recording
//! never changes what a tick draws; only frames rendered from two records are new, and those are display-only.

use crate::common::gfx::soft_surface::{self, BlitState, PixelsMut, Rect, Surface};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// What drew a write, by the address of the player, object, eyecandy or platform, so the same write in the next
/// tick's record can be found.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Owner {
    None,
    Drawable(usize),
}

/// The owner for writes drawn by the value at `drawable`.
pub fn owner_of<T: ?Sized>(drawable: *const T) -> Owner {
    Owner::Drawable(drawable as *const u8 as usize)
}

#[derive(Clone, Copy)]
enum Op {
    /// `patches`: the draw's patch count when this was recorded; the later patches of `src` restore it for this blit.
    Blit { src: *mut Surface, state: BlitState, src_rect: Option<Rect>, at: Rect, scaled: bool, patches: usize },
    Fill { rect: Option<Rect>, color: u32 },
}

/// One recorded screen write; `seq` numbers the owner's writes since [`set_owner`], pairing writes across ticks.
#[derive(Clone, Copy)]
pub struct Cmd {
    owner: Owner,
    seq: u16,
    clip: Rect,
    op: Op,
}

/// The pixels of `rect` of a source surface as they were before a write, which happened after blits from it.
#[derive(Clone)]
struct Patch {
    surface: *mut Surface,
    rect: Rect,
    old: Rc<Surface>,
}

/// One gameplay draw: its screen writes, and copies of what the draw changed in its sources after using them:
/// patches of the written rectangles when every blit from the source is unscaled, else whole snapshots.
#[derive(Clone)]
pub struct Draw {
    cmds: Vec<Cmd>,
    snapshots: Vec<Rc<Surface>>,
    patches: Vec<Patch>,
}

impl Draw {
    pub const fn new() -> Self {
        Draw { cmds: Vec::new(), snapshots: Vec::new(), patches: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }
}

/// A move larger than this between two ticks is a wrap, warp or respawn: the frame between them snaps, not slides.
const MAX_SLIDE: i32 = 48;

thread_local! {
    static RECORD: RefCell<Option<Draw>> = const { RefCell::new(None) };
    static SCREEN: Cell<*mut Surface> = const { Cell::new(std::ptr::null_mut()) };
    static OWNER: Cell<(Owner, u16)> = const { Cell::new((Owner::None, 0)) };
    static RECORD_ONLY: Cell<bool> = const { Cell::new(false) };
    /// The surfaces this draw's recorded blits read from, so far as no snapshot has replaced them, and whether
    /// every such blit is one that patches can restore.
    static SOURCES: RefCell<Vec<(*mut Surface, bool)>> = const { RefCell::new(Vec::new()) };
    /// Every snapshot made, so one that no draw holds any more is reused instead of allocated.
    static SNAPSHOTS: RefCell<Vec<Rc<Surface>>> = const { RefCell::new(Vec::new()) };
    /// `render`'s lookup of the previous draw's writes, kept for its allocation.
    static BEFORE: RefCell<HashMap<(Owner, u16), (i32, i32, *mut Surface, bool)>> = RefCell::new(HashMap::new());
}

/// Starts recording the writes to `screen`. With `record_only`, they are not drawn: the frames shown render from
/// the record, so a tick that is never shown costs no pixels.
pub fn begin(screen: *mut Surface, record_only: bool) {
    SCREEN.set(screen);
    RECORD_ONLY.set(record_only);
    OWNER.set((Owner::None, 0));
    SOURCES.with_borrow_mut(Vec::clear);
    RECORD.with_borrow_mut(|r| *r = Some(Draw::new()));
}

/// Stops recording and returns the writes since [`begin`].
pub fn end() -> Draw {
    SCREEN.set(std::ptr::null_mut());
    RECORD_ONLY.set(false);
    RECORD.with_borrow_mut(|r| r.take().unwrap_or(Draw::new()))
}

/// The owner of the writes that follow, until the next call.
pub fn set_owner(owner: Owner) {
    OWNER.set((owner, 0));
}

fn push(dst: *mut Surface, op: impl FnOnce() -> Op) {
    if dst.is_null() || dst != SCREEN.get() {
        return;
    }
    RECORD.with_borrow_mut(|r| {
        if let Some(r) = r {
            let (owner, seq) = OWNER.get();
            OWNER.set((owner, seq.wrapping_add(1)));
            r.cmds.push(Cmd { owner, seq, clip: unsafe { (*dst).clip }, op: op() });
        }
    });
}

/// Before anything draws into `area` of `dst`: if this draw already blitted from it, keeps what the write changes as
/// it was for those blits (the map's animated tiles redraw its background after the background is on the screen).
pub(crate) unsafe fn before_write(dst: *mut Surface, area: Rect) {
    if dst.is_null() || dst == SCREEN.get() {
        return;
    }
    RECORD.with_borrow_mut(|r| {
        let Some(r) = r else { return };
        let Some(plain) = SOURCES.with_borrow(|s| s.iter().find(|&&(p, _)| p == dst).map(|&(_, plain)| plain)) else {
            return;
        };
        let d = &*dst;
        if plain {
            let rect = area.intersect(&Rect::new(0, 0, d.w, d.h)).unwrap_or(Rect::new(0, 0, 0, 0));
            let mut old = Surface { w: rect.w, h: rect.h, pixels: Vec::with_capacity((rect.w * rect.h) as usize), ..*d };
            old.clip = Rect::new(0, 0, rect.w, rect.h);
            for y in rect.y..rect.y + rect.h {
                let row = (y * d.w + rect.x) as usize;
                old.pixels.extend_from_slice(&d.pixels[row..row + rect.w as usize]);
            }
            r.patches.push(Patch { surface: dst, rect, old: Rc::new(old) });
            return;
        }
        SOURCES.with_borrow_mut(|s| s.retain(|&(p, _)| p != dst));
        let copy = snapshot(d);
        let ptr = Rc::as_ptr(&copy) as *mut Surface;
        for c in r.cmds.iter_mut() {
            if let Op::Blit { src, .. } = &mut c.op {
                if *src == dst {
                    *src = ptr;
                }
            }
        }
        for p in r.patches.iter_mut().filter(|p| p.surface == dst) {
            p.surface = ptr;
        }
        r.snapshots.push(copy);
    });
}

/// A copy of `surface`, made in the pixels of a snapshot no draw holds any more when there is one.
fn snapshot(surface: &Surface) -> Rc<Surface> {
    SNAPSHOTS.with_borrow_mut(|all| {
        if let Some(spare) = all.iter_mut().find(|s| Rc::strong_count(s) == 1) {
            let s = Rc::get_mut(spare).expect("held only here");
            let mut pixels = std::mem::take(&mut s.pixels);
            pixels.clone_from(&surface.pixels);
            *s = Surface { pixels, ..*surface };
            return spare.clone();
        }
        let copy = Rc::new(surface.clone());
        all.push(copy.clone());
        copy
    })
}

/// Whether a write to `dst` is recorded and not drawn.
pub(crate) fn record_only(dst: *mut Surface) -> bool {
    RECORD_ONLY.get() && !dst.is_null() && dst == SCREEN.get()
}

pub(crate) unsafe fn record_blit(src: *mut Surface, src_rect: Option<Rect>, dst: *mut Surface, at: Option<Rect>, scaled: bool) {
    if !dst.is_null() && dst == SCREEN.get() {
        let s = &*src;
        let inside = src_rect.is_none_or(|r| r.w > 0 && r.h > 0 && r.x >= 0 && r.y >= 0 && r.x + r.w <= s.w && r.y + r.h <= s.h);
        let plain = inside && !scaled;
        SOURCES.with_borrow_mut(|sources| match sources.iter_mut().find(|(p, _)| *p == src) {
            Some((_, p)) => *p &= plain,
            None => sources.push((src, plain)),
        });
    }
    let patches = RECORD.with_borrow(|r| r.as_ref().map_or(0, |r| r.patches.len()));
    push(dst, || Op::Blit { src, state: (*src).state(), src_rect, at: at.unwrap_or(Rect::new(0, 0, (*dst).w, (*dst).h)), scaled, patches });
}

pub(crate) fn record_fill(dst: *mut Surface, rect: Option<Rect>, color: u32) {
    push(dst, || Op::Fill { rect, color });
}

/// Draws `cur` onto `screen` with each owned write placed `alpha` (0 to 1) of the way from its position in `prev`.
pub unsafe fn render(screen: *mut Surface, prev: &Draw, cur: &Draw, alpha: f32) {
    BEFORE.with_borrow_mut(|before| {
        before.clear();
        before.extend(prev.cmds.iter().filter(|c| c.owner != Owner::None).filter_map(|c| match c.op {
            Op::Blit { src, at, patches, .. } => Some(((c.owner, c.seq), (at.x, at.y, src, patched(prev, src, patches)))),
            Op::Fill { .. } => None,
        }));
        render_with(screen, before, cur, alpha);
    });
}

/// Whether the source of a blit recorded at patch count `from` changed after it.
fn patched(draw: &Draw, src: *mut Surface, from: usize) -> bool {
    draw.patches[from..].iter().any(|p| p.surface == src)
}

unsafe fn render_with(screen: *mut Surface, before: &HashMap<(Owner, u16), (i32, i32, *mut Surface, bool)>, cur: &Draw, alpha: f32) {
    let s = &mut *screen;
    for c in &cur.cmds {
        let mut view = PixelsMut { w: s.w, h: s.h, pitch: s.w as usize, pixels: &mut s.pixels, clip: c.clip, alpha: s.alpha };
        match c.op {
            Op::Fill { rect, color } => soft_surface::fill_rect(&mut view, rect.as_ref(), color),
            Op::Blit { src, state, src_rect, mut at, scaled, patches } => {
                let changed = patched(cur, src, patches);
                if let Some(&(px, py, psrc, prev_changed)) = before.get(&(c.owner, c.seq)) {
                    let same = psrc == src && !changed && !prev_changed;
                    if same && (at.x - px).abs() <= MAX_SLIDE && (at.y - py).abs() <= MAX_SLIDE {
                        at.x = px + ((at.x - px) as f32 * alpha).round() as i32;
                        at.y = py + ((at.y - py) as f32 * alpha).round() as i32;
                    }
                }
                if src == screen {
                    continue;
                }
                let surface = &*src;
                let mut out = at;
                if scaled {
                    soft_surface::blit_scaled(&surface.view(), &state, src_rect.as_ref(), &mut view, Some(&mut out));
                    continue;
                }
                let read = src_rect.unwrap_or(Rect::new(0, 0, surface.w, surface.h));
                if !changed {
                    soft_surface::blit(&surface.view(), &state, src_rect.as_ref(), &mut view, Some(&mut out));
                    continue;
                }
                // Each pixel is blitted once: from the oldest patch over it, which holds what this blit read, else live.
                let mut claimed: Vec<Rect> = Vec::new();
                let part = |from: &Surface, origin: (i32, i32), r: Rect, view: &mut PixelsMut| {
                    let rect = Rect::new(r.x - origin.0, r.y - origin.1, r.w, r.h);
                    let mut to = Rect::new(at.x + r.x - read.x, at.y + r.y - read.y, r.w, r.h);
                    soft_surface::blit(&from.view(), &state, Some(&rect), view, Some(&mut to));
                };
                for p in cur.patches[patches..].iter().filter(|p| p.surface == src) {
                    let Some(area) = p.rect.intersect(&read) else { continue };
                    for r in subtract(area, &claimed) {
                        part(&p.old, (p.rect.x, p.rect.y), r, &mut view);
                    }
                    claimed.push(area);
                }
                for r in subtract(read, &claimed) {
                    part(surface, (0, 0), r, &mut view);
                }
            }
        }
    }
}


/// The parts of `r` outside every rectangle of `holes`, as disjoint rectangles.
fn subtract(r: Rect, holes: &[Rect]) -> Vec<Rect> {
    let mut parts = vec![r];
    for hole in holes {
        parts = parts
            .into_iter()
            .flat_map(|p| {
                let Some(cut) = p.intersect(hole) else { return vec![p] };
                let (right, bottom) = (p.x + p.w, p.y + p.h);
                let (cut_right, cut_bottom) = (cut.x + cut.w, cut.y + cut.h);
                [
                    Rect::new(p.x, p.y, p.w, cut.y - p.y),
                    Rect::new(p.x, cut_bottom, p.w, bottom - cut_bottom),
                    Rect::new(p.x, cut.y, cut.x - p.x, cut.h),
                    Rect::new(cut_right, cut.y, right - cut_right, cut.h),
                ]
                .into_iter()
                .filter(|q| q.w > 0 && q.h > 0)
                .collect()
            })
            .collect();
    }
    parts
}

/// `SMW_INTERPOLATE=0` turns interpolation off; read once.
pub fn interpolation_allowed() -> bool {
    static ALLOWED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ALLOWED.get_or_init(|| std::env::var_os("SMW_INTERPOLATE").is_none_or(|v| v != "0"))
}

/// `SMW_CHECK_DISPLAY_LIST`, read once.
pub fn check_enabled() -> bool {
    static CHECK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CHECK.get_or_init(|| std::env::var_os("SMW_CHECK_DISPLAY_LIST").is_some())
}

/// `SMW_CHECK_DISPLAY_LIST`: whether rendering `draw` alone reproduces `screen` exactly; otherwise the number of pixels
/// that differ and their bounding box. A difference means a screen write the record missed.
pub unsafe fn check(screen: &Surface, draw: &Draw) -> Result<(), (usize, Rect)> {
    let mut copy = screen.clone();
    copy.pixels.fill(0);
    render(&mut copy, &Draw::new(), draw, 1.0);
    let w = screen.w as usize;
    let diff: Vec<usize> = screen.pixels.iter().zip(&copy.pixels).enumerate().filter(|(_, (a, b))| a != b).map(|(i, _)| i).collect();
    if diff.is_empty() {
        return Ok(());
    }
    let (x0, x1) = (diff.iter().map(|i| i % w).min().unwrap(), diff.iter().map(|i| i % w).max().unwrap());
    let (y0, y1) = (diff[0] / w, diff[diff.len() - 1] / w);
    Err((diff.len(), Rect::new(x0 as i32, y0 as i32, (x1 - x0 + 1) as i32, (y1 - y0 + 1) as i32)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::gfx::blit::{fill_rect, upper_blit, upper_blit_scaled};
    use crate::common::gfx::soft_surface::BlendMode;

    fn pattern(w: i32, h: i32, seed: u32) -> Box<Surface> {
        let mut s = Box::new(Surface::new(w, h));
        for (i, p) in s.pixels.iter_mut().enumerate() {
            *p = (i as u32).wrapping_mul(2654435761).wrapping_add(seed) | 0xFF00_0000;
        }
        s
    }

    unsafe fn blit(src: &mut Surface, from: Option<Rect>, dst: &mut Surface, x: i32, y: i32) {
        let mut at = Rect::new(x, y, 0, 0);
        upper_blit(src, from.as_ref().map_or(std::ptr::null(), |r| r as *const Rect), dst, &mut at);
    }

    /// Draws a background from `map`, redraws tiles of `map` around further blits from it, and checks the record.
    unsafe fn redraws_its_source(blend: BlendMode, key: bool, scaled_late: bool) -> Draw {
        let mut screen = Box::new(Surface::new(640, 480));
        let mut map = pattern(640, 480, 1);
        for p in map.pixels.iter_mut().step_by(7) {
            *p &= 0x80FF_FFFF;
        }
        map.blend = blend;
        if key {
            map.color_key = Some(map.pixels[3] & 0x00FF_FFFF);
        }
        let mut tiles = pattern(128, 128, 2);
        let mut sprites = pattern(64, 64, 3);
        begin(&mut *screen, false);
        blit(&mut map, None, &mut screen, 0, 0);
        blit(&mut tiles, Some(Rect::new(0, 0, 32, 32)), &mut map, 32, 32);
        blit(&mut sprites, Some(Rect::new(0, 0, 32, 32)), &mut screen, 40, 40);
        blit(&mut map, Some(Rect::new(16, 16, 64, 64)), &mut screen, 300, 200);
        blit(&mut tiles, Some(Rect::new(32, 0, 32, 32)), &mut map, 48, 48);
        fill_rect(&mut *map, &Rect::new(600, 440, 80, 80), 0xFF12_3456);
        if scaled_late {
            let mut at = Rect::new(500, 0, 50, 70);
            upper_blit_scaled(&mut *map, &Rect::new(0, 0, 100, 100), &mut *screen, &mut at);
        }
        blit(&mut tiles, Some(Rect::new(64, 0, 32, 32)), &mut map, 20, 60);
        blit(&mut map, Some(Rect::new(0, 0, 128, 128)), &mut screen, 200, 300);
        fill_rect(&mut *map, std::ptr::null(), 0xFF00_00FF);
        let draw = end();
        assert!(check(&screen, &draw).is_ok());
        draw
    }

    #[test]
    fn patches_restore_what_unscaled_blits_read() {
        for (blend, key) in [(BlendMode::None, false), (BlendMode::Blend, false), (BlendMode::Blend, true)] {
            let draw = unsafe { redraws_its_source(blend, key, false) };
            assert_eq!((draw.snapshots.len(), draw.patches.len()), (0, 5));
        }
    }

    #[test]
    fn a_scaled_blit_falls_back_to_a_snapshot_that_keeps_earlier_patches() {
        let draw = unsafe { redraws_its_source(BlendMode::None, false, true) };
        assert_eq!((draw.snapshots.len(), draw.patches.len()), (1, 4));
    }
}
