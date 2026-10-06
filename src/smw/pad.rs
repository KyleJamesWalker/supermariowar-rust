//! Not in upstream: with `SMW_PAD_TRANSLATE=1`, a game controller's raw joystick events renumbered through its mapping into `LAYOUT` (docs/PROGRESS.md, Deliberate deviations).

use crate::common::global::{joystickcount, joysticks};
use sdl2::sys::{
    SDL_Event, SDL_EventFilter, SDL_EventType, SDL_GameController, SDL_GameControllerAddMapping, SDL_GameControllerAxis, SDL_GameControllerBindType, SDL_GameControllerButton,
    SDL_GameControllerButtonBind, SDL_GameControllerClose, SDL_GameControllerGetBindForAxis, SDL_GameControllerGetBindForButton, SDL_GameControllerMappingForDeviceIndex, SDL_GameControllerOpen,
    SDL_GetError, SDL_GetEventFilter, SDL_InitSubSystem, SDL_IsGameController, SDL_Joystick, SDL_JoystickClose, SDL_JoystickGetAttached,
    SDL_JoystickGetDeviceInstanceID, SDL_JoystickGetGUID, SDL_JoystickOpen, SDL_JoystickGetGUIDString, SDL_JoystickInstanceID,
    SDL_JoystickNameForIndex, SDL_JoystickNumAxes, SDL_JoystickNumButtons, SDL_JoystickNumHats, SDL_NumJoysticks, SDL_SetEventFilter, SDL_bool, SDL_free, SDL_HAT_DOWN,
    SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP, SDL_INIT_GAMECONTROLLER,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::ffi::{CStr, CString};

/// Joystick button per controller button, in `SDL_GameControllerButton` order up to the right shoulder; -1 is handled apart.
const LAYOUT: [i32; 11] = [
    0,  // A
    1,  // B
    2,  // X
    5,  // Y
    4,  // Back
    -1, // Guide
    3,  // Start
    10, // Left stick
    11, // Right stick
    6,  // Left shoulder
    7,  // Right shoulder
];
const TRIGGER_BUTTONS: [u8; 2] = [8, 9];
const TRIGGER_THRESHOLD: i16 = 16384;
const GUIDE: usize = 5;
const DPAD_UP: usize = 11;
const DPAD_BITS: [u8; 4] = [SDL_HAT_UP as u8, SDL_HAT_DOWN as u8, SDL_HAT_LEFT as u8, SDL_HAT_RIGHT as u8];

/// (name, GUID prefix, mapping), applied to the first entry matching a joystick that has no mapping. Older muOS
/// images expose the H700 controls directly (bus 0x19) with different raw numbering from the `muinput` virtual pad.
const BUILTIN_MAPPINGS: [(&str, &str, &str); 3] = [
    ("muOS-Keys", "19000000", H700_RAW),
    ("Deeplay-keys", "", H700_RAW),
    ("muOS-Keys", "", MUINPUT),
];
const MUINPUT: &str =
    "a:b2,b:b3,x:b4,y:b5,leftshoulder:b6,rightshoulder:b7,lefttrigger:b8,righttrigger:b9,guide:b12,start:b11,back:b10,dpup:h0.1,dpleft:h0.8,dpright:h0.2,dpdown:h0.4,leftx:a0,lefty:a1,";
const H700_RAW: &str =
    "a:b3,b:b4,x:b6,y:b5,leftshoulder:b7,rightshoulder:b8,lefttrigger:b12,righttrigger:b13,guide:b11,start:b10,back:b9,dpup:h0.1,dpleft:h0.8,dpright:h0.2,dpdown:h0.4,leftx:a0,lefty:a1,";

#[derive(Clone, Copy, Debug)]
enum Target {
    Button(u8),
    Dpad(u8),
    Quit,
}

#[derive(Clone, Copy, Debug)]
enum AxisTarget {
    Axis(u8),
    Trigger(usize),
}

/// The raw input behind a controller button or axis, from `SDL_GameControllerGetBindForButton`/`Axis`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Bind {
    None,
    Button(u8),
    Axis(u8),
    Hat(u8, u8),
}

