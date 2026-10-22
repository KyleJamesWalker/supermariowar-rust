//! On-screen touch controls for Android (`SMW_TOUCH=1`), drawn beside the 4:3 picture. Like
//! web/touch.js they press player 1's default keys, so recordings hold ordinary key input;
//! a still tap off the controls is a menu tap.

use crate::common::keys::*;
use crate::game::Game;
use smw_platform::{Input, InputEvent, Overlay, TouchPhase, OverlayImage, OverlayQuad, SCREEN_H, SCREEN_W};
use std::sync::Arc;
use crate::common::path::get_home_directory;

/// A key the touch controls press; Jump presses Up.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Key {
    Left,
    Right,
    Up,
    Down,
    Run,
    Item,
    Start,
    Back,
}

const KEYS: [Key; 8] = [Key::Left, Key::Right, Key::Up, Key::Down, Key::Run, Key::Item, Key::Start, Key::Back];

impl Key {
    fn bit(self) -> u8 {
        1 << self as u8
    }

    /// The right keyboard set's default key for it.
    fn sym(self) -> i32 {
        (match self {
            Key::Left => SDLK_LEFT,
            Key::Right => SDLK_RIGHT,
            Key::Up => SDLK_UP,
            Key::Down => SDLK_DOWN,
            Key::Run => SDLK_RCTRL,
            Key::Item => SDLK_RSHIFT,
            Key::Start => SDLK_RETURN,
            Key::Back => SDLK_ESCAPE,
        }) as i32
    }
}

/// The left control: a fixed D-pad, or a stick that appears under the thumb.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Dpad,
    Stick,
}

/// The control a finger went down on; the finger keeps it until it lifts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Zone {
    Dpad,
    Stick,
    Keys,
    Toggle,
}

