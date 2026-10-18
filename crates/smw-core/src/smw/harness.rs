//! Port of src/smw/Harness.cpp from tools/cpp-harness.patch (not in upstream C++).
//!
//! Deterministic replay/dump harness. Every entry point is a no-op unless the corresponding
//! SMW_* environment variable is set; docs/REPLAY.md is the byte-level spec.

use crate::common::keys::*;
use crate::game::Game;
use crate::common::game_values::AppState;
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType};
use crate::common::sfx;
use crate::globals::*;
use crate::smw::game_state::GameStateManager;
use crate::smw::gs_gameplay::GameplayState;
use crate::smw::gs_menu::MenuState;
use crate::smw::gs_splash_screen::SplashScreenState;
use crate::smw::network::file_compressor::{deflate, inflate, read_maybe_gzip};
use smw_platform::{Input, InputEvent, Keycode, PadSlot, SCREEN_H, SCREEN_W};
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
    Tap,
    /// `<frame> await <name>`: later events wait until that menu or state is current.
    Await,
}

#[derive(Clone)]
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

pub struct Harness {
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
    /// Frames the script's `await` lines have waited so far; later events and SMW_FRAMES move by it.
    shift: u32,
    /// States and menus current since the last satisfied `await`, so one that has already passed still satisfies it.
    seen: Vec<&'static str>,
    rec: Option<Recorder>,
    segment: Option<Segment>,
    spawn_events: Vec<String>,
    /// The keyboard as the replayed key events left it; pushed events do not update SDL's own state.
    replay_keys: [u8; crate::common::keys::SDL_NUM_SCANCODES],
    watch_home: Option<std::path::PathBuf>,
    /// Bit d set: open joystick d is a standard-mapping pad. SDL opens the connected pads in page order.
    #[cfg(target_os = "emscripten")]
    web_standard: i32,
    #[cfg(target_os = "emscripten")]
    web_hats: [i32; 8],
}

/// A segment replay (SMW_SEGMENT, `--segment`, or a clip's `#@ segment=`): one match of the replay,
/// started from its checkpoint (smw/checkpoint.rs).
struct Segment {
    checkpoint: Vec<u8>,
}

impl Harness {
    /// The replay's input and counters for a copy of the match (`Game::clone_match`), without its outputs:
    /// no dump, screenshots, recording or segment.
    pub fn clone_inputs(&self) -> Self {
        Harness {
            seeded: self.seeded,
            seed: self.seed,
            noLimit: self.noLimit,
            map: self.map.clone(),
            maxFrames: self.maxFrames,
            events: self.events.clone(),
            joysticks: self.joysticks,
            devices: self.devices.clone(),
            nextDevice: self.nextDevice,
            touch: self.touch,
            replay: self.replay,
            nextEvent: self.nextEvent,
            frame: self.frame,
            audible: false,
            speed: self.speed,
            spawn_events: self.spawn_events.clone(),
            replay_keys: self.replay_keys,
            watch_home: self.watch_home.clone(),
            #[cfg(target_os = "emscripten")]
            web_standard: self.web_standard,
            #[cfg(target_os = "emscripten")]
            web_hats: self.web_hats,
            ..Harness::new()
        }
    }

    pub const fn new() -> Self {
        Harness {
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
            shift: 0,
            seen: Vec::new(),
            rec: None,
            segment: None,
            spawn_events: Vec::new(),
            replay_keys: [0; crate::common::keys::SDL_NUM_SCANCODES],
            watch_home: None,
            #[cfg(target_os = "emscripten")]
            web_standard: 0,
            #[cfg(target_os = "emscripten")]
            web_hats: [0; 8],
        }
    }
}