#[derive(Default)]
struct Pad {
    index: usize,
    instance: i32,
    controller: bool,
    buttons: HashMap<u8, Target>,
    hats: Vec<(u8, u8, u8)>,
    axes: HashMap<u8, AxisTarget>,
    /// (axis, D-pad bit, negative half): a D-pad bound to an axis that is not also a stick.
    dpad_axes: Vec<(u8, u8, bool)>,
    dpad: u8,
    triggers: [bool; 2],
}

static mut pads: Vec<Pad> = Vec::new();
static mut translate_on: bool = false;
static mut debug_input: bool = false;
static mut logged: [u32; 3] = [0; 3];
static HOTPLUG: AtomicBool = AtomicBool::new(false);
static GENERATION: AtomicU32 = AtomicU32::new(0);
/// The replay format's device limit.
const MAX_JOYSTICKS: usize = 8;

/// Logs every open joystick; with `translate`, reads the game controllers among them through their mapping.
pub fn init(translate: bool) {
    unsafe {
        translate_on = translate;
        debug_input = std::env::var_os("SMW_DEBUG_INPUT").is_some();
        if debug_input {
            start_watchdog();
        }
        SDL_InitSubSystem(SDL_INIT_GAMECONTROLLER);
        let count = SDL_NumJoysticks();
        print!("[pad] SDL_NumJoysticks = {}", count);
        if count < 0 {
            print!(" ({})", c_string(SDL_GetError()));
        }
        println!();
        for var in ["SDL_JOYSTICK_DEVICE", "SDL_JOYSTICK_DISABLE_UDEV", "SDL_LINUX_JOYSTICK_CLASSIC", "SDL_JOYSTICK_HIDAPI", "SDL_GAMECONTROLLERCONFIG_FILE"] {
            if let Some(value) = std::env::var_os(var) {
                println!("[pad] {}={}", var, value.to_string_lossy());
            }
        }
        if let Some(config) = std::env::var_os("SDL_GAMECONTROLLERCONFIG") {
            println!("[pad] SDL_GAMECONTROLLERCONFIG has {} mapping(s)", config.to_string_lossy().lines().filter(|l| l.contains(',')).count());
        }
        for i in 0..joystickcount.max(0) as usize {
            let js = *joysticks.add(i);
            if !js.is_null() {
                pads.push(open_pad(i as i32, js, i));
            }
        }

        // The recorder's filter calls translate itself.
        let mut current: SDL_EventFilter = None;
        let mut userdata = std::ptr::null_mut();
        if (debug_input || translate || pads.iter().any(|p| p.controller)) && SDL_GetEventFilter(&mut current, &mut userdata) == SDL_bool::SDL_FALSE {
            SDL_SetEventFilter(Some(filter), std::ptr::null_mut());
        }
    }
}

/// Reads and logs the joystick at `device` (an SDL device index), open as `js`, which the game knows as `index`.
unsafe fn open_pad(device: i32, js: *mut SDL_Joystick, index: usize) -> Pad {
    let name = c_string(SDL_JoystickNameForIndex(device));
    let mut guid = [0 as std::ffi::c_char; 33];
    SDL_JoystickGetGUIDString(SDL_JoystickGetGUID(js), guid.as_mut_ptr(), guid.len() as i32);
    let guid = CStr::from_ptr(guid.as_ptr()).to_string_lossy().into_owned();
    if translate_on && SDL_IsGameController(device) == SDL_bool::SDL_FALSE {
        if let Some((_, _, mapping)) = BUILTIN_MAPPINGS.iter().find(|(n, prefix, _)| *n == name && guid.starts_with(prefix)) {
            if let Ok(line) = CString::new(format!("{},{},{}platform:Linux,", guid, name, mapping)) {
                SDL_GameControllerAddMapping(line.as_ptr());
            }
        }
    }
    let mapping = SDL_GameControllerMappingForDeviceIndex(device);
    let mapping_text = if mapping.is_null() { String::from("none") } else { c_string(mapping) };
    if !mapping.is_null() {
        SDL_free(mapping as *mut _);
    }
    let mut pad = Pad { index, instance: SDL_JoystickInstanceID(js), ..Default::default() };
    if translate_on && SDL_IsGameController(device) == SDL_bool::SDL_TRUE {
        // Closed again at once: SDL's controller watcher would read the hat events translate makes as this
        // controller's own, and dereferences a null hat array for a joystick without hats (Android pads).
        let gc = SDL_GameControllerOpen(device);
        if !gc.is_null() {
            read_binds(gc, &mut pad);
            SDL_GameControllerClose(gc);
            pad.controller = true;
        }
    }
    println!(
        "[pad] joystick {}: \"{}\" guid {} buttons {} hats {} axes {}; mapping {}{}",
        index,
        name,
        guid,
        SDL_JoystickNumButtons(js),
        SDL_JoystickNumHats(js),
        SDL_JoystickNumAxes(js),
        mapping_text,
        if pad.controller { " (translated)" } else { "" }
    );
    if pad.controller && debug_input {
        println!("[pad]   buttons {:?} hats {:?} axes {:?} dpad axes {:?}", pad.buttons, pad.hats, pad.axes, pad.dpad_axes);
    }
    pad
}

