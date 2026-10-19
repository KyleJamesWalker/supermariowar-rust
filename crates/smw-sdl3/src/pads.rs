//! SDL3 joysticks as smw-platform's `Pads`. With `SMW_PAD_TRANSLATE`, a pad SDL knows as a gamepad is opened through
//! the gamepad API and its events renumbered into `LAYOUT`, as smw-sdl2's pad.rs does from the raw events.

use sdl3_sys::everything::*;
use smw_platform::{Input, InputEvent, PadChange, PadFamily, PadInfo, PadSlot, Pads};
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::rc::Rc;

/// Joystick button per gamepad button, in `SDL_GamepadButton` order up to the right shoulder; -1 is handled apart.
const LAYOUT: [i32; 11] = [
    0,  // South (A)
    1,  // East (B)
    2,  // West (X)
    5,  // North (Y)
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
/// Hat bits for D-pad up, down, left and right.
const DPAD_BITS: [u8; 4] = [1, 4, 8, 2];

/// (name, GUID prefix, mapping), applied to the first entry matching a joystick that is no gamepad, as smw-sdl2's.
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
    index: usize,
    instance: u32,
    /// Opened as a gamepad, whose events are translated.
    controller: bool,
    joystick: *mut SDL_Joystick,
    gamepad: *mut SDL_Gamepad,
    dpad: u8,
    triggers: [bool; 2],
}

#[derive(Default)]
pub struct PadState {
    pads: Vec<Pad>,
    translate_on: bool,
    /// A live session: pads connected or removed while it runs open and close.
    hotplug_on: bool,
    hotplug_pending: bool,
    debug_input: bool,
    logged: [u32; 3],
}

impl PadState {
    fn pad(&mut self, slot: PadSlot) -> Option<&mut Pad> {
        self.pads.iter_mut().find(|p| p.instance == slot.0 as u32)
    }

    fn any_controller(&self) -> bool {
        self.pads.iter().any(|p| p.controller)
    }

    /// The input the game reads for one SDL delivers, or `None` to drop it.
    pub fn translate(&mut self, input: Input) -> Option<Input> {
        self.log(0, &input);
        let timestamp = input.timestamp;
        let event = match input.event {
            InputEvent::Pad { .. } => {
                self.hotplug_pending = true;
                Some(input.event)
            }
            InputEvent::PadButton { pad, .. } | InputEvent::PadAxis { pad, .. } | InputEvent::PadHat { pad, .. } => {
                if self.pad(pad).is_some_and(|p| p.controller) {
                    None
                } else {
                    Some(input.event)
                }
            }
            InputEvent::GamepadButton { pad, button, down } => match self.pad(pad).filter(|p| p.controller) {
                Some(p) => translate_button(p, button, down),
                None => (!self.any_controller()).then_some(input.event),
            },
            InputEvent::GamepadAxis { pad, axis, value } => match self.pad(pad).filter(|p| p.controller) {
                Some(p) => translate_axis(p, axis, value),
                None => (!self.any_controller()).then_some(input.event),
            },
            event => Some(event),
        };
        let input = Input { timestamp, event: event? };
        self.log(1, &input);
        Some(input)
    }

    /// `SMW_DEBUG_INPUT=1`: the first 2000 inputs at each stage: 0 as SDL queues them, 1 after translation, 2 as the
    /// game reads them.
    fn log(&mut self, stage: usize, input: &Input) {
        if !self.debug_input || self.logged[stage] >= 2000 {
            return;
        }
        let detail = match &input.event {
            InputEvent::PadButton { pad, button, down } => format!("jbutton which {} button {} state {}", pad.0, button, *down as u8),
            InputEvent::PadHat { pad, hat, value } => format!("jhat which {} hat {} value {}", pad.0, hat, value),
            InputEvent::PadAxis { pad, axis, value } => format!("jaxis which {} axis {} value {}", pad.0, axis, value),
            InputEvent::GamepadButton { pad, button, down } => format!("cbutton which {} button {} state {}", pad.0, button, *down as u8),
            InputEvent::GamepadAxis { pad, axis, value } => format!("caxis which {} axis {} value {}", pad.0, axis, value),
            InputEvent::Key { key, scancode, down, .. } => format!("key {} sym {} scancode {}", if *down { "down" } else { "up" }, key.0, scancode),
            InputEvent::Quit => String::from("quit"),
            InputEvent::QuitRequest => String::from("quit request"),
            _ => return,
        };
        self.logged[stage] += 1;
        println!("[input] {} {}", ["raw", "translated", "game"][stage], detail);
    }
}

