//! Not in upstream: with `SMW_PAD_TRANSLATE=1`, a game controller's raw joystick events renumbered through its mapping into `LAYOUT` (docs/PROGRESS.md, Deliberate deviations).

use crate::common::global::{joystickcount, joysticks};
use sdl2::sys::{
    SDL_Event, SDL_EventFilter, SDL_EventType, SDL_GameController, SDL_GameControllerAddMapping, SDL_GameControllerAxis, SDL_GameControllerBindType, SDL_GameControllerButton,
    SDL_GameControllerButtonBind, SDL_GameControllerGetBindForAxis, SDL_GameControllerGetBindForButton, SDL_GameControllerMappingForDeviceIndex, SDL_GameControllerOpen,
    SDL_GetError, SDL_GetEventFilter, SDL_InitSubSystem, SDL_IsGameController, SDL_JoystickGetGUID, SDL_JoystickGetGUIDString, SDL_JoystickInstanceID,
    SDL_JoystickNameForIndex, SDL_JoystickNumAxes, SDL_JoystickNumButtons, SDL_JoystickNumHats, SDL_NumJoysticks, SDL_SetEventFilter, SDL_bool, SDL_free, SDL_HAT_DOWN,
    SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP, SDL_INIT_GAMECONTROLLER,
};
use std::collections::HashMap;
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

#[derive(Default)]
struct Pad {
    instance: i32,
    controller: bool,
    buttons: HashMap<u8, Target>,
    hats: Vec<(u8, u8, u8)>,
    axes: HashMap<u8, AxisTarget>,
    dpad: u8,
    triggers: [bool; 2],
}

static mut pads: Vec<Pad> = Vec::new();
static mut debug_input: bool = false;
static mut logged: [u32; 3] = [0; 3];

/// Logs every open joystick; with `translate`, reads the game controllers among them through their mapping.
pub fn init(translate: bool) {
    unsafe {
        debug_input = std::env::var_os("SMW_DEBUG_INPUT").is_some();
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
        for i in 0..joystickcount.max(0) as i32 {
            let js = *joysticks.add(i as usize);
            if js.is_null() {
                continue;
            }
            let name = c_string(SDL_JoystickNameForIndex(i));
            let mut guid = [0 as std::ffi::c_char; 33];
            SDL_JoystickGetGUIDString(SDL_JoystickGetGUID(js), guid.as_mut_ptr(), guid.len() as i32);
            let guid = CStr::from_ptr(guid.as_ptr()).to_string_lossy().into_owned();
            if translate && SDL_IsGameController(i) == SDL_bool::SDL_FALSE {
                if let Some((_, _, mapping)) = BUILTIN_MAPPINGS.iter().find(|(n, prefix, _)| *n == name && guid.starts_with(prefix)) {
                    if let Ok(line) = CString::new(format!("{},{},{}platform:Linux,", guid, name, mapping)) {
                        SDL_GameControllerAddMapping(line.as_ptr());
                    }
                }
            }
            let mapping = SDL_GameControllerMappingForDeviceIndex(i);
            let mapping_text = if mapping.is_null() { String::from("none") } else { c_string(mapping) };
            if !mapping.is_null() {
                SDL_free(mapping as *mut _);
            }
            let mut pad = Pad { instance: SDL_JoystickInstanceID(js), ..Default::default() };
            if translate && SDL_IsGameController(i) == SDL_bool::SDL_TRUE {
                let gc = SDL_GameControllerOpen(i);
                if !gc.is_null() {
                    read_binds(gc, &mut pad);
                    pad.controller = true;
                }
            }
            println!(
                "[pad] joystick {}: \"{}\" guid {} buttons {} hats {} axes {}; mapping {}{}",
                i,
                name,
                guid,
                SDL_JoystickNumButtons(js),
                SDL_JoystickNumHats(js),
                SDL_JoystickNumAxes(js),
                mapping_text,
                if pad.controller { " (translated)" } else { "" }
            );
            if pad.controller && debug_input {
                println!("[pad]   buttons {:?} hats {:?} axes {:?}", pad.buttons, pad.hats, pad.axes);
            }
            pads.push(pad);
        }

        // The recorder's filter calls translate itself.
        let mut current: SDL_EventFilter = None;
        let mut userdata = std::ptr::null_mut();
        if (debug_input || pads.iter().any(|p| p.controller)) && SDL_GetEventFilter(&mut current, &mut userdata) == SDL_bool::SDL_FALSE {
            SDL_SetEventFilter(Some(filter), std::ptr::null_mut());
        }
    }
}

/// The raw input behind each controller button and axis. SDL2 derives controller events from raw joystick events in
/// an event watcher, which never sees the raw events a filter drops, so the binds are applied here instead.
unsafe fn read_binds(gc: *mut SDL_GameController, pad: &mut Pad) {
    use SDL_GameControllerBindType::*;
    for b in 0..DPAD_UP + 4 {
        let target = if b == GUIDE {
            Target::Quit
        } else if b >= DPAD_UP {
            Target::Dpad(DPAD_BITS[b - DPAD_UP])
        } else {
            Target::Button(LAYOUT[b] as u8)
        };
        let bind: SDL_GameControllerButtonBind = SDL_GameControllerGetBindForButton(gc, std::mem::transmute::<i32, SDL_GameControllerButton>(b as i32));
        match bind.bindType {
            SDL_CONTROLLER_BINDTYPE_BUTTON => {
                pad.buttons.insert(bind.value.button as u8, target);
            }
            SDL_CONTROLLER_BINDTYPE_HAT => {
                if let Target::Dpad(bit) = target {
                    pad.hats.push((bind.value.hat.hat as u8, bind.value.hat.hat_mask as u8, bit));
                }
            }
            _ => {}
        }
    }
    for a in 0..6usize {
        let target = if a < 4 { AxisTarget::Axis(a as u8) } else { AxisTarget::Trigger(a - 4) };
        let bind: SDL_GameControllerButtonBind = SDL_GameControllerGetBindForAxis(gc, std::mem::transmute::<i32, SDL_GameControllerAxis>(a as i32));
        match (bind.bindType, target) {
            (SDL_CONTROLLER_BINDTYPE_AXIS, _) => {
                pad.axes.insert(bind.value.axis as u8, target);
            }
            (SDL_CONTROLLER_BINDTYPE_BUTTON, AxisTarget::Trigger(side)) => {
                pad.buttons.insert(bind.value.button as u8, Target::Button(TRIGGER_BUTTONS[side]));
            }
            _ => {}
        }
    }
}

/// The joysticks players get at launch: the translated controllers if there are any, otherwise every joystick.
pub fn player_pads() -> Vec<usize> {
    unsafe {
        let count = joystickcount.max(0) as usize;
        let controllers: Vec<usize> = (0..count).filter(|&i| pads.get(i).is_some_and(|p| p.controller)).collect();
        if controllers.is_empty() {
            (0..count).collect()
        } else {
            controllers
        }
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
        None => false,
    }
}

/// `SMW_DEBUG_INPUT=1`: a heartbeat every 120 frames.
pub fn debug_frame(frame: u32) {
    if unsafe { debug_input } && frame % 120 == 0 {
        println!("[loop] frame {}", frame);
    }
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
