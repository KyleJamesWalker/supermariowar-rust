//! Port of src/smw/player_components/PlayerSpinStatus.cpp

use crate::common::global_constants::*;
use crate::globals::*;
use crate::smw::player::CPlayer;

#[derive(Default)]
pub struct PlayerSpinStatus {
    pub timer: i8,
    pub state: u8,
    pub _alias: Aliased,
}

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

static spinStateToCapeSprite: [u8; 12] = [0, 32, 32, 0, 32, 32, 0, 32, 32, 0, 32, 32];

impl PlayerSpinStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.timer = 0;
        self.state = 0;
    }

    pub fn update(&mut self, player: &mut CPlayer) {
        if self.is_spin_in_progress() {
            self.timer = self.timer.wrapping_add(1);
            if self.timer > 3 {
                self.timer = 0;
                self.state = self.state.wrapping_add(1);
                if self.state == 6 || self.state == 12 {
                    self.state = 0;

                    if player.powerup == 3 && unsafe { game_values.featherlimit } > 0 {
                        player.decrease_projectile_limit();
                    }
                }
            }
        }
    }

    pub fn spin(&mut self, player: &mut CPlayer) {
        let reversewalk = unsafe { game_values.reversewalk };
        self.state = if player.is_facing_right() {
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

    pub fn is_reverse_walking(&self) -> bool {
        if (self.state >= 2 && self.state <= 4) || self.state == 7 || self.state == 11 {
            return true;
        }

        false
    }

    pub fn to_sprite_id(&mut self) -> u8 {
        let s = spinStateToSprite[self.state as usize];
        if unsafe { game_values.reversewalk } {
            if s & 0x1 != 0 { (s as i32 - 1) as u8 } else { (s as i32 + 1) as u8 }
        } else {
            s
        }
    }

    pub fn to_cape_frame_x(&mut self) -> u8 {
        spinStateToCapeSprite[self.state as usize]
    }
}
