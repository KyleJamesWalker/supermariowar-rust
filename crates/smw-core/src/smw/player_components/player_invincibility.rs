//! Star invincibility: timer and flashing palette.

use crate::game::Game;
use crate::common::gfx::gfx_palette::{invincibility_1, invincibility_2, invincibility_3, normal, PlayerPalette};
use crate::globals::*;
use crate::smw::player::CPlayer;
use crate::common::global_constants::{ticks};

const INVINCIBILITY_ANIMATION_STATES: [PlayerPalette; 4] = [normal, invincibility_1, invincibility_2, invincibility_3];
/// Invincibility duration, in ticks.
const INVINCIBILITY_TIME: i16 = ticks(580) as i16;
/// Final stretch, in ticks, where the palette flashes slower to warn it is ending.
const INVINCIBILITY_HURRYUP_TIME: i16 = ticks(100) as i16;
/// Ticks per palette frame, normally and during the hurry-up.
const INVINCIBILITY_ANIMATION_DELAY: i16 = 4;
const INVINCIBILITY_HURRYUP_ANIMATION_DELAY: i16 = 7;

#[derive(Clone, Default)]
pub struct PlayerInvincibility {
    pub invincible: bool,
    /// Ticks since invincibility began.
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

    /// Starts or restarts invincibility, removing the shield and taking over the music.
    pub fn turn_on(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
        self.invincible = true;
        self.timer = 0;
        player.shield.abort();

        //Stop the invincible music if a player is already invincible
        //(we don't want two invincible music sounds playing at the same time)
        gp.rm.sfx_invinciblemusic.stop(g);

        if !g.mode().gameover {
            g.game_values.flags.playinvinciblesound = true;

            if g.game_values.music && g.game_values.sound {
                gp.backgroundmusic[0].stop(g);
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

    /// Palette to draw this tick while invincible.
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