/// With `SMW_PAD_TRANSLATE`, opens joysticks connected since the last call and closes removed ones. A new one
/// takes the first free index, so a pad that reconnects gets its index back. True when one was opened.
pub fn hotplug() -> bool {
    unsafe {
        if !translate_on || !HOTPLUG.swap(false, Ordering::Relaxed) {
            return false;
        }
        for index in 0..joystickcount.max(0) as usize {
            let js = *joysticks.add(index);
            if !js.is_null() && SDL_JoystickGetAttached(js) == SDL_bool::SDL_FALSE {
                println!("[pad] joystick {} removed", index);
                SDL_JoystickClose(js);
                *joysticks.add(index) = std::ptr::null_mut();
                pads.retain(|p| p.index != index);
                GENERATION.fetch_add(1, Ordering::Relaxed);
            }
        }
        let mut added = false;
        for device in 0..SDL_NumJoysticks().max(0) {
            let instance = SDL_JoystickGetDeviceInstanceID(device);
            let open = (0..joystickcount.max(0) as usize).any(|i| {
                let js = *joysticks.add(i);
                !js.is_null() && SDL_JoystickInstanceID(js) == instance
            });
            if open {
                continue;
            }
            let Some(index) = free_index() else {
                continue;
            };
            let js = SDL_JoystickOpen(device);
            if js.is_null() {
                continue;
            }
            *joysticks.add(index) = js;
            pads.push(open_pad(device, js, index));
            GENERATION.fetch_add(1, Ordering::Relaxed);
            added = true;
        }
        added
    }
}

/// Changes whenever `hotplug` opens or closes a joystick.
pub fn generation() -> u32 {
    GENERATION.load(Ordering::Relaxed)
}

/// The first empty slot in `joysticks`, growing it by one when every slot is taken.
unsafe fn free_index() -> Option<usize> {
    let count = joystickcount.max(0) as usize;
    if let Some(i) = (0..count).find(|&i| (*joysticks.add(i)).is_null()) {
        return Some(i);
    }
    if count >= MAX_JOYSTICKS {
        return None;
    }
    let mut grown = vec![std::ptr::null_mut::<SDL_Joystick>(); count + 1];
    for (i, slot) in grown.iter_mut().enumerate().take(count) {
        *slot = *joysticks.add(i);
    }
    joysticks = Box::leak(grown.into_boxed_slice()).as_mut_ptr();
    joystickcount = (count + 1) as i16;
    Some(count)
}

/// The raw input behind each controller button and axis. SDL2 derives controller events from raw joystick events in
/// an event watcher, which never sees the raw events a filter drops, so the binds are applied here instead.
unsafe fn read_binds(gc: *mut SDL_GameController, pad: &mut Pad) {
    let mut buttons = [Bind::None; DPAD_UP + 4];
    for (b, bind) in buttons.iter_mut().enumerate() {
        *bind = to_bind(SDL_GameControllerGetBindForButton(gc, std::mem::transmute::<i32, SDL_GameControllerButton>(b as i32)));
    }
    let mut axes = [Bind::None; 6];
    for (a, bind) in axes.iter_mut().enumerate() {
        *bind = to_bind(SDL_GameControllerGetBindForAxis(gc, std::mem::transmute::<i32, SDL_GameControllerAxis>(a as i32)));
    }
    apply_binds(pad, &buttons, &axes);
}

