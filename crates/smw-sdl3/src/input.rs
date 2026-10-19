//! SDL3 events to smw-platform's normalized `InputEvent` and back, in SDL2's key, scancode and pad numbering
//! (docs/design/ARCHITECTURE_V2.md, Normalized input). `to_sdl(&from_sdl(e))` reads the same as `e` for every field the
//! game reads.

use sdl3_sys::everything::*;
use smw_platform::{DeviceChange, Input, InputEvent, Keycode, MouseEvent, PadSlot, TouchPhase};
use std::ffi::{c_char, CStr, CString};

/// The `SDL_EVENT_USER` codes that carry an `InputEvent::Tap` and an `InputEvent::QuitRequest`, as smw-sdl2's.
const TAP_CODE: i32 = 0x5441;
const QUIT_REQUEST_CODE: i32 = 0x5152;
const SCANCODE_MASK: i32 = 1 << 30;

/// SDL2 scancodes of the consumer keys and the SDL3 scancode for each, where SDL3 has one. The rest of the two
/// numberings agree.
const CONSUMER_KEYS: [(i32, i32); 17] = [
    (258, 267), // AudioNext, MEDIA_NEXT_TRACK
    (259, 268), // AudioPrev
    (260, 269), // AudioStop
    (261, 262), // AudioPlay
    (263, 272), // MediaSelect
    (268, 280), // AC Search
    (269, 281), // AC Home
    (270, 282), // AC Back
    (271, 283), // AC Forward
    (272, 284), // AC Stop
    (273, 285), // AC Refresh
    (274, 286), // AC Bookmarks
    (281, 270), // Eject
    (282, 258), // Sleep
    (285, 266), // AudioRewind
    (286, 265), // AudioFastForward
    (257, 257), // ModeSwitch
];

/// SDL3's scancode in SDL2's numbering; 0 for a consumer key SDL2 does not have.
pub fn scancode_to_sdl2(scancode: i32) -> i32 {
    if !(258..=286).contains(&scancode) {
        return scancode;
    }
    CONSUMER_KEYS.iter().find(|&&(_, s3)| s3 == scancode).map_or(0, |&(s2, _)| s2)
}

/// SDL2's scancode in SDL3's numbering; 0 for a consumer key SDL3 does not have.
pub fn scancode_from_sdl2(scancode: i32) -> i32 {
    if !(258..=286).contains(&scancode) {
        return scancode;
    }
    CONSUMER_KEYS.iter().find(|&&(s2, _)| s2 == scancode).map_or(0, |&(_, s3)| s3)
}

/// SDL3's keycode in SDL2's values: the same, except keys named by a scancode follow `scancode_to_sdl2`.
pub fn key_to_sdl2(key: u32) -> i32 {
    let key = key as i32;
    if key & SCANCODE_MASK != 0 {
        let s = scancode_to_sdl2(key & !SCANCODE_MASK);
        if s == 0 {
            0
        } else {
            SCANCODE_MASK | s
        }
    } else {
        key
    }
}

pub fn key_from_sdl2(key: i32) -> u32 {
    if key & SCANCODE_MASK != 0 {
        let s = scancode_from_sdl2(key & !SCANCODE_MASK);
        (if s == 0 { 0 } else { SCANCODE_MASK | s }) as u32
    } else {
        key as u32
    }
}

