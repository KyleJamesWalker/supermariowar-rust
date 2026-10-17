//! A record of the screen writes of one gameplay draw, so the main loop can show frames between two ticks. Recording
//! never changes what a tick draws; only frames rendered from two records are new, and those are display-only.

use crate::common::gfx::soft_surface::{self, BlitState, PixelsMut, Rect, Surface};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// What drew a write, by the address of the player, object, eyecandy or platform, so the same write in the next
/// tick's record can be found.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Owner {
    None,
    Drawable(usize),
}

pub fn owner_of<T: ?Sized>(drawable: *const T) -> Owner {
    Owner::Drawable(drawable as *const u8 as usize)
}

#[derive(Clone, Copy)]
enum Op {
    Blit { src: *mut Surface, state: BlitState, src_rect: Option<Rect>, at: Rect, scaled: bool },
    Fill { rect: Option<Rect>, color: u32 },
}

#[derive(Clone, Copy)]
pub struct Cmd {
    owner: Owner,
    seq: u16,
    clip: Rect,
    op: Op,
}

/// One gameplay draw: its screen writes, and copies of the sources the draw changed after using them.
#[derive(Clone)]
pub struct Draw {
    cmds: Vec<Cmd>,
    snapshots: Vec<std::rc::Rc<Surface>>,
}

impl Draw {
    pub const fn new() -> Self {
        Draw { cmds: Vec::new(), snapshots: Vec::new() }
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
}

pub fn begin(screen: *mut Surface) {
    SCREEN.set(screen);
    OWNER.set((Owner::None, 0));
    RECORD.with_borrow_mut(|r| *r = Some(Draw::new()));
}

pub fn end() -> Draw {
    SCREEN.set(std::ptr::null_mut());
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

/// Before anything draws into `dst`: if this draw already blitted from it, keeps a copy of it as it was for those
/// blits (the map's animated tiles redraw its background after the background is on the screen).
pub(crate) unsafe fn before_write(dst: *mut Surface) {
    if dst.is_null() || dst == SCREEN.get() {
        return;
    }
    RECORD.with_borrow_mut(|r| {
        let Some(r) = r else { return };
        if !r.cmds.iter().any(|c| matches!(c.op, Op::Blit { src, .. } if src == dst)) {
            return;
        }
        let copy = std::rc::Rc::new((*dst).clone());
        let ptr = std::rc::Rc::as_ptr(&copy) as *mut Surface;
        for c in r.cmds.iter_mut() {
            if let Op::Blit { src, .. } = &mut c.op {
                if *src == dst {
                    *src = ptr;
                }
            }
        }
        r.snapshots.push(copy);
    });
}

pub(crate) unsafe fn record_blit(src: *mut Surface, src_rect: Option<Rect>, dst: *mut Surface, at: Option<Rect>, scaled: bool) {
    push(dst, || Op::Blit { src, state: (*src).state(), src_rect, at: at.unwrap_or(Rect::new(0, 0, (*dst).w, (*dst).h)), scaled });
}

pub(crate) fn record_fill(dst: *mut Surface, rect: Option<Rect>, color: u32) {
    push(dst, || Op::Fill { rect, color });
}

/// Draws `cur` onto `screen` with each owned write placed `alpha` (0 to 1) of the way from its position in `prev`.
pub unsafe fn render(screen: *mut Surface, prev: &Draw, cur: &Draw, alpha: f32) {
    let before: HashMap<(Owner, u16), (i32, i32, *mut Surface)> = prev
        .cmds
        .iter()
        .filter(|c| c.owner != Owner::None)
        .filter_map(|c| match c.op {
            Op::Blit { src, at, .. } => Some(((c.owner, c.seq), (at.x, at.y, src))),
            Op::Fill { .. } => None,
        })
        .collect();
    let s = &mut *screen;
    for c in &cur.cmds {
        let mut view = PixelsMut { w: s.w, h: s.h, pitch: s.w as usize, pixels: &mut s.pixels, clip: c.clip, alpha: s.alpha };
        match c.op {
            Op::Fill { rect, color } => soft_surface::fill_rect(&mut view, rect.as_ref(), color),
            Op::Blit { src, state, src_rect, mut at, scaled } => {
                if let Some(&(px, py, psrc)) = before.get(&(c.owner, c.seq)) {
                    if psrc == src && (at.x - px).abs() <= MAX_SLIDE && (at.y - py).abs() <= MAX_SLIDE {
                        at.x = px + ((at.x - px) as f32 * alpha).round() as i32;
                        at.y = py + ((at.y - py) as f32 * alpha).round() as i32;
                    }
                }
                if src == screen {
                    continue;
                }
                let src = &*src;
                let mut out = at;
                if scaled {
                    soft_surface::blit_scaled(&src.view(), &state, src_rect.as_ref(), &mut view, Some(&mut out));
                } else {
                    soft_surface::blit(&src.view(), &state, src_rect.as_ref(), &mut view, Some(&mut out));
                }
            }
        }
    }
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
