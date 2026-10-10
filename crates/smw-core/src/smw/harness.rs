//! Port of src/smw/Harness.cpp from tools/cpp-harness.patch (not in upstream C++).
//!
//! Deterministic replay/dump harness. Every entry point is a no-op unless the corresponding
//! SMW_* environment variable is set; docs/REPLAY.md is the byte-level spec.

use crate::common::game_values::AppState;
use crate::common::global_constants::WAITTIME;
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType};
use crate::common::sfx;
use crate::globals::*;
use crate::smw::game_state::GameStateManager;
use crate::smw::gs_gameplay::GameplayState;
use crate::smw::gs_menu::MenuState;
use crate::smw::gs_splash_screen::SplashScreenState;
use crate::smw::network::file_compressor::{deflate, inflate, read_maybe_gzip};
use sdl2::sys::{
    SDL_Event, SDL_EventType, SDL_GetError, SDL_GetKeyFromName, SDL_GetKeyName, SDL_GetScancodeFromKey, SDL_InitSubSystem, SDL_JoystickAttachVirtual,
    SDL_JoystickInstanceID, SDL_JoystickType, SDL_KeyCode, SDL_Keymod, SDL_SetHint, SDL_INIT_JOYSTICK,
    SDL_PRESSED, SDL_RELEASED,
};
use smw_platform::{Input, InputEvent, Keycode, PadSlot};
use std::ffi::CStr;
use std::collections::BTreeSet;
use std::ffi::CString;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};

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

const VIRTUAL_AXES: i32 = 64;
const VIRTUAL_BUTTONS: i32 = 64;
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
    shotEvery: u32,
    shotFrom: u32,
    shotTo: u32,
    shotStream: Option<BufWriter<File>>,
    events: Vec<ReplayEvent>,
    joysticks: i32,
    /// `jadd`/`jremove` lines: (frame, device, added).
    devices: Vec<(u32, i32, bool)>,
    nextDevice: usize,
    /// `#@ touch=1`: the session had Android's touch controls.
    touch: bool,
    replay: bool,
    nextEvent: usize,
    frame: u32,
    audible: bool,
    speed: f32,
    rec: Option<Recorder>,
    segment: Option<Segment>,
}

/// A segment replay (SMW_SEGMENT, `--segment`, or a clip's `#@ segment=`): one match of the replay,
/// started from its checkpoint (smw/checkpoint.rs).
struct Segment {
    checkpoint: Vec<u8>,
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
    shotEvery: 0,
    shotFrom: 0,
    shotTo: u32::MAX,
    shotStream: None,
    events: Vec::new(),
    joysticks: 0,
    devices: Vec::new(),
    nextDevice: 0,
    touch: false,
    replay: false,
    nextEvent: 0,
    frame: 0,
    audible: false,
    speed: 1.0,
    rec: None,
    segment: None,
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

/// A recording's text: plain, or gzipped (.smwrp, .txt.gz).
pub fn read_recording(path: impl AsRef<std::path::Path>) -> Option<String> {
    String::from_utf8(read_maybe_gzip(path).ok()?).ok()
}

fn load_events(path: &str) -> Vec<ReplayEvent> {
    let text = read_recording(path).unwrap_or_else(|| fail(format!("cannot open replay {}", path)));
    let mut events: Vec<ReplayEvent> = Vec::new();
    let events = &mut events;
    let mut last_frame = 0;
    // Per device: whether its first line is a `jadd`, which leaves it out of the joysticks attached at launch.
    let mut added_later: [Option<bool>; 8] = [None; 8];
    for (i, raw) in text.split('\n').enumerate() {
        let lineno = i + 1;
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let trimmed = line.trim_start_matches([' ', '\t']);
        if let Some(value) = trimmed.strip_prefix("#@ touch=") {
            unsafe { h.touch = value.trim() == "1" };
        }
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
        if frame < last_frame {
            fail(format!("{}:{}: frames must be non-decreasing", path, lineno));
        }
        last_frame = frame;
        let mut ev = ReplayEvent { frame, kind: EventKind::Key, down: false, keyName: String::new(), device: 0, index: 0, value: 0 };
        if dir == "jadd" || dir == "jremove" {
            let device = match rest.split_whitespace().collect::<Vec<_>>().as_slice() {
                [d] => d.parse::<i32>().ok().filter(|d| (0..=7).contains(d)),
                _ => None,
            };
            let Some(device) = device else { fail(format!("{}:{}: expected '<frame> {} <dev>'", path, lineno, dir)) };
            added_later[device as usize].get_or_insert(dir == "jadd");
            unsafe {
                h.devices.push((frame, device, dir == "jadd"));
            }
            continue;
        }
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
            added_later[ev.device as usize].get_or_insert(false);
        } else {
            ev.keyName = rest.trim_start_matches([' ', '\t']).to_string();
            if (dir != "down" && dir != "up") || ev.keyName.is_empty() {
                fail(format!("{}:{}: expected '<frame> <down|up> <key>'", path, lineno));
            }
            ev.down = dir == "down";
        }
        events.push(ev);
    }
    unsafe {
        h.joysticks = (0..8).filter(|&d| added_later[d as usize] == Some(false)).map(|d| d + 1).max().unwrap_or(0);
    }
    std::mem::take(events)
}

