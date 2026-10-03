//! Port of src/smw/player_components/PlayerInvincibility.cpp

use crate::common::gfx::gfx_palette::{invincibility_1, invincibility_2, invincibility_3, normal, PlayerPalette};
use crate::globals::*;
use crate::smw::player::CPlayer;

const INVINCIBILITY_ANIMATION_STATES: [PlayerPalette; 4] = [normal, invincibility_1, invincibility_2, invincibility_3];
const INVINCIBILITY_TIME: i16 = 580;
const INVINCIBILITY_HURRYUP_TIME: i16 = 100;
const INVINCIBILITY_ANIMATION_DELAY: i16 = 4;
const INVINCIBILITY_HURRYUP_ANIMATION_DELAY: i16 = 7;

#[derive(Default)]
pub struct PlayerInvincibility {
    pub invincible: bool,
    pub timer: i16,
    pub _alias: Aliased,
}

impl PlayerInvincibility {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.invincible = false;
        self.timer = 0;
    }

    pub fn turn_on(&mut self, player: &mut CPlayer) {
        unsafe {
            self.invincible = true;
            self.timer = 0;
            player.shield.abort();

            //Stop the invincible music if a player is already invincible
            //(we don't want two invincible music sounds playing at the same time)
            rm.sfx_invinciblemusic.stop();

            if !game_values.gamemode.gameover {
                game_values.flags.playinvinciblesound = true;

                if game_values.music && game_values.sound {
                    rm.backgroundmusic[0].stop();
                }
            }
        }
    }

    pub fn is_on(&self) -> bool {
        self.invincible
    }

    pub fn update(&mut self, _player: &mut CPlayer) {
        if self.invincible {
            self.timer += 1;
            if self.timer > INVINCIBILITY_TIME {
                self.timer = 0;
                self.invincible = false;
            }
        }
    }

    pub fn get_player_palette(&self) -> PlayerPalette {
        let len = INVINCIBILITY_ANIMATION_STATES.len();
        if (self.timer as i32) < (INVINCIBILITY_TIME as i32 - INVINCIBILITY_HURRYUP_TIME as i32) {
            INVINCIBILITY_ANIMATION_STATES[((self.timer as i32 / INVINCIBILITY_ANIMATION_DELAY as i32) as usize) % len]
        } else {
            INVINCIBILITY_ANIMATION_STATES[(((self.timer as i32 - (INVINCIBILITY_TIME as i32 - INVINCIBILITY_HURRYUP_TIME as i32))
                / INVINCIBILITY_HURRYUP_ANIMATION_DELAY as i32) as usize)
                % len]
        }
    }
}
