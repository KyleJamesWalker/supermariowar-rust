//! Port of src/smw/Harness.cpp from tools/cpp-harness.patch (not in upstream C++).
//!
//! Deterministic replay/dump harness. Every entry point is a no-op unless the corresponding
//! SMW_* environment variable is set; REPLAY.md is the byte-level spec.

use crate::common::game_values::AppState;
use crate::common::global_constants::WAITTIME;
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType};
use crate::common::sfx;
use crate::globals::*;
use crate::smw::game_state::GameStateManager;
use crate::smw::gs_gameplay::GameplayState;
use crate::smw::gs_menu::MenuState;
use crate::smw::gs_splash_screen::SplashScreenState;
use sdl2::sys::{
    SDL_Event, SDL_EventType, SDL_GetError, SDL_GetKeyFromName, SDL_GetKeyName, SDL_GetScancodeFromKey, SDL_InitSubSystem, SDL_JoystickAttachVirtual,
    SDL_JoystickInstanceID, SDL_JoystickType, SDL_KeyCode, SDL_Keymod, SDL_PumpEvents, SDL_PushEvent, SDL_SetEventFilter, SDL_WaitEvent, SDL_INIT_JOYSTICK,
    SDL_PRESSED, SDL_RELEASED,
};
use std::ffi::CStr;
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::File;
use std::io::{BufWriter, Write};

extern "C" {
    fn srand(seed: u32);
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum EventKind {
    Key,
    JoyAxis,
    JoyButton,
    JoyHat,
}

struct ReplayEvent {
    frame: u32,
    kind: EventKind,
    down: bool,
    keyName: String,
    device: i32,
    index: i32,
    value: i32,
}

const VIRTUAL_AXES: i32 = 6;
const VIRTUAL_BUTTONS: i32 = 16;
const VIRTUAL_HATS: i32 = 1;

struct Harness {
    seeded: bool,
    seed: u32,
    noLimit: bool,
    map: String,
    maxFrames: i64,
    dump: Option<BufWriter<File>>,
    shotFrames: BTreeSet<u32>,
    shotDir: String,
    events: Vec<ReplayEvent>,
    joysticks: i32,
    replay: bool,
    nextEvent: usize,
    frame: u32,
    audible: bool,
    speed: f32,
    rec: Option<Recorder>,
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
    joysticks: 0,
    replay: false,
    nextEvent: 0,
    frame: 0,
    audible: false,
    speed: 1.0,
    rec: None,
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

fn load_events(path: &str) -> Vec<ReplayEvent> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|_| fail(format!("cannot open replay {}", path)));
    let mut events: Vec<ReplayEvent> = Vec::new();
    let events = &mut events;
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
        let mut ev = ReplayEvent { frame, kind: EventKind::Key, down: false, keyName: String::new(), device: 0, index: 0, value: 0 };
        if dir == "jaxis" || dir == "jbutton" || dir == "jhat" {
            ev.kind = match dir {
                "jaxis" => EventKind::JoyAxis,
                "jbutton" => EventKind::JoyButton,
                _ => EventKind::JoyHat,
            };
            let indexLimit = match ev.kind {
                EventKind::JoyAxis => VIRTUAL_AXES,
                EventKind::JoyButton => VIRTUAL_BUTTONS,
                _ => VIRTUAL_HATS,
            };
            let nums: Vec<Option<i32>> = rest.split_whitespace().map(|t| t.parse::<i32>().ok()).collect();
            let valid = match nums.as_slice() {
                [Some(d), Some(i), Some(v)] => {
                    ev.device = *d;
                    ev.index = *i;
                    ev.value = *v;
                    (0..=7).contains(d)
                        && (0..indexLimit).contains(i)
                        && match ev.kind {
                            EventKind::JoyAxis => (-32768..=32767).contains(v),
                            EventKind::JoyButton => *v == 0 || *v == 1,
                            _ => (0..=15).contains(v),
                        }
                }
                _ => false,
            };
            if !valid {
                fail(format!("{}:{}: expected '<frame> {} <dev> <index> <value>'", path, lineno, dir));
            }
            unsafe {
                h.joysticks = h.joysticks.max(ev.device + 1);
            }
        } else {
            ev.keyName = rest.trim_start_matches([' ', '\t']).to_string();
            if (dir != "down" && dir != "up") || ev.keyName.is_empty() {
                fail(format!("{}:{}: expected '<frame> <down|up> <key>'", path, lineno));
            }
            ev.down = dir == "down";
        }
        if let Some(last) = events.last() {
            if frame < last.frame {
                fail(format!("{}:{}: frames must be non-decreasing", path, lineno));
            }
        }
        events.push(ev);
    }
    std::mem::take(events)
}

