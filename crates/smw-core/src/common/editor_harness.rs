//! Port of src/common/EditorHarness.cpp from tools/cpp-harness.patch (not in upstream C++).
//!
//! Deterministic replay/dump harness for the level and world editors. Every entry point is a
//! no-op unless the corresponding SMW_* environment variable is set; docs/EDITOR_REPLAY.md is
//! the byte-level spec.

use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType};
use crate::globals::*;
use sdl2::sys::{
    SDL_Delay, SDL_Event, SDL_EventType, SDL_GetError, SDL_GetKeyFromName, SDL_GetScancodeFromKey, SDL_KeyCode, SDL_Keymod, SDL_PushEvent, SDL_RWFromFile, SDL_SaveBMP_RW,
    SDL_BUTTON_LEFT, SDL_BUTTON_MIDDLE, SDL_BUTTON_RIGHT, SDL_PRESSED, SDL_RELEASED,
};
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

pub type Dumper = fn(out: &mut dyn Write);

struct EditorHarness {
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
}

static mut h: EditorHarness = EditorHarness {
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
};

static mut keys: [u8; sdl2::sys::SDL_Scancode::SDL_NUM_SCANCODES as usize] = [0; sdl2::sys::SDL_Scancode::SDL_NUM_SCANCODES as usize];

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

fn load_replay(path: &str) {
    let text = crate::smw::harness::read_recording(path).unwrap_or_else(|| {
        eprintln!("[editorharness] cannot open replay {}", path);
        std::process::exit(2);
    });
    let events = unsafe { &mut h.events };
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
    let m = if key == SDL_KeyCode::SDLK_LSHIFT as i32 {
        SDL_Keymod::KMOD_LSHIFT
    } else if key == SDL_KeyCode::SDLK_RSHIFT as i32 {
        SDL_Keymod::KMOD_RSHIFT
    } else if key == SDL_KeyCode::SDLK_LCTRL as i32 {
        SDL_Keymod::KMOD_LCTRL
    } else if key == SDL_KeyCode::SDLK_RCTRL as i32 {
        SDL_Keymod::KMOD_RCTRL
    } else if key == SDL_KeyCode::SDLK_LALT as i32 {
        SDL_Keymod::KMOD_LALT
    } else if key == SDL_KeyCode::SDLK_RALT as i32 {
        SDL_Keymod::KMOD_RALT
    } else {
        SDL_Keymod::KMOD_NONE
    };
    m as u16
}

fn sdl_button_mask(button: u8) -> u32 {
    1u32 << (button as u32 - 1)
}

fn push_event(ev: &ScriptEvent) {
    unsafe {
        let mut event: SDL_Event = std::mem::zeroed();

        match ev.kind {
            EventKind::Key => {
                let cname = CString::new(ev.keyName.as_bytes()).unwrap();
                let key = SDL_GetKeyFromName(cname.as_ptr());
                if key == SDL_KeyCode::SDLK_UNKNOWN as i32 {
                    eprintln!("[editorharness] unknown SDL key name '{}'", ev.keyName);
                    std::process::exit(2);
                }

                let m = modifier_for(key);
                if ev.down {
                    h.mod_ |= m;
                } else {
                    h.mod_ &= !m;
                }

                event.key.type_ = if ev.down { SDL_EventType::SDL_KEYDOWN as u32 } else { SDL_EventType::SDL_KEYUP as u32 };
                event.key.state = if ev.down { SDL_PRESSED as u8 } else { SDL_RELEASED as u8 };
                event.key.keysym.sym = key;
                event.key.keysym.scancode = SDL_GetScancodeFromKey(key);
                event.key.keysym.mod_ = h.mod_;
                keys[event.key.keysym.scancode as usize] = ev.down as u8;
            }
            EventKind::MouseButton => {
                if ev.down {
                    h.buttons |= sdl_button_mask(ev.button);
                } else {
                    h.buttons &= !sdl_button_mask(ev.button);
                }

                event.button.type_ = if ev.down { SDL_EventType::SDL_MOUSEBUTTONDOWN as u32 } else { SDL_EventType::SDL_MOUSEBUTTONUP as u32 };
                event.button.button = ev.button;
                event.button.state = if ev.down { SDL_PRESSED as u8 } else { SDL_RELEASED as u8 };
                event.button.clicks = 1;
                event.button.x = ev.x;
                event.button.y = ev.y;
                h.mouseX = ev.x;
                h.mouseY = ev.y;
            }
            EventKind::MouseMove => {
                event.motion.type_ = SDL_EventType::SDL_MOUSEMOTION as u32;
                event.motion.state = h.buttons;
                event.motion.x = ev.x;
                event.motion.y = ev.y;
                event.motion.xrel = ev.x - h.mouseX;
                event.motion.yrel = ev.y - h.mouseY;
                h.mouseX = ev.x;
                h.mouseY = ev.y;
            }
        }

        SDL_PushEvent(&mut event);
    }
}

