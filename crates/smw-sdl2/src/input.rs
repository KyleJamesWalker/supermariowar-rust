//! SDL2 events to smw-platform's normalized `InputEvent` and back (docs/ARCHITECTURE_V2.md, Normalized input).
//! `from_sdl` keeps every field the game reads, so `to_sdl(&from_sdl(e))` reads the same as `e`.

use sdl2::sys::SDL_EventType::*;
use sdl2::sys::{SDL_Event, SDL_PRESSED, SDL_RELEASED};
use smw_platform::{DeviceChange, Input, InputEvent, Keycode, MouseEvent, PadSlot, TouchPhase};

fn state(down: bool) -> u8 {
    (if down { SDL_PRESSED } else { SDL_RELEASED }) as u8
}

pub fn from_sdl(e: &SDL_Event) -> Input {
    unsafe {
        let t = e.type_;
        let is = |k: sdl2::sys::SDL_EventType| t == k as u32;
        let event = if is(SDL_QUIT) {
            InputEvent::Quit
        } else if is(SDL_KEYDOWN) || is(SDL_KEYUP) {
            let k = e.key;
            InputEvent::Key {
                key: Keycode(k.keysym.sym),
                scancode: k.keysym.scancode as i32,
                mods: k.keysym.mod_,
                down: is(SDL_KEYDOWN),
                repeat: k.repeat != 0,
                window: k.windowID,
            }
        } else if is(SDL_JOYAXISMOTION) {
            InputEvent::PadAxis { pad: PadSlot(e.jaxis.which), axis: e.jaxis.axis, value: e.jaxis.value }
        } else if is(SDL_JOYBUTTONDOWN) || is(SDL_JOYBUTTONUP) {
            InputEvent::PadButton { pad: PadSlot(e.jbutton.which), button: e.jbutton.button, down: is(SDL_JOYBUTTONDOWN) }
        } else if is(SDL_JOYHATMOTION) {
            InputEvent::PadHat { pad: PadSlot(e.jhat.which), hat: e.jhat.hat, value: e.jhat.value }
        } else if is(SDL_JOYDEVICEADDED) || is(SDL_JOYDEVICEREMOVED) {
            let change = if is(SDL_JOYDEVICEADDED) { DeviceChange::Added } else { DeviceChange::Removed };
            InputEvent::Pad { pad: PadSlot(e.jdevice.which), change }
        } else if is(SDL_CONTROLLERAXISMOTION) {
            InputEvent::GamepadAxis { pad: PadSlot(e.caxis.which), axis: e.caxis.axis, value: e.caxis.value }
        } else if is(SDL_CONTROLLERBUTTONDOWN) || is(SDL_CONTROLLERBUTTONUP) {
            InputEvent::GamepadButton { pad: PadSlot(e.cbutton.which), button: e.cbutton.button, down: is(SDL_CONTROLLERBUTTONDOWN) }
        } else if is(SDL_CONTROLLERDEVICEADDED) || is(SDL_CONTROLLERDEVICEREMOVED) || is(SDL_CONTROLLERDEVICEREMAPPED) {
            let change = if is(SDL_CONTROLLERDEVICEADDED) {
                DeviceChange::Added
            } else if is(SDL_CONTROLLERDEVICEREMOVED) {
                DeviceChange::Removed
            } else {
                DeviceChange::Remapped
            };
            InputEvent::Gamepad { pad: PadSlot(e.cdevice.which), change }
        } else if is(SDL_FINGERDOWN) || is(SDL_FINGERMOTION) || is(SDL_FINGERUP) {
            let f = e.tfinger;
            let phase = if is(SDL_FINGERDOWN) {
                TouchPhase::Down
            } else if is(SDL_FINGERMOTION) {
                TouchPhase::Motion
            } else {
                TouchPhase::Up
            };
            InputEvent::Touch { touch: f.touchId, finger: f.fingerId, phase, x: f.x, y: f.y, dx: f.dx, dy: f.dy, pressure: f.pressure }
        } else if is(SDL_TEXTINPUT) {
            let text = &e.text.text;
            let len = text.iter().position(|&c| c == 0).unwrap_or(text.len());
            InputEvent::Text(String::from_utf8_lossy(&*(&text[..len] as *const [std::ffi::c_char] as *const [u8])).into_owned())
        } else if is(SDL_MOUSEMOTION) {
            let m = e.motion;
            InputEvent::Mouse {
                window: m.windowID,
                which: m.which,
                event: MouseEvent::Motion { x: m.x, y: m.y, xrel: m.xrel, yrel: m.yrel, buttons: m.state },
            }
        } else if is(SDL_MOUSEBUTTONDOWN) || is(SDL_MOUSEBUTTONUP) {
            let b = e.button;
            InputEvent::Mouse {
                window: b.windowID,
                which: b.which,
                event: MouseEvent::Button { button: b.button, down: is(SDL_MOUSEBUTTONDOWN), clicks: b.clicks, x: b.x, y: b.y },
            }
        } else if is(SDL_MOUSEWHEEL) {
            let w = e.wheel;
            InputEvent::Mouse { window: w.windowID, which: w.which, event: MouseEvent::Wheel { x: w.x, y: w.y } }
        } else {
            InputEvent::Other(t)
        };
        Input { timestamp: e.common.timestamp, event }
    }
}

