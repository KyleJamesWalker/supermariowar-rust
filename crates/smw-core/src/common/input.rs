//! Key bindings and per-player key state: turns keyboard, mouse and joystick events into game and menu keys.
//!
//! `keys[NUM_KEYS]` holds a control set's bindings or states, and named accessors (`game_jump()` /
//! `game_jump_mut()`) reach each slot; the game and menu names alias the same eight slots.

use crate::common::keys::*;
use crate::game::Game;
use smw_platform::{Input, InputEvent, MouseEvent};
use crate::common::global_constants::MAX_PLAYERS;
use crate::globals::*;
use crate::common::keys::{SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP};

/// Slots in one control set.
pub const NUM_KEYS: usize = 8;
/// The start key's slot in `keys`.
pub const KEY_GAME_START: usize = 6;

/// `CInputPlayerControl::iDevice` for the keyboard; a joystick uses its device number.
pub const DEVICE_KEYBOARD: i16 = -1;

/// Axis values within this of center count as released.
pub const JOYSTICK_DEAD_ZONE: i32 = 16384;
pub const MOUSE_X_DEAD_ZONE: i32 = 0;
pub const MOUSE_Y_DEAD_ZONE: i32 = 5;

/// Binding codes for mouse motion; `MOUSE_BUTTON_START + n` is mouse button `n`.
pub const MOUSE_UP: i32 = 323;
pub const MOUSE_DOWN: i32 = 324;
pub const MOUSE_LEFT: i32 = 325;
pub const MOUSE_RIGHT: i32 = 326;
pub const MOUSE_BUTTON_START: i32 = 327;

/// Binding codes for joystick sticks and hat; `JOY_BUTTON_START + n` is button `n`.
pub const JOY_STICK_1_UP: i32 = 0;
pub const JOY_STICK_1_DOWN: i32 = 1;
pub const JOY_STICK_1_LEFT: i32 = 2;
pub const JOY_STICK_1_RIGHT: i32 = 3;

pub const JOY_STICK_2_UP: i32 = 4;
pub const JOY_STICK_2_DOWN: i32 = 5;
pub const JOY_STICK_2_LEFT: i32 = 6;
pub const JOY_STICK_2_RIGHT: i32 = 7;

pub const JOY_HAT_UP: i32 = 8;
pub const JOY_HAT_DOWN: i32 = 9;
pub const JOY_HAT_LEFT: i32 = 10;
pub const JOY_HAT_RIGHT: i32 = 11;
pub const JOY_BUTTON_START: i32 = 12;

/// An unbound slot.
pub const KEY_NONE: i32 = -1;

/// One key's state this tick.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct CKeyState {
    /// Held down.
    pub fDown: bool,
    /// Went down this tick; `clear_pressed_keys` clears it.
    pub fPressed: bool,
}

macro_rules! key_aliases {
    ($ty:ty, $elem:ty; $($name:ident, $name_mut:ident = $idx:expr;)*) => {
        impl $ty {
            $(
                #[inline(always)]
                pub fn $name(&self) -> &$elem { &self.keys[$idx] }
                #[inline(always)]
                pub fn $name_mut(&mut self) -> &mut $elem { &mut self.keys[$idx] }
            )*
        }
    };
}

macro_rules! all_key_aliases {
    ($ty:ty, $elem:ty) => {
        key_aliases!($ty, $elem;
            menu_up, menu_up_mut = 0;
            menu_down, menu_down_mut = 1;
            menu_left, menu_left_mut = 2;
            menu_right, menu_right_mut = 3;
            menu_select, menu_select_mut = 4;
            menu_cancel, menu_cancel_mut = 5;
            menu_random, menu_random_mut = 6;
            menu_scrollfast, menu_scrollfast_mut = 7;
            game_left, game_left_mut = 0;
            game_right, game_right_mut = 1;
            game_jump, game_jump_mut = 2;
            game_down, game_down_mut = 3;
            game_turbo, game_turbo_mut = 4;
            game_powerup, game_powerup_mut = 5;
            game_start, game_start_mut = KEY_GAME_START;
            game_cancel, game_cancel_mut = 7;
        );
    };
}

/// One control set's bindings: a keycode, mouse code or joystick code per slot.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct CInputControl {
    pub keys: [Keycode; NUM_KEYS],
}
all_key_aliases!(CInputControl, Keycode);