fn attach_joysticks() {
    unsafe {
        SDL_InitSubSystem(SDL_INIT_JOYSTICK);
        for i in 0..h.joysticks {
            if SDL_JoystickAttachVirtual(SDL_JoystickType::SDL_JOYSTICK_TYPE_GAMECONTROLLER, VIRTUAL_AXES, VIRTUAL_BUTTONS, VIRTUAL_HATS) != i {
                fail(format!("cannot attach virtual joystick {}: {}", i, CStr::from_ptr(SDL_GetError()).to_string_lossy()));
            }
        }
    }
}

fn fill_joystick(ev: &ReplayEvent, event: &mut SDL_Event) {
    unsafe {
        *event = std::mem::zeroed();
        match ev.kind {
            EventKind::JoyAxis => {
                event.jaxis.type_ = SDL_EventType::SDL_JOYAXISMOTION as u32;
                event.jaxis.which = ev.device;
                event.jaxis.axis = ev.index as u8;
                event.jaxis.value = ev.value as i16;
            }
            EventKind::JoyButton => {
                event.jbutton.type_ = if ev.value != 0 { SDL_EventType::SDL_JOYBUTTONDOWN as u32 } else { SDL_EventType::SDL_JOYBUTTONUP as u32 };
                event.jbutton.which = ev.device;
                event.jbutton.button = ev.index as u8;
                event.jbutton.state = if ev.value != 0 { SDL_PRESSED as u8 } else { SDL_RELEASED as u8 };
            }
            _ => {
                event.jhat.type_ = SDL_EventType::SDL_JOYHATMOTION as u32;
                event.jhat.which = ev.device;
                event.jhat.hat = ev.index as u8;
                event.jhat.value = ev.value as u8;
            }
        }
    }
}

fn fill_key(ev: &ReplayEvent, event: &mut SDL_Event) {
    unsafe {
        let cname = CString::new(ev.keyName.as_bytes()).unwrap();
        let key = SDL_GetKeyFromName(cname.as_ptr());
        if key == SDL_KeyCode::SDLK_UNKNOWN as i32 {
            fail(format!("unknown SDL key name '{}'", ev.keyName));
        }

        *event = std::mem::zeroed();
        event.key.type_ = if ev.down { SDL_EventType::SDL_KEYDOWN as u32 } else { SDL_EventType::SDL_KEYUP as u32 };
        event.key.state = if ev.down { SDL_PRESSED as u8 } else { SDL_RELEASED as u8 };
        event.key.keysym.sym = key;
        event.key.keysym.scancode = SDL_GetScancodeFromKey(key);
        event.key.keysym.mod_ = SDL_Keymod::KMOD_NONE as u16;
    }
}

