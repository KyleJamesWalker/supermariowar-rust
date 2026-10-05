//! Not in upstream: with `SMW_PAD_TRANSLATE=1`, game controllers' events rewritten as joystick events in `LAYOUT` (docs/PROGRESS.md, Deliberate deviations).

use crate::common::global::{joystickcount, joysticks};
use sdl2::sys::{
    SDL_Event, SDL_EventFilter, SDL_EventType, SDL_GetError, SDL_NumJoysticks, SDL_GameControllerAddMapping, SDL_GameControllerMappingForDeviceIndex, SDL_GameControllerOpen, SDL_GetEventFilter,
    SDL_InitSubSystem, SDL_IsGameController, SDL_JoystickGetGUID, SDL_JoystickGetGUIDString, SDL_JoystickInstanceID, SDL_JoystickNameForIndex, SDL_JoystickNumAxes,
    SDL_JoystickNumButtons, SDL_JoystickNumHats, SDL_SetEventFilter, SDL_bool, SDL_free, SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP, SDL_INIT_GAMECONTROLLER,
};
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
const GUIDE: u8 = 5;
const DPAD_UP: u8 = 11;

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

struct Pad {
    instance: i32,
    controller: bool,
    hat: u8,
    triggers: [bool; 2],
}

static mut pads: Vec<Pad> = Vec::new();

/// Logs every open joystick; with `translate`, opens the game controllers among them.
pub fn init(translate: bool) {
    unsafe {
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
            let controller = translate && SDL_IsGameController(i) == SDL_bool::SDL_TRUE && !SDL_GameControllerOpen(i).is_null();
            println!(
                "[pad] joystick {}: \"{}\" guid {} buttons {} hats {} axes {}; mapping {}{}",
                i,
                name,
                guid,
                SDL_JoystickNumButtons(js),
                SDL_JoystickNumHats(js),
                SDL_JoystickNumAxes(js),
                mapping_text,
                if controller { " (translated)" } else { "" }
            );
            pads.push(Pad { instance: SDL_JoystickInstanceID(js), controller, hat: 0, triggers: [false; 2] });
        }

        // The recorder's filter calls translate itself.
        if translate && pads.iter().any(|p| p.controller) {
            let mut current: SDL_EventFilter = None;
            let mut userdata = std::ptr::null_mut();
            if SDL_GetEventFilter(&mut current, &mut userdata) == SDL_bool::SDL_FALSE {
                SDL_SetEventFilter(Some(filter), std::ptr::null_mut());
            }
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

/// Rewrites a controller's event in place as the joystick event the game reads. False: drop the event.
pub fn translate(event: &mut SDL_Event) -> bool {
    use SDL_EventType::*;
    unsafe {
        let t = event.type_;
        let which = if t == SDL_JOYAXISMOTION as u32 {
            event.jaxis.which
        } else if t == SDL_JOYBUTTONDOWN as u32 || t == SDL_JOYBUTTONUP as u32 {
            event.jbutton.which
        } else if t == SDL_JOYHATMOTION as u32 {
            event.jhat.which
        } else if t == SDL_JOYBALLMOTION as u32 {
            event.jball.which
        } else if t == SDL_CONTROLLERBUTTONDOWN as u32 || t == SDL_CONTROLLERBUTTONUP as u32 {
            return translate_button(event);
        } else if t == SDL_CONTROLLERAXISMOTION as u32 {
            return translate_axis(event);
        } else {
            return true;
        };
        !pad(which).is_some_and(|p| p.controller)
    }
}

unsafe fn pad(instance: i32) -> Option<&'static mut Pad> {
    pads.iter_mut().find(|p| p.instance == instance)
}

unsafe fn translate_button(event: &mut SDL_Event) -> bool {
    use SDL_EventType::*;
    let b = event.cbutton;
    let Some(p) = pad(b.which).filter(|p| p.controller) else { return false };
    let down = b.type_ == SDL_CONTROLLERBUTTONDOWN as u32;
    if b.button == GUIDE {
        if down {
            event.type_ = SDL_QUIT as u32;
            event.quit.timestamp = b.timestamp;
        }
        return down;
    }
    if (DPAD_UP..DPAD_UP + 4).contains(&b.button) {
        let bit = [SDL_HAT_UP, SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT][(b.button - DPAD_UP) as usize] as u8;
        p.hat = if down { p.hat | bit } else { p.hat & !bit };
        event.type_ = SDL_JOYHATMOTION as u32;
        event.jhat.timestamp = b.timestamp;
        event.jhat.which = b.which;
        event.jhat.hat = 0;
        event.jhat.value = p.hat;
        return true;
    }
    let Some(&button) = LAYOUT.get(b.button as usize).filter(|&&v| v >= 0) else { return false };
    set_button(event, b.timestamp, b.which, button as u8, down);
    true
}

unsafe fn translate_axis(event: &mut SDL_Event) -> bool {
    let a = event.caxis;
    let Some(p) = pad(a.which).filter(|p| p.controller) else { return false };
    if a.axis >= 4 {
        let side = (a.axis - 4) as usize;
        let pressed = a.value >= TRIGGER_THRESHOLD;
        if pressed == p.triggers[side] {
            return false;
        }
        p.triggers[side] = pressed;
        set_button(event, a.timestamp, a.which, TRIGGER_BUTTONS[side], pressed);
        return true;
    }
    event.type_ = SDL_EventType::SDL_JOYAXISMOTION as u32;
    event.jaxis.timestamp = a.timestamp;
    event.jaxis.which = a.which;
    event.jaxis.axis = a.axis;
    event.jaxis.value = a.value;
    true
}

unsafe fn set_button(event: &mut SDL_Event, timestamp: u32, which: i32, button: u8, down: bool) {
    use SDL_EventType::*;
    event.type_ = if down { SDL_JOYBUTTONDOWN } else { SDL_JOYBUTTONUP } as u32;
    event.jbutton.timestamp = timestamp;
    event.jbutton.which = which;
    event.jbutton.button = button;
    event.jbutton.state = down as u8;
}

unsafe fn c_string(p: *const std::ffi::c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}
