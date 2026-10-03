//! Port of src/smw/player_components/PlayerShield.cpp

use crate::common::game_mode::game_mode_survival;
use crate::common::gameplay_styles::ShieldStyle;
use crate::globals::*;

pub type PlayerShieldType = i32;
pub const OFF: PlayerShieldType = 0;
pub const SOFT: PlayerShieldType = 1;
pub const SOFT_WITH_STOMP: PlayerShieldType = 2;
pub const HARD: PlayerShieldType = 3;

#[derive(Default)]
pub struct PlayerShield {
    pub r#type: PlayerShieldType,
    pub timer: i16,
    pub _alias: Aliased,
}

impl PlayerShield {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.abort();

        unsafe {
            if game_values.gamemode.getgamemode() == game_mode_survival {
                if game_values.gamemodesettings.survival.shield {
                    self.timer = game_values.shieldtime;
                    self.r#type = game_values.shieldstyle as PlayerShieldType;
                }
            } else if game_values.shieldstyle != ShieldStyle::NoShield {
                self.timer = game_values.shieldtime;
                self.r#type = game_values.shieldstyle as PlayerShieldType;
            }
        }
    }

    pub fn turn_on(&mut self) {
        unsafe {
            self.r#type = if game_values.shieldstyle != ShieldStyle::NoShield { game_values.shieldstyle as PlayerShieldType } else { SOFT };
        }
        self.timer = 60;
    }

    pub fn abort(&mut self) {
        self.r#type = OFF;
        self.timer = 0;
    }

    pub fn is_on(&self) -> bool {
        self.r#type > OFF
    }

    pub fn get_type(&self) -> PlayerShieldType {
        self.r#type
    }

    pub fn set_type(&mut self, r#type: PlayerShieldType) {
        self.r#type = r#type;
    }

    pub fn update(&mut self) {
        //If player is shielded, count down that timer
        if self.timer > 0 && {
            self.timer -= 1;
            self.timer == 0
        } {
            self.r#type = OFF;
        }
    }

    pub fn time_left(&self) -> u16 {
        self.timer as u16
    }
}