fn translate_button(p: &mut Pad, button: u8, down: bool) -> Option<InputEvent> {
    let pad = PadSlot(p.instance as i32);
    if button == GUIDE {
        return down.then_some(InputEvent::QuitRequest);
    }
    if (DPAD_UP..DPAD_UP + 4).contains(&button) {
        let bit = DPAD_BITS[(button - DPAD_UP) as usize];
        return set_dpad(p, if down { p.dpad | bit } else { p.dpad & !bit });
    }
    let target = *LAYOUT.get(button as usize)?;
    Some(InputEvent::PadButton { pad, button: target as u8, down })
}

fn translate_axis(p: &mut Pad, axis: u8, value: i16) -> Option<InputEvent> {
    let pad = PadSlot(p.instance as i32);
    match axis {
        0..=3 => Some(InputEvent::PadAxis { pad, axis, value }),
        4 | 5 => {
            let side = (axis - 4) as usize;
            let pressed = value >= TRIGGER_THRESHOLD;
            if pressed == p.triggers[side] {
                return None;
            }
            p.triggers[side] = pressed;
            Some(InputEvent::PadButton { pad, button: TRIGGER_BUTTONS[side], down: pressed })
        }
        _ => None,
    }
}

fn set_dpad(p: &mut Pad, dpad: u8) -> Option<InputEvent> {
    if dpad == p.dpad {
        return None;
    }
    p.dpad = dpad;
    Some(InputEvent::PadHat { pad: PadSlot(p.instance as i32), hat: 0, value: dpad })
}

