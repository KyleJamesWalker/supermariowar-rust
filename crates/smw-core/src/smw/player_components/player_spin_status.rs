//! Spin attack shared by the cape and the tail: animation state and sprite mapping.

use crate::game::Game;
use crate::common::global_constants::*;
use crate::globals::*;
use crate::smw::player::CPlayer;

#[derive(Clone, Default)]
pub struct PlayerSpinStatus {
    /// Ticks in the current spin frame; each frame lasts 4 ticks.
    pub timer: i8,
    /// Spin frame: 0 not spinning, 1-5 a spin starting right, 7-11 one starting left.
    pub state: u8,
    pub _alias: Aliased,
}

/// Body sprite for each spin state.
static spinStateToSprite: [u8; 12] = [
    PGFX_JUMPING_R as u8,
    PGFX_STOPPING_R as u8,
    PGFX_STOPPING_L as u8,
    PGFX_STANDING_L as u8,
    PGFX_STOPPING_L as u8,
    PGFX_STOPPING_R as u8,
    PGFX_JUMPING_L as u8,
    PGFX_STOPPING_L as u8,
    PGFX_STOPPING_R as u8,
    PGFX_STANDING_R as u8,
    PGFX_STOPPING_R as u8,
    PGFX_STOPPING_L as u8,
];

/// Cape source x offset for each spin state.
static spinStateToCapeSprite: [u8; 12] = [0, 32, 32, 0, 32, 32, 0, 32, 32, 0, 32, 32];

impl PlayerSpinStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.timer = 0;
        self.state = 0;
    }

    /// Advances the spin; a finished feather spin uses up one of the projectile limit.
    pub fn update(&mut self, g: &mut Game, player: &mut CPlayer) {
        if self.is_spin_in_progress() {
            self.timer = self.timer.wrapping_add(1);
            if self.timer > 3 {
                self.timer = 0;
                self.state = self.state.wrapping_add(1);
                if self.state == 6 || self.state == 12 {
                    self.state = 0;

                    if player.powerup == 3 && g.game_values.featherlimit > 0 {
                        player.decrease_projectile_limit(g);
                    }
                }
            }
        }
    }

    /// Starts a spin from the direction the player faces.
    pub fn spin(&mut self, g: &mut Game, player: &mut CPlayer) {
        let reversewalk = g.game_values.reversewalk;
        self.state = if player.is_facing_right(g) {
            if reversewalk { 7 } else { 1 }
        } else if reversewalk {
            1
        } else {
            7
        };
        self.timer = 0;
    }

    pub fn is_spin_in_progress(&self) -> bool {
        self.state > 0
    }

    /// True on spin frames where the body faces away from the spin's starting direction.
    pub fn is_reverse_walking(&self) -> bool {
        if (self.state >= 2 && self.state <= 4) || self.state == 7 || self.state == 11 {
            return true;
        }

        false
    }

    /// Body sprite for the current frame, mirrored under reverse walk.
    pub fn to_sprite_id(&mut self, g: &mut Game) -> u8 {
        let s = spinStateToSprite[self.state as usize];
        if g.game_values.reversewalk {
            if s & 0x1 != 0 { (s as i32 - 1) as u8 } else { (s as i32 + 1) as u8 }
        } else {
            s
        }
    }

    pub fn to_cape_frame_x(&mut self) -> u8 {
        spinStateToCapeSprite[self.state as usize]
    }
}
