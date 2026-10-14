//! The editors' last input event. They read SDL2 union fields after their poll loops and in arms for other event types,
//! so each accessor returns what that field of SDL2's `SDL_Event` holds for the event, whatever its type.

use smw_platform::{DeviceChange, Input, InputEvent, MouseEvent, TouchPhase};

const QUIT: u32 = 0x100;
const KEYDOWN: u32 = 0x300;
const KEYUP: u32 = 0x301;
const TEXTINPUT: u32 = 0x303;
const MOUSEMOTION: u32 = 0x400;
const MOUSEBUTTONDOWN: u32 = 0x401;
const MOUSEBUTTONUP: u32 = 0x402;
const MOUSEWHEEL: u32 = 0x403;
const JOYAXISMOTION: u32 = 0x600;
const JOYHATMOTION: u32 = 0x602;
const JOYBUTTONDOWN: u32 = 0x603;
const JOYBUTTONUP: u32 = 0x604;
const JOYDEVICEADDED: u32 = 0x605;
const JOYDEVICEREMOVED: u32 = 0x606;
const CONTROLLERAXISMOTION: u32 = 0x650;
const CONTROLLERBUTTONDOWN: u32 = 0x651;
const CONTROLLERBUTTONUP: u32 = 0x652;
const CONTROLLERDEVICEADDED: u32 = 0x653;
const CONTROLLERDEVICEREMOVED: u32 = 0x654;
const CONTROLLERDEVICEREMAPPED: u32 = 0x655;
const FINGERDOWN: u32 = 0x700;
const FINGERUP: u32 = 0x701;
const FINGERMOTION: u32 = 0x702;

pub struct EditorEvent(Input);

impl EditorEvent {
    /// A zeroed `SDL_Event`.
    pub const fn new() -> Self {
        EditorEvent(Input { timestamp: 0, event: InputEvent::Other(0) })
    }

    pub fn set(&mut self, input: Input) {
        self.0 = input;
    }

    pub fn input(&self) -> &Input {
        &self.0
    }

    pub fn type_(&self) -> u32 {
        let device = |change: DeviceChange, added, removed, remapped| match change {
            DeviceChange::Added => added,
            DeviceChange::Removed => removed,
            DeviceChange::Remapped => remapped,
        };
        match &self.0.event {
            InputEvent::Key { down, .. } => if *down { KEYDOWN } else { KEYUP },
            InputEvent::PadAxis { .. } => JOYAXISMOTION,
            InputEvent::PadButton { down, .. } => if *down { JOYBUTTONDOWN } else { JOYBUTTONUP },
            InputEvent::PadHat { .. } => JOYHATMOTION,
            InputEvent::Pad { change, .. } => device(*change, JOYDEVICEADDED, JOYDEVICEREMOVED, JOYDEVICEREMOVED),
            InputEvent::GamepadAxis { .. } => CONTROLLERAXISMOTION,
            InputEvent::GamepadButton { down, .. } => if *down { CONTROLLERBUTTONDOWN } else { CONTROLLERBUTTONUP },
            InputEvent::Gamepad { change, .. } => device(*change, CONTROLLERDEVICEADDED, CONTROLLERDEVICEREMOVED, CONTROLLERDEVICEREMAPPED),
            InputEvent::Touch { phase, .. } => match phase {
                TouchPhase::Down => FINGERDOWN,
                TouchPhase::Motion => FINGERMOTION,
                TouchPhase::Up => FINGERUP,
            },
            InputEvent::Text(_) => TEXTINPUT,
            InputEvent::Mouse { event, .. } => match event {
                MouseEvent::Motion { .. } => MOUSEMOTION,
                MouseEvent::Button { down, .. } => if *down { MOUSEBUTTONDOWN } else { MOUSEBUTTONUP },
                MouseEvent::Wheel { .. } => MOUSEWHEEL,
            },
            InputEvent::Quit => QUIT,
            InputEvent::Other(t) => *t,
        }
    }