fn h(g: &mut Game) -> &mut Harness {
    &mut g.harness
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

pub fn fail(msg: String) -> ! {
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

fn virtual_ticks(g: &mut Game) -> u32 {
    1000u32.wrapping_add((h(g).frame as u64 * 1000 / crate::common::global_constants::TICKS_PER_SECOND as u64) as u32)
}

/// A recording's text: plain, or gzipped (.smwrp, .txt.gz).
pub fn read_recording(path: impl AsRef<std::path::Path>) -> Option<String> {
    String::from_utf8(read_maybe_gzip(path).ok()?).ok()
}

fn load_events(g: &mut Game, path: &str) -> Vec<ReplayEvent> {
    let mut gp = g.ptr();
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
            h(gp.get()).touch = value.trim() == "1";
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
            h(gp.get()).devices.push((frame, device, dir == "jadd"));
            continue;
        }
        if dir == "await" {
            ev.kind = EventKind::Await;
            ev.keyName = rest.trim().to_string();
            if ev.keyName.is_empty() {
                fail(format!("{}:{}: expected '<frame> await <menu|state>'", path, lineno));
            }
        } else if dir == "tap" {
            let nums: Vec<Option<i32>> = rest.split_whitespace().map(|t| t.parse::<i32>().ok()).collect();
            match nums.as_slice() {
                [Some(x), Some(y)] if (0..SCREEN_W as i32).contains(x) && (0..SCREEN_H as i32).contains(y) => {
                    ev.kind = EventKind::Tap;
                    ev.index = *x;
                    ev.value = *y;
                }
                _ => fail(format!("{}:{}: expected '<frame> tap <x> <y>'", path, lineno)),
            }
        } else if dir == "jaxis" || dir == "jbutton" || dir == "jhat" {
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
    h(gp.get()).joysticks = (0..8).filter(|&d| added_later[d as usize] == Some(false)).map(|d| d + 1).max().unwrap_or(0);
    std::mem::take(events)
}

/// Not in the C++. A replay's `jadd`/`jremove` lines for this frame: (device, connected).
pub fn replay_device_changes(g: &mut Game) -> Vec<(usize, bool)> {
    let mut gp = g.ptr();
    let mut changes = Vec::new();
    while h(gp.get()).nextDevice < h(gp.get()).devices.len() && h(gp.get()).devices[h(gp.get()).nextDevice].0 <= h(gp.get()).frame {
        let (frame, device, added) = h(gp.get()).devices[h(gp.get()).nextDevice];
        if frame == h(gp.get()).frame {
            changes.push((device as usize, added));
        }
        h(gp.get()).nextDevice += 1;
    }
    changes
}

/// Not in the C++. While recording, a `jadd`/`jremove` line for a pad connected or disconnected before this frame.
pub fn record_device(g: &mut Game, index: usize, added: bool) {
    let mut gp = g.ptr();
    let Some(r) = rec(g).filter(|r| r.live) else { return };
    if index < 8 {
        let _ = writeln!(r.out, "{} {} {}", h(gp.get()).frame, if added { "jadd" } else { "jremove" }, index);
    }
}

/// The normalized event a replay line pushes, before the backend gives it a timestamp.
fn input_for(g: &mut Game, ev: &ReplayEvent) -> InputEvent {
    let pad = PadSlot(ev.device);
    match ev.kind {
        EventKind::Key => {
            let kb = &crate::services::services(g).keyboard;
            let key = kb.key_from_name(&ev.keyName);
            if key == SDLK_UNKNOWN {
                fail(format!("unknown SDL key name '{}'", ev.keyName));
            }
            let scancode = kb.scancode_from_key(key);
            InputEvent::Key { key: Keycode(key), scancode, mods: KMOD_NONE, down: ev.down, repeat: false, window: 0 }
        }
        EventKind::JoyAxis => InputEvent::PadAxis { pad, axis: ev.index as u8, value: ev.value as i16 },
        EventKind::JoyButton => InputEvent::PadButton { pad, button: ev.index as u8, down: ev.value != 0 },
        EventKind::JoyHat => InputEvent::PadHat { pad, hat: ev.index as u8, value: ev.value as u8 },
        EventKind::Tap => InputEvent::Tap { x: ev.index as i16, y: ev.value as i16 },
        EventKind::Await => InputEvent::Other(0),
    }
}

fn input_of(g: &mut Game, ev: &ReplayEvent) -> Input {
    Input { timestamp: 0, event: input_for(g, ev) }
}

fn state_name(g: &mut Game) -> &'static str {
    let mut gp = g.ptr();
    use crate::smw::game_state::StateId;
    match GameStateManager::instance(gp.get()).currentState {
        StateId::Splash => "splash",
        StateId::Menu => "menu",
        StateId::Gameplay => "gameplay",
        StateId::Segment => "other",
    }
}


fn note_replay_key(g: &mut Game, input: &Input) {
    if let InputEvent::Key { scancode, down, .. } = input.event {
        h(g).replay_keys[scancode as usize] = down as u8;
    }
}

/// `SDL_GetKeyboardState(NULL)` for the game; while replaying, the replayed keys, so Shift held while typing
/// in a text field replays as it was recorded.
pub fn keyboard_state(g: &mut Game) -> *const u8 {
    let mut gp = g.ptr();
    if h(gp.get()).replay {
        (&raw const h(gp.get()).replay_keys).cast()
    } else {
        crate::services::services(gp.get()).keyboard.state().as_ptr()
    }
}

/// Net games only (not in the C++ harness): a `C` line, compared across clients.
pub fn note_net(g: &mut Game, line: String) {
    let mut gp = g.ptr();
    if gp.netplay.active && h(gp.get()).dump.is_some() {
        h(gp.get()).spawn_events.push(format!("C {}", line));
    }
}

pub fn note_powerup_spawn(g: &mut Game, iType: i16, x: i16, y: i16) {
    note_net(g, format!("powerup type={} x={} y={}", iType, x, y));
}

fn dump_frame(g: &mut Game, out: &mut impl Write) -> std::io::Result<()> {
    let mut gp = g.ptr();
    let state = state_name(g);
    writeln!(out, "F {} {}", h(gp.get()).frame, state)?;

    if state == "menu" {
        let menu = MenuState::instance(g);
        writeln!(out, "M {} focus={} modifying={}", menu.harness_menu_name(), menu.harness_focus_index(), menu.harness_modifying() as i32)?;
    } else if state == "gameplay" {
        GameplayState::instance(g).harness_dump(g, out)?;
    }

    for line in gp.sfx.sfx_events.iter().chain(h(gp.get()).spawn_events.iter()) {
        writeln!(out, "{}", line)?;
    }

    writeln!(out, "R calls={} last={}", RandomNumberGenerator::call_count(gp.get()), RandomNumberGenerator::last_value(gp.get()))
}

pub fn init(g: &mut Game) {
    let mut gp = g.ptr();
    unsafe {
        // A normal launch records the session (seeded, so it can be replayed); replays never record.
        let record = env("SMW_REPLAY").is_none() && env("SMW_NO_RECORD").is_none();
        let seed_text = env("SMW_SEED").or_else(|| if record { Some(random_seed().to_string()) } else { None });
        if let Some(seed) = seed_text {
            h(gp.get()).seeded = true;
            h(gp.get()).seed = strtoul0(&seed);
            RandomNumberGenerator::generator(gp.get()).reseed(h(gp.get()).seed);
            srand(h(gp.get()).seed);
            RandomNumberGenerator::reset_call_count(gp.get());
            g.sfx.sfx_ticks = virtual_ticks;
            g.sfx.sfx_ignore_channel_failure = true;
            g.sfx.sfx_log_events = true;
        }

        h(gp.get()).noLimit = env("SMW_NOLIMIT").is_some();

        if let Some(map) = env("SMW_MAP") {
            h(gp.get()).map = map;
        }

        if let Some(frames) = env("SMW_FRAMES") {
            h(gp.get()).maxFrames = strtol10(&frames);
        }

        if let Some(replay) = env("SMW_REPLAY") {
            h(gp.get()).events = load_events(g, &replay);
            h(gp.get()).replay = true;
            load_segment(g, &replay);
            // A segment starts mid-session with every joystick the replay uses.
            if h(gp.get()).segment.is_some() {
                h(gp.get()).joysticks = h(gp.get()).joysticks.max(h(gp.get()).devices.iter().map(|&(_, d, _)| d + 1).max().unwrap_or(0));
                h(gp.get()).devices.clear();
            }
            let from = if h(gp.get()).segment.is_some() { h(gp.get()).frame + 1 } else { 0 };
            crate::smw::network::net_record::load(g, &read_recording(&replay).unwrap_or_default(), from);
        }

        h(gp.get()).audible = record || env("SMW_AUDIBLE").is_some();
        // Unseeded runs (SMW_NO_RECORD) are live play, so they are audible too.
        g.sfx.sfx_audible = h(gp.get()).audible || !h(gp.get()).seeded;
        if let Some(speed) = env("SMW_REPLAY_SPEED").and_then(|v| v.parse::<f32>().ok()).filter(|v| *v > 0.0) {
            h(gp.get()).speed = speed;
        }
        if record {
            start_recording(g, h(gp.get()).seed, None);
        } else if let Some(path) = env("SMW_RECORD_TO").filter(|_| h(gp.get()).replay && h(gp.get()).segment.is_none()) {
            start_recording(g, h(gp.get()).seed, Some(path));
        }

        if let Some(dump) = env("SMW_DUMP") {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&dump)
                .unwrap_or_else(|_| fail(format!("cannot open dump {}", dump)));
            h(gp.get()).dump = Some(BufWriter::new(file));
        }

        if let Some(shots) = env("SMW_SHOT_FRAMES") {
            for item in shots.split(',').filter(|s| !s.is_empty()) {
                h(gp.get()).shotFrames.insert(strtol10(item) as u32);
            }
        }
        h(gp.get()).shotDir = env("SMW_SHOT_DIR").unwrap_or_else(|| ".".to_string());
        if let Some(range) = env("SMW_SHOT_RANGE") {
            let (from, to) = range.split_once('-').unwrap_or((&range, ""));
            h(gp.get()).shotFrom = strtol10(from) as u32;
            if !to.is_empty() {
                h(gp.get()).shotTo = strtol10(to) as u32;
            }
            h(gp.get()).shotEvery = 1;
        }
        if let Some(every) = env("SMW_SHOT_EVERY") {
            h(gp.get()).shotEvery = strtol10(&every).max(1) as u32;
        }
        if let Some(stream) = env("SMW_SHOT_STREAM") {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&stream)
                .unwrap_or_else(|_| fail(format!("cannot open shot stream {}", stream)));
            h(gp.get()).shotStream = Some(BufWriter::with_capacity(640 * 480 * 4, file));
        }
    }
}