unsafe fn to_bind(bind: SDL_GameControllerButtonBind) -> Bind {
    use SDL_GameControllerBindType::*;
    match bind.bindType {
        SDL_CONTROLLER_BINDTYPE_BUTTON => Bind::Button(bind.value.button as u8),
        SDL_CONTROLLER_BINDTYPE_AXIS => Bind::Axis(bind.value.axis as u8),
        SDL_CONTROLLER_BINDTYPE_HAT => Bind::Hat(bind.value.hat.hat as u8, bind.value.hat.hat_mask as u8),
        _ => Bind::None,
    }
}

/// `buttons` in `SDL_GameControllerButton` order up to the D-pad, `axes` in `SDL_GameControllerAxis` order.
fn apply_binds(pad: &mut Pad, buttons: &[Bind; DPAD_UP + 4], axes: &[Bind; 6]) {
    for (a, &bind) in axes.iter().enumerate() {
        let target = if a < 4 { AxisTarget::Axis(a as u8) } else { AxisTarget::Trigger(a - 4) };
        match (bind, target) {
            (Bind::Axis(n), _) => {
                pad.axes.insert(n, target);
            }
            (Bind::Button(n), AxisTarget::Trigger(side)) => {
                pad.buttons.insert(n, Target::Button(TRIGGER_BUTTONS[side]));
            }
            _ => {}
        }
    }
    for (b, &bind) in buttons.iter().enumerate() {
        let target = if b == GUIDE {
            Target::Quit
        } else if b >= DPAD_UP {
            Target::Dpad(DPAD_BITS[b - DPAD_UP])
        } else {
            Target::Button(LAYOUT[b] as u8)
        };
        match (bind, target) {
            (Bind::Button(n), _) => {
                pad.buttons.insert(n, target);
            }
            (Bind::Hat(hat, mask), Target::Dpad(bit)) => pad.hats.push((hat, mask, bit)),
            // SDL2 reports no half for an axis bind; mappings put up and left on the negative one.
            (Bind::Axis(n), Target::Dpad(bit)) if !pad.axes.contains_key(&n) => {
                pad.dpad_axes.push((n, bit, bit == SDL_HAT_UP as u8 || bit == SDL_HAT_LEFT as u8));
            }
            _ => {}
        }
    }
}

/// The joysticks players get: the translated controllers if there are any, otherwise every open joystick.
pub fn player_pads() -> Vec<usize> {
    unsafe {
        let mut open: Vec<usize> = (0..joystickcount.max(0) as usize).filter(|&i| !(*joysticks.add(i)).is_null()).collect();
        if pads.iter().any(|p| p.controller) {
            open.retain(|&i| pads.iter().any(|p| p.index == i && p.controller));
        }
        open
    }
}

unsafe extern "C" fn filter(_userdata: *mut std::ffi::c_void, event: *mut SDL_Event) -> i32 {
    translate(&mut *event) as i32
}

/// Rewrites a controller's raw joystick event in place as the joystick event the game reads. False: drop the event.
pub fn translate(event: &mut SDL_Event) -> bool {
    use SDL_EventType::*;
    unsafe {
        log_event(0, event);
        let t = event.type_;
        let keep = if t == SDL_CONTROLLERBUTTONDOWN as u32 || t == SDL_CONTROLLERBUTTONUP as u32 || t == SDL_CONTROLLERAXISMOTION as u32 {
            !pads.iter().any(|p| p.controller)
        } else if t == SDL_JOYBUTTONDOWN as u32 || t == SDL_JOYBUTTONUP as u32 {
            match pad(event.jbutton.which) {
                Some(p) => translate_button(p, event),
                None => true,
            }
        } else if t == SDL_JOYHATMOTION as u32 {
            match pad(event.jhat.which) {
                Some(p) => translate_hat(p, event),
                None => true,
            }
        } else if t == SDL_JOYAXISMOTION as u32 {
            match pad(event.jaxis.which) {
                Some(p) => translate_axis(p, event),
                None => true,
            }
        } else if t == SDL_JOYBALLMOTION as u32 {
            pad(event.jball.which).is_none()
        } else {
            if t == SDL_JOYDEVICEADDED as u32 || t == SDL_JOYDEVICEREMOVED as u32 {
                HOTPLUG.store(true, Ordering::Relaxed);
            }
            true
        };
        if keep {
            log_event(1, event);
        }
        keep
    }
}