pub fn to_sdl(input: &Input) -> SDL_Event {
    unsafe {
        let mut e: SDL_Event = std::mem::zeroed();
        match &input.event {
            InputEvent::Quit => e.type_ = SDL_QUIT as u32,
            &InputEvent::Key { key, scancode, mods, down, repeat, window } => {
                e.key.type_ = (if down { SDL_KEYDOWN } else { SDL_KEYUP }) as u32;
                e.key.windowID = window;
                e.key.state = state(down);
                e.key.repeat = repeat as u8;
                e.key.keysym.scancode = std::mem::transmute::<i32, sdl2::sys::SDL_Scancode>(scancode);
                e.key.keysym.sym = key.0;
                e.key.keysym.mod_ = mods;
            }
            &InputEvent::PadAxis { pad, axis, value } => {
                e.jaxis.type_ = SDL_JOYAXISMOTION as u32;
                e.jaxis.which = pad.0;
                e.jaxis.axis = axis;
                e.jaxis.value = value;
            }
            &InputEvent::PadButton { pad, button, down } => {
                e.jbutton.type_ = (if down { SDL_JOYBUTTONDOWN } else { SDL_JOYBUTTONUP }) as u32;
                e.jbutton.which = pad.0;
                e.jbutton.button = button;
                e.jbutton.state = state(down);
            }
            &InputEvent::PadHat { pad, hat, value } => {
                e.jhat.type_ = SDL_JOYHATMOTION as u32;
                e.jhat.which = pad.0;
                e.jhat.hat = hat;
                e.jhat.value = value;
            }
            &InputEvent::Pad { pad, change } => {
                e.jdevice.type_ = (if change == DeviceChange::Added { SDL_JOYDEVICEADDED } else { SDL_JOYDEVICEREMOVED }) as u32;
                e.jdevice.which = pad.0;
            }
            &InputEvent::GamepadAxis { pad, axis, value } => {
                e.caxis.type_ = SDL_CONTROLLERAXISMOTION as u32;
                e.caxis.which = pad.0;
                e.caxis.axis = axis;
                e.caxis.value = value;
            }
            &InputEvent::GamepadButton { pad, button, down } => {
                e.cbutton.type_ = (if down { SDL_CONTROLLERBUTTONDOWN } else { SDL_CONTROLLERBUTTONUP }) as u32;
                e.cbutton.which = pad.0;
                e.cbutton.button = button;
                e.cbutton.state = state(down);
            }
            &InputEvent::Gamepad { pad, change } => {
                e.cdevice.type_ = match change {
                    DeviceChange::Added => SDL_CONTROLLERDEVICEADDED,
                    DeviceChange::Removed => SDL_CONTROLLERDEVICEREMOVED,
                    DeviceChange::Remapped => SDL_CONTROLLERDEVICEREMAPPED,
                } as u32;
                e.cdevice.which = pad.0;
            }
            &InputEvent::Touch { touch, finger, phase, x, y, dx, dy, pressure } => {
                e.tfinger.type_ = match phase {
                    TouchPhase::Down => SDL_FINGERDOWN,
                    TouchPhase::Motion => SDL_FINGERMOTION,
                    TouchPhase::Up => SDL_FINGERUP,
                } as u32;
                e.tfinger.touchId = touch;
                e.tfinger.fingerId = finger;
                e.tfinger.x = x;
                e.tfinger.y = y;
                e.tfinger.dx = dx;
                e.tfinger.dy = dy;
                e.tfinger.pressure = pressure;
            }
            InputEvent::Text(text) => {
                e.text.type_ = SDL_TEXTINPUT as u32;
                for (dst, &b) in e.text.text.iter_mut().zip(text.as_bytes().iter().take(31)) {
                    *dst = b as std::ffi::c_char;
                }
            }
            &InputEvent::Mouse { window, which, event } => match event {
                MouseEvent::Motion { x, y, xrel, yrel, buttons } => {
                    e.motion.type_ = SDL_MOUSEMOTION as u32;
                    e.motion.windowID = window;
                    e.motion.which = which;
                    e.motion.state = buttons;
                    e.motion.x = x;
                    e.motion.y = y;
                    e.motion.xrel = xrel;
                    e.motion.yrel = yrel;
                }
                MouseEvent::Button { button, down, clicks, x, y } => {
                    e.button.type_ = (if down { SDL_MOUSEBUTTONDOWN } else { SDL_MOUSEBUTTONUP }) as u32;
                    e.button.windowID = window;
                    e.button.which = which;
                    e.button.button = button;
                    e.button.state = state(down);
                    e.button.clicks = clicks;
                    e.button.x = x;
                    e.button.y = y;
                }
                MouseEvent::Wheel { x, y } => {
                    e.wheel.type_ = SDL_MOUSEWHEEL as u32;
                    e.wheel.windowID = window;
                    e.wheel.which = which;
                    e.wheel.x = x;
                    e.wheel.y = y;
                }
            },
            &InputEvent::Other(t) => e.type_ = t,
        }
        e.common.timestamp = input.timestamp;
        e
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(event: InputEvent) {
        let input = Input { timestamp: 1234, event };
        assert_eq!(from_sdl(&to_sdl(&input)), input);
    }

    #[test]
    fn every_kind_round_trips() {
        let pad = PadSlot(3);
        for down in [false, true] {
            round_trip(InputEvent::Key { key: Keycode(0x4000_0052), scancode: 82, mods: 0x1000, down, repeat: down, window: 1 });
            round_trip(InputEvent::PadButton { pad, button: 15, down });
            round_trip(InputEvent::GamepadButton { pad, button: 11, down });
            round_trip(InputEvent::Mouse { window: 1, which: 0, event: MouseEvent::Button { button: 3, down, clicks: 2, x: 115, y: -4 } });
        }
        round_trip(InputEvent::Quit);
        round_trip(InputEvent::PadAxis { pad, axis: 5, value: -32768 });
        round_trip(InputEvent::PadHat { pad, hat: 0, value: 9 });
        round_trip(InputEvent::GamepadAxis { pad, axis: 2, value: 32767 });
        for change in [DeviceChange::Added, DeviceChange::Removed] {
            round_trip(InputEvent::Pad { pad, change });
        }
        for change in [DeviceChange::Added, DeviceChange::Removed, DeviceChange::Remapped] {
            round_trip(InputEvent::Gamepad { pad, change });
        }
        for phase in [TouchPhase::Down, TouchPhase::Motion, TouchPhase::Up] {
            round_trip(InputEvent::Touch { touch: -7, finger: 2, phase, x: 0.25, y: 0.75, dx: -0.5, dy: 0.125, pressure: 1.0 });
        }
        round_trip(InputEvent::Text("Kyle_7! é".to_string()));
        round_trip(InputEvent::Mouse { window: 1, which: 0, event: MouseEvent::Motion { x: 320, y: 240, xrel: -3, yrel: 6, buttons: 5 } });
        round_trip(InputEvent::Mouse { window: 1, which: 0, event: MouseEvent::Wheel { x: 0, y: -1 } });
        round_trip(InputEvent::Other(0x200));
    }

    #[test]
    fn keeps_the_sdl_fields_the_game_reads() {
        unsafe {
            let mut e: SDL_Event = std::mem::zeroed();
            e.key.type_ = SDL_KEYDOWN as u32;
            e.key.timestamp = 99;
            e.key.windowID = 2;
            e.key.state = SDL_PRESSED as u8;
            e.key.repeat = 1;
            e.key.keysym.scancode = sdl2::sys::SDL_Scancode::SDL_SCANCODE_RETURN;
            e.key.keysym.sym = 13;
            e.key.keysym.mod_ = 0x1000;
            let back = to_sdl(&from_sdl(&e));
            assert_eq!(std::slice::from_raw_parts(&e as *const _ as *const u8, 32), std::slice::from_raw_parts(&back as *const _ as *const u8, 32));
        }
    }
}