/// Dumps to `path` from the next frame on, as SMW_DUMP does from the first.
pub fn dump_to(g: &mut Game, path: &str) {
    let file = File::create(path).unwrap_or_else(|_| fail(format!("cannot open dump {}", path)));
    h(g).dump = Some(BufWriter::new(file));
}

pub fn libc_seed(g: &mut Game, fallback: u32) -> u32 {
    let mut gp = g.ptr();
    if h(gp.get()).seeded {
        h(gp.get()).seed
    } else {
        fallback
    }
}

/// Not in upstream: in a browser replay, the number of joysticks the replay uses. They exist only as
/// the device index its events carry (`which`), which is what the game compares with its bindings.
/// A replay's device count; a replay opens no backend pads.
pub fn replay_joysticks(g: &mut Game) -> Option<usize> {
    h(g).replay.then(|| h(g).joysticks.max(0) as usize)
}

/// Marks the recording as made with Android's touch controls on, before its first frame.
pub fn record_touch(g: &mut Game) {
    if let Some(r) = rec(g).filter(|r| r.live) {
        let _ = writeln!(r.out, "#@ touch=1");
    }
}

/// Whether the replay was recorded with Android's touch controls on (`touch::touch_first`).
pub fn replay_touch(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    h(gp.get()).replay && h(gp.get()).touch
}