fn push_frame_events() {
    unsafe {
        while h.nextEvent < h.events.len() && h.events[h.nextEvent].frame <= h.frame {
            if h.events[h.nextEvent].frame == h.frame {
                let ev: *const ScriptEvent = &h.events[h.nextEvent];
                push_event(&*ev);
            }
            h.nextEvent += 1;
        }
    }
}

fn push_quit() {
    unsafe {
        let mut event: SDL_Event = std::mem::zeroed();
        event.type_ = SDL_EventType::SDL_QUIT as u32;
        SDL_PushEvent(&mut event);
    }
}

pub fn init() {
    unsafe {
        if let Some(seed) = env("SMW_SEED") {
            let value = strtoul0(&seed);
            RandomNumberGenerator::generator().reseed(value);
            srand(value);
            RandomNumberGenerator::reset_call_count();
        }

        h.noLimit = env("SMW_NOLIMIT").is_some();

        if let Some(frames) = env("SMW_FRAMES") {
            h.maxFrames = strtol10(&frames);
        }

        if let Some(replay) = env("SMW_REPLAY") {
            load_replay(&replay);
        }

        if let Some(dump) = env("SMW_DUMP") {
            match std::fs::OpenOptions::new().append(true).create(true).open(&dump) {
                Ok(f) => h.dump = Some(f),
                Err(_) => {
                    eprintln!("[editorharness] cannot open dump {}", dump);
                    std::process::exit(2);
                }
            }
        }

        if let Some(shots) = env("SMW_SHOT_FRAMES") {
            for item in shots.split(',') {
                if !item.is_empty() {
                    h.shotFrames.insert(strtoul0(item));
                }
            }
        }
        h.shotDir = env("SMW_SHOT_DIR").unwrap_or_else(|| ".".to_string());

        h.active = true;
        push_frame_events();
    }
}

pub fn set_dumper(dumper: Dumper) {
    unsafe { h.dumper = Some(dumper) };
}

pub fn no_limit() -> bool {
    unsafe { h.noLimit }
}

/// Ends one editor frame: dump, screenshot, advance, push the next frame's scripted events,
/// then sleep `delay` ms unless SMW_NOLIMIT is set. The C++ editors `#define SDL_Delay` to this.
pub fn frame_delay(delay: u32) {
    unsafe {
        if h.active {
            if h.maxFrames < 0 || (h.frame as i64) < h.maxFrames {
                if let Some(dump) = h.dump.as_mut() {
                    let mut block: Vec<u8> = Vec::new();
                    let _ = writeln!(block, "F {}", h.frame);
                    if let Some(dumper) = h.dumper {
                        dumper(&mut block);
                    }
                    let _ = writeln!(block, "R calls={} last={}", RandomNumberGenerator::call_count(), RandomNumberGenerator::last_value());
                    let _ = dump.write_all(&block);
                    let _ = dump.flush();
                }

                if h.shotFrames.contains(&h.frame) {
                    let path = format!("{}/frame_{}.bmp", h.shotDir, h.frame);
                    let cpath = CString::new(path.as_bytes()).unwrap();
                    let rw = SDL_RWFromFile(cpath.as_ptr(), b"wb\0".as_ptr() as *const _);
                    if SDL_SaveBMP_RW(screen, rw, 1) != 0 {
                        eprintln!("[editorharness] cannot save {}: {}", path, CStr::from_ptr(SDL_GetError()).to_string_lossy());
                    }
                }
            }

            h.frame += 1;

            if h.maxFrames >= 0 && h.frame as i64 >= h.maxFrames {
                h.dump = None;
                if h.frame as i64 >= h.maxFrames + QUIT_GRACE_FRAMES as i64 {
                    eprintln!("[editorharness] editor ignored SDL_QUIT for {} frames", QUIT_GRACE_FRAMES);
                    std::process::exit(3);
                }
                push_quit();
            } else {
                push_frame_events();
            }
        }

        if !h.noLimit {
            SDL_Delay(delay);
        }
    }
}

/// `SDL_GetKeyboardState(NULL)`; while active, the scripted key state, since pushed events do not update SDL's.
pub fn keyboard_state() -> *const u8 {
    unsafe {
        if h.active {
            keys.as_ptr()
        } else {
            sdl2::sys::SDL_GetKeyboardState(std::ptr::null_mut())
        }
    }
}

/// `SDL_GetMouseState(NULL, NULL)`; while active, the scripted button mask.
pub fn mouse_state() -> u32 {
    unsafe {
        if h.active {
            h.buttons
        } else {
            sdl2::sys::SDL_GetMouseState(std::ptr::null_mut(), std::ptr::null_mut())
        }
    }
}