#[derive(Clone, Copy, Debug)]
struct Finger {
    id: i64,
    zone: Zone,
    /// The keys the finger holds, as `Key::bit`s.
    keys: u8,
    /// Where it went down; for a stick, the base, which follows the thumb.
    origin: (f32, f32),
    pos: (f32, f32),
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Circle {
    x: f32,
    y: f32,
    r: f32,
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Positions in window pixels; `dp` is pixels per Android dp. Sizes follow web/touch.css.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
struct Layout {
    w: f32,
    h: f32,
    dp: f32,
    view: Rect,
    /// Where the floating stick's base may sit: the left bar, clear of the display cutout.
    stick_area: Rect,
    dpad: Circle,
    jump: Circle,
    run: Circle,
    item: Circle,
    back: Rect,
    toggle: Rect,
    start: Rect,
}

/// `want` shrunk to `room`, unless that leaves under 60% of it: then it overlaps the picture.
fn fit(want: f32, room: f32) -> f32 {
    if room >= want * 0.6 {
        want.min(room)
    } else {
        want
    }
}

/// `cutouts` are the display cutouts' bounding boxes; a control that would sit on one moves sideways off it.
fn layout(w: f32, h: f32, dp: f32, cutouts: &[Rect]) -> Layout {
    let scale = (w / 640.0).min(h / 480.0);
    let view = Rect { x: (w - 640.0 * scale) / 2.0, y: (h - 480.0 * scale) / 2.0, w: 640.0 * scale, h: 480.0 * scale };
    let edge = 12.0 * dp;
    let room = view.x - 2.0 * edge;
    let (x0, y0, x1, y1) = (edge, edge, w - edge, h - edge);
    let d = fit((160.0 * dp).min(0.42 * w.min(h)), room);
    let btn = fit((0.17 * w.min(h)).clamp(56.0 * dp, 68.0 * dp), (room - 10.0 * dp) / 2.0);
    let block = 2.0 * btn + 10.0 * dp;
    let (kw, kh) = (fit(68.0 * dp, room), 56.0 * dp);
    let left = x1 - block + btn / 2.0;
    let mut l = Layout {
        w,
        h,
        dp,
        view,
        stick_area: Rect { x: 0.0, y: 0.0, w: view.x, h },
        dpad: Circle { x: x0 + d / 2.0, y: y1 - d / 2.0, r: d / 2.0 },
        jump: Circle { x: x1 - btn / 2.0, y: y1 - btn, r: btn / 2.0 },
        run: Circle { x: left, y: y1 - btn / 2.0, r: btn / 2.0 },
        item: Circle { x: left, y: y1 - block + btn / 2.0, r: btn / 2.0 },
        back: Rect { x: x0, y: y0, w: kw, h: kh },
        toggle: Rect { x: x0, y: y0 + kh + 8.0 * dp, w: kw, h: 44.0 * dp },
        start: Rect { x: x1 - kw, y: y0, w: kw, h: kh },
    };
    for c in cutouts {
        let side = |x: f32, half: f32| if x < w / 2.0 { c.x + c.w + edge + half } else { c.x - edge - half };
        for circle in [&mut l.dpad, &mut l.jump, &mut l.run, &mut l.item] {
            let (nx, ny) = (circle.x.clamp(c.x, c.x + c.w), circle.y.clamp(c.y, c.y + c.h));
            if (circle.x - nx).hypot(circle.y - ny) < circle.r + edge {
                circle.x = side(circle.x, circle.r);
            }
        }
        for r in [&mut l.back, &mut l.toggle, &mut l.start] {
            if c.x - edge < r.x + r.w && r.x < c.x + c.w + edge && c.y - edge < r.y + r.h && r.y < c.y + c.h + edge {
                r.x = side(r.x + r.w / 2.0, r.w / 2.0) - r.w / 2.0;
            }
        }
    }
    l
}

/// 8-way, with diagonals overlapping the straight directions as in web/touch.js.
fn direction_keys(dx: f32, dy: f32, dead: f32) -> u8 {
    if dx.hypot(dy) < dead {
        return 0;
    }
    let angle = dy.atan2(dx).to_degrees();
    let mut keys = 0;
    if angle.abs() < 67.5 {
        keys |= Key::Right.bit();
    }
    if angle.abs() > 112.5 {
        keys |= Key::Left.bit();
    }
    if angle > 22.5 && angle < 157.5 {
        keys |= Key::Down.bit();
    }
    if angle < -22.5 && angle > -157.5 {
        keys |= Key::Up.bit();
    }
    keys
}

impl Layout {
    fn stick_radius(&self) -> f32 {
        40.0 * self.dp
    }

    fn stick_base_radius(&self) -> f32 {
        self.stick_radius() + 10.0 * self.dp
    }

    /// `p` moved so a circle of radius `r` around it stays inside the stick area (centred if it cannot fit).
    fn in_stick_area(&self, p: (f32, f32), r: f32) -> (f32, f32) {
        let a = self.stick_area;
        let clamp = |v: f32, lo: f32, hi: f32| if lo <= hi { v.clamp(lo, hi) } else { (lo + hi) / 2.0 };
        (clamp(p.0, a.x + r, a.x + a.w - r), clamp(p.1, a.y + r, a.y + a.h - r))
    }

    /// Where the knob is drawn: the thumb, kept on the base and inside the stick area.
    fn knob(&self, finger: &Finger) -> (f32, f32) {
        let (dx, dy) = (finger.pos.0 - finger.origin.0, finger.pos.1 - finger.origin.1);
        let dist = dx.hypot(dy);
        let scale = if dist > self.stick_radius() { self.stick_radius() / dist } else { 1.0 };
        self.in_stick_area((finger.origin.0 + dx * scale, finger.origin.1 + dy * scale), 26.0 * self.dp)
    }

    fn buttons(&self) -> [(Key, Circle); 3] {
        [(Key::Up, self.jump), (Key::Run, self.run), (Key::Item, self.item)]
    }

    fn button_keys(&self, x: f32, y: f32) -> u8 {
        let mut keys = 0;
        for (key, c) in self.buttons() {
            if (x - c.x).hypot(y - c.y) < c.r * 1.2 {
                keys |= key.bit();
            }
        }
        for (key, r) in [(Key::Start, self.start), (Key::Back, self.back)] {
            let reach = r.w.max(r.h) / 2.0 * 1.2;
            if r.contains(x, y) || (x - (r.x + r.w / 2.0)).hypot(y - (r.y + r.h / 2.0)) < reach {
                keys |= key.bit();
            }
        }
        keys
    }

    /// (x, y) in the game's 640x480 coordinates, if it is on the picture.
    fn logical(&self, x: f32, y: f32) -> Option<(i16, i16)> {
        let lx = ((x - self.view.x) * SCREEN_W as f32 / self.view.w).floor();
        let ly = ((y - self.view.y) * SCREEN_H as f32 / self.view.h).floor();
        ((0.0..SCREEN_W as f32).contains(&lx) && (0.0..SCREEN_H as f32).contains(&ly)).then_some((lx as i16, ly as i16))
    }

    fn zone(&self, mode: Mode, x: f32, y: f32) -> Option<Zone> {
        if self.toggle.contains(x, y) {
            return Some(Zone::Toggle);
        }
        if self.button_keys(x, y) != 0 {
            return Some(Zone::Keys);
        }
        match mode {
            Mode::Dpad if (x - self.dpad.x).hypot(y - self.dpad.y) < self.dpad.r * 1.3 => Some(Zone::Dpad),
            Mode::Stick if x < self.view.x.max(self.dpad.x + self.dpad.r) && y > self.toggle.y + self.toggle.h => Some(Zone::Stick),
            _ => None,
        }
    }

    /// The keys `finger` holds at (x, y). A stick's base follows the thumb past its radius.
    fn track(&self, finger: &mut Finger, x: f32, y: f32) -> u8 {
        finger.pos = (x, y);
        match finger.zone {
            Zone::Dpad => direction_keys(x - self.dpad.x, y - self.dpad.y, self.dpad.r * 0.24),
            Zone::Stick => {
                let (dx, dy) = (x - finger.origin.0, y - finger.origin.1);
                let dist = dx.hypot(dy);
                let radius = self.stick_radius();
                if dist > radius {
                    finger.origin = (x - dx / dist * radius, y - dy / dist * radius);
                }
                finger.origin = self.in_stick_area(finger.origin, self.stick_base_radius());
                direction_keys(x - finger.origin.0, y - finger.origin.1, 12.0 * self.dp)
            }
            Zone::Keys => self.button_keys(x, y),
            Zone::Toggle => 0,
        }
    }
}

struct Controls {
    mode: Mode,
    layout: Layout,
    fingers: Vec<Finger>,
    /// How many fingers hold each key, by `Key`.
    held: [u8; 8],
    /// Key presses and releases waiting for `flush`.
    queue: Vec<(Key, bool)>,
    /// Fingers that went down outside the controls, by where they went down.
    presses: Vec<(i64, (f32, f32))>,
    /// Menu taps in 640x480 coordinates waiting for `flush`.
    taps: Vec<(i16, i16)>,
}

impl Controls {
    fn new(mode: Mode, layout: Layout) -> Self {
        Controls { mode, layout, fingers: Vec::new(), held: [0; 8], queue: Vec::new(), presses: Vec::new(), taps: Vec::new() }
    }

    /// Sets finger `i`'s keys, queueing a press or release only when no other finger holds that key.
    fn set_keys(&mut self, i: usize, keys: u8) {
        let old = self.fingers[i].keys;
        for key in KEYS.into_iter().filter(|k| old & k.bit() != 0 && keys & k.bit() == 0) {
            self.held[key as usize] -= 1;
            if self.held[key as usize] == 0 {
                self.queue.push((key, false));
            }
        }
        for key in KEYS.into_iter().filter(|k| old & k.bit() == 0 && keys & k.bit() != 0) {
            self.held[key as usize] += 1;
            if self.held[key as usize] == 1 {
                self.queue.push((key, true));
            }
        }
        self.fingers[i].keys = keys;
    }

    fn down(&mut self, g: &mut Game, id: i64, x: f32, y: f32) {
        self.presses.retain(|p| p.0 != id);
        self.up(g, id, x, y);
        let Some(zone) = self.layout.zone(self.mode, x, y) else {
            self.presses.push((id, (x, y)));
            return;
        };
        self.fingers.push(Finger { id, zone, keys: 0, origin: (x, y), pos: (x, y) });
        self.moved(id, x, y);
    }

    fn moved(&mut self, id: i64, x: f32, y: f32) {
        let slop = 12.0 * self.layout.dp;
        self.presses.retain(|&(p, (px, py))| p != id || (x - px).hypot(y - py) <= slop);
        let Some(i) = self.fingers.iter().position(|f| f.id == id) else { return };
        let layout = self.layout;
        let keys = layout.track(&mut self.fingers[i], x, y);
        self.set_keys(i, keys);
    }

    fn up(&mut self, g: &mut Game, id: i64, x: f32, y: f32) {
        if let Some(i) = self.presses.iter().position(|p| p.0 == id) {
            self.presses.remove(i);
            self.taps.extend(self.layout.logical(x, y));
        }
        let Some(i) = self.fingers.iter().position(|f| f.id == id) else { return };
        if self.fingers[i].zone == Zone::Toggle && self.layout.toggle.contains(x, y) {
            self.mode = if self.mode == Mode::Dpad { Mode::Stick } else { Mode::Dpad };
            save_mode(g, self.mode);
        }
        self.set_keys(i, 0);
        self.fingers.remove(i);
    }

    fn release_all(&mut self) {
        self.presses.clear();
        while !self.fingers.is_empty() {
            self.set_keys(self.fingers.len() - 1, 0);
            self.fingers.pop();
        }
    }

    fn lit(&self, zone: Zone) -> u8 {
        self.fingers.iter().filter(|f| f.zone == zone).fold(0, |k, f| k | f.keys)
    }
}

struct Touch {
    controls: Controls,
    /// Off after key or pad input, on again at the next touch.
    visible: bool,
    /// The overlay's fade, 0 to 1.
    alpha: f32,
    cutouts: Vec<Rect>,
    images: Option<Arc<[OverlayImage]>>,
    /// Bumped whenever `images` is rebuilt.
    generation: u64,
    /// The last overlay, kept so each present reuses its quad buffer.
    frame: Option<Overlay>,
}

/// The Android touch controls' state inside `Game`.
pub struct TouchState {
    /// The live controls; None without `SMW_TOUCH` and in replays.
    touch: Option<Touch>,
    session_on: bool,
    first: bool,
    debug: bool,
}

impl TouchState {
    pub const fn new() -> Self {
        TouchState { touch: None, session_on: false, first: false, debug: false }
    }
}

fn st(g: &mut Game) -> &mut TouchState {
    &mut g.touch
}
static CUTOUTS: std::sync::Mutex<Vec<Rect>> = std::sync::Mutex::new(Vec::new());

/// The display cutouts' bounding boxes in window pixels (left, top, right, bottom each), from MainActivity.
pub fn set_cutouts(boxes: &[[i32; 4]]) {
    let rects = boxes.iter().filter(|b| b[2] > b[0] && b[3] > b[1]).map(|b| Rect { x: b[0] as f32, y: b[1] as f32, w: (b[2] - b[0]) as f32, h: (b[3] - b[1]) as f32 });
    *CUTOUTS.lock().unwrap_or_else(|e| e.into_inner()) = rects.collect();
}
/// File in the settings directory holding the left control's mode: `dpad` or `stick`.
const MODE_FILE: &str = "touch_controls.txt";

fn load_mode(g: &mut Game) -> Mode {
    match std::fs::read_to_string(get_home_directory(g) + MODE_FILE) {
        Ok(text) if text.trim() == "stick" => Mode::Stick,
        _ => Mode::Dpad,
    }
}

fn save_mode(g: &mut Game, mode: Mode) {
    if !cfg!(test) {
        let _ = std::fs::write(get_home_directory(g) + MODE_FILE, if mode == Mode::Stick { "stick\n" } else { "dpad\n" });
    }
}

/// After `init_joysticks`, which `touch_first` depends on.
pub fn init(g: &mut Game) {
    let mut gp = g.ptr();
    let pads = !gp.devices.is_empty();
    let live = std::env::var_os("SMW_TOUCH").is_some() && !crate::smw::harness::replaying(g);
    st(gp.get()).debug = std::env::var_os("SMW_DEBUG_INPUT").is_some();
    st(gp.get()).session_on = live || crate::smw::harness::replay_touch(g);
    st(gp.get()).first = st(gp.get()).session_on && !pads;
    if live {
        crate::smw::harness::record_touch(g);
        st(gp.get()).touch = Some(Touch { controls: Controls::new(load_mode(g), Layout::default()), visible: !pads, alpha: if pads { 0.0 } else { 1.0 }, cutouts: Vec::new(), images: None, generation: 0, frame: None });
        println!("[touch] on, {}", if pads { "hidden while a pad is connected" } else { "player 1" });
    }
}

/// Whether this session has the touch controls, live or in the recording being replayed.
pub fn session(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    st(gp.get()).session_on
}

/// Whether the touch keys (the right keyboard set) go to player 1 ahead of the pads: touch is on and no pad was
/// connected at launch. Replays read it from the recording, so it must not depend on anything later.
pub fn touch_first(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    st(gp.get()).first
}

/// False: drop the input. `input::poll` calls it on live input before the recorder sees it.
pub fn filter(g: &mut Game, input: &Input) -> bool {
    let mut gp = g.ptr();
    let Some(t) = st(gp.get()).touch.as_mut() else { return true };
    if let InputEvent::Touch { finger, phase, x, y, .. } = input.event {
        let l = t.controls.layout;
        if l.w <= 0.0 {
            return false;
        }
        let (x, y) = (x * l.w, y * l.h);
        if st(gp.get()).debug {
            println!("[touch] finger {} {:?} at {:.0},{:.0} of {}x{}", finger, phase, x, y, l.w, l.h);
        }
        match phase {
            TouchPhase::Down => {
                t.visible = true;
                t.controls.down(g, finger, x, y);
            }
            TouchPhase::Motion => t.controls.moved(finger, x, y),
            TouchPhase::Up => t.controls.up(g, finger, x, y),
        }
        return false;
    }
    // A key other than Android's Back, a pad button or a stick past half way hides the controls.
    let other_input = match input.event {
        InputEvent::Key { key, down: true, .. } => key.0 != SDLK_AC_BACK,
        InputEvent::PadButton { down: true, .. } => true,
        InputEvent::PadHat { value, .. } => value != 0,
        InputEvent::PadAxis { value, .. } => (value as i32).abs() > 16384,
        _ => false,
    };
    if other_input && t.visible {
        t.visible = false;
        t.controls.release_all();
    }
    true
}

/// Queues the keys and taps the fingers made.
pub fn flush(g: &mut Game) {
    let mut gp = g.ptr();
    let Some(t) = st(gp.get()).touch.as_mut() else { return };
    if !crate::services::services(gp.get()).keyboard.has_focus() {
        t.controls.release_all();
    }
    for (key, down) in std::mem::take(&mut t.controls.queue) {
        let scancode = crate::services::services(gp.get()).keyboard.scancode_from_key(key.sym());
        let event = InputEvent::Key { key: smw_platform::Keycode(key.sym()), scancode, mods: 0, down, repeat: false, window: 0 };
        crate::smw::input::push_touch(gp.get(), Input { timestamp: 0, event });
    }
    for (x, y) in std::mem::take(&mut t.controls.taps) {
        crate::smw::input::push_touch(gp.get(), Input { timestamp: 0, event: InputEvent::Tap { x, y } });
    }
}

/// What to draw over the presented frame, outside the game's 640x480 logical viewport.
pub fn overlay(g: &mut Game) -> Option<&Overlay> {
    let mut gp = g.ptr();
    let t = st(gp.get()).touch.as_mut()?;
    let video = &crate::services::services(gp.get()).video;
    let (w, h) = video.output_size();
    if w <= 0 || h <= 0 {
        return None;
    }
    let cutouts = CUTOUTS.lock().unwrap_or_else(|e| e.into_inner());
    if t.controls.layout.w != w as f32 || t.controls.layout.h != h as f32 || t.cutouts != *cutouts {
        t.cutouts = cutouts.clone();
        let dp = match video.display_dpi() {
            Some(dpi) => dpi / 160.0,
            None => h as f32 / 400.0,
        };
        t.controls.release_all();
        t.controls.layout = layout(w as f32, h as f32, dp, &cutouts);
        t.images = Some(build_images(&t.controls.layout).into());
        t.generation += 1;
    }
    t.alpha = if t.visible { (t.alpha + 0.2).min(1.0) } else { (t.alpha - 0.08).max(0.0) };
    if t.alpha <= 0.0 {
        return None;
    }
    let images = t.images.clone()?;
    let mut buf = t.frame.take().map(|f| f.quads).unwrap_or_default();
    quads(t, &mut buf);
    t.frame = Some(Overlay { generation: t.generation, images, quads: buf });
    t.frame.as_ref()
}

/// Indexes into the images `build_images` makes; `ARM` is four images, one per direction.
const DPAD_BASE: usize = 0;
const ARM: usize = 1;
const DPAD_CENTER: usize = 5;
const JUMP: usize = 6;
const RUN: usize = 7;
const ITEM: usize = 8;
const BUTTON_PRESSED: usize = 9;
const BACK: usize = 10;
const START: usize = 11;
const KEY_PRESSED: usize = 12;
const TOGGLE_DPAD: usize = 13;
const TOGGLE_STICK: usize = 14;
const STICK_BASE: usize = 15;
const STICK_KNOB: usize = 16;
const TEXTURES: usize = 17;

/// The overlay's quads in window pixels for the current mode and held keys.
fn quads(t: &Touch, out: &mut Vec<OverlayQuad>) {
    let c = &t.controls;
    let l = c.layout;
    out.clear();
    let out = std::cell::RefCell::new(out);
    let tint = std::cell::Cell::new([255u8; 3]);
    let put = |index: usize, x: f32, y: f32, w: f32, h: f32, alpha: f32| {
        out.borrow_mut().push(OverlayQuad {
            image: index as u16,
            x: x.round() as i32,
            y: y.round() as i32,
            w: w.round() as i32,
            h: h.round() as i32,
            alpha: (alpha * t.alpha * 255.0) as u8,
            color: tint.replace([255; 3]),
        });
    };
    let circle = |index: usize, c: Circle, alpha: f32| put(index, c.x - c.r, c.y - c.r, 2.0 * c.r, 2.0 * c.r, alpha);
    let rect = |index: usize, r: Rect, alpha: f32| put(index, r.x, r.y, r.w, r.h, alpha);

    match c.mode {
        Mode::Dpad => {
            let d = l.dpad;
            circle(DPAD_BASE, d, 1.0);
            let lit = c.lit(Zone::Dpad);
            let cell = 0.66 * d.r;
            let arm = 0.68 * d.r;
            let (x0, y0) = (d.x - d.r, d.y - d.r);
            for (i, (key, (cx, cy))) in [(Key::Up, (1.0, 0.0)), (Key::Down, (1.0, 2.0)), (Key::Left, (0.0, 1.0)), (Key::Right, (2.0, 1.0))].into_iter().enumerate() {
                if lit & key.bit() != 0 {
                    tint.set([255, 204, 51]);
                }
                put(ARM + i, x0 + cx * cell, y0 + cy * cell, arm, arm, 1.0);
            }
            put(DPAD_CENTER, x0 + cell, y0 + cell, arm, arm, 1.0);
        }
        Mode::Stick => {
            let base = |x: f32, y: f32, alpha: f32| circle(STICK_BASE, Circle { x, y, r: l.stick_base_radius() }, alpha);
            let mut any = false;
            for f in c.fingers.iter().filter(|f| f.zone == Zone::Stick) {
                any = true;
                base(f.origin.0, f.origin.1, 1.0);
                let (kx, ky) = l.knob(f);
                circle(STICK_KNOB, Circle { x: kx, y: ky, r: 26.0 * l.dp }, 1.0);
            }
            if !any {
                base(l.dpad.x, l.dpad.y, 0.5);
            }
        }
    }

    let lit = c.lit(Zone::Keys);
    for (index, (key, b)) in [JUMP, RUN, ITEM].into_iter().zip(l.buttons()) {
        circle(index, b, 1.0);
        if lit & key.bit() != 0 {
            circle(BUTTON_PRESSED, b, 1.0);
        }
    }
    for (index, key, r) in [(BACK, Key::Back, l.back), (START, Key::Start, l.start)] {
        rect(index, r, 1.0);
        if lit & key.bit() != 0 {
            rect(KEY_PRESSED, r, 1.0);
        }
    }
    rect(if c.mode == Mode::Dpad { TOGGLE_DPAD } else { TOGGLE_STICK }, l.toggle, 1.0);
}

type Rgba = [f32; 4];
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

/// Straight-alpha RGBA pixels, rasterised once per window size.
struct Canvas {
    w: usize,
    h: usize,
    px: Vec<Rgba>,
}

impl Canvas {
    fn new(w: f32, h: f32) -> Self {
        let (w, h) = (w.round().max(1.0) as usize, h.round().max(1.0) as usize);
        Canvas { w, h, px: vec![[0.0; 4]; w * h] }
    }

    fn blend(&mut self, x: usize, y: usize, color: [f32; 3], cover: f32) {
        let d = &mut self.px[y * self.w + x];
        let out = cover + d[3] * (1.0 - cover);
        for i in 0..3 {
            d[i] = (color[i] * cover + d[i] * d[3] * (1.0 - cover)) / out;
        }
        d[3] = out;
    }

    /// `sd` is a signed distance, negative inside; edges are antialiased over one pixel.
    fn paint(&mut self, color: [f32; 3], a: f32, sd: impl Fn(f32, f32) -> f32) {
        for y in 0..self.h {
            for x in 0..self.w {
                let cover = (0.5 - sd(x as f32 + 0.5, y as f32 + 0.5)).clamp(0.0, 1.0) * a;
                if cover > 0.0 {
                    self.blend(x, y, color, cover);
                }
            }
        }
    }

    fn circle(&mut self, fill: Rgba, border: f32, border_a: f32) {
        let (cx, cy, r) = (self.w as f32 / 2.0, self.h as f32 / 2.0, self.w.min(self.h) as f32 / 2.0 - 1.0);
        self.paint([fill[0], fill[1], fill[2]], fill[3], |x, y| (x - cx).hypot(y - cy) - r);
        if border > 0.0 {
            self.paint(WHITE, border_a, |x, y| ((x - cx).hypot(y - cy) - (r - border / 2.0)).abs() - border / 2.0);
        }
    }

    fn rounded(&mut self, radius: f32, fill: Rgba, border: f32, border_a: f32) {
        let (w, h) = (self.w as f32, self.h as f32);
        let sd = move |x: f32, y: f32| {
            let qx = (x - w / 2.0).abs() - (w / 2.0 - radius);
            let qy = (y - h / 2.0).abs() - (h / 2.0 - radius);
            qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - radius
        };
        self.paint([fill[0], fill[1], fill[2]], fill[3], sd);
        if border > 0.0 {
            self.paint(WHITE, border_a, move |x, y| (sd(x, y) + border / 2.0).abs() - border / 2.0);
        }
    }

    fn triangle(&mut self, p: [(f32, f32); 3], a: f32) {
        let edge = |(ax, ay): (f32, f32), (bx, by): (f32, f32), x: f32, y: f32| (bx - ax) * (y - ay) - (by - ay) * (x - ax);
        for y in 0..self.h {
            for x in 0..self.w {
                let mut n = 0;
                for s in 0..16 {
                    let (px, py) = (x as f32 + ((s % 4) as f32 + 0.5) / 4.0, y as f32 + ((s / 4) as f32 + 0.5) / 4.0);
                    let e = [edge(p[0], p[1], px, py), edge(p[1], p[2], px, py), edge(p[2], p[0], px, py)];
                    if e.iter().all(|&v| v >= 0.0) || e.iter().all(|&v| v <= 0.0) {
                        n += 1;
                    }
                }
                if n > 0 {
                    self.blend(x, y, WHITE, n as f32 / 16.0 * a);
                }
            }
        }
    }

    fn label(&mut self, text: &str, size: f32, a: f32) {
        let s = (size / 7.0).round().max(1.0);
        let width = text.len() as f32 * 6.0 * s - s;
        let (x0, y0) = ((self.w as f32 - width) / 2.0, (self.h as f32 - 7.0 * s) / 2.0);
        for (i, ch) in text.bytes().enumerate() {
            let Some(rows) = glyph(ch) else { continue };
            for (row, bits) in rows.iter().enumerate() {
                for col in 0..5 {
                    if bits & (0x10 >> col) != 0 {
                        let (gx, gy) = (x0 + (i as f32 * 6.0 + col as f32) * s, y0 + row as f32 * s);
                        self.paint(WHITE, a, move |x, y| (gx - x).max(x - gx - s).max(gy - y).max(y - gy - s));
                    }
                }
            }
        }
    }

    fn image(&self) -> OverlayImage {
        let rgba = self.px.iter().flat_map(|p| p.map(|v| (v * 255.0).round() as u8)).collect();
        OverlayImage { w: self.w as u32, h: self.h as u32, rgba }
    }
}

/// 5x7 glyphs for the labels' letters.
fn glyph(ch: u8) -> Option<[u8; 7]> {
    Some(match ch {
        b'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        b'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        b'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        b'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        b'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        b'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        b'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        b'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        b'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        b'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        b'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        b'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        b'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        b'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        b'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        _ => return None,
    })
}

/// Rasterises every overlay image for `l`, in the order of the index constants.
fn build_images(l: &Layout) -> Vec<OverlayImage> {
    let dp = l.dp;
    let border = 2.0 * dp;
    let mut canvases = Vec::with_capacity(TEXTURES);

    let d = 2.0 * l.dpad.r;
    let mut base = Canvas::new(d, d);
    base.circle([1.0, 1.0, 1.0, 0.06], border, 0.12);
    canvases.push(base);
    let arm = 0.34 * d;
    for dir in 0..4 {
        let mut c = Canvas::new(arm, arm);
        c.rounded(8.0 * dp, [1.0, 1.0, 1.0, 0.22], border, 0.3);
        let (m, a) = (c.w as f32 / 2.0, c.w as f32 * 0.18);
        let tri = match dir {
            0 => [(m - a, m + a / 2.0), (m + a, m + a / 2.0), (m, m - a)],
            1 => [(m - a, m - a / 2.0), (m + a, m - a / 2.0), (m, m + a)],
            2 => [(m + a / 2.0, m - a), (m + a / 2.0, m + a), (m - a, m)],
            _ => [(m - a / 2.0, m - a), (m - a / 2.0, m + a), (m + a, m)],
        };
        c.triangle(tri, 0.7);
        canvases.push(c);
    }
    let mut center = Canvas::new(arm, arm);
    center.paint(WHITE, 0.22, |_, _| -1.0);
    canvases.push(center);

    let b = 2.0 * l.jump.r;
    for (label, fill) in [("JUMP", [0.89, 0.0, 0.17, 0.35]), ("RUN", [1.0, 0.8, 0.2, 0.25]), ("ITEM", [0.24, 0.55, 1.0, 0.3])] {
        let mut c = Canvas::new(b, b);
        c.circle(fill, border, 0.35);
        c.label(label, 12.0 * dp, 0.85);
        canvases.push(c);
    }
    let mut pressed = Canvas::new(b, b);
    pressed.circle([1.0, 1.0, 1.0, 0.45], 0.0, 0.0);
    canvases.push(pressed);

    for (label, r, fill) in [("BACK", l.back, 0.14), ("START", l.start, 0.14), ("", l.back, 0.45), ("DPAD", l.toggle, 0.14), ("STICK", l.toggle, 0.14)] {
        let mut c = Canvas::new(r.w, r.h);
        c.rounded(12.0 * dp, [1.0, 1.0, 1.0, fill], border, 0.3);
        c.label(label, 11.0 * dp, 0.85);
        canvases.push(c);
    }

    let r = l.stick_base_radius();
    let mut stick = Canvas::new(2.0 * r, 2.0 * r);
    stick.circle([1.0, 1.0, 1.0, 0.08], border, 0.35);
    canvases.push(stick);
    let k = 52.0 * dp;
    let mut knob = Canvas::new(k, k);
    knob.circle([1.0, 1.0, 1.0, 0.4], border, 0.5);
    canvases.push(knob);

    canvases.iter().map(Canvas::image).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: Key = Key::Left;
    const R: Key = Key::Right;
    const U: Key = Key::Up;
    const D: Key = Key::Down;

    fn bits(keys: &[Key]) -> u8 {
        keys.iter().fold(0, |b, k| b | k.bit())
    }

    /// Pixel 9 Pro XL, landscape.
    fn pixel() -> Layout {
        layout(2244.0, 1008.0, 3.0, &[])
    }

    #[test]
    fn directions_are_8_way_with_a_dead_zone() {
        assert_eq!(direction_keys(1.0, 0.0, 5.0), 0);
        assert_eq!(direction_keys(10.0, 0.0, 5.0), bits(&[R]));
        assert_eq!(direction_keys(-10.0, 0.0, 5.0), bits(&[L]));
        assert_eq!(direction_keys(0.0, -10.0, 5.0), bits(&[U]));
        assert_eq!(direction_keys(0.0, 10.0, 5.0), bits(&[D]));
        assert_eq!(direction_keys(10.0, -10.0, 5.0), bits(&[U, R]));
        assert_eq!(direction_keys(-10.0, 10.0, 5.0), bits(&[D, L]));
        assert_eq!(direction_keys(10.0, -3.0, 5.0), bits(&[R]));
        assert_eq!(direction_keys(10.0, -5.0, 5.0), bits(&[U, R]));
    }

    #[test]
    fn the_overlay_draws_each_control_once_and_tints_the_held_arm() {
        let g = crate::game::Game::create();
        let l = pixel();
        let mut t = Touch { controls: Controls::new(Mode::Dpad, l), visible: true, alpha: 1.0, cutouts: Vec::new(), images: None, generation: 0, frame: None };
        let images = build_images(&l);
        assert_eq!(images.len(), TEXTURES);
        assert!(images.iter().all(|i| i.rgba.len() == (i.w * i.h * 4) as usize));
        let mut idle = Vec::new();
        quads(&t, &mut idle);
        assert_eq!(idle.iter().map(|q| q.image as usize).collect::<Vec<_>>(), [DPAD_BASE, ARM, ARM + 1, ARM + 2, ARM + 3, DPAD_CENTER, JUMP, RUN, ITEM, BACK, START, TOGGLE_DPAD]);
        assert!(idle.iter().all(|q| q.alpha == 255 && q.color == [255; 3]));
        t.controls.down(g, 1, l.dpad.x + l.dpad.r * 0.8, l.dpad.y);
        let mut held = Vec::new();
        quads(&t, &mut held);
        assert_eq!(held.iter().map(|q| q.color).collect::<Vec<_>>()[1..5], [[255; 3], [255; 3], [255; 3], [255, 204, 51]]);
    }

    #[test]
    fn controls_sit_beside_the_picture() {
        let l = pixel();
        assert_eq!((l.view.x, l.view.w), (450.0, 1344.0));
        assert!(l.dpad.x + l.dpad.r <= l.view.x);
        for (_, c) in l.buttons() {
            assert!(c.x - c.r >= l.view.x + l.view.w, "{:?}", c);
            assert!(c.r * 2.0 >= 56.0 * l.dp);
        }
        for r in [l.back, l.toggle] {
            assert!(r.x + r.w <= l.view.x);
        }
        assert!(l.start.x >= l.view.x + l.view.w);
    }

    #[test]
    fn multi_touch_holds_a_direction_run_and_jump() {
        let g = crate::game::Game::create();
        let l = pixel();
        let mut c = Controls::new(Mode::Dpad, l);
        c.down(g, 1, l.dpad.x + l.dpad.r * 0.8, l.dpad.y);
        c.down(g, 2, l.run.x, l.run.y);
        c.down(g, 3, l.jump.x, l.jump.y);
        assert_eq!(c.queue, [(R, true), (Key::Run, true), (U, true)]);
        c.queue.clear();
        // Up is already held by jump, so sliding to up-right presses nothing new.
        c.moved(1, l.dpad.x + l.dpad.r * 0.6, l.dpad.y - l.dpad.r * 0.6);
        assert!(c.queue.is_empty());
        c.up(g, 3, l.jump.x, l.jump.y);
        assert!(c.queue.is_empty());
        c.up(g, 1, 0.0, 0.0);
        assert_eq!(c.queue, [(R, false), (U, false)]);
        c.queue.clear();
        c.release_all();
        assert_eq!(c.queue, [(Key::Run, false)]);
        assert!(c.fingers.is_empty() && c.held == [0; 8]);
    }

    #[test]
    fn floating_stick_spawns_under_the_thumb_and_follows_it() {
        let g = crate::game::Game::create();
        let l = pixel();
        let mut c = Controls::new(Mode::Stick, l);
        let (x, y) = (155.0, 600.0);
        c.down(g, 7, x, y);
        assert!(c.queue.is_empty());
        c.moved(7, x + 6.0 * l.dp, y);
        assert!(c.queue.is_empty());
        c.moved(7, x + 20.0 * l.dp, y);
        assert_eq!(c.queue, [(R, true)]);
        c.queue.clear();
        c.moved(7, x + 80.0 * l.dp, y);
        let origin = c.fingers[0].origin;
        assert!((origin.0 - (x + 40.0 * l.dp)).abs() < 0.01);
        c.moved(7, x + 20.0 * l.dp, y);
        assert_eq!(c.queue, [(R, false), (L, true)]);
        c.queue.clear();
        c.up(g, 7, 0.0, 0.0);
        assert_eq!(c.queue, [(L, false)]);
    }

    #[test]
    fn the_stick_stays_in_the_left_bar() {
        let g = crate::game::Game::create();
        let l = pixel();
        let mut c = Controls::new(Mode::Stick, l);
        c.down(g, 1, l.view.x - 10.0, 700.0);
        c.moved(1, l.view.x + 400.0, 700.0);
        let f = c.fingers[0];
        assert!(f.origin.0 + l.stick_base_radius() <= l.view.x + 0.01, "{:?}", f.origin);
        assert!(l.knob(&f).0 + 26.0 * l.dp <= l.view.x + 0.01);
        assert_eq!(c.held[Key::Right as usize], 1);
        c.moved(1, 0.0, l.h);
        let f = c.fingers[0];
        assert!(f.origin.0 >= l.stick_base_radius() && f.origin.1 <= l.h - l.stick_base_radius());
    }

    #[test]
    fn controls_keep_clear_of_the_cutout() {
        let free = pixel();
        // A punch hole in the middle of the left edge, as on a phone held in landscape, touches nothing.
        let middle = layout(2244.0, 1008.0, 3.0, &[Rect { x: 0.0, y: 440.0, w: 130.0, h: 130.0 }]);
        assert_eq!(middle, Layout { ..free });
        // One in the bottom left corner moves the D-pad off it, and only the D-pad.
        let corner = layout(2244.0, 1008.0, 3.0, &[Rect { x: 0.0, y: 872.0, w: 136.0, h: 136.0 }]);
        assert!(corner.dpad.x - corner.dpad.r >= 136.0);
        assert_eq!((corner.back, corner.jump), (free.back, free.jump));
    }

    #[test]
    fn stick_mode_leaves_the_keys_and_toggle_alone() {
        let g = crate::game::Game::create();
        let l = pixel();
        let mut c = Controls::new(Mode::Stick, l);
        c.down(g, 1, l.back.x + 5.0, l.back.y + 5.0);
        assert_eq!(c.fingers[0].zone, Zone::Keys);
        assert_eq!(c.queue, [(Key::Back, true)]);
        c.down(g, 2, l.toggle.x + 5.0, l.toggle.y + 5.0);
        c.up(g, 2, l.toggle.x + 5.0, l.toggle.y + 5.0);
        assert_eq!(c.mode, Mode::Dpad);
        c.down(g, 3, l.view.x + 300.0, 500.0);
        assert_eq!(c.fingers.len(), 1);
    }

    #[test]
    fn sliding_between_buttons_presses_both() {
        let g = crate::game::Game::create();
        let l = pixel();
        let mut c = Controls::new(Mode::Dpad, l);
        c.down(g, 1, l.run.x, (l.run.y + l.item.y) / 2.0);
        assert_eq!(c.queue, [(Key::Run, true), (Key::Item, true)]);
        c.queue.clear();
        c.moved(1, l.jump.x, l.jump.y);
        assert_eq!(c.queue, [(Key::Run, false), (Key::Item, false), (U, true)]);
    }

    #[test]
    fn a_still_finger_off_the_controls_taps_the_picture() {
        let g = crate::game::Game::create();
        let l = pixel();
        let mut c = Controls::new(Mode::Dpad, l);
        c.down(g, 1, 1122.0, 504.0);
        c.moved(1, 1122.0 + 6.0 * l.dp, 504.0);
        c.up(g, 1, 1122.0 + 6.0 * l.dp, 504.0);
        assert_eq!(c.taps, [(328, 240)]);
        c.taps.clear();
        c.down(g, 2, 1122.0, 504.0);
        c.moved(2, 1122.0 + 20.0 * l.dp, 504.0);
        c.moved(2, 1122.0, 504.0);
        c.up(g, 2, 1122.0, 504.0);
        c.down(g, 3, l.jump.x, l.jump.y);
        c.up(g, 3, l.jump.x, l.jump.y);
        c.down(g, 4, l.view.x + l.view.w + 5.0, 10.0);
        c.up(g, 4, l.view.x + l.view.w + 5.0, 10.0);
        assert!(c.taps.is_empty());
        assert_eq!(c.queue, [(U, true), (U, false)]);
        c.down(g, 5, 1122.0, 504.0);
        c.release_all();
        c.up(g, 5, 1122.0, 504.0);
        assert!(c.taps.is_empty() && c.presses.is_empty());
    }

    #[test]
    fn narrow_screens_overlap_rather_than_vanish() {
        let l = layout(1024.0, 768.0, 2.0, &[]);
        assert!(l.dpad.r * 2.0 >= 100.0);
        assert!(l.jump.r * 2.0 >= 56.0 * 2.0);
    }
}