unsafe fn pad(instance: i32) -> Option<&'static mut Pad> {
    pads.iter_mut().find(|p| p.instance == instance && p.controller)
}

unsafe fn translate_button(p: &mut Pad, event: &mut SDL_Event) -> bool {
    use SDL_EventType::*;
    let b = event.jbutton;
    let down = b.type_ == SDL_JOYBUTTONDOWN as u32;
    match p.buttons.get(&b.button).copied() {
        Some(Target::Button(n)) => {
            event.jbutton.button = n;
            true
        }
        Some(Target::Dpad(bit)) => {
            let dpad = if down { p.dpad | bit } else { p.dpad & !bit };
            set_dpad(p, event, b.timestamp, b.which, dpad)
        }
        Some(Target::Quit) => {
            if down {
                event.type_ = SDL_QUIT as u32;
                event.quit.timestamp = b.timestamp;
            }
            down
        }
        None => false,
    }
}

unsafe fn translate_hat(p: &mut Pad, event: &mut SDL_Event) -> bool {
    let h = event.jhat;
    let mut dpad = p.dpad;
    let mut bound = false;
    for &(hat, mask, bit) in &p.hats {
        if hat == h.hat {
            bound = true;
            dpad = if h.value & mask != 0 { dpad | bit } else { dpad & !bit };
        }
    }
    bound && set_dpad(p, event, h.timestamp, h.which, dpad)
}

unsafe fn set_dpad(p: &mut Pad, event: &mut SDL_Event, timestamp: u32, which: i32, dpad: u8) -> bool {
    if dpad == p.dpad {
        return false;
    }
    p.dpad = dpad;
    event.type_ = SDL_EventType::SDL_JOYHATMOTION as u32;
    event.jhat.timestamp = timestamp;
    event.jhat.which = which;
    event.jhat.hat = 0;
    event.jhat.value = dpad;
    true
}

unsafe fn translate_axis(p: &mut Pad, event: &mut SDL_Event) -> bool {
    use SDL_EventType::*;
    let a = event.jaxis;
    match p.axes.get(&a.axis).copied() {
        Some(AxisTarget::Axis(n)) => {
            event.jaxis.axis = n;
            true
        }
        Some(AxisTarget::Trigger(side)) => {
            let pressed = a.value >= TRIGGER_THRESHOLD;
            if pressed == p.triggers[side] {
                return false;
            }
            p.triggers[side] = pressed;
            event.type_ = if pressed { SDL_JOYBUTTONDOWN } else { SDL_JOYBUTTONUP } as u32;
            event.jbutton.timestamp = a.timestamp;
            event.jbutton.which = a.which;
            event.jbutton.button = TRIGGER_BUTTONS[side];
            event.jbutton.state = pressed as u8;
            true
        }
        None => {
            let mut dpad = p.dpad;
            let mut bound = false;
            for &(axis, bit, negative) in &p.dpad_axes {
                if axis == a.axis {
                    bound = true;
                    let on = if negative { a.value <= -TRIGGER_THRESHOLD } else { a.value >= TRIGGER_THRESHOLD };
                    dpad = if on { dpad | bit } else { dpad & !bit };
                }
            }
            bound && set_dpad(p, event, a.timestamp, a.which, dpad)
        }
    }
}

/// `SMW_DEBUG_INPUT=1`: a heartbeat every 120 frames.
pub fn debug_frame(frame: u32) {
    FRAME.store(frame, Ordering::Relaxed);
    if unsafe { debug_input } && frame % 120 == 0 {
        println!("[loop] frame {}", frame);
    }
}

static FRAME: AtomicU32 = AtomicU32::new(0);
static PHASE: AtomicUsize = AtomicUsize::new(0);
static BEATS: AtomicU64 = AtomicU64::new(0);
const PHASES: [&str; 8] = ["startup", "fps limiter", "sound", "recorder", "replay events", "update", "frame end", "flip"];

/// Marks the main loop's progress for the `SMW_DEBUG_INPUT` watchdog; `phase` indexes `PHASES`.
pub fn phase(phase: usize) {
    PHASE.store(phase, Ordering::Relaxed);
    BEATS.fetch_add(1, Ordering::Relaxed);
}

