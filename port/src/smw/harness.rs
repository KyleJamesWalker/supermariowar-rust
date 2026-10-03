//! Port of src/smw/Harness.cpp from port/tools/cpp-harness.patch (not in upstream C++).
//!
//! Deterministic replay/dump harness. Every entry point is a no-op unless the corresponding
//! SMW_* environment variable is set; port/REPLAY.md is the byte-level spec.

use crate::common::game_values::AppState;
use crate::common::global_constants::WAITTIME;
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType};
use crate::common::sfx;
use crate::globals::*;
use crate::smw::game_state::GameStateManager;
use crate::smw::gs_gameplay::GameplayState;
use crate::smw::gs_menu::MenuState;
use crate::smw::gs_splash_screen::SplashScreenState;
use sdl2::sys::{SDL_Event, SDL_EventType, SDL_GetKeyFromName, SDL_GetScancodeFromKey, SDL_KeyCode, SDL_Keymod, SDL_PushEvent, SDL_PRESSED, SDL_RELEASED};
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::File;
use std::io::{BufWriter, Write};

extern "C" {
    fn srand(seed: u32);
}

struct KeyEvent {
    frame: u32,
    down: bool,
    keyName: String,
}

struct Harness {
    seeded: bool,
    seed: u32,
    noLimit: bool,
    map: String,
    maxFrames: i64,
    dump: Option<BufWriter<File>>,
    shotFrames: BTreeSet<u32>,
    shotDir: String,
    events: Vec<KeyEvent>,
    nextEvent: usize,
    frame: u32,
}

static mut h: Harness = Harness {
    seeded: false,
    seed: 0,
    noLimit: false,
    map: String::new(),
    maxFrames: -1,
    dump: None,
    shotFrames: BTreeSet::new(),
    shotDir: String::new(),
    events: Vec::new(),
    nextEvent: 0,
    frame: 0,
};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn fail(msg: String) -> ! {
    eprintln!("[harness] {}", msg);
    std::process::exit(2);
}

/// C `strtoul(s, nullptr, 0)`: decimal, `0x` hex or leading-zero octal prefix, stops at the first bad digit.
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

/// C `strtol(s, nullptr, 10)` for the non-negative values the harness reads.
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

extern "C" fn virtual_ticks() -> u32 {
    unsafe { 1000u32.wrapping_add(h.frame.wrapping_mul(WAITTIME as u32)) }
}

fn load_replay(path: &str) {
    let text = std::fs::read_to_string(path).unwrap_or_else(|_| fail(format!("cannot open replay {}", path)));
    let events = unsafe { &mut h.events };
    for (i, raw) in text.split('\n').enumerate() {
        let lineno = i + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = line.trim_start_matches([' ', '\t']);
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let mut rest = line.trim_start();
        let mut next_token = || {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let tok = &rest[..end];
            rest = rest[end..].trim_start();
            tok
        };
        let frame_tok = next_token();
        let dir = next_token();
        let frame = match frame_tok.parse::<u32>() {
            Ok(f) if !dir.is_empty() => f,
            _ => fail(format!("{}:{}: malformed line", path, lineno)),
        };
        let keyName = rest.trim_start_matches([' ', '\t']).to_string();
        if (dir != "down" && dir != "up") || keyName.is_empty() {
            fail(format!("{}:{}: expected '<frame> <down|up> <key>'", path, lineno));
        }
        if let Some(last) = events.last() {
            if frame < last.frame {
                fail(format!("{}:{}: frames must be non-decreasing", path, lineno));
            }
        }
        events.push(KeyEvent { frame, down: dir == "down", keyName });
    }
}

fn push_key(ev: &KeyEvent) {
    unsafe {
        let cname = CString::new(ev.keyName.as_bytes()).unwrap();
        let key = SDL_GetKeyFromName(cname.as_ptr());
        if key == SDL_KeyCode::SDLK_UNKNOWN as i32 {
            fail(format!("unknown SDL key name '{}'", ev.keyName));
        }

        let mut event: SDL_Event = std::mem::zeroed();
        event.key.type_ = if ev.down { SDL_EventType::SDL_KEYDOWN as u32 } else { SDL_EventType::SDL_KEYUP as u32 };
        event.key.state = if ev.down { SDL_PRESSED as u8 } else { SDL_RELEASED as u8 };
        event.key.keysym.sym = key;
        event.key.keysym.scancode = SDL_GetScancodeFromKey(key);
        event.key.keysym.mod_ = SDL_Keymod::KMOD_NONE as u16;
        SDL_PushEvent(&mut event);
    }
}