pub fn replaying(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    h(gp.get()).replay
}

pub fn frame(g: &mut Game) -> u32 {
    let mut gp = g.ptr();
    h(gp.get()).frame
}

/// The clock net code decides by: frame-based in seeded runs, so a replay reads the recorded session's times.
pub fn net_ticks(g: &mut Game) -> u32 {
    let mut gp = g.ptr();
    if h(gp.get()).seeded {
        virtual_ticks(g)
    } else {
        crate::services::ticks(gp.get())
    }
}

/// A `#@` line for the recording, if one is being made.
pub fn record_line(g: &mut Game, line: &str) {
    if let Some(r) = rec(g) {
        let _ = writeln!(r.out, "{}", line);
    }
}

/// SMW_NET_SEED: the seed a net game's host syncs with instead of the clock, so a scripted net game spawns alike every run.
pub fn net_seed() -> Option<u32> {
    env("SMW_NET_SEED").and_then(|s| s.parse().ok())
}

pub fn no_limit(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    h(gp.get()).noLimit
}

pub fn forced_map(g: &mut Game) -> Option<&'static str> {
    let mut gp = g.ptr();
    if h(gp.get()).map.is_empty() {
        None
    } else {
        Some(h(gp.get()).map.as_str())
    }
}

pub fn frame_start(g: &mut Game) {
    let mut gp = g.ptr();
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::pad::debug_frame(g, h(gp.get()).frame);
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::pad::phase(2);
    sfx::sfx_virtual_advance(g);
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::pad::phase(3);
    record_frame_start(g, h(gp.get()).frame);
    #[cfg(not(target_os = "emscripten"))]
    crate::smw::pad::phase(4);

    note_seen(g);
    let frame = h(gp.get()).frame;
    while let Some(ev) = h(gp.get()).events.get(h(gp.get()).nextEvent).cloned() {
        match due(g, &ev, frame) {
            Due::Later => break,
            Due::Now => {
                let input = input_of(g, &ev);
                note_replay_key(g, &input);
                crate::smw::input::push_ready(g, input);
                copy_replay_event(g, &ReplayEvent { frame, ..ev });
            }
            Due::Past => {}
        }
        h(gp.get()).nextEvent += 1;
    }
}

enum Due {
    Now,
    /// Passed, or an `await` that is satisfied.
    Past,
    Later,
}

/// When a script event plays at `frame`, after the shift its `await` lines have built up. An unsatisfied `await`
/// moves itself and every later event to the next frame.
fn due(g: &mut Game, ev: &ReplayEvent, frame: u32) -> Due {
    let at = ev.frame + h(g).shift;
    if at > frame {
        return Due::Later;
    }
    if ev.kind == EventKind::Await {
        if awaited(g, &ev.keyName) {
            h(g).seen.clear();
            return Due::Past;
        }
        h(g).shift = frame + 1 - ev.frame;
        return Due::Later;
    }
    if at == frame {
        Due::Now
    } else {
        Due::Past
    }
}