/// `SMW_DEBUG_INPUT=1`: reports a main loop that has not moved for 3 s.
fn start_watchdog() {
    std::thread::spawn(|| {
        let mut last = u64::MAX;
        let mut still = 0;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let beats = BEATS.load(Ordering::Relaxed);
            if beats != last {
                last = beats;
                still = 0;
                continue;
            }
            still += 1;
            if still == 6 || (still > 6 && still % 20 == 0) {
                println!(
                    "[watchdog] main loop stalled for {} s at frame {}, phase {}",
                    still / 2,
                    FRAME.load(Ordering::Relaxed),
                    PHASES.get(PHASE.load(Ordering::Relaxed)).unwrap_or(&"?")
                );
            }
        }
    });
}

/// `SMW_DEBUG_INPUT=1`: the first 2000 input events at each stage: 0 as SDL queues them, 1 after translation, 2 as the game reads them.
pub fn log_event(stage: usize, event: &SDL_Event) {
    use SDL_EventType::*;
    unsafe {
        if !debug_input || logged[stage] >= 2000 {
            return;
        }
        let t = event.type_;
        let detail = if t == SDL_JOYBUTTONDOWN as u32 || t == SDL_JOYBUTTONUP as u32 {
            format!("jbutton which {} button {} state {}", event.jbutton.which, event.jbutton.button, event.jbutton.state)
        } else if t == SDL_JOYHATMOTION as u32 {
            format!("jhat which {} hat {} value {}", event.jhat.which, event.jhat.hat, event.jhat.value)
        } else if t == SDL_JOYAXISMOTION as u32 {
            format!("jaxis which {} axis {} value {}", event.jaxis.which, event.jaxis.axis, event.jaxis.value)
        } else if t == SDL_CONTROLLERBUTTONDOWN as u32 || t == SDL_CONTROLLERBUTTONUP as u32 {
            format!("cbutton which {} button {} state {}", event.cbutton.which, event.cbutton.button, event.cbutton.state)
        } else if t == SDL_CONTROLLERAXISMOTION as u32 {
            format!("caxis which {} axis {} value {}", event.caxis.which, event.caxis.axis, event.caxis.value)
        } else if t == SDL_KEYDOWN as u32 || t == SDL_KEYUP as u32 {
            format!("key {} sym {} scancode {}", if t == SDL_KEYDOWN as u32 { "down" } else { "up" }, event.key.keysym.sym, event.key.keysym.scancode as i32)
        } else if t == SDL_QUIT as u32 {
            String::from("quit")
        } else if (SDL_JOYAXISMOTION as u32..=SDL_CONTROLLERDEVICEREMAPPED as u32).contains(&t) {
            format!("type {:#x}", t)
        } else {
            return;
        };
        logged[stage] += 1;
        println!("[input] {} {}", ["raw", "translated", "game"][stage], detail);
    }
}

