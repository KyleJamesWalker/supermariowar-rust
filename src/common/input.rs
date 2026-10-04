//! Port of src/common/input.cpp
//!
//! The C++ unions that alias `keys[NUM_KEYS]` with named members (`menu_up`, `game_jump`, ...)
//! become a plain `keys` array plus same-named accessor methods (`game_jump()` / `game_jump_mut()`).

use crate::common::global_constants::MAX_PLAYERS;
use crate::globals::*;
use sdl2::sys::{SDL_Event, SDL_EventType, SDL_Keycode, SDL_HAT_DOWN, SDL_HAT_LEFT, SDL_HAT_RIGHT, SDL_HAT_UP};

pub const NUM_KEYS: usize = 8;

pub const DEVICE_KEYBOARD: i16 = -1;

pub const JOYSTICK_DEAD_ZONE: i32 = 16384;
pub const MOUSE_X_DEAD_ZONE: i32 = 0;
pub const MOUSE_Y_DEAD_ZONE: i32 = 5;

pub const MOUSE_UP: i32 = 323;
pub const MOUSE_DOWN: i32 = 324;
pub const MOUSE_LEFT: i32 = 325;
pub const MOUSE_RIGHT: i32 = 326;
pub const MOUSE_BUTTON_START: i32 = 327;

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

pub const KEY_NONE: i32 = -1;

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct CKeyState {
    pub fDown: bool,
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
            game_start, game_start_mut = 6;
            game_cancel, game_cancel_mut = 7;
        );
    };
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct CInputControl {
    pub keys: [SDL_Keycode; NUM_KEYS],
}
all_key_aliases!(CInputControl, SDL_Keycode);

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

#[derive(Clone, Copy, Default, Debug)]
pub struct CInputPlayerControl {
    pub iDevice: i16,
    pub inputGameControls: [CInputControl; 2], //0 == game controls, 1 == menu controls
}

pub struct CPlayerInput {
    //Points to the input controls in the global inputConfiguration class
    pub inputControls: [Ptr<CInputPlayerControl>; 4],

    //Use these structures to see what input has been pressed
    pub outputControls: [COutputControl; 4],

    pub iPressedKey: SDL_Keycode,
    /// Not in upstream: per joystick (`which`), the stick 1 and hat directions held (up, down, left, right).
    joyDirections: Vec<(i32, [bool; 4], [bool; 4])>,
    pub _alias: Aliased,
}