/// Whether `name`, a state (`state_name`) or a menu (`MenuState::harness_names`), has been current since the last
/// satisfied `await`.
fn awaited(g: &mut Game, name: &str) -> bool {
    note_seen(g);
    h(g).seen.contains(&name)
}

fn note_seen(g: &mut Game) {
    let state = state_name(g);
    let mut names = vec![state];
    if state == "menu" {
        names.extend(MenuState::instance(g).harness_names());
    }
    for name in names {
        if !h(g).seen.contains(&name) {
            h(g).seen.push(name);
        }
    }
}

/// Blocking waits take the next replay event immediately instead of waiting for input.
pub fn wait_event(g: &mut Game) -> Input {
    let mut gp = g.ptr();
    if h(gp.get()).rec.as_ref().is_some_and(|r| r.live) {
        return record_wait_event(g);
    }
    if !h(gp.get()).replay {
        return crate::services::services(g).events.wait();
    }
    while h(gp.get()).events.get(h(gp.get()).nextEvent).is_some_and(|ev| ev.kind == EventKind::Await) {
        h(gp.get()).nextEvent += 1;
    }
    let Some(ev) = h(gp.get()).events.get(h(gp.get()).nextEvent).cloned() else {
        fail(format!("blocking wait at frame {} but the replay has no events left", h(gp.get()).frame));
    };
    let input = input_of(g, &ev);
    note_replay_key(g, &input);
    copy_replay_event(g, &ReplayEvent { frame: ev.frame + h(gp.get()).shift, ..ev });
    h(gp.get()).nextEvent += 1;
    input
}

pub fn frame_end(g: &mut Game) {
    let mut gp = g.ptr();
    if let Some(mut out) = h(gp.get()).dump.take() {
        let _ = dump_frame(g, &mut out);
        let _ = out.flush();
        h(gp.get()).dump = Some(out);
    }
    g.sfx.sfx_events.clear();
    h(gp.get()).spawn_events.clear();
    record_markers(g, h(gp.get()).frame);

    let f = h(gp.get()).frame;
    let periodic = h(gp.get()).shotEvery > 0 && f >= h(gp.get()).shotFrom && f <= h(gp.get()).shotTo && (f - h(gp.get()).shotFrom) % h(gp.get()).shotEvery == 0;
    if periodic {
        if let Some(out) = h(gp.get()).shotStream.as_mut() {
            if crate::common::gfx::gfx_write_screen_raw(g, out).and_then(|_| out.flush()).is_err() {
                fail(format!("cannot write shot stream at frame {}", f));
            }
        }
    }
    if h(gp.get()).shotFrames.contains(&f) || (periodic && h(gp.get()).shotStream.is_none()) {
        let path = format!("{}/frame_{}.bmp", h(gp.get()).shotDir, h(gp.get()).frame);
        if !crate::common::gfx::gfx_save_screen_bmp(g, &path) {
            eprintln!("[harness] cannot save {}", path);
        }
    }

    h(gp.get()).frame += 1;
    if h(gp.get()).maxFrames >= 0 && h(gp.get()).frame as i64 >= h(gp.get()).maxFrames + h(gp.get()).shift as i64 {
        g.game_values.appstate = AppState::Quit;
        h(gp.get()).dump = None;
    }
}

//------------------------------------------------------------------------------------------------
// Session recording (not in the C++). A normal launch, i.e. one without SMW_REPLAY, runs seeded and
// records every input the game sees to <settings dir>/replays/<timestamp>.txt in the replay format,
// so the session can be watched again or replayed on the C++ reference. See docs/REPLAY.md, "Recordings".
//------------------------------------------------------------------------------------------------

const KEEP_RECORDINGS: usize = 10;

/// The settings files a recording embeds, as `#@ <key>=<base64>`.
const SETTINGS_FILES: [(&str, &str); 3] = [("options_b64", "options.bin"), ("controls_b64", "controls.sdl2.bin"), ("servers_b64", "servers.toml")];

