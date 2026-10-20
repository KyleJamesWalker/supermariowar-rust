//! Secret code input: a key sequence entered in play that unlocks a secret.

use crate::game::Game;
use crate::globals::*;
use crate::smw::objectgame::check_secret;
use crate::smw::player::CPlayer;
use crate::common::global_constants::{TICKS_PER_SECOND};

/// Key mask bits to press in order: left, right, left, right, jump, jump, three times.
const SECRET_CODE: [u8; 18] = [4, 8, 4, 8, 1, 1, 4, 8, 4, 8, 1, 1, 4, 8, 4, 8, 1, 1];
/// Ticks allowed to enter the whole sequence.
const SECRET_CODE_TIME_MAX: u16 = 3 * TICKS_PER_SECOND as u16;

#[derive(Clone, Default)]
pub struct PlayerSecretCode {
    /// Ticks since the first key of the current attempt.
    pub timer: u16,
    /// Keys matched so far; a wrong key resets it.
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

    /// Feeds this tick's pressed-key mask and fires secret 3 on a complete sequence.
    pub fn update(&mut self, g: &mut Game, player: &mut CPlayer, keymask: u8) {
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
            check_secret(g, 3);
        }
    }
}