    /// The little-endian words at byte offsets 16, 20 and 24 of `SDL_Event`, where every field the editors read lives.
    fn words(&self) -> [u32; 3] {
        match &self.0.event {
            &InputEvent::Key { key, scancode, mods, .. } => [scancode as u32, key.0 as u32, mods as u32],
            &InputEvent::PadAxis { value, .. } | &InputEvent::GamepadAxis { value, .. } => [value as u16 as u32, 0, 0],
            &InputEvent::Touch { finger, x, .. } => [finger as u32, (finger as u64 >> 32) as u32, x.to_bits()],
            InputEvent::Text(text) => {
                let mut bytes = [0u8; 12];
                for (dst, &b) in bytes.iter_mut().zip(text.as_bytes().iter().take(31).skip(4)) {
                    *dst = b;
                }
                let word = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
                [word(0), word(4), word(8)]
            }
            &InputEvent::Mouse { event, .. } => match event {
                MouseEvent::Motion { x, y, buttons, .. } => [buttons, x as u32, y as u32],
                MouseEvent::Button { button, down, clicks, x, y } => {
                    [u32::from_le_bytes([button, down as u8, clicks, 0]), x as u32, y as u32]
                }
                MouseEvent::Wheel { x, y } => [x as u32, y as u32, 0],
            },
            _ => [0; 3],
        }
    }

    pub fn key_sym(&self) -> i32 {
        self.words()[1] as i32
    }

    pub fn key_mod(&self) -> u16 {
        self.words()[2] as u16
    }

    pub fn button_button(&self) -> u8 {
        self.words()[0] as u8
    }

    pub fn button_x(&self) -> i32 {
        self.words()[1] as i32
    }

    pub fn button_y(&self) -> i32 {
        self.words()[2] as i32
    }

    pub fn motion_state(&self) -> u32 {
        self.words()[0]
    }

    pub fn motion_x(&self) -> i32 {
        self.words()[1] as i32
    }

    pub fn motion_y(&self) -> i32 {
        self.words()[2] as i32
    }
}

impl Default for EditorEvent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smw_platform::{Keycode, PadSlot};

    fn check(event: InputEvent) {
        let input = Input { timestamp: 77, event };
        let sdl = smw_sdl2::input::to_sdl(&input);
        let mut e = EditorEvent::new();
        e.set(input.clone());
        unsafe {
            assert_eq!(e.type_(), sdl.type_, "{input:?}");
            assert_eq!(e.key_sym(), sdl.key.keysym.sym, "{input:?}");
            assert_eq!(e.key_mod(), sdl.key.keysym.mod_, "{input:?}");
            assert_eq!(e.button_button(), sdl.button.button, "{input:?}");
            assert_eq!(e.button_x(), sdl.button.x, "{input:?}");
            assert_eq!(e.button_y(), sdl.button.y, "{input:?}");
            assert_eq!(e.motion_state(), sdl.motion.state, "{input:?}");
            assert_eq!(e.motion_x(), sdl.motion.x, "{input:?}");
            assert_eq!(e.motion_y(), sdl.motion.y, "{input:?}");
        }
    }

    #[test]
    fn reads_the_sdl2_union_fields_of_every_kind() {
        let pad = PadSlot(3);
        for down in [false, true] {
            check(InputEvent::Key { key: Keycode(0x4000_0052), scancode: 82, mods: 0x1001, down, repeat: down, window: 1 });
            check(InputEvent::PadButton { pad, button: 15, down });
            check(InputEvent::GamepadButton { pad, button: 11, down });
            check(InputEvent::Mouse { window: 1, which: 0, event: MouseEvent::Button { button: 3, down, clicks: 2, x: 115, y: -4 } });
        }
        check(InputEvent::Quit);
        check(InputEvent::PadAxis { pad, axis: 5, value: -32768 });
        check(InputEvent::PadHat { pad, hat: 0, value: 9 });
        check(InputEvent::GamepadAxis { pad, axis: 2, value: 32767 });
        for change in [DeviceChange::Added, DeviceChange::Removed] {
            check(InputEvent::Pad { pad, change });
        }
        for change in [DeviceChange::Added, DeviceChange::Removed, DeviceChange::Remapped] {
            check(InputEvent::Gamepad { pad, change });
        }
        for phase in [TouchPhase::Down, TouchPhase::Motion, TouchPhase::Up] {
            check(InputEvent::Touch { touch: -7, finger: -2, phase, x: 0.25, y: 0.75, dx: -0.5, dy: 0.125, pressure: 1.0 });
        }
        check(InputEvent::Text("Kyle_7! é and some more text past 32 bytes".to_string()));
        check(InputEvent::Text("ab".to_string()));
        check(InputEvent::Mouse { window: 1, which: 0, event: MouseEvent::Motion { x: 320, y: -240, xrel: -3, yrel: 6, buttons: 5 } });
        check(InputEvent::Mouse { window: 1, which: 0, event: MouseEvent::Wheel { x: -2, y: 7 } });
        check(InputEvent::Other(0x200));
    }

    #[test]
    fn starts_zeroed() {
        let e = EditorEvent::new();
        assert_eq!((e.type_(), e.button_x(), e.button_y(), e.motion_state()), (0, 0, 0, 0));
    }
}