unsafe fn c_string(p: *const std::ffi::c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

/// The connected joysticks, in SDL's order: the backend device indices.
fn joystick_ids() -> Vec<SDL_JoystickID> {
    unsafe {
        let mut count = 0;
        let ids = SDL_GetJoysticks(&mut count);
        if ids.is_null() {
            return Vec::new();
        }
        let list = std::slice::from_raw_parts(ids, count.max(0) as usize).to_vec();
        SDL_free(ids as *mut _);
        list
    }
}

pub struct Sdl3Pads {
    state: Rc<RefCell<PadState>>,
}

impl Sdl3Pads {
    /// Shares `state` with the event queue, which translates through it.
    pub fn new(state: Rc<RefCell<PadState>>) -> Self {
        Sdl3Pads { state }
    }

    /// Opens joystick `id` as device number `index`.
    fn open_pad(&mut self, id: SDL_JoystickID, index: usize) -> Option<PadInfo> {
        unsafe {
            let mut st = self.state.borrow_mut();
            let name = c_string(SDL_GetJoystickNameForID(id));
            let mut guid = [0 as std::ffi::c_char; 33];
            SDL_GUIDToString(SDL_GetJoystickGUIDForID(id), guid.as_mut_ptr(), guid.len() as i32);
            let guid = CStr::from_ptr(guid.as_ptr()).to_string_lossy().into_owned();
            if st.translate_on && !SDL_IsGamepad(id) {
                if let Some((_, _, mapping)) = BUILTIN_MAPPINGS.iter().find(|(n, prefix, _)| *n == name && guid.starts_with(prefix)) {
                    if let Ok(line) = CString::new(format!("{},{},{}platform:Linux,", guid, name, mapping)) {
                        SDL_AddGamepadMapping(line.as_ptr());
                    }
                }
            }
            let (mut joystick, mut gamepad) = (std::ptr::null_mut(), std::ptr::null_mut());
            if st.translate_on && SDL_IsGamepad(id) {
                gamepad = SDL_OpenGamepad(id);
                if !gamepad.is_null() {
                    joystick = SDL_GetGamepadJoystick(gamepad);
                }
            }
            if joystick.is_null() {
                joystick = SDL_OpenJoystick(id);
            }
            if joystick.is_null() {
                return None;
            }
            let mapping = SDL_GetGamepadMappingForID(id);
            let mapping_text = if mapping.is_null() { String::from("none") } else { c_string(mapping) };
            if !mapping.is_null() {
                SDL_free(mapping as *mut _);
            }
            let controller = !gamepad.is_null();
            println!(
                "[pad] joystick {}: \"{}\" guid {} buttons {} hats {} axes {}; mapping {}{}",
                index,
                name,
                guid,
                SDL_GetNumJoystickButtons(joystick),
                SDL_GetNumJoystickHats(joystick),
                SDL_GetNumJoystickAxes(joystick),
                mapping_text,
                if controller { " (translated)" } else { "" }
            );
            st.pads.push(Pad { index, instance: id.0, controller, joystick, gamepad, dpad: 0, triggers: [false; 2] });
            Some(PadInfo { pad: PadSlot(id.0 as i32), name, guid, controller })
        }
    }

    fn close(&mut self, instance: u32) {
        let mut st = self.state.borrow_mut();
        let Some(i) = st.pads.iter().position(|p| p.instance == instance) else { return };
        let pad = st.pads.remove(i);
        println!("[pad] joystick {} removed", pad.index);
        unsafe {
            if pad.gamepad.is_null() {
                SDL_CloseJoystick(pad.joystick);
            } else {
                SDL_CloseGamepad(pad.gamepad);
            }
        }
    }
}

impl Pads for Sdl3Pads {
    fn open_all(&mut self, open: bool, translate: bool) -> Vec<Option<PadInfo>> {
        unsafe {
            SDL_InitSubSystem(SDL_INIT_JOYSTICK | SDL_INIT_GAMEPAD);
            {
                let mut st = self.state.borrow_mut();
                st.translate_on = translate;
                st.hotplug_on = open;
                st.debug_input = std::env::var_os("SMW_DEBUG_INPUT").is_some();
            }
            let ids = joystick_ids();
            println!("[pad] SDL_GetJoysticks = {}", ids.len());
            if !open {
                return Vec::new();
            }
            ids.into_iter().enumerate().map(|(i, id)| self.open_pad(id, i)).collect()
        }
    }

    fn hotplug(&mut self) -> Vec<PadChange> {
        let mut changes = Vec::new();
        {
            let mut st = self.state.borrow_mut();
            if !st.hotplug_on || !std::mem::take(&mut st.hotplug_pending) {
                return changes;
            }
        }
        let mut open: Vec<(usize, u32, *mut SDL_Joystick)> = self.state.borrow().pads.iter().map(|p| (p.index, p.instance, p.joystick)).collect();
        open.sort_unstable_by_key(|&(index, _, _)| index);
        for (_, instance, joystick) in open {
            if !unsafe { SDL_JoystickConnected(joystick) } {
                self.close(instance);
                changes.push(PadChange::Removed(PadSlot(instance as i32)));
            }
        }
        for id in joystick_ids() {
            if !self.state.borrow().pads.iter().any(|p| p.instance == id.0) {
                changes.push(PadChange::Connected(id.0 as i32));
            }
        }
        changes
    }

    /// `device` is the joystick's instance ID, which `hotplug` reports.
    fn open(&mut self, device: i32, index: usize) -> Option<PadInfo> {
        self.open_pad(SDL_JoystickID(device as u32), index)
    }

    fn name(&self, index: usize, pad: Option<PadSlot>) -> Option<String> {
        let id = match pad {
            Some(pad) => SDL_JoystickID(pad.0 as u32),
            None => *joystick_ids().get(index)?,
        };
        let p = unsafe { SDL_GetJoystickNameForID(id) };
        (!p.is_null()).then(|| unsafe { c_string(p) })
    }

    fn family(&self, index: usize) -> PadFamily {
        let Some(&id) = joystick_ids().get(index) else { return PadFamily::Generic };
        match unsafe { SDL_GetGamepadTypeForID(id) } {
            SDL_GAMEPAD_TYPE_XBOX360 | SDL_GAMEPAD_TYPE_XBOXONE => PadFamily::Xbox,
            SDL_GAMEPAD_TYPE_PS3 | SDL_GAMEPAD_TYPE_PS4 | SDL_GAMEPAD_TYPE_PS5 => PadFamily::PlayStation,
            SDL_GAMEPAD_TYPE_NINTENDO_SWITCH_PRO
            | SDL_GAMEPAD_TYPE_NINTENDO_SWITCH_JOYCON_LEFT
            | SDL_GAMEPAD_TYPE_NINTENDO_SWITCH_JOYCON_RIGHT
            | SDL_GAMEPAD_TYPE_NINTENDO_SWITCH_JOYCON_PAIR => PadFamily::Nintendo,
            _ => PadFamily::Generic,
        }
    }

    fn set_led(&mut self, player: i32, rgb: [u8; 3]) {
        unsafe {
            let js = SDL_GetJoystickFromPlayerIndex(player);
            if !js.is_null() {
                SDL_SetJoystickLED(js, rgb[0], rgb[1], rgb[2]);
            }
        }
    }

    fn log_input(&mut self, input: &Input) {
        self.state.borrow_mut().log(2, input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(controller: bool) -> PadState {
        let pad = Pad { index: 0, instance: 7, controller, joystick: std::ptr::null_mut(), gamepad: std::ptr::null_mut(), dpad: 0, triggers: [false; 2] };
        PadState { pads: vec![pad], ..Default::default() }
    }

    fn run(st: &mut PadState, event: InputEvent) -> Option<InputEvent> {
        st.translate(Input { timestamp: 5, event }).map(|i| i.event)
    }

    const PAD: PadSlot = PadSlot(7);
    fn button(button: u8, down: bool) -> InputEvent {
        InputEvent::GamepadButton { pad: PAD, button, down }
    }
    fn axis(axis: u8, value: i16) -> InputEvent {
        InputEvent::GamepadAxis { pad: PAD, axis, value }
    }
    fn pad_button(button: u8, down: bool) -> Option<InputEvent> {
        Some(InputEvent::PadButton { pad: PAD, button, down })
    }
    fn hat(value: u8) -> Option<InputEvent> {
        Some(InputEvent::PadHat { pad: PAD, hat: 0, value })
    }

    #[test]
    fn face_buttons_and_triggers_take_the_layout() {
        let mut st = state(true);
        for (b, want) in [(0, 0), (1, 1), (2, 2), (3, 5), (4, 4), (6, 3), (7, 10), (8, 11), (9, 6), (10, 7)] {
            assert_eq!(run(&mut st, button(b, true)), pad_button(want, true), "gamepad button {}", b);
        }
        assert_eq!(run(&mut st, axis(4, 32767)), pad_button(8, true));
        assert_eq!(run(&mut st, axis(4, 30000)), None);
        assert_eq!(run(&mut st, axis(4, 0)), pad_button(8, false));
        assert_eq!(run(&mut st, axis(5, 20000)), pad_button(9, true));
        assert_eq!(run(&mut st, button(GUIDE, true)), Some(InputEvent::QuitRequest));
        assert_eq!(run(&mut st, button(GUIDE, false)), None);
        assert_eq!(run(&mut st, button(15, true)), None, "misc buttons are dropped");
    }

    #[test]
    fn dpad_is_hat_0() {
        let mut st = state(true);
        assert_eq!(run(&mut st, button(11, true)), hat(1));
        assert_eq!(run(&mut st, button(14, true)), hat(1 | 2));
        assert_eq!(run(&mut st, button(11, false)), hat(2));
        assert_eq!(run(&mut st, button(14, false)), hat(0));
        assert_eq!(run(&mut st, button(12, true)), hat(4));
        assert_eq!(run(&mut st, button(13, true)), hat(4 | 8));
    }

    #[test]
    fn sticks_keep_their_axes() {
        let mut st = state(true);
        assert_eq!(run(&mut st, axis(1, -32768)), Some(InputEvent::PadAxis { pad: PAD, axis: 1, value: -32768 }));
        assert_eq!(run(&mut st, axis(3, 100)), Some(InputEvent::PadAxis { pad: PAD, axis: 3, value: 100 }));
    }

    #[test]
    fn a_translated_pads_raw_events_are_dropped() {
        let mut st = state(true);
        assert_eq!(run(&mut st, InputEvent::PadButton { pad: PAD, button: 0, down: true }), None);
        assert_eq!(run(&mut st, InputEvent::PadHat { pad: PAD, hat: 0, value: 1 }), None);
        assert_eq!(run(&mut st, InputEvent::PadAxis { pad: PAD, axis: 0, value: 5 }), None);
        let other = InputEvent::PadButton { pad: PadSlot(8), button: 0, down: true };
        assert_eq!(run(&mut st, other.clone()), Some(other));
    }

    #[test]
    fn a_plain_joystick_passes_unchanged() {
        let mut st = state(false);
        let raw = InputEvent::PadButton { pad: PAD, button: 3, down: true };
        assert_eq!(run(&mut st, raw.clone()), Some(raw));
        assert_eq!(run(&mut st, button(0, true)), Some(button(0, true)));
    }

    #[test]
    fn device_events_mark_a_hotplug() {
        let mut st = state(false);
        let added = InputEvent::Pad { pad: PadSlot(9), change: smw_platform::DeviceChange::Added };
        assert_eq!(run(&mut st, added.clone()), Some(added));
        assert!(st.hotplug_pending);
    }
}