/// Not in upstream: the direction (up, down, left, right) of a stick 1 or hat binding. A joystick's
/// stick 1 and hat drive the same direction bindings, so either one moves a player or a menu.
fn joy_direction(key: SDL_Keycode) -> Option<usize> {
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
            inputControls: [Ptr::null(); 4],
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

    pub fn clear_game_action_keys(&mut self) {
        for iPlayer in 0..MAX_PLAYERS as usize {
            let outputControl = &mut self.outputControls[iPlayer];

            // 0-3: direction keys, 4: turbo
            for iKey in 5..NUM_KEYS {
                outputControl.keys[iKey].fPressed = false;
                outputControl.keys[iKey].fDown = false;
            }
        }

        self.iPressedKey = 0;
    }

    /// Not in the C++: the held stick and hat directions, for replay checkpoints (smw/checkpoint.rs).
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
    pub fn update(&mut self, event: SDL_Event, iGameState: i16) {
        unsafe {
            let event_type = event.type_;
            let is = |t: SDL_EventType| event_type == t as u32;

            let mut joyDirection: Option<([bool; 4], &[usize])> = None;
            if is(SDL_EventType::SDL_JOYHATMOTION) || (is(SDL_EventType::SDL_JOYAXISMOTION) && event.jaxis.axis < 2) {
                let which = if is(SDL_EventType::SDL_JOYHATMOTION) { event.jhat.which } else { event.jaxis.which };
                let i = match self.joyDirections.iter().position(|d| d.0 == which) {
                    Some(i) => i,
                    None => {
                        self.joyDirections.push((which, [false; 4], [false; 4]));
                        self.joyDirections.len() - 1
                    }
                };
                let (_, stick, hat) = &mut self.joyDirections[i];
                let changed: &[usize] = if is(SDL_EventType::SDL_JOYHATMOTION) {
                    let value = event.jhat.value as u32;
                    *hat = [value & SDL_HAT_UP != 0, value & SDL_HAT_DOWN != 0, value & SDL_HAT_LEFT != 0, value & SDL_HAT_RIGHT != 0];
                    &[0, 1, 2, 3]
                } else {
                    let value = event.jaxis.value as i32;
                    let (neg, pos) = if event.jaxis.axis == 0 { (2, 3) } else { (0, 1) };
                    stick[neg] = value < -JOYSTICK_DEAD_ZONE;
                    stick[pos] = value > JOYSTICK_DEAD_ZONE;
                    if event.jaxis.axis == 0 {
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

                //Allow keyboard input from player 1 at all times (even when he is configured to use joystick)
                // Not in upstream: unless another player now has those keys (see assign_inputs).
                if iPlayer == -1 {
                    let keyboard = &mut game_values.inputConfiguration[0][0] as *mut CInputPlayerControl;
                    if iGameState == 1 && self.inputControls[0].iDevice != DEVICE_KEYBOARD && !self.inputControls.iter().any(|c| c.as_ptr() == keyboard) {
                        inputControl = &game_values.inputConfiguration[0][0].inputGameControls[1];
                        outputControl = &mut self.outputControls[0];
                        iDeviceID = game_values.inputConfiguration[0][0].iDevice;
                    } else {
                        continue;
                    }
                } else {
                    if self.inputControls[iPlayer as usize].is_null() {
                        continue;
                    }

                    inputControl = &self.inputControls[iPlayer as usize].inputGameControls[iGameState as usize];
                    outputControl = &mut self.outputControls[iPlayer as usize];
                    iDeviceID = self.inputControls[iPlayer as usize].iDevice;
                }

                let inputControl = &*inputControl;
                let outputControl = &mut *outputControl;

                // game_values.playercontrol[-1] is read for the keyboard fallback player in C++ only
                // when iGameState == 0, which the iPlayer == -1 branch excludes.
                let ignore_cpu = |iKey: usize| -> bool {
                    iGameState == 0 && game_values.playercontrol[iPlayer as usize] != 1 && iKey < 6
                };

                if iDeviceID == DEVICE_KEYBOARD {
                    if is(SDL_EventType::SDL_KEYDOWN) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == event.key.keysym.sym {
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

                        self.iPressedKey = event.key.keysym.sym;
                    } else if is(SDL_EventType::SDL_KEYUP) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == event.key.keysym.sym {
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
                    } else if is(SDL_EventType::SDL_MOUSEMOTION) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            let k = inputControl.keys[iKey];
                            if k >= MOUSE_UP {
                                if (k == MOUSE_UP && event.motion.yrel < -MOUSE_Y_DEAD_ZONE)
                                    || (k == MOUSE_DOWN && event.motion.yrel > MOUSE_Y_DEAD_ZONE)
                                    || (k == MOUSE_LEFT && event.motion.xrel < -MOUSE_X_DEAD_ZONE)
                                    || (k == MOUSE_RIGHT && event.motion.xrel > MOUSE_X_DEAD_ZONE)
                                    || (k >= MOUSE_BUTTON_START && (event.motion.state & sdl_button(k - MOUSE_BUTTON_START)) != 0)
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
                    } else if is(SDL_EventType::SDL_MOUSEBUTTONDOWN) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == event.button.button as i32 + MOUSE_BUTTON_START {
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
                    } else if is(SDL_EventType::SDL_MOUSEBUTTONUP) {
                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == event.button.button as i32 + MOUSE_BUTTON_START {
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
                        let which = if is(SDL_EventType::SDL_JOYHATMOTION) { event.jhat.which } else { event.jaxis.which };
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
                    } else if is(SDL_EventType::SDL_JOYBUTTONDOWN) {
                        if iDeviceID as i32 != event.jbutton.which {
                            continue;
                        }

                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == event.jbutton.button as i32 + JOY_BUTTON_START {
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
                    } else if is(SDL_EventType::SDL_JOYBUTTONUP) {
                        if iDeviceID as i32 != event.jbutton.which {
                            continue;
                        }

                        let mut iKey = 0;
                        while iKey < NUM_KEYS && !fFound {
                            if inputControl.keys[iKey] == event.jbutton.button as i32 + JOY_BUTTON_START {
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
                    } else if is(SDL_EventType::SDL_JOYAXISMOTION) {
                        if iDeviceID as i32 != event.jaxis.which {
                            continue;
                        }

                        let axis = event.jaxis.axis;
                        let value = event.jaxis.value as i32;
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