/// A player's key states this tick.
#[derive(Clone, Copy, Default, Debug)]
pub struct COutputControl {
    pub keys: [CKeyState; NUM_KEYS],
}
all_key_aliases!(COutputControl, CKeyState);

impl COutputControl {
    pub fn copy_from(&mut self, other: &COutputControl) {
        for k in 0..NUM_KEYS {
            self.keys[k] = other.keys[k];
        }
    }
}

impl PartialEq for COutputControl {
    fn eq(&self, control: &COutputControl) -> bool {
        for k in 0..NUM_KEYS {
            if self.keys[k].fDown != control.keys[k].fDown || self.keys[k].fPressed != control.keys[k].fPressed {
                return false;
            }
        }
        true
    }
}

/// A device and its bindings.
#[derive(Clone, Copy, Default, Debug)]
pub struct CInputPlayerControl {
    /// `DEVICE_KEYBOARD`, or the joystick's device number.
    pub iDevice: i16,
    pub inputGameControls: [CInputControl; 2], //0 == game controls, 1 == menu controls
}

/// Turns input events into each player's key states.
#[derive(Clone)]

pub struct CPlayerInput {
    /// Each player's `inputConfiguration[set][kind]` (kind 0 is the keyboard, 1 the joystick); `Game::input_control` reads it.
    pub inputControls: [Option<(u8, u8)>; 4],

    //Use these structures to see what input has been pressed
    pub outputControls: [COutputControl; 4],

    /// The last key pressed this tick, for fields that read raw keys; 0 for none.
    pub iPressedKey: Keycode,
    /// Per joystick (`which`), the stick 1 and hat directions held (up, down, left, right).
    joyDirections: Vec<(i32, [bool; 4], [bool; 4])>,
    pub _alias: Aliased,
}

/// The direction (up, down, left, right) of a stick 1 or hat binding. A joystick's
/// stick 1 and hat drive the same direction bindings, so either one moves a player or a menu.
fn joy_direction(key: Keycode) -> Option<usize> {
    match key {
        JOY_STICK_1_UP..=JOY_STICK_1_RIGHT => Some(key as usize),
        JOY_HAT_UP..=JOY_HAT_RIGHT => Some((key - JOY_HAT_UP) as usize),
        _ => None,
    }
}

impl Default for CPlayerInput {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn sdl_button(x: i32) -> u32 {
    1u32.wrapping_shl((x - 1) as u32)
}

impl CPlayerInput {
    pub fn new() -> Self {
        let mut this = CPlayerInput { _alias: Aliased::new(),
            inputControls: [None; 4],
            outputControls: [COutputControl::default(); 4],
            iPressedKey: 0,
            joyDirections: Vec::new(),
        };
        for iPlayer in 0..MAX_PLAYERS as usize {
            for iKey in 0..NUM_KEYS {
                this.outputControls[iPlayer].keys[iKey].fPressed = false;
                this.outputControls[iPlayer].keys[iKey].fDown = false;
            }
        }

        this.iPressedKey = 0;
        this
    }

    //Pass in 0 for game and 1 for menu
    //Clear old button pushed states
    pub fn clear_pressed_keys(&mut self, iGameState: i16) {
        for iPlayer in 0..MAX_PLAYERS as usize {
            let outputControl = &mut self.outputControls[iPlayer];
            for iKey in 0..NUM_KEYS {
                outputControl.keys[iKey].fPressed = false;
            }
        }

        self.iPressedKey = 0;
    }

    /// Releases every game key but left, right, jump, down and turbo.
    pub fn clear_game_action_keys(&mut self) {
        for iPlayer in 0..MAX_PLAYERS as usize {
            let outputControl = &mut self.outputControls[iPlayer];

            // 0-4: left, right, jump, down, turbo
            for iKey in 5..NUM_KEYS {
                outputControl.keys[iKey].fPressed = false;
                outputControl.keys[iKey].fDown = false;
            }
        }

        self.iPressedKey = 0;
    }

    /// The held stick and hat directions, for replay checkpoints (smw/checkpoint.rs).
    pub fn joy_directions_mut(&mut self) -> &mut Vec<(i32, [bool; 4], [bool; 4])> {
        &mut self.joyDirections
    }

    //Clear all button pushed and down states
    //Call this when switching from menu to game
    pub fn reset_keys(&mut self) {
        for iPlayer in 0..MAX_PLAYERS as usize {
            for iKey in 0..NUM_KEYS {
                self.outputControls[iPlayer].keys[iKey].fPressed = false;
                self.outputControls[iPlayer].keys[iKey].fDown = false;
            }
        }

        self.iPressedKey = 0;
    }

