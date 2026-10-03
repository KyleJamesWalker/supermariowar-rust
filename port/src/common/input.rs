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
        let mut this = CPlayerInput {
            inputControls: [Ptr::null(); 4],
            outputControls: [COutputControl::default(); 4],
            iPressedKey: 0,
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

    // TODO(input): update() lands once game_values (inputConfiguration, playercontrol) is ported.
}