fn fill_event(ev: &ReplayEvent, event: &mut SDL_Event) {
    if ev.kind == EventKind::Key {
        fill_key(ev, event);
    } else {
        fill_joystick(ev, event);
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

    for line in unsafe { sfx::sfx_events.iter() } {
        writeln!(out, "{}", line)?;
    }

    writeln!(out, "R calls={} last={}", RandomNumberGenerator::call_count(), RandomNumberGenerator::last_value())
}

pub fn init() {
    unsafe {
        // A normal launch records the session (seeded, so it can be replayed); replays never record.
        let record = env("SMW_REPLAY").is_none() && env("SMW_NO_RECORD").is_none();
        let seed_text = env("SMW_SEED").or_else(|| if record { Some(random_seed().to_string()) } else { None });
        if let Some(seed) = seed_text {
            h.seeded = true;
            h.seed = strtoul0(&seed);
            RandomNumberGenerator::generator().reseed(h.seed);
            srand(h.seed);
            RandomNumberGenerator::reset_call_count();
            sfx::sfx_ticks = virtual_ticks;
            sfx::sfx_ignore_channel_failure = true;
            sfx::sfx_virtual_mixer = true;
        }

        h.noLimit = env("SMW_NOLIMIT").is_some();

        if let Some(map) = env("SMW_MAP") {
            h.map = map;
        }

        if let Some(frames) = env("SMW_FRAMES") {
            h.maxFrames = strtol10(&frames);
        }

        if let Some(replay) = env("SMW_REPLAY") {
            h.events = load_events(&replay);
            h.replay = true;
        }

        h.audible = record || env("SMW_AUDIBLE").is_some();
        sfx::sfx_audible = h.audible && h.seeded;
        if let Some(speed) = env("SMW_REPLAY_SPEED").and_then(|v| v.parse::<f32>().ok()).filter(|v| *v > 0.0) {
            h.speed = speed;
        }
        if record {
            start_recording(h.seed);
        }

        if h.joysticks > 0 {
            attach_joysticks();
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
    sfx::sfx_virtual_advance();
    record_frame_start(unsafe { h.frame });

    unsafe {
        while h.nextEvent < h.events.len() && h.events[h.nextEvent].frame <= h.frame {
            if h.events[h.nextEvent].frame == h.frame {
                let mut event: SDL_Event = std::mem::zeroed();
                fill_event(&h.events[h.nextEvent], &mut event);
                SDL_PushEvent(&mut event);
            }
            h.nextEvent += 1;
        }
    }
}

/// Blocking waits take the next replay event immediately instead of waiting for input.
pub fn wait_event(event: &mut SDL_Event) {
    unsafe {
        if h.rec.is_some() {
            record_wait_event(event);
            return;
        }
        if !h.replay {
            SDL_WaitEvent(event);
            return;
        }
        if h.nextEvent >= h.events.len() {
            fail(format!("blocking wait at frame {} but the replay has no events left", h.frame));
        }
        fill_event(&h.events[h.nextEvent], event);
        h.nextEvent += 1;
    }
}

pub fn frame_end() {
    unsafe {
        if let Some(mut out) = h.dump.take() {
            let _ = dump_frame(&mut out);
            let _ = out.flush();
            h.dump = Some(out);
        }
        sfx::sfx_events.clear();

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

//------------------------------------------------------------------------------------------------
// Session recording (not in the C++). A normal launch, i.e. one without SMW_REPLAY, runs seeded and
// records every input the game sees to <settings dir>/replays/<timestamp>.txt in the replay format,
// so the session can be watched again or replayed on the C++ reference. See REPLAY.md, "Recordings".
//------------------------------------------------------------------------------------------------

const KEEP_RECORDINGS: usize = 10;

struct Recorder {
    out: BufWriter<File>,
    path: String,
    /// Raw OS input events held back by the event filter until the next frame start (or blocking wait).
    pending: Vec<SDL_Event>,
    quit: bool,
    /// The frame a window close arrived in; the recording ends before it.
    quitFrame: Option<u32>,
    injecting: bool,
    /// SMW_LIVE_SCRIPT: replay-format lines pushed into SDL's queue as if the OS delivered them.
    script: Vec<ReplayEvent>,
    nextScript: usize,
}

fn rec() -> Option<&'static mut Recorder> {
    unsafe { h.rec.as_mut() }
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(BASE64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits = 0;
    for c in text.bytes() {
        if c == b'=' {
            break;
        }
        let v = BASE64.iter().position(|&x| x == c)? as u32;
        acc = acc << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// UTC `YYYY-MM-DD_HHMMSS` for the recording file name (sorts chronologically).
fn utc_timestamp() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{:04}-{:02}-{:02}_{:02}{:02}{:02}", y, m, d, rem / 3600, rem / 60 % 60, rem % 60)
}

fn random_seed() -> u32 {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mixed = (nanos as u64) ^ ((std::process::id() as u64) << 32) ^ (nanos >> 64) as u64;
    (mixed ^ (mixed >> 29)).wrapping_mul(0x9E3779B97F4A7C15).wrapping_shr(32) as u32
}

fn prune_recordings(dir: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut names: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".txt"))
        .collect();
    names.sort();
    while names.len() > KEEP_RECORDINGS {
        let _ = std::fs::remove_file(format!("{}{}", dir, names.remove(0)));
    }
}

/// Opens the recording and writes its header. Called from `init` before the settings files are read.
fn start_recording(seed: u32) {
    let dir = crate::common::path::get_home_directory() + "replays/";
    if std::fs::create_dir_all(&dir).is_err() {
        eprintln!("[harness] cannot create {}; not recording", dir);
        return;
    }
    let stamp = utc_timestamp();
    let mut path = format!("{}{}.txt", dir, stamp);
    let mut n = 1;
    while std::path::Path::new(&path).exists() {
        n += 1;
        path = format!("{}{}_{}.txt", dir, stamp, n);
    }
    let Ok(file) = File::create(&path) else {
        eprintln!("[harness] cannot create {}; not recording", path);
        return;
    };
    let mut out = BufWriter::new(file);
    let home = crate::common::path::get_home_directory();
    let _ = writeln!(out, "# Super Mario War session recorded {} UTC (Rust port).", stamp.replace('_', " "));
    let _ = writeln!(out, "#@ seed={}", seed);
    for (key, file) in [("options_b64", "options.bin"), ("controls_b64", "controls.sdl2.bin")] {
        if let Ok(bytes) = std::fs::read(home.clone() + file) {
            let _ = writeln!(out, "#@ {}={}", key, base64_encode(&bytes));
        }
    }
    let _ = out.flush();
    println!("[harness] recording to {}", path);

    let script = env("SMW_LIVE_SCRIPT").map(|p| load_events(&p)).unwrap_or_default();
    unsafe {
        h.rec = Some(Recorder { out, path, pending: Vec::new(), quit: false, quitFrame: None, injecting: false, script, nextScript: 0 });
        SDL_SetEventFilter(Some(record_filter), std::ptr::null_mut());
    }
    prune_recordings(&dir);
}

fn is_input_event(t: u32) -> bool {
    use SDL_EventType::*;
    [
        SDL_KEYDOWN, SDL_KEYUP, SDL_TEXTEDITING, SDL_TEXTINPUT, SDL_KEYMAPCHANGED, SDL_MOUSEMOTION, SDL_MOUSEBUTTONDOWN, SDL_MOUSEBUTTONUP,
        SDL_MOUSEWHEEL, SDL_JOYAXISMOTION, SDL_JOYBALLMOTION, SDL_JOYHATMOTION, SDL_JOYBUTTONDOWN, SDL_JOYBUTTONUP, SDL_JOYDEVICEADDED,
        SDL_JOYDEVICEREMOVED, SDL_CONTROLLERAXISMOTION, SDL_CONTROLLERBUTTONDOWN, SDL_CONTROLLERBUTTONUP, SDL_CONTROLLERDEVICEADDED,
        SDL_CONTROLLERDEVICEREMOVED, SDL_CONTROLLERDEVICEREMAPPED, SDL_FINGERDOWN, SDL_FINGERUP, SDL_FINGERMOTION, SDL_QUIT,
    ]
    .iter()
    .any(|&e| e as u32 == t)
}

/// While recording, every input event SDL queues is held back here; the frame start (or a blocking
/// wait) turns the held events into replay lines and pushes those, so the game only ever sees input
/// in the exact form a replay of the recording will produce.
unsafe extern "C" fn record_filter(_userdata: *mut std::ffi::c_void, event: *mut SDL_Event) -> i32 {
    let Some(r) = rec() else { return 1 };
    if r.injecting || !is_input_event((*event).type_) {
        return 1;
    }
    r.pending.push(*event);
    0
}

/// The replay line for a raw SDL event, or None for input the replay format cannot express.
fn to_replay_event(raw: &SDL_Event, frame: u32) -> Option<ReplayEvent> {
    unsafe {
        let t = raw.type_;
        let mut ev = ReplayEvent { frame, kind: EventKind::Key, down: false, keyName: String::new(), device: 0, index: 0, value: 0 };
        if t == SDL_EventType::SDL_KEYDOWN as u32 || t == SDL_EventType::SDL_KEYUP as u32 {
            let sym = raw.key.keysym.sym;
            let name = CStr::from_ptr(SDL_GetKeyName(sym)).to_string_lossy().into_owned();
            if name.is_empty() || SDL_GetKeyFromName(CString::new(name.as_bytes()).ok()?.as_ptr()) != sym {
                return None;
            }
            ev.down = t == SDL_EventType::SDL_KEYDOWN as u32;
            ev.keyName = name;
            return Some(ev);
        }
        let which = if t == SDL_EventType::SDL_JOYAXISMOTION as u32 {
            raw.jaxis.which
        } else if t == SDL_EventType::SDL_JOYBUTTONDOWN as u32 || t == SDL_EventType::SDL_JOYBUTTONUP as u32 {
            raw.jbutton.which
        } else if t == SDL_EventType::SDL_JOYHATMOTION as u32 {
            raw.jhat.which
        } else {
            return None;
        };
        ev.device = device_index(which)?;
        if t == SDL_EventType::SDL_JOYAXISMOTION as u32 {
            ev.kind = EventKind::JoyAxis;
            ev.index = raw.jaxis.axis as i32;
            ev.value = raw.jaxis.value as i32;
        } else if t == SDL_EventType::SDL_JOYHATMOTION as u32 {
            ev.kind = EventKind::JoyHat;
            ev.index = raw.jhat.hat as i32;
            ev.value = raw.jhat.value as i32;
        } else {
            ev.kind = EventKind::JoyButton;
            ev.index = raw.jbutton.button as i32;
            ev.value = (t == SDL_EventType::SDL_JOYBUTTONDOWN as u32) as i32;
        }
        let limit = match ev.kind {
            EventKind::JoyAxis => VIRTUAL_AXES,
            EventKind::JoyButton => VIRTUAL_BUTTONS,
            _ => VIRTUAL_HATS,
        };
        if ev.device > 7 || ev.index >= limit {
            return None;
        }
        Some(ev)
    }
}

/// The game addresses a joystick by its open index; SDL events carry the instance id.
fn device_index(which: i32) -> Option<i32> {
    unsafe {
        let js = crate::common::global::joysticks;
        for i in 0..crate::common::global::joystickcount.max(0) as usize {
            let j = *js.add(i);
            if !j.is_null() && SDL_JoystickInstanceID(j) == which {
                return Some(i as i32);
            }
        }
        None
    }
}

fn line_for(ev: &ReplayEvent) -> String {
    match ev.kind {
        EventKind::Key => format!("{} {} {}", ev.frame, if ev.down { "down" } else { "up" }, ev.keyName),
        EventKind::JoyAxis => format!("{} jaxis {} {} {}", ev.frame, ev.device, ev.index, ev.value),
        EventKind::JoyButton => format!("{} jbutton {} {} {}", ev.frame, ev.device, ev.index, ev.value),
        EventKind::JoyHat => format!("{} jhat {} {} {}", ev.frame, ev.device, ev.index, ev.value),
    }
}

fn inject(r: &mut Recorder, event: &mut SDL_Event) {
    r.injecting = true;
    unsafe { SDL_PushEvent(event) };
    r.injecting = false;
}

/// SMW_LIVE_SCRIPT events due this frame enter SDL's queue like OS input, with the modifier and
/// repeat flags a real keyboard sets, so they take the same capture path as real input.
fn push_live_script(r: &mut Recorder, frame: u32) {
    while r.nextScript < r.script.len() && r.script[r.nextScript].frame <= frame {
        if r.script[r.nextScript].frame == frame {
            let mut event: SDL_Event = unsafe { std::mem::zeroed() };
            fill_event(&r.script[r.nextScript], &mut event);
            unsafe {
                if event.type_ == SDL_EventType::SDL_KEYDOWN as u32 {
                    event.key.keysym.mod_ = SDL_Keymod::KMOD_NUM as u16;
                    event.key.repeat = (r.nextScript % 3 == 2) as u8;
                } else if event.type_ == SDL_EventType::SDL_JOYAXISMOTION as u32 {
                    event.jaxis.which = sdl_instance(event.jaxis.which);
                } else if event.type_ == SDL_EventType::SDL_JOYBUTTONDOWN as u32 || event.type_ == SDL_EventType::SDL_JOYBUTTONUP as u32 {
                    event.jbutton.which = sdl_instance(event.jbutton.which);
                } else if event.type_ == SDL_EventType::SDL_JOYHATMOTION as u32 {
                    event.jhat.which = sdl_instance(event.jhat.which);
                }
                SDL_PushEvent(&mut event);
            }
        }
        r.nextScript += 1;
    }
}

fn sdl_instance(device: i32) -> i32 {
    unsafe {
        let js = crate::common::global::joysticks;
        if device >= 0 && device < crate::common::global::joystickcount as i32 {
            let j = *js.add(device as usize);
            if !j.is_null() {
                return SDL_JoystickInstanceID(j);
            }
        }
        -1
    }
}

/// Frame start while recording: every joystick open at frame 0 gets a centred-hat line so a replay
/// attaches the same number of joysticks; then held input becomes replay lines pushed in order.
fn record_frame_start(frame: u32) {
    let Some(r) = rec() else { return };
    if frame == 0 {
        for d in 0..unsafe { crate::common::global::joystickcount }.min(8) as i32 {
            let ev = ReplayEvent { frame: 0, kind: EventKind::JoyHat, down: false, keyName: String::new(), device: d, index: 0, value: 0 };
            let _ = writeln!(r.out, "{}", line_for(&ev));
            let mut event: SDL_Event = unsafe { std::mem::zeroed() };
            fill_event(&ev, &mut event);
            inject(r, &mut event);
        }
    }
    // A browser tab can close at any moment without finish(); keep the file current for IDBFS syncs.
    #[cfg(target_os = "emscripten")]
    if frame % 60 == 0 {
        let _ = r.out.flush();
    }
    push_live_script(r, frame);
    unsafe { SDL_PumpEvents() };
    let held = std::mem::take(&mut r.pending);
    for raw in held.iter() {
        if unsafe { raw.type_ } == SDL_EventType::SDL_QUIT as u32 {
            r.quit = true;
            continue;
        }
        if let Some(ev) = to_replay_event(raw, frame) {
            let _ = writeln!(r.out, "{}", line_for(&ev));
            let mut event: SDL_Event = unsafe { std::mem::zeroed() };
            fill_event(&ev, &mut event);
            inject(r, &mut event);
        }
    }
    if r.quit {
        // The recording ends before this frame; frames= excludes it.
        r.quitFrame.get_or_insert(frame);
        unsafe { crate::common::global::game_values.appstate = AppState::Quit };
    }
}

/// A blocking wait while recording returns the next held input event, written with the next frame
/// number so the replay's frame start leaves it for the replay's own blocking wait.
fn record_wait_event(event: &mut SDL_Event) {
    loop {
        let Some(r) = rec() else { return };
        push_live_script(r, unsafe { h.frame } + 1);
        unsafe { SDL_PumpEvents() };
        while !r.pending.is_empty() {
            let raw = r.pending.remove(0);
            if unsafe { raw.type_ } == SDL_EventType::SDL_QUIT as u32 {
                r.quit = true;
                r.pending.insert(0, raw);
                break;
            }
            if let Some(ev) = to_replay_event(&raw, unsafe { h.frame } + 1) {
                let _ = writeln!(r.out, "{}", line_for(&ev));
                fill_event(&ev, event);
                return;
            }
        }
        if r.quit {
            // Hand the game an event its wait loop ignores until the frame start ends the session.
            unsafe { *event = std::mem::zeroed() };
            return;
        }
        unsafe { sdl2::sys::SDL_Delay(5) };
    }
}

static mut watch_home: Option<std::path::PathBuf> = None;

/// About three seconds at the default 16 ms frame time.
const UNFINISHED_TAIL_FRAMES: u32 = 188;

/// Writes `#@ frames=` and closes the recording, or removes a watch session's throwaway HOME.
/// Safe to call more than once.
pub fn finish() {
    unsafe {
        if let Some(home) = watch_home.take() {
            let _ = std::fs::remove_dir_all(home);
        }
        if let Some(mut r) = h.rec.take() {
            SDL_SetEventFilter(None, std::ptr::null_mut());
            let frames = r.quitFrame.unwrap_or(h.frame);
            let _ = writeln!(r.out, "#@ frames={}", frames);
            let _ = r.out.flush();
            println!("[harness] recorded {} frames to {}", frames, r.path);
        }
    }
}

/// Real SDL_mixer output on top of the virtual mixer (recorded sessions and --replay watching).
pub fn audible() -> bool {
    unsafe { h.audible }
}

/// Playback speed multiplier for watching a recording (SMW_REPLAY_SPEED); 1 otherwise.
pub fn speed() -> f32 {
    unsafe { h.speed }
}

/// `--replay <file>`: watch a recording in a normal window at normal speed with sound. The settings
/// embedded in the recording go into a throwaway HOME so the user's own settings stay untouched.
pub fn prepare_watch(file: &str, speed: Option<f32>) {
    let path = std::fs::canonicalize(file).unwrap_or_else(|_| fail(format!("cannot open replay {}", file)));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| fail(format!("cannot read replay {}", path.display())));
    let directive = |key: &str| {
        text.lines().filter_map(|l| l.strip_prefix("#@ ")).filter_map(|l| l.strip_prefix(key)).filter_map(|l| l.strip_prefix('=')).last().map(|v| v.to_string())
    };
    let home = std::env::temp_dir().join(format!("smw-watch-{}", std::process::id()));
    let settings = home.join("Library/Preferences/.smw");
    std::fs::create_dir_all(&settings).unwrap_or_else(|e| fail(format!("cannot create {}: {}", settings.display(), e)));
    for (key, file) in [("options_b64", "options.bin"), ("controls_b64", "controls.sdl2.bin")] {
        if let Some(bytes) = directive(key).and_then(|v| base64_decode(&v)) {
            let _ = std::fs::write(settings.join(file), bytes);
        }
    }
    std::env::set_var("HOME", &home);
    unsafe { watch_home = Some(home) };
    std::env::set_var("SMW_REPLAY", &path);
    std::env::set_var("SMW_SEED", directive("seed").unwrap_or_else(|| "1".to_string()));
    // A session that ended without finish() (killed, or a browser tab closed) has no frames= line:
    // play it to its last input and a few seconds beyond.
    let frames = directive("frames").unwrap_or_else(|| {
        let last = text.lines().filter(|l| !l.trim_start().starts_with('#')).filter_map(|l| l.split_whitespace().next()?.parse::<u32>().ok()).max();
        (last.unwrap_or(0) + UNFINISHED_TAIL_FRAMES).to_string()
    });
    std::env::set_var("SMW_FRAMES", frames);
    if let Some(map) = directive("map") {
        std::env::set_var("SMW_MAP", map);
    }
    std::env::set_var("SMW_AUDIBLE", "1");
    if let Some(s) = speed {
        std::env::set_var("SMW_REPLAY_SPEED", s.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips_like_coreutils() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        let bytes: Vec<u8> = (0..=255u8).collect();
        assert_eq!(base64_decode(&base64_encode(&bytes)).unwrap(), bytes);
        assert!(base64_decode("not base64!").is_none());
    }

    #[test]
    fn timestamps_sort_chronologically() {
        let t = utc_timestamp();
        assert_eq!(t.len(), "2026-10-03_223716".len());
        assert_eq!(&t[4..5], "-");
        assert_eq!(&t[10..11], "_");
    }
}