unsafe fn c_string(p: *const std::ffi::c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl2::sys::SDL_EventType::*;

    const NAMES: [&str; 15] = [
        "a", "b", "x", "y", "back", "guide", "start", "leftstick", "rightstick", "leftshoulder", "rightshoulder", "dpup", "dpdown", "dpleft", "dpright",
    ];
    const AXES: [&str; 6] = ["leftx", "lefty", "rightx", "righty", "lefttrigger", "righttrigger"];

    fn pad(mapping: &str) -> Pad {
        let mut buttons = [Bind::None; 15];
        let mut axes = [Bind::None; 6];
        for (key, value) in mapping.split(',').filter_map(|f| f.split_once(':')) {
            let value = value.trim_start_matches(['+', '-']);
            let bind = if let Some(n) = value.strip_prefix('b') {
                Bind::Button(n.parse().unwrap())
            } else if let Some(n) = value.strip_prefix('a') {
                Bind::Axis(n.parse().unwrap())
            } else if let Some((h, m)) = value.strip_prefix('h').and_then(|v| v.split_once('.')) {
                Bind::Hat(h.parse().unwrap(), m.parse().unwrap())
            } else {
                continue;
            };
            if let Some(i) = NAMES.iter().position(|n| *n == key) {
                buttons[i] = bind;
            } else if let Some(i) = AXES.iter().position(|n| *n == key) {
                axes[i] = bind;
            }
        }
        let mut p = Pad { controller: true, ..Default::default() };
        apply_binds(&mut p, &buttons, &axes);
        p
    }

    /// The event the game gets for a raw one, None when dropped.
    fn run(p: &mut Pad, mut event: SDL_Event) -> Option<SDL_Event> {
        let t = unsafe { event.type_ };
        let keep = unsafe {
            if t == SDL_JOYBUTTONDOWN as u32 || t == SDL_JOYBUTTONUP as u32 {
                translate_button(p, &mut event)
            } else if t == SDL_JOYHATMOTION as u32 {
                translate_hat(p, &mut event)
            } else {
                translate_axis(p, &mut event)
            }
        };
        keep.then_some(event)
    }

    fn button(b: u8, down: bool) -> SDL_Event {
        let mut e: SDL_Event = unsafe { std::mem::zeroed() };
        e.jbutton.type_ = if down { SDL_JOYBUTTONDOWN } else { SDL_JOYBUTTONUP } as u32;
        e.jbutton.button = b;
        e.jbutton.state = down as u8;
        e
    }

    fn hat(h: u8, value: u8) -> SDL_Event {
        let mut e: SDL_Event = unsafe { std::mem::zeroed() };
        e.jhat.type_ = SDL_JOYHATMOTION as u32;
        e.jhat.hat = h;
        e.jhat.value = value;
        e
    }

    fn axis(a: u8, value: i16) -> SDL_Event {
        let mut e: SDL_Event = unsafe { std::mem::zeroed() };
        e.jaxis.type_ = SDL_JOYAXISMOTION as u32;
        e.jaxis.axis = a;
        e.jaxis.value = value;
        e
    }

    fn as_button(e: Option<SDL_Event>) -> Option<(u8, bool)> {
        e.filter(|e| unsafe { e.type_ == SDL_JOYBUTTONDOWN as u32 || e.type_ == SDL_JOYBUTTONUP as u32 })
            .map(|e| unsafe { (e.jbutton.button, e.type_ == SDL_JOYBUTTONDOWN as u32) })
    }

    fn as_hat(e: Option<SDL_Event>) -> Option<(u8, u8)> {
        e.filter(|e| unsafe { e.type_ == SDL_JOYHATMOTION as u32 }).map(|e| unsafe { (e.jhat.hat, e.jhat.value) })
    }

    const UP: u8 = SDL_HAT_UP as u8;
    const DOWN: u8 = SDL_HAT_DOWN as u8;
    const LEFT: u8 = SDL_HAT_LEFT as u8;
    const RIGHT: u8 = SDL_HAT_RIGHT as u8;

    /// SDL's generated Android mapping: the D-pad on buttons 11-14 (Android turns a hat into those), no hat.
    const ANDROID: &str = "a:b0,b:b1,x:b2,y:b3,back:b4,guide:b5,start:b6,leftstick:b7,rightstick:b8,leftshoulder:b9,rightshoulder:b10,\
        dpup:b11,dpdown:b12,dpleft:b13,dpright:b14,leftx:a0,lefty:a1,rightx:a2,righty:a3,lefttrigger:a4,righttrigger:a5";

    #[test]
    fn android_dpad_buttons_become_hat_0() {
        let mut p = pad(ANDROID);
        assert_eq!(as_hat(run(&mut p, button(11, true))), Some((0, UP)));
        assert_eq!(as_hat(run(&mut p, button(14, true))), Some((0, UP | RIGHT)));
        assert_eq!(as_hat(run(&mut p, button(11, false))), Some((0, RIGHT)));
        assert_eq!(as_hat(run(&mut p, button(14, false))), Some((0, 0)));
        assert_eq!(as_hat(run(&mut p, button(12, true))), Some((0, DOWN)));
        assert_eq!(as_hat(run(&mut p, button(13, true))), Some((0, DOWN | LEFT)));
    }

    #[test]
    fn android_face_buttons_and_triggers_take_the_layout() {
        let mut p = pad(ANDROID);
        assert_eq!(as_button(run(&mut p, button(0, true))), Some((0, true)));
        assert_eq!(as_button(run(&mut p, button(1, true))), Some((1, true)));
        assert_eq!(as_button(run(&mut p, button(2, true))), Some((2, true)));
        assert_eq!(as_button(run(&mut p, button(3, true))), Some((5, true)));
        assert_eq!(as_button(run(&mut p, button(4, true))), Some((4, true)));
        assert_eq!(as_button(run(&mut p, button(6, true))), Some((3, true)));
        assert_eq!(as_button(run(&mut p, button(9, true))), Some((6, true)));
        assert_eq!(as_button(run(&mut p, button(10, true))), Some((7, true)));
        assert_eq!(as_button(run(&mut p, axis(4, 32767))), Some((8, true)));
        assert!(run(&mut p, axis(4, 30000)).is_none());
        assert_eq!(as_button(run(&mut p, axis(4, -32767))), Some((8, false)));
        assert_eq!(as_button(run(&mut p, axis(5, 32767))), Some((9, true)));
        let quit = run(&mut p, button(5, true)).unwrap();
        assert_eq!(unsafe { quit.type_ }, SDL_QUIT as u32);
        assert!(run(&mut p, button(5, false)).is_none());
    }

    #[test]
    fn unmapped_inputs_are_dropped() {
        let mut p = pad("a:b0,b:b1,start:b6,dpup:b11,dpdown:b12,dpleft:b13,dpright:b14");
        assert!(run(&mut p, button(20, true)).is_none());
        assert!(run(&mut p, button(255, true)).is_none());
        assert!(run(&mut p, hat(0, UP)).is_none());
        assert!(run(&mut p, hat(3, UP)).is_none());
        assert!(run(&mut p, axis(0, 32767)).is_none());
        assert!(run(&mut p, axis(255, -32768)).is_none());
    }

    #[test]
    fn hat_dpad_is_hat_0() {
        let mut p = pad("a:b0,b:b1,dpup:h1.1,dpright:h1.2,dpdown:h1.4,dpleft:h1.8,leftx:a0,lefty:a1");
        assert_eq!(as_hat(run(&mut p, hat(1, UP | LEFT))), Some((0, UP | LEFT)));
        assert!(run(&mut p, hat(1, UP | LEFT)).is_none());
        assert_eq!(as_hat(run(&mut p, hat(1, 0))), Some((0, 0)));
        assert!(run(&mut p, hat(0, UP)).is_none());
    }

    #[test]
    fn axis_dpad_is_hat_0() {
        let mut p = pad("a:b0,b:b1,leftx:a0,lefty:a1,dpup:-a7,dpdown:+a7,dpleft:-a6,dpright:+a6");
        assert_eq!(as_hat(run(&mut p, axis(7, -32768))), Some((0, UP)));
        assert_eq!(as_hat(run(&mut p, axis(6, 32767))), Some((0, UP | RIGHT)));
        assert!(run(&mut p, axis(6, 32000)).is_none());
        assert_eq!(as_hat(run(&mut p, axis(7, 0))), Some((0, RIGHT)));
        assert_eq!(as_hat(run(&mut p, axis(6, -32768))), Some((0, LEFT)));
        assert_eq!(unsafe { run(&mut p, axis(0, 1234)).unwrap().jaxis.axis }, 0);
    }

    #[test]
    fn a_stick_axis_is_not_also_a_dpad() {
        let mut p = pad("a:b0,leftx:a0,lefty:a1,dpup:-a1,dpdown:+a1");
        assert!(p.dpad_axes.is_empty());
        let e = run(&mut p, axis(1, -32768)).unwrap();
        assert_eq!(unsafe { (e.type_, e.jaxis.axis) }, (SDL_JOYAXISMOTION as u32, 1));
    }

    #[test]
    fn triggers_on_buttons() {
        let mut p = pad("a:b0,lefttrigger:b15,righttrigger:b16");
        assert_eq!(as_button(run(&mut p, button(15, true))), Some((8, true)));
        assert_eq!(as_button(run(&mut p, button(16, false))), Some((9, false)));
    }

    #[test]
    fn empty_mapping_drops_everything() {
        let mut p = pad("");
        assert!(run(&mut p, button(0, true)).is_none());
        assert!(run(&mut p, hat(0, UP)).is_none());
        assert!(run(&mut p, axis(0, 100)).is_none());
    }
}