#[cfg(not(target_os = "emscripten"))]
fn attach_joysticks() {
    unsafe {
        SDL_InitSubSystem(SDL_INIT_JOYSTICK);
        for i in 0..h.joysticks {
            if attach_virtual() != i {
                fail(format!("cannot attach virtual joystick {}: {}", i, CStr::from_ptr(SDL_GetError()).to_string_lossy()));
            }
        }
    }
}

/// Attaches a replay's virtual joystick; its SDL device index, or -1.
#[cfg(not(target_os = "emscripten"))]
pub fn attach_virtual() -> i32 {
    unsafe { SDL_JoystickAttachVirtual(SDL_JoystickType::SDL_JOYSTICK_TYPE_GAMECONTROLLER, VIRTUAL_AXES, VIRTUAL_BUTTONS, VIRTUAL_HATS) }
}

/// Not in the C++. A replay's `jadd`/`jremove` lines for this frame: (device, connected).
pub fn replay_device_changes() -> Vec<(usize, bool)> {
    let mut changes = Vec::new();
    unsafe {
        while h.nextDevice < h.devices.len() && h.devices[h.nextDevice].0 <= h.frame {
            let (frame, device, added) = h.devices[h.nextDevice];
            if frame == h.frame {
                changes.push((device as usize, added));
            }
            h.nextDevice += 1;
        }
    }
    changes
}

/// Not in the C++. While recording, a `jadd`/`jremove` line for a pad connected or disconnected before this frame.
pub fn record_device(index: usize, added: bool) {
    let Some(r) = rec().filter(|r| r.live) else { return };
    if index < 8 {
        let _ = writeln!(r.out, "{} {} {}", unsafe { h.frame }, if added { "jadd" } else { "jremove" }, index);
    }
}

/// The normalized event a replay line pushes, before the backend gives it a timestamp.
fn input_for(ev: &ReplayEvent) -> InputEvent {
    let pad = PadSlot(ev.device);
    match ev.kind {
        EventKind::Key => unsafe {
            let cname = CString::new(ev.keyName.as_bytes()).unwrap();
            let key = SDL_GetKeyFromName(cname.as_ptr());
            if key == SDL_KeyCode::SDLK_UNKNOWN as i32 {
                fail(format!("unknown SDL key name '{}'", ev.keyName));
            }
            let scancode = SDL_GetScancodeFromKey(key) as i32;
            InputEvent::Key { key: Keycode(key), scancode, mods: SDL_Keymod::KMOD_NONE as u16, down: ev.down, repeat: false, window: 0 }
        },
        EventKind::JoyAxis => InputEvent::PadAxis { pad, axis: ev.index as u8, value: ev.value as i16 },
        EventKind::JoyButton => InputEvent::PadButton { pad, button: ev.index as u8, down: ev.value != 0 },
        EventKind::JoyHat => InputEvent::PadHat { pad, hat: ev.index as u8, value: ev.value as u8 },
    }
}

