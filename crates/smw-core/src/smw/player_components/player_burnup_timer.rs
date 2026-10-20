//! Burn-up: a player falling at terminal velocity for too long catches fire and dies.

use crate::game::Game;
use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::{MAXVELY, ticks};
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::player::CPlayer;

/// Kills an unprotected player who falls at `MAXVELY` for `ticks(20) + ticks(80)` ticks.
#[derive(Clone, Default)]
pub struct PlayerBurnupTimer {
    /// Ticks spent burning, after the `starttimer` delay.
    pub timer: i16,
    /// Ticks spent at terminal velocity; reset when the player slows.
    pub starttimer: i16,
    pub _alias: Aliased,
}

impl PlayerBurnupTimer {
    pub fn new() -> Self {
        PlayerBurnupTimer { timer: 0, starttimer: 0, _alias: Aliased::default() }
    }

    pub fn update(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
        if player.vely >= MAXVELY {
            if !player.is_invincible() && !player.is_shielded() {
                self.starttimer += 1;
                if self.starttimer as i32 >= ticks(20) {
                    if self.starttimer == 20 {
                        if_sound_on_play(g, &mut gp.rm.sfx_burnup);
                    }

                    self.timer += 1;
                    if self.timer as i32 > ticks(80) {
                        player.kill_player_map_hazard(g, true, KillStyle::Environment, false, -1);
                    } else {
                        g.eyecandy[0].emplace(EC_SingleAnimation::new(&gp.rm, 
                            g.rm.spr_burnup,
                            (player.center_x() as i32 - 16) as i16,
                            (player.center_y() as i32 - 16) as i16,
                            5,
                            4,
                        ));
                    }
                }
            }
        } else {
            self.timer = 0;
            self.starttimer = 0;
        }
    }
}
