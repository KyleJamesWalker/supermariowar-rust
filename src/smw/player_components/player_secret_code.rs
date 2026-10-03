//! Port of src/smw/player_components/PlayerSecretCode.cpp

use crate::globals::*;
use crate::smw::objectgame::check_secret;
use crate::smw::player::CPlayer;

const SECRET_CODE: [u8; 18] = [4, 8, 4, 8, 1, 1, 4, 8, 4, 8, 1, 1, 4, 8, 4, 8, 1, 1];
const SECRET_CODE_TIME_MAX: u16 = 186;

#[derive(Default)]
pub struct PlayerSecretCode {
    pub timer: u16,
    pub index: u16,
    pub _alias: Aliased,
}

impl PlayerSecretCode {
    pub fn new() -> Self {
        let mut this = Self::default();
        this.reset();
        this
    }

    pub fn reset(&mut self) {
        self.timer = 0;
        self.index = 0;
    }

    pub fn update(&mut self, player: &mut CPlayer, keymask: u8) {
        if !player.isready() {
            return;
        }

        if self.index > 0 {
            self.timer += 1;
            if self.timer >= SECRET_CODE_TIME_MAX {
                self.index = 0;
                self.timer = 0;
            }
        }

        if keymask as i32 & SECRET_CODE[self.index as usize] as i32 != 0 {
            self.index += 1;
        } else if keymask as i32 & !(SECRET_CODE[self.index as usize] as i32) != 0 {
            self.index = 0;
            self.timer = 0;
        }

        if self.index == 18 && self.timer < SECRET_CODE_TIME_MAX {
            self.index = 0;
            self.timer = 0;
            check_secret(3);
        }
    }
}