    //Called during game loop to read input events and see if
    //configured keys were pressed.  If they were, then turn on
    //key flags to be used by game logic
    //iGameState == 0 for in game and 1 for menu
    pub fn update_input(&mut self, g: &mut Game, input: &Input, iGameState: i16) {
        crate::services::services(g).pads.log_input(input);
        let event = &input.event;
        use InputEvent as E;
        use MouseEvent as M;
        let key_sym = if let E::Key { key, .. } = *event { key.0 } else { 0 };
        let (motion_xrel, motion_yrel, motion_state) = if let E::Mouse { event: M::Motion { xrel, yrel, buttons, .. }, .. } = *event { (xrel, yrel, buttons) } else { (0, 0, 0) };
        let mouse_button = if let E::Mouse { event: M::Button { button, .. }, .. } = *event { button } else { 0 };
        let (jhat_which, jhat_value) = if let E::PadHat { pad, value, .. } = *event { (pad.0, value) } else { (0, 0) };
        let (jaxis_which, jaxis_axis, jaxis_value) = if let E::PadAxis { pad, axis, value } = *event { (pad.0, axis, value) } else { (0, 0, 0) };
        let (jbutton_which, jbutton_button) = if let E::PadButton { pad, button, .. } = *event { (pad.0, button) } else { (0, 0) };
        unsafe {

            let mut joyDirection: Option<([bool; 4], &[usize])> = None;
            if matches!(event, E::PadHat { .. }) || (matches!(event, E::PadAxis { .. }) && jaxis_axis < 2) {
                let which = if matches!(event, E::PadHat { .. }) { jhat_which } else { jaxis_which };
                let i = match self.joyDirections.iter().position(|d| d.0 == which) {
                    Some(i) => i,
                    None => {
                        self.joyDirections.push((which, [false; 4], [false; 4]));
                        self.joyDirections.len() - 1
                    }
                };
                let (_, stick, hat) = &mut self.joyDirections[i];
                let changed: &[usize] = if matches!(event, E::PadHat { .. }) {
                    let value = jhat_value as u32;
                    *hat = [value & SDL_HAT_UP != 0, value & SDL_HAT_DOWN != 0, value & SDL_HAT_LEFT != 0, value & SDL_HAT_RIGHT != 0];
                    &[0, 1, 2, 3]
                } else {
                    let value = jaxis_value as i32;
                    let (neg, pos) = if jaxis_axis == 0 { (2, 3) } else { (0, 1) };
                    stick[neg] = value < -JOYSTICK_DEAD_ZONE;
                    stick[pos] = value > JOYSTICK_DEAD_ZONE;
                    if jaxis_axis == 0 {
                        &[2, 3]
                    } else {
                        &[0, 1]
                    }
                };
                joyDirection = Some(([0, 1, 2, 3].map(|d| stick[d] || hat[d]), changed));
            }

            let mut fFound = false;
            for iPlayer in -1..MAX_PLAYERS as i16 {
                let inputControl: *const CInputControl;
                let outputControl: *mut COutputControl;
                let mut iDeviceID: i16 = DEVICE_KEYBOARD;

                //Allow keyboard input from player 1 in menus (even when they are configured to use joystick)
                // unless another player now has those keys (see assign_inputs).
                if iPlayer == -1 {
                    let keyboard = &mut g.game_values.inputConfiguration[0][0] as *mut CInputPlayerControl;
                    if iGameState == 1 && g.input_control(self.inputControls[0]).iDevice != DEVICE_KEYBOARD && !self.inputControls.contains(&Some((0, 0))) {
                        inputControl = &g.game_values.inputConfiguration[0][0].inputGameControls[1];
                        outputControl = &mut self.outputControls[0];
                        iDeviceID = g.game_values.inputConfiguration[0][0].iDevice;
                    } else {
                        continue;
                    }
                } else {
                    if g.input_control(self.inputControls[iPlayer as usize]).is_null() {
                        continue;
                    }

                    inputControl = &g.input_control(self.inputControls[iPlayer as usize]).inputGameControls[iGameState as usize];
                    outputControl = &mut self.outputControls[iPlayer as usize];
                    iDeviceID = g.input_control(self.inputControls[iPlayer as usize]).iDevice;
                }

                let inputControl = &*inputControl;
                let outputControl = &mut *outputControl;

                // Only gameplay (`iGameState` 0) asks; `iPlayer` is -1 only in menus. In a net match input slot i is local
                // player i, whose room slot may be another, so `playercontrol` says nothing.
                let human = iGameState != 0
                    || if g.netplay.active { (iPlayer as usize) < g.netplay.localSlots.len() } else { g.game_values.playercontrol[iPlayer as usize] == 1 };
                let ignore_cpu = |iKey: usize| -> bool { iGameState == 0 && !human && iKey < 6 };

                if iDeviceID == DEVICE_KEYBOARD {
                    if matches!(event, E::Key { down: true, .. }) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == key_sym {
                                fFound = true;

                                //Ignore input for cpu controlled players
                                if ignore_cpu(iKey) {
                                    iKey += 1;
                                    continue;
                                }

                                if !outputControl.keys[iKey].fDown {
                                    outputControl.keys[iKey].fPressed = true;
                                }

                                outputControl.keys[iKey].fDown = true;
                            }
                            iKey += 1;
                        }

                        self.iPressedKey = key_sym;
                    } else if matches!(event, E::Key { down: false, .. }) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == key_sym {
                                fFound = true;

                                //Ignore input for cpu controlled players
                                if ignore_cpu(iKey) {
                                    iKey += 1;
                                    continue;
                                }

                                outputControl.keys[iKey].fDown = false;
                            }
                            iKey += 1;
                        }
                    } else if matches!(event, E::Mouse { event: M::Motion { .. }, .. }) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            let k = inputControl.keys[iKey];
                            if k >= MOUSE_UP {
                                if (k == MOUSE_UP && motion_yrel < -MOUSE_Y_DEAD_ZONE)
                                    || (k == MOUSE_DOWN && motion_yrel > MOUSE_Y_DEAD_ZONE)
                                    || (k == MOUSE_LEFT && motion_xrel < -MOUSE_X_DEAD_ZONE)
                                    || (k == MOUSE_RIGHT && motion_xrel > MOUSE_X_DEAD_ZONE)
                                    || (k >= MOUSE_BUTTON_START && (motion_state & sdl_button(k - MOUSE_BUTTON_START)) != 0)
                                {
                                    fFound = true;

                                    //Ignore input for cpu controlled players
                                    if ignore_cpu(iKey) {
                                        iKey += 1;
                                        continue;
                                    }

                                    if !outputControl.keys[iKey].fDown {
                                        outputControl.keys[iKey].fPressed = true;
                                    }

                                    outputControl.keys[iKey].fDown = true;
                                } else {
                                    //Ignore input for cpu controlled players
                                    if ignore_cpu(iKey) {
                                        iKey += 1;
                                        continue;
                                    }

                                    //Mouse scroll wheel up/down events happen on same frame so ignore up event (and clear it in the ClearPressedKeys() method)
                                    if k == MOUSE_BUTTON_START + 4 || k == MOUSE_BUTTON_START + 5 {
                                        iKey += 1;
                                        continue;
                                    }

                                    outputControl.keys[iKey].fDown = false;
                                }
                            }
                            iKey += 1;
                        }
                    } else if matches!(event, E::Mouse { event: M::Button { down: true, .. }, .. }) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == mouse_button as i32 + MOUSE_BUTTON_START {
                                fFound = true;

                                //Ignore input for cpu controlled players
                                if ignore_cpu(iKey) {
                                    iKey += 1;
                                    continue;
                                }

                                if !outputControl.keys[iKey].fDown {
                                    outputControl.keys[iKey].fPressed = true;
                                }

                                outputControl.keys[iKey].fDown = true;
                            }
                            iKey += 1;
                        }
                    } else if matches!(event, E::Mouse { event: M::Button { down: false, .. }, .. }) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == mouse_button as i32 + MOUSE_BUTTON_START {
                                fFound = true;

                                //Mouse scroll wheel up/down events happen on same frame so ignore up event (and clear it in the ClearPressedKeys() method)
                                if inputControl.keys[iKey] == MOUSE_BUTTON_START + 4
                                    || inputControl.keys[iKey] == MOUSE_BUTTON_START + 5
                                {
                                    iKey += 1;
                                    continue;
                                }

                                //Ignore input for cpu controlled players
                                if ignore_cpu(iKey) {
                                    iKey += 1;
                                    continue;
                                }

                                outputControl.keys[iKey].fDown = false;
                            }
                            iKey += 1;
                        }
                    }
                } else {
                    if let Some((held, changed)) = joyDirection {
                        let which = if matches!(event, E::PadHat { .. }) { jhat_which } else { jaxis_which };
                        if iDeviceID as i32 != which {
                            continue;
                        }

                        for iKey in 0..NUM_KEYS {
                            let Some(direction) = joy_direction(inputControl.keys[iKey]) else { continue };
                            if !changed.contains(&direction) {
                                continue;
                            }

                            //Ignore input for cpu controlled players
                            if ignore_cpu(iKey) {
                                continue;
                            }

                            if held[direction] {
                                fFound = true;

                                if !outputControl.keys[iKey].fDown {
                                    outputControl.keys[iKey].fPressed = true;
                                }

                                outputControl.keys[iKey].fDown = true;
                            } else {
                                outputControl.keys[iKey].fDown = false;
                            }
                        }
                    } else if matches!(event, E::PadButton { down: true, .. }) {
                        if iDeviceID as i32 != jbutton_which {
                            continue;
                        }

                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == jbutton_button as i32 + JOY_BUTTON_START {
                                fFound = true;

                                //Ignore input for cpu controlled players
                                if ignore_cpu(iKey) {
                                    iKey += 1;
                                    continue;
                                }

                                if !outputControl.keys[iKey].fDown {
                                    outputControl.keys[iKey].fPressed = true;
                                }

                                outputControl.keys[iKey].fDown = true;
                            }
                            iKey += 1;
                        }
                    } else if matches!(event, E::PadButton { down: false, .. }) {
                        if iDeviceID as i32 != jbutton_which {
                            continue;
                        }

                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == jbutton_button as i32 + JOY_BUTTON_START {
                                fFound = true;

                                //Ignore input for cpu controlled players
                                if ignore_cpu(iKey) {
                                    iKey += 1;
                                    continue;
                                }

                                outputControl.keys[iKey].fDown = false;
                            }
                            iKey += 1;
                        }
                    } else if matches!(event, E::PadAxis { .. }) {
                        if iDeviceID as i32 != jaxis_which {
                            continue;
                        }

                        let axis = jaxis_axis;
                        let value = jaxis_value as i32;
                        for iKey in 0..NUM_KEYS {
                            let k = inputControl.keys[iKey];
                            let mut fUseJoystickInput = false;
                            let mut fJoystickDown = false;

                            if axis == 2 && k == JOY_STICK_2_LEFT {
                                fUseJoystickInput = true;
                                if value < -JOYSTICK_DEAD_ZONE {
                                    fJoystickDown = true;
                                }
                            } else if axis == 2 && k == JOY_STICK_2_RIGHT {
                                fUseJoystickInput = true;
                                if value > JOYSTICK_DEAD_ZONE {
                                    fJoystickDown = true;
                                }
                            } else if axis == 3 && k == JOY_STICK_2_UP {
                                fUseJoystickInput = true;
                                if value < -JOYSTICK_DEAD_ZONE {
                                    fJoystickDown = true;
                                }
                            } else if axis == 3 && k == JOY_STICK_2_DOWN {
                                fUseJoystickInput = true;
                                if value > JOYSTICK_DEAD_ZONE {
                                    fJoystickDown = true;
                                }
                            }

                            if fUseJoystickInput {
                                //Ignore input for cpu controlled players
                                if ignore_cpu(iKey) {
                                    continue;
                                }

                                if fJoystickDown {
                                    fFound = true;

                                    if !outputControl.keys[iKey].fDown {
                                        outputControl.keys[iKey].fPressed = true;
                                    }

                                    outputControl.keys[iKey].fDown = true;
                                } else {
                                    outputControl.keys[iKey].fDown = false;
                                }
                            }
                        }
                    }
                }

                //This line might be causing input from some players not to be read
                //if (fFound)
                //break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smw_platform::{Keycode, PadSlot};

    /// Player 1 on a pad in a menu takes the keyboard fallback slot, `iPlayer` -1.
    #[test]
    fn menu_input_with_player_one_on_a_pad() {
        let g = crate::game::Game::create();
        g.game_values.inputConfiguration[0][1].iDevice = 0;
        let mut input = CPlayerInput::new();
        input.inputControls[0] = Some((0, 1));
        for event in [
            InputEvent::Key { key: Keycode(13), scancode: 0, mods: 0, down: true, repeat: false, window: 0 },
            InputEvent::PadButton { pad: PadSlot(0), button: 0, down: true },
        ] {
            input.update_input(g, &Input { timestamp: 0, event }, 1);
        }
    }
}
