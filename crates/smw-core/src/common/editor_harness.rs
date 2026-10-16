//! Port of src/common/EditorHarness.cpp from tools/cpp-harness.patch (not in upstream C++).
//!
//! Deterministic replay/dump harness for the level and world editors. Every entry point is a
//! no-op unless the corresponding SMW_* environment variable is set; docs/EDITOR_REPLAY.md is
//! the byte-level spec.

use crate::common::keys::*;
use crate::game::Game;
use smw_platform::{Input, InputEvent, MouseEvent};
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType};
use crate::globals::*;
use sdl2::sys::{SDL_Event, SDL_EventType, SDL_GetKeyFromName, SDL_GetScancodeFromKey, SDL_BUTTON_LEFT, SDL_BUTTON_MIDDLE, SDL_BUTTON_RIGHT, SDL_PRESSED, SDL_RELEASED};
use std::collections::BTreeSet;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::Write;

extern "C" {
    fn srand(seed: u32);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EventKind {
    Key,
    MouseButton,
    MouseMove,
}

struct ScriptEvent {
    frame: u32,
    kind: EventKind,
    down: bool,
    keyName: String,
    button: u8,
    x: i32,
    y: i32,
}

const QUIT_GRACE_FRAMES: u32 = 60;

pub type Dumper = fn(g: &mut Game, out: &mut dyn Write);

pub struct EditorHarness {
    active: bool,
    noLimit: bool,
    maxFrames: i64,
    dump: Option<File>,
    shotFrames: BTreeSet<u32>,
    shotDir: String,
    events: Vec<ScriptEvent>,
    nextEvent: usize,
    frame: u32,
    dumper: Option<Dumper>,

    buttons: u32,
    mod_: u16,
    mouseX: i32,
    mouseY: i32,
    keys: [u8; sdl2::sys::SDL_Scancode::SDL_NUM_SCANCODES as usize],
}

impl EditorHarness {
    pub const fn new() -> Self {
        EditorHarness {
            active: false,
            noLimit: false,
            maxFrames: -1,
            dump: None,
            shotFrames: BTreeSet::new(),
            shotDir: String::new(),
            events: Vec::new(),
            nextEvent: 0,
            frame: 0,
            dumper: None,
            buttons: 0,
            mod_: 0,
            mouseX: 0,
            mouseY: 0,
            keys: [0; sdl2::sys::SDL_Scancode::SDL_NUM_SCANCODES as usize],
        }
    }
}

fn h(g: &mut Game) -> &mut EditorHarness {
    &mut g.editor_harness
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn fail(path: &str, lineno: usize, msg: &str) -> ! {
    eprintln!("[editorharness] {}:{}: {}", path, lineno, msg);
    std::process::exit(2);
}

/// C `strtoul(s, nullptr, 0)`.
fn strtoul0(s: &str) -> u32 {
    let s = s.trim_start();
    let (digits, radix) = if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (rest, 16)
    } else if s.len() > 1 && s.starts_with('0') {
        (&s[1..], 8)
    } else {
        (s, 10)
    };
    let end = digits.find(|c: char| !c.is_digit(radix)).unwrap_or(digits.len());
    u64::from_str_radix(&digits[..end], radix).unwrap_or(0) as u32
}

/// C `strtol(s, nullptr, 10)`.
fn strtol10(s: &str) -> i64 {
    let s = s.trim_start();
    let neg = s.starts_with('-');
    let body = s.trim_start_matches(['-', '+']);
    let end = body.find(|c: char| !c.is_ascii_digit()).unwrap_or(body.len());
    let v = body[..end].parse::<i64>().unwrap_or(0);
    if neg {
        -v
    } else {
        v
    }
}

/// `std::istringstream >> int`: skips whitespace, takes an optional sign and digits.
fn parse_int(tok: Option<&str>) -> Option<i32> {
    let tok = tok?;
    tok.parse::<i32>().ok()
}

fn parse_button(name: &str, path: &str, lineno: usize) -> u8 {
    match name {
        "left" => SDL_BUTTON_LEFT as u8,
        "middle" => SDL_BUTTON_MIDDLE as u8,
        "right" => SDL_BUTTON_RIGHT as u8,
        _ => fail(path, lineno, "expected left, middle or right"),
    }
}

fn load_replay(g: &mut Game, path: &str) {
    let mut gp = g.ptr();
    let text = crate::smw::harness::read_recording(path).unwrap_or_else(|| {
        eprintln!("[editorharness] cannot open replay {}", path);
        std::process::exit(2);
    });
    let events = &mut h(gp.get()).events;
    for (i, raw) in text.split('\n').enumerate() {
        let lineno = i + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = line.trim_start_matches([' ', '\t']);
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let mut tokens = trimmed.split_whitespace();
        let frame = tokens.next().and_then(|t| t.parse::<u32>().ok());
        let verb = tokens.next();
        let (frame, verb) = match (frame, verb) {
            (Some(f), Some(v)) => (f, v),
            _ => fail(path, lineno, "malformed line"),
        };

        let mut ev = ScriptEvent { frame, kind: EventKind::Key, down: false, keyName: String::new(), button: 0, x: 0, y: 0 };

        if verb == "down" || verb == "up" {
            ev.kind = EventKind::Key;
            ev.down = verb == "down";
            let after_frame = trimmed[trimmed.find(char::is_whitespace).unwrap()..].trim_start_matches([' ', '\t']);
            let after_verb = &after_frame[verb.len()..];
            ev.keyName = after_verb.trim_start_matches([' ', '\t']).to_string();
            if ev.keyName.is_empty() {
                fail(path, lineno, "expected '<frame> <down|up> <key>'");
            }
        } else if verb == "mousedown" || verb == "mouseup" {
            ev.kind = EventKind::MouseButton;
            ev.down = verb == "mousedown";
            let button = tokens.next();
            let x = parse_int(tokens.next());
            let y = parse_int(tokens.next());
            match (button, x, y) {
                (Some(b), Some(x), Some(y)) => {
                    ev.x = x;
                    ev.y = y;
                    ev.button = parse_button(b, path, lineno);
                }
                _ => fail(path, lineno, "expected '<frame> <mousedown|mouseup> <button> <x> <y>'"),
            }
        } else if verb == "mousemove" {
            ev.kind = EventKind::MouseMove;
            match (parse_int(tokens.next()), parse_int(tokens.next())) {
                (Some(x), Some(y)) => {
                    ev.x = x;
                    ev.y = y;
                }
                _ => fail(path, lineno, "expected '<frame> mousemove <x> <y>'"),
            }
        } else {
            fail(path, lineno, "unknown event");
        }

        if let Some(last) = events.last() {
            if ev.frame < last.frame {
                fail(path, lineno, "frames must be non-decreasing");
            }
        }
        events.push(ev);
    }
}

fn modifier_for(key: i32) -> u16 {
    let m = if key == SDLK_LSHIFT {
        KMOD_LSHIFT
    } else if key == SDLK_RSHIFT {
        KMOD_RSHIFT
    } else if key == SDLK_LCTRL {
        KMOD_LCTRL
    } else if key == SDLK_RCTRL {
        KMOD_RCTRL
    } else if key == SDLK_LALT {
        KMOD_LALT
    } else if key == SDLK_RALT {
        KMOD_RALT
    } else {
        KMOD_NONE
    };
    m as u16
}

fn sdl_button_mask(button: u8) -> u32 {
    1u32 << (button as u32 - 1)
}

fn push_event(g: &mut Game, ev: &ScriptEvent) {
    let mut gp = g.ptr();
    let event = match ev.kind {
        EventKind::Key => {
            let kb = &crate::services::services(gp.get()).keyboard;
            let key = kb.key_from_name(&ev.keyName);
            if key == SDLK_UNKNOWN {
                eprintln!("[editorharness] unknown SDL key name '{}'", ev.keyName);
                std::process::exit(2);
            }
            let scancode = kb.scancode_from_key(key);

            let m = modifier_for(key);
            if ev.down {
                h(gp.get()).mod_ |= m;
            } else {
                h(gp.get()).mod_ &= !m;
            }
            h(gp.get()).keys[scancode as usize] = ev.down as u8;
            InputEvent::Key { key: smw_platform::Keycode(key), scancode, mods: h(gp.get()).mod_, down: ev.down, repeat: false, window: 0 }
        }
        EventKind::MouseButton => {
            if ev.down {
                h(gp.get()).buttons |= sdl_button_mask(ev.button);
            } else {
                h(gp.get()).buttons &= !sdl_button_mask(ev.button);
            }
            h(gp.get()).mouseX = ev.x;
            h(gp.get()).mouseY = ev.y;
            InputEvent::Mouse { window: 0, which: 0, event: MouseEvent::Button { button: ev.button, down: ev.down, clicks: 1, x: ev.x, y: ev.y } }
        }
        EventKind::MouseMove => {
            let (xrel, yrel) = (ev.x - h(gp.get()).mouseX, ev.y - h(gp.get()).mouseY);
            h(gp.get()).mouseX = ev.x;
            h(gp.get()).mouseY = ev.y;
            InputEvent::Mouse { window: 0, which: 0, event: MouseEvent::Motion { x: ev.x, y: ev.y, xrel, yrel, buttons: h(gp.get()).buttons } }
        }
    };
    crate::services::services(g).events.push(&Input { timestamp: 0, event });
}

fn push_frame_events(g: &mut Game) {
    let mut gp = g.ptr();
    unsafe {
        while h(gp.get()).nextEvent < h(gp.get()).events.len() && h(gp.get()).events[h(gp.get()).nextEvent].frame <= h(gp.get()).frame {
            if h(gp.get()).events[h(gp.get()).nextEvent].frame == h(gp.get()).frame {
                let ev: *const ScriptEvent = &h(gp.get()).events[h(gp.get()).nextEvent];
                push_event(g, &*ev);
            }
            h(gp.get()).nextEvent += 1;
        }
    }
}

fn push_quit(g: &mut Game) {
    crate::services::services(g).events.push(&Input { timestamp: 0, event: InputEvent::Quit });
}

pub fn init(g: &mut Game) {
    let mut gp = g.ptr();
    unsafe {
        if let Some(seed) = env("SMW_SEED") {
            let value = strtoul0(&seed);
            RandomNumberGenerator::generator(gp.get()).reseed(value);
            srand(value);
            RandomNumberGenerator::reset_call_count(gp.get());
        }

        h(gp.get()).noLimit = env("SMW_NOLIMIT").is_some();

        if let Some(frames) = env("SMW_FRAMES") {
            h(gp.get()).maxFrames = strtol10(&frames);
        }

        if let Some(replay) = env("SMW_REPLAY") {
            load_replay(g, &replay);
        }

        if let Some(dump) = env("SMW_DUMP") {
            match std::fs::OpenOptions::new().append(true).create(true).open(&dump) {
                Ok(f) => h(gp.get()).dump = Some(f),
                Err(_) => {
                    eprintln!("[editorharness] cannot open dump {}", dump);
                    std::process::exit(2);
                }
            }
        }

        if let Some(shots) = env("SMW_SHOT_FRAMES") {
            for item in shots.split(',') {
                if !item.is_empty() {
                    h(gp.get()).shotFrames.insert(strtoul0(item));
                }
            }
        }
        h(gp.get()).shotDir = env("SMW_SHOT_DIR").unwrap_or_else(|| ".".to_string());

        h(gp.get()).active = true;
        push_frame_events(g);
    }
}

pub fn set_dumper(g: &mut Game, dumper: Dumper) {
    let mut gp = g.ptr();
    h(gp.get()).dumper = Some(dumper);
}

pub fn no_limit(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    h(gp.get()).noLimit
}

/// Ends one editor frame: dump, screenshot, advance, push the next frame's scripted events,
/// then sleep `delay` ms unless SMW_NOLIMIT is set. The C++ editors `#define SDL_Delay` to this.
pub fn frame_delay(g: &mut Game, delay: u32) {
    let mut gp = g.ptr();
    unsafe {
        if h(gp.get()).active {
            if h(gp.get()).maxFrames < 0 || (h(gp.get()).frame as i64) < h(gp.get()).maxFrames {
                if let Some(dump) = h(gp.get()).dump.as_mut() {
                    let mut block: Vec<u8> = Vec::new();
                    let _ = writeln!(block, "F {}", h(gp.get()).frame);
                    if let Some(dumper) = h(gp.get()).dumper {
                        dumper(g, &mut block);
                    }
                    let _ = writeln!(block, "R calls={} last={}", RandomNumberGenerator::call_count(gp.get()), RandomNumberGenerator::last_value(gp.get()));
                    let _ = dump.write_all(&block);
                    let _ = dump.flush();
                }

                if h(gp.get()).shotFrames.contains(&h(gp.get()).frame) {
                    let path = format!("{}/frame_{}.bmp", h(gp.get()).shotDir, h(gp.get()).frame);
                    if let Err(e) = (*g.screen).save_bmp(std::path::Path::new(&path)) {
                        eprintln!("[editorharness] cannot save {}: {}", path, e);
                    }
                }
            }

            h(gp.get()).frame += 1;

            if h(gp.get()).maxFrames >= 0 && h(gp.get()).frame as i64 >= h(gp.get()).maxFrames {
                h(gp.get()).dump = None;
                if h(gp.get()).frame as i64 >= h(gp.get()).maxFrames + QUIT_GRACE_FRAMES as i64 {
                    eprintln!("[editorharness] editor ignored SDL_QUIT for {} frames", QUIT_GRACE_FRAMES);
                    std::process::exit(3);
                }
                push_quit(g);
            } else {
                push_frame_events(g);
            }
        }

        if !h(gp.get()).noLimit {
            crate::services::delay(gp.get(), delay);
        }
    }
}

/// `SDL_GetKeyboardState(NULL)`; while active, the scripted key state, since pushed events do not update SDL's.
pub fn keyboard_state(g: &mut Game) -> *const u8 {
    let mut gp = g.ptr();
    if h(gp.get()).active {
        h(gp.get()).keys.as_ptr()
    } else {
        crate::services::services(gp.get()).keyboard.state().as_ptr()
    }
}

/// `SDL_GetMouseState(NULL, NULL)`; while active, the scripted button mask.
pub fn mouse_state(g: &mut Game) -> u32 {
    let mut gp = g.ptr();
    if h(gp.get()).active {
        h(gp.get()).buttons
    } else {
        crate::services::services(gp.get()).keyboard.mouse_buttons()
    }
}