struct Recorder {
    out: BufWriter<File>,
    path: String,
    /// Live input held back by `input::poll` until the next frame start (or blocking wait).
    pending: Vec<Input>,
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

fn rec(g: &mut Game) -> Option<&'static mut Recorder> {
    let mut gp = g.ptr();
    h(gp.get()).rec.as_mut()
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
fn start_recording(g: &mut Game, seed: u32, to: Option<String>) {
    let mut gp = g.ptr();
    let dir = crate::common::path::get_home_directory(g) + "replays/";
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
    let home = crate::common::path::get_home_directory(g);
    let _ = writeln!(out, "# Super Mario War session recorded {} UTC (Rust port).", stamp.replace('_', " "));
    let _ = writeln!(out, "#@ seed={}", seed);
    let _ = writeln!(out, "#@ netrec=1");
    for (key, file) in SETTINGS_FILES {
        if let Ok(bytes) = std::fs::read(home.clone() + file) {
            let _ = writeln!(out, "#@ {}={}", key, base64_encode(&bytes));
        }
    }
    if let Some(map) = forced_map(g) {
        let _ = writeln!(out, "#@ map={}", map);
    }
    let _ = out.flush();
    println!("[harness] recording to {}", path);

    let script = if live { env("SMW_LIVE_SCRIPT").map(|p| load_events(g, &p)).unwrap_or_default() } else { Vec::new() };
    h(gp.get()).rec = Some(Recorder {
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
        prune_recordings(&dir);
    }
}

/// While recording, live input is held back here; the frame start (or a blocking wait) turns it into replay
/// lines and queues those, so the game only ever sees input in the exact form a replay of the recording will produce.
pub fn capture(g: &mut Game, input: Input) -> Option<Input> {
    match rec(g).filter(|r| r.live) {
        Some(r) if !matches!(input.event, InputEvent::Other(_)) => {
            r.pending.push(input);
            None
        }
        _ => Some(input),
    }
}

/// The replay line for live input, or None for input the replay format cannot express.
fn to_replay_event(g: &mut Game, input: &Input, frame: u32) -> Option<ReplayEvent> {
    let mut gp = g.ptr();
    let mut ev = ReplayEvent { frame, kind: EventKind::Key, down: false, keyName: String::new(), device: 0, index: 0, value: 0 };
    if let Some((x, y)) = input.event.tap() {
        ev.kind = EventKind::Tap;
        ev.index = x as i32;
        ev.value = y as i32;
        return Some(ev);
    }
    let (which, kind, index, value) = match input.event {
        InputEvent::Key { key, down, .. } => {
            let kb = &crate::services::services(gp.get()).keyboard;
            let name = kb.key_name(key.0);
            if name.is_empty() || kb.key_from_name(&name) != key.0 {
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
    ev.device = device_index(g, which)?;
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
fn device_index(g: &mut Game, which: i32) -> Option<i32> {
    (0..g.devices.len()).find(|&i| g.pad(i) == Some(PadSlot(which))).map(|i| i as i32)
}

fn line_for(ev: &ReplayEvent) -> String {
    match ev.kind {
        EventKind::Key => format!("{} {} {}", ev.frame, if ev.down { "down" } else { "up" }, ev.keyName),
        EventKind::JoyAxis => format!("{} jaxis {} {} {}", ev.frame, ev.device, ev.index, ev.value),
        EventKind::JoyButton => format!("{} jbutton {} {} {}", ev.frame, ev.device, ev.index, ev.value),
        EventKind::JoyHat => format!("{} jhat {} {} {}", ev.frame, ev.device, ev.index, ev.value),
        EventKind::Tap => format!("{} tap {} {}", ev.frame, ev.index, ev.value),
        EventKind::Await => format!("{} await {}", ev.frame, ev.keyName),
    }
}

/// SMW_LIVE_SCRIPT events due this frame enter SDL's queue like OS input, with the modifier and
/// repeat flags a real keyboard sets, so they take the same capture path as real input.
fn push_live_script(g: &mut Game, r: &mut Recorder, frame: u32) {
    while let Some(ev) = r.script.get(r.nextScript).cloned() {
        match due(g, &ev, frame) {
            Due::Later => break,
            Due::Past => {}
            Due::Now => {
                let mut input = input_of(g, &ev);
                match &mut input.event {
                    InputEvent::Key { mods, repeat, down: true, .. } => {
                        *mods = KMOD_NUM;
                        *repeat = r.nextScript % 3 == 2;
                    }
                    InputEvent::PadAxis { pad, .. } | InputEvent::PadButton { pad, .. } | InputEvent::PadHat { pad, .. } => {
                        *pad = PadSlot(sdl_instance(g, pad.0));
                    }
                    _ => {}
                }
                crate::services::services(g).events.push(&input);
            }
        }
        r.nextScript += 1;
    }
}

fn sdl_instance(g: &mut Game, device: i32) -> i32 {
    if device >= 0 {
        if let Some(pad) = g.pad(device as usize) {
            return pad.0;
        }
    }
    -1
}

fn record_event(g: &mut Game, r: &mut Recorder, ev: &ReplayEvent) {
    let _ = writeln!(r.out, "{}", line_for(ev));
    let input = input_of(g, ev);
    crate::smw::input::push_ready(g, input);
}

/// Not in upstream. Browsers report a standard-mapping pad's D-pad as buttons 12-15 (up, down, left,
/// right) and desktop SDL as hat 0, so each of those button lines is preceded by the hat line it implies.
#[cfg(target_os = "emscripten")]
mod web_dpad {
    use super::{EventKind, ReplayEvent};
    use crate::game::Game;
    use crate::common::keys::{SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP};

    extern "C" {
        fn emscripten_run_script_int(script: *const std::ffi::c_char) -> i32;
    }


    pub fn init(g: &mut Game) {
        unsafe {
            super::h(g).web_standard = emscripten_run_script_int(
                c"(navigator.getGamepads ? Array.prototype.filter.call(navigator.getGamepads(), Boolean) : []).reduce((m, p, i) => p.mapping === 'standard' ? m | (1 << i) : m, 0)".as_ptr(),
            );
        }
    }

    pub fn hat_for(g: &mut Game, ev: &ReplayEvent) -> Option<ReplayEvent> {
        if ev.kind != EventKind::JoyButton || !(12..=15).contains(&ev.index) || super::h(g).web_standard & (1 << ev.device) == 0 {
            return None;
        }
        let bit = [SDL_HAT_UP, SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT][(ev.index - 12) as usize] as i32;
        let hat = &mut super::h(g).web_hats[ev.device as usize];
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
fn record_frame_start(g: &mut Game, frame: u32) {
    let mut gp = g.ptr();
    let Some(r) = rec(g).filter(|r| r.live) else { return };
    if frame == 0 {
        for d in 0..gp.devices.len().min(8) as i32 {
            let ev = ReplayEvent { frame: 0, kind: EventKind::JoyHat, down: false, keyName: String::new(), device: d, index: 0, value: 0 };
            record_event(g, r, &ev);
        }
        #[cfg(target_os = "emscripten")]
        web_dpad::init(g);
    }
    // A browser tab can close at any moment without finish(), and muOS kills apps; keep the file current.
    if frame % 60 == 0 {
        let _ = r.out.flush();
    }
    push_live_script(g, r, frame);
    crate::services::services(g).events.pump();
    crate::smw::input::drain(g);
    let held = std::mem::take(&mut r.pending);
    for input in held.iter() {
        if input.event == InputEvent::Quit {
            r.quit = true;
            continue;
        }
        if let Some(ev) = to_replay_event(g, input, frame) {
            #[cfg(target_os = "emscripten")]
            if let Some(hat) = web_dpad::hat_for(g, &ev) {
                record_event(g, r, &hat);
            }
            record_event(g, r, &ev);
        }
    }
    if r.quit {
        // The recording ends before this frame; frames= excludes it.
        r.quitFrame.get_or_insert(frame);
        g.game_values.appstate = AppState::Quit;
    }
}

/// A blocking wait while recording returns the next held input event, written with the next frame
/// number so the replay's frame start leaves it for the replay's own blocking wait.
fn record_wait_event(g: &mut Game) -> Input {
    let mut gp = g.ptr();
    loop {
        #[cfg(not(target_os = "emscripten"))]
        crate::smw::touch::flush(g);
        let Some(r) = rec(g) else { return Input { timestamp: 0, event: InputEvent::Other(0) } };
        push_live_script(g, r, h(gp.get()).frame + 1);
        crate::services::services(gp.get()).events.pump();
        crate::smw::input::drain(g);
        while !r.pending.is_empty() {
            let input = r.pending.remove(0);
            if input.event == InputEvent::Quit {
                r.quit = true;
                r.pending.insert(0, input);
                break;
            }
            if let Some(ev) = to_replay_event(g, &input, h(gp.get()).frame + 1) {
                let _ = writeln!(r.out, "{}", line_for(&ev));
                return input_of(g, &ev);
            }
        }
        if r.quit {
            // Hand the game an event its wait loop ignores until the frame start ends the session.
            return Input { timestamp: 0, event: InputEvent::Other(0) };
        }
        crate::services::delay(gp.get(), 5);
    }
}

/// About three seconds at the default 16 ms frame time.
const UNFINISHED_TAIL_FRAMES: u32 = 188;

/// Writes `#@ frames=` and closes the recording, or removes a watch session's throwaway HOME.
/// Safe to call more than once.
pub fn finish(g: &mut Game) {
    let mut gp = g.ptr();
    if let Some(home) = h(gp.get()).watch_home.take() {
        let _ = std::fs::remove_dir_all(home);
    }
    if let Some(mut r) = h(gp.get()).rec.take() {
        let frames = r.quitFrame.unwrap_or(h(gp.get()).frame);
        let _ = writeln!(r.out, "#@ frames={}", frames);
        let _ = r.out.flush();
        println!("[harness] recorded {} frames to {}", frames, r.path);
    }
}

/// Real SDL_mixer output on top of the virtual mixer (recorded sessions and --replay watching).
pub fn audible(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    h(gp.get()).audible
}

/// Playback speed multiplier for watching a recording (SMW_REPLAY_SPEED); 1 otherwise.
pub fn speed(g: &mut Game) -> f32 {
    let mut gp = g.ptr();
    h(gp.get()).speed
}

/// `--replay <file>`: watch a recording in a normal window at normal speed with sound. The settings
/// embedded in the recording go into a throwaway HOME so the user's own settings stay untouched.
pub fn prepare_watch(g: &mut Game, file: &str, speed: Option<f32>, segment: Option<u32>) {
    let mut gp = g.ptr();
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
    for (key, file) in SETTINGS_FILES {
        if let Some(bytes) = directive(key).and_then(|v| base64_decode(&v)) {
            let _ = std::fs::write(settings.join(file), bytes);
        }
    }
    std::env::set_var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }, &home);
    h(gp.get()).watch_home = Some(home);
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
fn copy_replay_event(g: &mut Game, ev: &ReplayEvent) {
    if let Some(r) = rec(g).filter(|r| !r.live) {
        let _ = writeln!(r.out, "{}", line_for(ev));
    }
}

pub fn note_match_map(g: &mut Game, path: &str) {
    crate::smw::checkpoint::note_match_map(g, path);
}

/// Called where a match starts (`MenuState::enter_gameplay`): the next marker opens a new match.
pub fn match_checkpoint(g: &mut Game) {
    let mut gp = g.ptr();
    let Some(r) = rec(g) else { return };
    r.matches += 1;
    r.checkpoint = Some(match crate::smw::checkpoint::unsupported_reason(g) {
        Some(reason) => format!("#@ checkpoint match={} frame={} unsupported={}", r.matches, h(gp.get()).frame, reason),
        None => format!("#@ checkpoint match={} frame={} z64={}", r.matches, h(gp.get()).frame, base64_encode(&deflate(&crate::smw::checkpoint::capture(g)))),
    });
}

/// The game state markers name: `splash`, `menu`, `worldmap`, `gameplay`, `scoreboard` (gameplay after
/// the game is over) or `other`.
fn marker_state(g: &mut Game) -> &'static str {
    let mut gp = g.ptr();
    match state_name(g) {
        "menu" if MenuState::instance(gp.get()).harness_menu_name() == "world" => "worldmap",
        "gameplay" if gp.mode().gameover => "scoreboard",
        name => name,
    }
}

/// Frame end while recording: a `#@ mark` line when the state changed this frame, and when a match
/// started, its checkpoint.
fn record_markers(g: &mut Game, frame: u32) {
    let Some(r) = rec(g) else { return };
    let state = marker_state(g);
    if state == r.state {
        return;
    }
    let was_match = r.state == "gameplay" || r.state == "scoreboard";
    r.state = state.to_string();
    let mut line = format!("#@ mark frame={} state={}", frame, state);
    if state == "gameplay" && !was_match {
        line += &format!(" match={} {}", r.matches, crate::smw::checkpoint::start_fields(g));
        let checkpoint = r.checkpoint.take();
        let _ = writeln!(r.out, "{}", line);
        if let Some(checkpoint) = checkpoint {
            let _ = writeln!(r.out, "{}", checkpoint);
        }
        let _ = r.out.flush();
        return;
    }
    if state == "scoreboard" || was_match {
        line += &format!(" match={} {}", r.matches, crate::smw::checkpoint::result_fields(g));
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
fn load_segment(g: &mut Game, path: &str) {
    let mut gp = g.ptr();
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
    h(gp.get()).frame = start;
    if let Some(end) = end {
        h(gp.get()).maxFrames = if h(gp.get()).maxFrames >= 0 { h(gp.get()).maxFrames.min(end as i64) } else { end as i64 };
    }
    // The menu consumed this frame's input before the checkpoint.
    while h(gp.get()).nextEvent < h(gp.get()).events.len() && h(gp.get()).events[h(gp.get()).nextEvent].frame <= start {
        h(gp.get()).nextEvent += 1;
    }
    h(gp.get()).segment = Some(Segment { checkpoint: bytes });
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
pub fn segment_state(g: &mut Game) -> Option<crate::smw::checkpoint::SegmentState> {
    let mut gp = g.ptr();
    h(gp.get()).segment.as_mut().map(|s| crate::smw::checkpoint::SegmentState { checkpoint: std::mem::take(&mut s.checkpoint), _alias: Aliased::new() })
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