fn state_name() -> &'static str {
    let current = GameStateManager::instance().currentState;
    if current.addr() == SplashScreenState::instance() as *mut SplashScreenState as usize {
        return "splash";
    }
    if current.addr() == MenuState::instance() as *mut MenuState as usize {
        return "menu";
    }
    if current.addr() == GameplayState::instance() as *mut GameplayState as usize {
        return "gameplay";
    }
    "other"
}

fn dump_frame(out: &mut impl Write) -> std::io::Result<()> {
    let state = state_name();
    writeln!(out, "F {} {}", unsafe { h.frame }, state)?;

    if state == "menu" {
        let menu = MenuState::instance();
        writeln!(out, "M {} focus={} modifying={}", menu.harness_menu_name(), menu.harness_focus_index(), menu.harness_modifying() as i32)?;
    } else if state == "gameplay" {
        GameplayState::instance().harness_dump(out)?;
    }

    writeln!(out, "R calls={} last={}", RandomNumberGenerator::call_count(), RandomNumberGenerator::last_value())
}

pub fn init() {
    unsafe {
        if let Some(seed) = env("SMW_SEED") {
            h.seeded = true;
            h.seed = strtoul0(&seed);
            RandomNumberGenerator::generator().reseed(h.seed);
            srand(h.seed);
            RandomNumberGenerator::reset_call_count();
            sfx::sfx_ticks = virtual_ticks;
            sfx::sfx_ignore_channel_failure = true;
        }

        h.noLimit = env("SMW_NOLIMIT").is_some();

        if let Some(map) = env("SMW_MAP") {
            h.map = map;
        }

        if let Some(frames) = env("SMW_FRAMES") {
            h.maxFrames = strtol10(&frames);
        }

        if let Some(replay) = env("SMW_REPLAY") {
            load_replay(&replay);
        }

        if let Some(dump) = env("SMW_DUMP") {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&dump)
                .unwrap_or_else(|_| fail(format!("cannot open dump {}", dump)));
            h.dump = Some(BufWriter::new(file));
        }

        if let Some(shots) = env("SMW_SHOT_FRAMES") {
            for item in shots.split(',').filter(|s| !s.is_empty()) {
                h.shotFrames.insert(strtol10(item) as u32);
            }
        }
        h.shotDir = env("SMW_SHOT_DIR").unwrap_or_else(|| ".".to_string());
    }
}

pub fn libc_seed(fallback: u32) -> u32 {
    unsafe {
        if h.seeded {
            h.seed
        } else {
            fallback
        }
    }
}

pub fn no_limit() -> bool {
    unsafe { h.noLimit }
}

pub fn forced_map() -> Option<&'static str> {
    unsafe {
        if h.map.is_empty() {
            None
        } else {
            Some(h.map.as_str())
        }
    }
}

pub fn frame_start() {
    unsafe {
        while h.nextEvent < h.events.len() && h.events[h.nextEvent].frame <= h.frame {
            if h.events[h.nextEvent].frame == h.frame {
                push_key(&h.events[h.nextEvent]);
            }
            h.nextEvent += 1;
        }
    }
}

pub fn frame_end() {
    unsafe {
        if let Some(mut out) = h.dump.take() {
            let _ = dump_frame(&mut out);
            let _ = out.flush();
            h.dump = Some(out);
        }

        if h.shotFrames.contains(&h.frame) {
            let path = format!("{}/frame_{}.bmp", h.shotDir, h.frame);
            if !crate::common::gfx::gfx_save_screen_bmp(&path) {
                eprintln!("[harness] cannot save {}", path);
            }
        }

        h.frame += 1;
        if h.maxFrames >= 0 && h.frame as i64 >= h.maxFrames {
            crate::common::global::game_values.appstate = AppState::Quit;
            h.dump = None;
        }
    }
}