fn is(e: &SDL_Event, t: SDL_EventType) -> bool {
    unsafe { e.r#type == t.0 }
}

fn slot(which: SDL_JoystickID) -> PadSlot {
    PadSlot(which.0 as i32)
}

fn joystick(pad: PadSlot) -> SDL_JoystickID {
    SDL_JoystickID(pad.0 as u32)
}

/// The SDL2 timestamp: milliseconds, wrapping at 2^32.
fn millis(ns: u64) -> u32 {
    (ns / 1_000_000) as u32
}

pub fn from_sdl(e: &SDL_Event) -> Input {
    unsafe {
        let event = if is(e, SDL_EVENT_QUIT) {
            InputEvent::Quit
        } else if is(e, SDL_EVENT_KEY_DOWN) || is(e, SDL_EVENT_KEY_UP) {
            let k = e.key;
            InputEvent::Key {
                key: Keycode(key_to_sdl2(k.key.0)),
                scancode: scancode_to_sdl2(k.scancode.0),
                mods: k.r#mod.0,
                down: k.down,
                repeat: k.repeat,
                window: k.windowID.0,
            }
        } else if is(e, SDL_EVENT_JOYSTICK_AXIS_MOTION) {
            InputEvent::PadAxis { pad: slot(e.jaxis.which), axis: e.jaxis.axis, value: e.jaxis.value }
        } else if is(e, SDL_EVENT_JOYSTICK_BUTTON_DOWN) || is(e, SDL_EVENT_JOYSTICK_BUTTON_UP) {
            InputEvent::PadButton { pad: slot(e.jbutton.which), button: e.jbutton.button, down: e.jbutton.down }
        } else if is(e, SDL_EVENT_JOYSTICK_HAT_MOTION) {
            InputEvent::PadHat { pad: slot(e.jhat.which), hat: e.jhat.hat, value: e.jhat.value }
        } else if is(e, SDL_EVENT_JOYSTICK_ADDED) || is(e, SDL_EVENT_JOYSTICK_REMOVED) {
            let change = if is(e, SDL_EVENT_JOYSTICK_ADDED) { DeviceChange::Added } else { DeviceChange::Removed };
            InputEvent::Pad { pad: slot(e.jdevice.which), change }
        } else if is(e, SDL_EVENT_GAMEPAD_AXIS_MOTION) {
            InputEvent::GamepadAxis { pad: slot(e.gaxis.which), axis: e.gaxis.axis, value: e.gaxis.value }
        } else if is(e, SDL_EVENT_GAMEPAD_BUTTON_DOWN) || is(e, SDL_EVENT_GAMEPAD_BUTTON_UP) {
            InputEvent::GamepadButton { pad: slot(e.gbutton.which), button: e.gbutton.button, down: e.gbutton.down }
        } else if is(e, SDL_EVENT_GAMEPAD_ADDED) || is(e, SDL_EVENT_GAMEPAD_REMOVED) || is(e, SDL_EVENT_GAMEPAD_REMAPPED) {
            let change = if is(e, SDL_EVENT_GAMEPAD_ADDED) {
                DeviceChange::Added
            } else if is(e, SDL_EVENT_GAMEPAD_REMOVED) {
                DeviceChange::Removed
            } else {
                DeviceChange::Remapped
            };
            InputEvent::Gamepad { pad: slot(e.gdevice.which), change }
        } else if is(e, SDL_EVENT_FINGER_DOWN) || is(e, SDL_EVENT_FINGER_MOTION) || is(e, SDL_EVENT_FINGER_UP) {
            let f = e.tfinger;
            let phase = if is(e, SDL_EVENT_FINGER_DOWN) {
                TouchPhase::Down
            } else if is(e, SDL_EVENT_FINGER_MOTION) {
                TouchPhase::Motion
            } else {
                TouchPhase::Up
            };
            InputEvent::Touch { touch: f.touchID.0 as i64, finger: f.fingerID.0 as i64, phase, x: f.x, y: f.y, dx: f.dx, dy: f.dy, pressure: f.pressure }
        } else if is(e, SDL_EVENT_TEXT_INPUT) {
            let text = e.text.text;
            InputEvent::Text(if text.is_null() { String::new() } else { CStr::from_ptr(text).to_string_lossy().into_owned() })
        } else if is(e, SDL_EVENT_MOUSE_MOTION) {
            let m = e.motion;
            InputEvent::Mouse {
                window: m.windowID.0,
                which: m.which.0,
                event: MouseEvent::Motion { x: m.x.floor() as i32, y: m.y.floor() as i32, xrel: m.xrel as i32, yrel: m.yrel as i32, buttons: m.state.0 },
            }
        } else if is(e, SDL_EVENT_MOUSE_BUTTON_DOWN) || is(e, SDL_EVENT_MOUSE_BUTTON_UP) {
            let b = e.button;
            InputEvent::Mouse {
                window: b.windowID.0,
                which: b.which.0,
                event: MouseEvent::Button { button: b.button, down: b.down, clicks: b.clicks, x: b.x.floor() as i32, y: b.y.floor() as i32 },
            }
        } else if is(e, SDL_EVENT_USER) && e.user.code == QUIT_REQUEST_CODE {
            InputEvent::QuitRequest
        } else if is(e, SDL_EVENT_USER) && e.user.code == TAP_CODE {
            InputEvent::Tap { x: e.user.data1 as isize as i16, y: e.user.data2 as isize as i16 }
        } else if is(e, SDL_EVENT_MOUSE_WHEEL) {
            let w = e.wheel;
            InputEvent::Mouse { window: w.windowID.0, which: w.which.0, event: MouseEvent::Wheel { x: w.integer_x, y: w.integer_y } }
        } else {
            InputEvent::Other(e.r#type)
        };
        Input { timestamp: millis(e.common.timestamp), event }
    }
}

/// SDL3 keeps a text event's text behind a pointer; `push` holds the last ones here until they are read.
pub struct TextArena(Vec<CString>);

impl TextArena {
    pub const fn new() -> Self {
        TextArena(Vec::new())
    }

    fn hold(&mut self, text: &str) -> *const c_char {
        if self.0.len() >= 64 {
            self.0.remove(0);
        }
        let bytes: Vec<u8> = text.bytes().filter(|&b| b != 0).collect();
        self.0.push(CString::new(bytes).unwrap_or_default());
        self.0.last().unwrap().as_ptr()
    }
}

impl Default for TextArena {
    fn default() -> Self {
        Self::new()
    }
}

pub fn to_sdl(input: &Input, texts: &mut TextArena) -> SDL_Event {
    let mut e = SDL_Event { r#type: 0 };
    let timestamp = input.timestamp as u64 * 1_000_000;
    match &input.event {
        InputEvent::Quit => e.quit = SDL_QuitEvent { r#type: SDL_EVENT_QUIT, timestamp, ..Default::default() },
        InputEvent::QuitRequest => {
            e.user = SDL_UserEvent { r#type: SDL_EVENT_USER.0, timestamp, code: QUIT_REQUEST_CODE, ..Default::default() }
        }
        &InputEvent::Key { key, scancode, mods, down, repeat, window } => {
            e.key = SDL_KeyboardEvent {
                r#type: if down { SDL_EVENT_KEY_DOWN } else { SDL_EVENT_KEY_UP },
                timestamp,
                windowID: SDL_WindowID(window),
                scancode: SDL_Scancode(scancode_from_sdl2(scancode)),
                key: SDL_Keycode(key_from_sdl2(key.0)),
                r#mod: SDL_Keymod(mods),
                down,
                repeat,
                ..Default::default()
            }
        }
        &InputEvent::PadAxis { pad, axis, value } => {
            e.jaxis = SDL_JoyAxisEvent { r#type: SDL_EVENT_JOYSTICK_AXIS_MOTION, timestamp, which: joystick(pad), axis, value, ..Default::default() }
        }
        &InputEvent::PadButton { pad, button, down } => {
            let t = if down { SDL_EVENT_JOYSTICK_BUTTON_DOWN } else { SDL_EVENT_JOYSTICK_BUTTON_UP };
            e.jbutton = SDL_JoyButtonEvent { r#type: t, timestamp, which: joystick(pad), button, down, ..Default::default() }
        }
        &InputEvent::PadHat { pad, hat, value } => {
            e.jhat = SDL_JoyHatEvent { r#type: SDL_EVENT_JOYSTICK_HAT_MOTION, timestamp, which: joystick(pad), hat, value, ..Default::default() }
        }
        &InputEvent::Pad { pad, change } => {
            let t = if change == DeviceChange::Added { SDL_EVENT_JOYSTICK_ADDED } else { SDL_EVENT_JOYSTICK_REMOVED };
            e.jdevice = SDL_JoyDeviceEvent { r#type: t, timestamp, which: joystick(pad), ..Default::default() }
        }
        &InputEvent::GamepadAxis { pad, axis, value } => {
            e.gaxis = SDL_GamepadAxisEvent { r#type: SDL_EVENT_GAMEPAD_AXIS_MOTION, timestamp, which: joystick(pad), axis, value, ..Default::default() }
        }
        &InputEvent::GamepadButton { pad, button, down } => {
            let t = if down { SDL_EVENT_GAMEPAD_BUTTON_DOWN } else { SDL_EVENT_GAMEPAD_BUTTON_UP };
            e.gbutton = SDL_GamepadButtonEvent { r#type: t, timestamp, which: joystick(pad), button, down, ..Default::default() }
        }
        &InputEvent::Gamepad { pad, change } => {
            let t = match change {
                DeviceChange::Added => SDL_EVENT_GAMEPAD_ADDED,
                DeviceChange::Removed => SDL_EVENT_GAMEPAD_REMOVED,
                DeviceChange::Remapped => SDL_EVENT_GAMEPAD_REMAPPED,
            };
            e.gdevice = SDL_GamepadDeviceEvent { r#type: t, timestamp, which: joystick(pad), ..Default::default() }
        }
        &InputEvent::Touch { touch, finger, phase, x, y, dx, dy, pressure } => {
            let t = match phase {
                TouchPhase::Down => SDL_EVENT_FINGER_DOWN,
                TouchPhase::Motion => SDL_EVENT_FINGER_MOTION,
                TouchPhase::Up => SDL_EVENT_FINGER_UP,
            };
            e.tfinger = SDL_TouchFingerEvent {
                r#type: t,
                timestamp,
                touchID: SDL_TouchID(touch as u64),
                fingerID: SDL_FingerID(finger as u64),
                x,
                y,
                dx,
                dy,
                pressure,
                ..Default::default()
            }
        }
        InputEvent::Text(text) => e.text = SDL_TextInputEvent { r#type: SDL_EVENT_TEXT_INPUT, timestamp, text: texts.hold(text), ..Default::default() },
        &InputEvent::Mouse { window, which, event } => match event {
            MouseEvent::Motion { x, y, xrel, yrel, buttons } => {
                e.motion = SDL_MouseMotionEvent {
                    r#type: SDL_EVENT_MOUSE_MOTION,
                    timestamp,
                    windowID: SDL_WindowID(window),
                    which: SDL_MouseID(which),
                    state: SDL_MouseButtonFlags(buttons),
                    x: x as f32,
                    y: y as f32,
                    xrel: xrel as f32,
                    yrel: yrel as f32,
                    ..Default::default()
                }
            }
            MouseEvent::Button { button, down, clicks, x, y } => {
                e.button = SDL_MouseButtonEvent {
                    r#type: if down { SDL_EVENT_MOUSE_BUTTON_DOWN } else { SDL_EVENT_MOUSE_BUTTON_UP },
                    timestamp,
                    windowID: SDL_WindowID(window),
                    which: SDL_MouseID(which),
                    button,
                    down,
                    clicks,
                    x: x as f32,
                    y: y as f32,
                    ..Default::default()
                }
            }
            MouseEvent::Wheel { x, y } => {
                e.wheel = SDL_MouseWheelEvent {
                    r#type: SDL_EVENT_MOUSE_WHEEL,
                    timestamp,
                    windowID: SDL_WindowID(window),
                    which: SDL_MouseID(which),
                    x: x as f32,
                    y: y as f32,
                    integer_x: x,
                    integer_y: y,
                    ..Default::default()
                }
            }
        },
        &InputEvent::Tap { x, y } => {
            e.user = SDL_UserEvent {
                r#type: SDL_EVENT_USER.0,
                timestamp,
                code: TAP_CODE,
                data1: x as isize as *mut std::ffi::c_void,
                data2: y as isize as *mut std::ffi::c_void,
                ..Default::default()
            }
        }
        &InputEvent::Other(t) => e.common = SDL_CommonEvent { r#type: t, timestamp, ..Default::default() },
    }
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(event: InputEvent) {
        let mut texts = TextArena::new();
        let input = Input { timestamp: 1234, event };
        assert_eq!(from_sdl(&to_sdl(&input, &mut texts)), input);
    }

    #[test]
    fn every_kind_round_trips() {
        let pad = PadSlot(3);
        round_trip(InputEvent::Quit);
        round_trip(InputEvent::QuitRequest);
        round_trip(InputEvent::Key { key: Keycode(97), scancode: 4, mods: 0x40, down: true, repeat: false, window: 1 });
        round_trip(InputEvent::Key { key: Keycode(SCANCODE_MASK | 82), scancode: 82, mods: 0, down: false, repeat: true, window: 1 });
        round_trip(InputEvent::Key { key: Keycode(SCANCODE_MASK | 258), scancode: 258, mods: 0, down: true, repeat: false, window: 1 });
        round_trip(InputEvent::PadAxis { pad, axis: 1, value: -32768 });
        round_trip(InputEvent::PadButton { pad, button: 7, down: true });
        round_trip(InputEvent::PadHat { pad, hat: 0, value: 9 });
        round_trip(InputEvent::Pad { pad, change: DeviceChange::Added });
        round_trip(InputEvent::Pad { pad, change: DeviceChange::Removed });
        round_trip(InputEvent::GamepadAxis { pad, axis: 4, value: 32767 });
        round_trip(InputEvent::GamepadButton { pad, button: 11, down: false });
        round_trip(InputEvent::Gamepad { pad, change: DeviceChange::Remapped });
        round_trip(InputEvent::Touch { touch: 5, finger: 2, phase: TouchPhase::Motion, x: 0.25, y: 0.75, dx: 0.01, dy: -0.02, pressure: 1.0 });
        round_trip(InputEvent::Text("Ab ç".into()));
        round_trip(InputEvent::Mouse { window: 1, which: 0, event: MouseEvent::Motion { x: 10, y: 20, xrel: -1, yrel: 2, buttons: 1 } });
        round_trip(InputEvent::Mouse { window: 1, which: 0, event: MouseEvent::Button { button: 1, down: false, clicks: 2, x: 639, y: 479 } });
        round_trip(InputEvent::Mouse { window: 1, which: 0, event: MouseEvent::Wheel { x: 0, y: -1 } });
        round_trip(InputEvent::Tap { x: 320, y: 240 });
        round_trip(InputEvent::Other(SDL_EVENT_WINDOW_RESIZED.0));
    }

    #[test]
    fn consumer_keys_map_both_ways() {
        for &(s2, s3) in &CONSUMER_KEYS {
            assert_eq!(scancode_to_sdl2(s3), s2);
            assert_eq!(scancode_from_sdl2(s2), s3);
        }
        assert_eq!(scancode_to_sdl2(259), 0, "SDL3 Wake has no SDL2 scancode");
        assert_eq!(scancode_from_sdl2(265), 0, "SDL2 Mail has no SDL3 scancode");
        for s in (0..257).chain(287..=290) {
            assert_eq!(scancode_to_sdl2(s), s);
            assert_eq!(scancode_from_sdl2(s), s);
        }
        assert_eq!(key_to_sdl2(SDLK_MEDIA_NEXT_TRACK.0), SCANCODE_MASK | 258);
        assert_eq!(key_to_sdl2(SDLK_RETURN.0), 13);
    }
}