fn fill_event(ev: &ReplayEvent, event: &mut SDL_Event) {
    *event = smw_sdl2::input::to_sdl(&Input { timestamp: 0, event: input_for(ev) });
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

static mut spawn_events: Vec<String> = Vec::new();

/// The keyboard as the replayed key events left it; pushed events do not update SDL's own state.
static mut replay_keys: [u8; sdl2::sys::SDL_Scancode::SDL_NUM_SCANCODES as usize] = [0; sdl2::sys::SDL_Scancode::SDL_NUM_SCANCODES as usize];

fn note_replay_key(event: &SDL_Event) {
    unsafe {
        let t = event.type_;
        if t == SDL_EventType::SDL_KEYDOWN as u32 || t == SDL_EventType::SDL_KEYUP as u32 {
            replay_keys[event.key.keysym.scancode as usize] = (t == SDL_EventType::SDL_KEYDOWN as u32) as u8;
        }
    }
}

/// `SDL_GetKeyboardState(NULL)` for the game; while replaying, the replayed keys, so Shift held while typing
/// in a text field replays as it was recorded.
pub fn keyboard_state() -> *const u8 {
    unsafe {
        if h.replay {
            (&raw const replay_keys).cast()
        } else {
            sdl2::sys::SDL_GetKeyboardState(std::ptr::null_mut())
        }
    }
}

/// Net games only (not in the C++ harness): a `C` line, compared across clients.
pub fn note_net(line: String) {
    unsafe {
        if crate::smw::net::netplay.active && h.dump.is_some() {
            spawn_events.push(format!("C {}", line));
        }
    }
}

pub fn note_powerup_spawn(iType: i16, x: i16, y: i16) {
    note_net(format!("powerup type={} x={} y={}", iType, x, y));
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

    for line in unsafe { sfx::sfx_events.iter().chain(spawn_events.iter()) } {
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
            sfx::sfx_log_events = true;
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
            load_segment(&replay);
            // A segment starts mid-session with every joystick the replay uses.
            if h.segment.is_some() {
                h.joysticks = h.joysticks.max(h.devices.iter().map(|&(_, d, _)| d + 1).max().unwrap_or(0));
                h.devices.clear();
            }
        }

        h.audible = record || env("SMW_AUDIBLE").is_some();
        // Unseeded runs (SMW_NO_RECORD) are live play, so they are audible too.
        sfx::sfx_audible = h.audible || !h.seeded;
        if let Some(speed) = env("SMW_REPLAY_SPEED").and_then(|v| v.parse::<f32>().ok()).filter(|v| *v > 0.0) {
            h.speed = speed;
        }
        if record {
            start_recording(h.seed, None);
        } else if let Some(path) = env("SMW_RECORD_TO").filter(|_| h.replay && h.segment.is_none()) {
            start_recording(h.seed, Some(path));
        }

        // Not in upstream: a connected pad must not join a replay's joysticks.
        #[cfg(not(target_os = "emscripten"))]
        if h.replay {
            for hint in [c"SDL_JOYSTICK_HIDAPI", c"SDL_JOYSTICK_MFI", c"SDL_JOYSTICK_IOKIT"] {
                SDL_SetHint(hint.as_ptr(), c"0".as_ptr());
            }
        }

        // The browser's SDL has no virtual joysticks; there init_joysticks takes the count from replay_joysticks.
        #[cfg(not(target_os = "emscripten"))]
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
        if let Some(range) = env("SMW_SHOT_RANGE") {
            let (from, to) = range.split_once('-').unwrap_or((&range, ""));
            h.shotFrom = strtol10(from) as u32;
            if !to.is_empty() {
                h.shotTo = strtol10(to) as u32;
            }
            h.shotEvery = 1;
        }
        if let Some(every) = env("SMW_SHOT_EVERY") {
            h.shotEvery = strtol10(&every).max(1) as u32;
        }
        if let Some(stream) = env("SMW_SHOT_STREAM") {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&stream)
                .unwrap_or_else(|_| fail(format!("cannot open shot stream {}", stream)));
            h.shotStream = Some(BufWriter::with_capacity(640 * 480 * 4, file));
        }
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

/// Not in upstream: in a browser replay, the number of joysticks the replay uses. They exist only as
/// the device index its events carry (`which`), which is what the game compares with its bindings.
pub fn replay_joysticks() -> Option<i16> {
    #[cfg(target_os = "emscripten")]
    unsafe {
        if h.replay {
            return Some(h.joysticks as i16);
        }
    }
    None
}

/// Marks the recording as made with Android's touch controls on, before its first frame.
pub fn record_touch() {
    if let Some(r) = rec().filter(|r| r.live) {
        let _ = writeln!(r.out, "#@ touch=1");
    }
}

/// Whether the replay was recorded with Android's touch controls on (`touch::touch_first`).
pub fn replay_touch() -> bool {
    unsafe { h.replay && h.touch }
}

pub fn replaying() -> bool {
    unsafe { h.replay }
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
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::pad::debug_frame(unsafe { h.frame });
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::pad::phase(2);
    sfx::sfx_virtual_advance();
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::pad::phase(3);
    record_frame_start(unsafe { h.frame });
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::pad::phase(4);

    unsafe {
        while h.nextEvent < h.events.len() && h.events[h.nextEvent].frame <= h.frame {
            if h.events[h.nextEvent].frame == h.frame {
                let mut event: SDL_Event = std::mem::zeroed();
                fill_event(&h.events[h.nextEvent], &mut event);
                note_replay_key(&event);
                smw_sdl2::events::push(&mut event);
                copy_replay_event(&h.events[h.nextEvent]);
            }
            h.nextEvent += 1;
        }
    }
}

/// Blocking waits take the next replay event immediately instead of waiting for input.
pub fn wait_event(event: &mut SDL_Event) {
    unsafe {
        if h.rec.as_ref().is_some_and(|r| r.live) {
            record_wait_event(event);
            return;
        }
        if !h.replay {
            smw_sdl2::events::wait(event);
            return;
        }
        if h.nextEvent >= h.events.len() {
            fail(format!("blocking wait at frame {} but the replay has no events left", h.frame));
        }
        fill_event(&h.events[h.nextEvent], event);
        note_replay_key(event);
        copy_replay_event(&h.events[h.nextEvent]);
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
        spawn_events.clear();
        record_markers(h.frame);

        let f = h.frame;
        let periodic = h.shotEvery > 0 && f >= h.shotFrom && f <= h.shotTo && (f - h.shotFrom) % h.shotEvery == 0;
        if periodic {
            if let Some(out) = h.shotStream.as_mut() {
                if crate::common::gfx::gfx_write_screen_raw(out).and_then(|_| out.flush()).is_err() {
                    fail(format!("cannot write shot stream at frame {}", f));
                }
            }
        }
        if h.shotFrames.contains(&f) || (periodic && h.shotStream.is_none()) {
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
// so the session can be watched again or replayed on the C++ reference. See docs/REPLAY.md, "Recordings".
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
    /// SMW_LIVE_SCRIPT: replay-format lines pushed into SDL's queue as if the OS delivered them.
    script: Vec<ReplayEvent>,
    nextScript: usize,
    /// False for SMW_RECORD_TO, which records a replay's own events instead of OS input.
    live: bool,
    /// Markers: the last state written, the number of the current match and its checkpoint, if any.
    state: String,
    matches: u32,
    checkpoint: Option<String>,
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
/// `to` is SMW_RECORD_TO: a replay's own events, markers and checkpoints go to that file instead.
fn start_recording(seed: u32, to: Option<String>) {
    let dir = crate::common::path::get_home_directory() + "replays/";
    let stamp = utc_timestamp();
    let live = to.is_none();
    let path = if let Some(path) = to {
        path
    } else {
        if std::fs::create_dir_all(&dir).is_err() {
            eprintln!("[harness] cannot create {}; not recording", dir);
            return;
        }
        let mut path = format!("{}{}.txt", dir, stamp);
        let mut n = 1;
        while std::path::Path::new(&path).exists() {
            n += 1;
            path = format!("{}{}_{}.txt", dir, stamp, n);
        }
        path
    };
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
    if let Some(map) = forced_map() {
        let _ = writeln!(out, "#@ map={}", map);
    }
    let _ = out.flush();
    println!("[harness] recording to {}", path);

    let script = if live { env("SMW_LIVE_SCRIPT").map(|p| load_events(&p)).unwrap_or_default() } else { Vec::new() };
    unsafe {
        h.rec = Some(Recorder {
            out,
            path,
            pending: Vec::new(),
            quit: false,
            quitFrame: None,
            script,
            nextScript: 0,
            live,
            state: String::new(),
            matches: 0,
            checkpoint: None,
        });
        if live {
            smw_sdl2::events::set_filter(Some(record_filter), std::ptr::null_mut());
        }
    }
    if live {
        prune_recordings(&dir);
    }
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
    if INJECTING.load(Ordering::Relaxed) || !is_input_event((*event).type_) {
        return 1;
    }
    #[cfg(not(target_os = "emscripten"))]
    if !crate::smw::pad::translate(&mut *event) {
        return 0;
    }
    r.pending.push(*event);
    0
}

/// The replay line for a raw SDL event, or None for input the replay format cannot express.
fn to_replay_event(raw: &SDL_Event, frame: u32) -> Option<ReplayEvent> {
    let mut ev = ReplayEvent { frame, kind: EventKind::Key, down: false, keyName: String::new(), device: 0, index: 0, value: 0 };
    let (which, kind, index, value) = match smw_sdl2::input::from_sdl(raw).event {
        InputEvent::Key { key, down, .. } => unsafe {
            let name = CStr::from_ptr(SDL_GetKeyName(key.0)).to_string_lossy().into_owned();
            if name.is_empty() || SDL_GetKeyFromName(CString::new(name.as_bytes()).ok()?.as_ptr()) != key.0 {
                return None;
            }
            ev.down = down;
            ev.keyName = name;
            return Some(ev);
        },
        InputEvent::PadAxis { pad, axis, value } => (pad.0, EventKind::JoyAxis, axis as i32, value as i32),
        InputEvent::PadHat { pad, hat, value } => (pad.0, EventKind::JoyHat, hat as i32, value as i32),
        InputEvent::PadButton { pad, button, down } => (pad.0, EventKind::JoyButton, button as i32, down as i32),
        _ => return None,
    };
    ev.device = device_index(which)?;
    ev.kind = kind;
    ev.index = index;
    ev.value = value;
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

/// Set while the recorder pushes an event, so its filter lets that event through. Not a Recorder field:
/// the compiler may drop a store through `&mut Recorder` that only the filter callback reads.
static INJECTING: AtomicBool = AtomicBool::new(false);

fn inject(event: &mut SDL_Event) {
    INJECTING.store(true, Ordering::Relaxed);
    unsafe { smw_sdl2::events::push(event) };
    INJECTING.store(false, Ordering::Relaxed);
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
                smw_sdl2::events::push(&mut event);
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

fn record_event(r: &mut Recorder, ev: &ReplayEvent) {
    let _ = writeln!(r.out, "{}", line_for(ev));
    let mut event: SDL_Event = unsafe { std::mem::zeroed() };
    fill_event(ev, &mut event);
    inject(&mut event);
}

/// Not in upstream. Browsers report a standard-mapping pad's D-pad as buttons 12-15 (up, down, left,
/// right) and desktop SDL as hat 0, so each of those button lines is preceded by the hat line it implies.
#[cfg(target_os = "emscripten")]
mod web_dpad {
    use super::{EventKind, ReplayEvent};
    use sdl2::sys::{SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP};

    extern "C" {
        fn emscripten_run_script_int(script: *const std::ffi::c_char) -> i32;
    }

    /// Bit d set: open joystick d is a standard-mapping pad. SDL opens the connected pads in page order.
    static mut standard: i32 = 0;
    static mut hats: [i32; 8] = [0; 8];

    pub fn init() {
        unsafe {
            standard = emscripten_run_script_int(
                c"(navigator.getGamepads ? Array.prototype.filter.call(navigator.getGamepads(), Boolean) : []).reduce((m, p, i) => p.mapping === 'standard' ? m | (1 << i) : m, 0)".as_ptr(),
            );
        }
    }

    pub fn hat_for(ev: &ReplayEvent) -> Option<ReplayEvent> {
        if ev.kind != EventKind::JoyButton || !(12..=15).contains(&ev.index) || unsafe { standard } & (1 << ev.device) == 0 {
            return None;
        }
        let bit = [SDL_HAT_UP, SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT][(ev.index - 12) as usize] as i32;
        let hat = unsafe { &mut hats[ev.device as usize] };
        if ev.value != 0 {
            *hat |= bit;
        } else {
            *hat &= !bit;
        }
        Some(ReplayEvent { frame: ev.frame, kind: EventKind::JoyHat, down: false, keyName: String::new(), device: ev.device, index: 0, value: *hat })
    }
}

/// Frame start while recording: every joystick open at frame 0 gets a centred-hat line so a replay
/// attaches the same number of joysticks; then held input becomes replay lines pushed in order.
fn record_frame_start(frame: u32) {
    let Some(r) = rec().filter(|r| r.live) else { return };
    if frame == 0 {
        for d in 0..unsafe { crate::common::global::joystickcount }.min(8) as i32 {
            let ev = ReplayEvent { frame: 0, kind: EventKind::JoyHat, down: false, keyName: String::new(), device: d, index: 0, value: 0 };
            record_event(r, &ev);
        }
        #[cfg(target_os = "emscripten")]
        web_dpad::init();
    }
    // A browser tab can close at any moment without finish(), and muOS kills apps; keep the file current.
    if frame % 60 == 0 {
        let _ = r.out.flush();
    }
    push_live_script(r, frame);
    smw_sdl2::events::pump();
    let held = std::mem::take(&mut r.pending);
    for raw in held.iter() {
        if unsafe { raw.type_ } == SDL_EventType::SDL_QUIT as u32 {
            r.quit = true;
            continue;
        }
        if let Some(ev) = to_replay_event(raw, frame) {
            #[cfg(target_os = "emscripten")]
            if let Some(hat) = web_dpad::hat_for(&ev) {
                record_event(r, &hat);
            }
            record_event(r, &ev);
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
        #[cfg(not(target_os = "emscripten"))]
        crate::smw::touch::flush();
        let Some(r) = rec() else { return };
        push_live_script(r, unsafe { h.frame } + 1);
        smw_sdl2::events::pump();
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
        crate::services::delay(5);
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
            if r.live {
                smw_sdl2::events::set_filter(None, std::ptr::null_mut());
            }
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
pub fn prepare_watch(file: &str, speed: Option<f32>, segment: Option<u32>) {
    let path = std::fs::canonicalize(file).unwrap_or_else(|_| fail(format!("cannot open replay {}", file)));
    let text = read_recording(&path).unwrap_or_else(|| fail(format!("cannot read replay {}", path.display())));
    let directive = |key: &str| {
        text.lines().filter_map(|l| l.strip_prefix("#@ ")).filter_map(|l| l.strip_prefix(key)).filter_map(|l| l.strip_prefix('=')).last().map(|v| v.to_string())
    };
    let home = std::env::temp_dir().join(format!("smw-watch-{}", std::process::id()));
    #[cfg(not(windows))]
    let settings = home.join("Library/Preferences/.smw");
    #[cfg(windows)]
    let settings = home.join(".smw");
    std::fs::create_dir_all(&settings).unwrap_or_else(|e| fail(format!("cannot create {}: {}", settings.display(), e)));
    for (key, file) in [("options_b64", "options.bin"), ("controls_b64", "controls.sdl2.bin")] {
        if let Some(bytes) = directive(key).and_then(|v| base64_decode(&v)) {
            let _ = std::fs::write(settings.join(file), bytes);
        }
    }
    std::env::set_var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }, &home);
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
    if let Some(k) = segment {
        std::env::set_var("SMW_SEGMENT", k.to_string());
    }
}

//------------------------------------------------------------------------------------------------
// Markers, checkpoints and segment replays (not in the C++). A recording marks each change of game
// state with a `#@ mark` line and saves a `#@ checkpoint` at the start of each match, from which a
// segment replay starts that match directly. See docs/REPLAY.md, "Markers, checkpoints and clips".
//------------------------------------------------------------------------------------------------

/// A replay's event, written to its SMW_RECORD_TO recording as it is pushed.
fn copy_replay_event(ev: &ReplayEvent) {
    if let Some(r) = rec().filter(|r| !r.live) {
        let _ = writeln!(r.out, "{}", line_for(ev));
    }
}

pub fn note_match_map(path: &str) {
    crate::smw::checkpoint::note_match_map(path);
}

/// Called where a match starts (`MenuState::enter_gameplay`): the next marker opens a new match.
pub fn match_checkpoint() {
    let Some(r) = rec() else { return };
    r.matches += 1;
    r.checkpoint = Some(match crate::smw::checkpoint::unsupported_reason() {
        Some(reason) => format!("#@ checkpoint match={} frame={} unsupported={}", r.matches, unsafe { h.frame }, reason),
        None => format!("#@ checkpoint match={} frame={} z64={}", r.matches, unsafe { h.frame }, base64_encode(&deflate(&crate::smw::checkpoint::capture()))),
    });
}

/// The game state markers name: `splash`, `menu`, `worldmap`, `gameplay`, `scoreboard` (gameplay after
/// the game is over) or `other`.
fn marker_state() -> &'static str {
    match state_name() {
        "menu" if MenuState::instance().harness_menu_name() == "world" => "worldmap",
        "gameplay" if unsafe { crate::common::global::game_values.gamemode.gameover } => "scoreboard",
        name => name,
    }
}

/// Frame end while recording: a `#@ mark` line when the state changed this frame, and when a match
/// started, its checkpoint.
fn record_markers(frame: u32) {
    let Some(r) = rec() else { return };
    let state = marker_state();
    if state == r.state {
        return;
    }
    let was_match = r.state == "gameplay" || r.state == "scoreboard";
    r.state = state.to_string();
    let mut line = format!("#@ mark frame={} state={}", frame, state);
    if state == "gameplay" && !was_match {
        line += &format!(" match={} {}", r.matches, crate::smw::checkpoint::start_fields());
        let checkpoint = r.checkpoint.take();
        let _ = writeln!(r.out, "{}", line);
        if let Some(checkpoint) = checkpoint {
            let _ = writeln!(r.out, "{}", checkpoint);
        }
        let _ = r.out.flush();
        return;
    }
    if state == "scoreboard" || was_match {
        line += &format!(" match={} {}", r.matches, crate::smw::checkpoint::result_fields());
    }
    let _ = writeln!(r.out, "{}", line);
}

/// `#@ key=value` fields of a directive line, in order (values may be double-quoted).
fn directive_fields(line: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let mut rest = line.trim();
    let first = rest.split(' ').next().unwrap_or("");
    if !first.contains('=') {
        fields.push((first.to_string(), String::new()));
        rest = rest[first.len()..].trim_start();
    }
    while !rest.is_empty() {
        let Some(eq) = rest.find('=') else { break };
        let key = rest[..eq].trim().to_string();
        rest = &rest[eq + 1..];
        let value = if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted.find('"').unwrap_or(quoted.len());
            let v = quoted[..end].to_string();
            rest = quoted.get(end + 1..).unwrap_or("");
            v
        } else {
            let end = rest.find(' ').unwrap_or(rest.len());
            let v = rest[..end].to_string();
            rest = &rest[end..];
            v
        };
        fields.push((key, value));
        rest = rest.trim_start();
    }
    fields
}

/// Segment mode: SMW_SEGMENT=<k>, or `#@ segment=<k>` in the replay (a clip). Finds checkpoint k,
/// starts the frame counter at its frame and ends the replay where the match left gameplay.
fn load_segment(path: &str) {
    let text = read_recording(path).unwrap_or_default();
    let directives: Vec<Vec<(String, String)>> = text.lines().filter_map(|l| l.strip_prefix("#@ ")).map(directive_fields).collect();
    let field = |d: &[(String, String)], key: &str| d.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
    let wanted = env("SMW_SEGMENT").or_else(|| directives.iter().filter(|d| d.len() == 1).find_map(|d| field(d, "segment")));
    let Some(wanted) = wanted else { return };
    let checkpoint = directives.iter().find(|d| d.first().is_some_and(|(k, v)| k == "checkpoint" && v.is_empty()) && field(d, "match").as_deref() == Some(wanted.as_str()));
    let Some(checkpoint) = checkpoint else {
        fail(format!("{} has no checkpoint for match {}", path, wanted));
    };
    let start: u32 = field(checkpoint, "frame").and_then(|v| v.parse().ok()).unwrap_or_else(|| fail("checkpoint without frame=".to_string()));
    if let Some(reason) = field(checkpoint, "unsupported") {
        fail(format!("match {} has no checkpoint: {} matches are not supported", wanted, reason));
    }
    let bytes = checkpoint_bytes(checkpoint).unwrap_or_else(|e| fail(format!("match {}: {}", wanted, e)));
    let end = directives
        .iter()
        .filter(|d| d.first().is_some_and(|(k, v)| k == "mark" && v.is_empty()))
        .filter_map(|d| Some((field(d, "frame")?.parse::<u32>().ok()?, field(d, "state")?)))
        .find(|(f, state)| *f > start && state != "gameplay" && state != "scoreboard")
        .map(|(f, _)| f);
    unsafe {
        h.frame = start;
        if let Some(end) = end {
            h.maxFrames = if h.maxFrames >= 0 { h.maxFrames.min(end as i64) } else { end as i64 };
        }
        // The menu consumed this frame's input before the checkpoint.
        while h.nextEvent < h.events.len() && h.events[h.nextEvent].frame <= start {
            h.nextEvent += 1;
        }
        h.segment = Some(Segment { checkpoint: bytes });
    }
}

/// A checkpoint line's binary: `z64=` (zlib, then base64) or, in older recordings, `b64=` (base64).
fn checkpoint_bytes(d: &[(String, String)]) -> Result<Vec<u8>, String> {
    let mut bytes = None;
    for (key, value) in d.iter().skip(1) {
        bytes = match key.as_str() {
            "match" | "frame" => continue,
            "b64" => base64_decode(value),
            "z64" => base64_decode(value).and_then(|z| inflate(&z)),
            _ => return Err(format!("checkpoint has an unknown field {}= (this build reads b64= and z64=)", key)),
        };
        if bytes.is_none() {
            return Err(format!("checkpoint {}= is damaged", key));
        }
    }
    bytes.ok_or_else(|| "checkpoint without z64= or b64=".to_string())
}

/// The state a segment replay starts in, if this is one.
pub fn segment_state() -> Option<crate::smw::checkpoint::SegmentState> {
    unsafe { h.segment.as_mut().map(|s| crate::smw::checkpoint::SegmentState { checkpoint: std::mem::take(&mut s.checkpoint), _alias: Aliased::new() }) }
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
    fn directive_fields_split_words_and_quotes() {
        let f = directive_fields("mark frame=12 state=gameplay map=\"Wacky Woods\" p1=pad0,team1,Mario");
        let pairs: Vec<(&str, &str)> = f.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(pairs, [("mark", ""), ("frame", "12"), ("state", "gameplay"), ("map", "Wacky Woods"), ("p1", "pad0,team1,Mario")]);
        assert_eq!(directive_fields("segment=2"), [("segment".to_string(), "2".to_string())]);
    }

    #[test]
    fn checkpoints_read_old_and_new_encodings() {
        let read = |name: &str| -> Vec<Vec<u8>> {
            let text = std::fs::read_to_string(format!("{}/../../tools/checkpoint_fixtures/{}", env!("CARGO_MANIFEST_DIR"), name)).unwrap();
            text.lines().filter_map(|l| l.strip_prefix("#@ checkpoint ")).map(|l| checkpoint_bytes(&directive_fields(&format!("checkpoint {}", l))).unwrap()).collect()
        };
        let old = read("web_gamepad_ztar_b64.txt");
        assert_eq!(old.len(), 1);
        assert_eq!(&old[0][..5], b"SMWC\x01");
        assert_eq!(read("web_gamepad_ztar_z64.txt"), old);

        let err = |line: &str| checkpoint_bytes(&directive_fields(line)).unwrap_err();
        assert!(err("checkpoint match=1 frame=9 x64=AAAA").contains("unknown field x64="));
        assert!(err("checkpoint match=1 frame=9 z64=bm90IHpsaWI=").contains("z64= is damaged"));
        assert!(err("checkpoint match=1 frame=9").contains("without z64= or b64="));
    }

    #[test]
    fn timestamps_sort_chronologically() {
        let t = utc_timestamp();
        assert_eq!(t.len(), "2026-10-03_223716".len());
        assert_eq!(&t[4..5], "-");
        assert_eq!(&t[10..11], "_");
    }
}
