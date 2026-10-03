//! Port of src/smw/player_components/PlayerBurnupTimer.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::MAXVELY;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::player::CPlayer;

#[derive(Default)]
pub struct PlayerBurnupTimer {
    pub timer: i16,
    pub starttimer: i16,
    pub _alias: Aliased,
}

impl PlayerBurnupTimer {
    pub fn new() -> Self {
        PlayerBurnupTimer { timer: 0, starttimer: 0, _alias: Aliased::default() }
    }

    pub fn update(&mut self, player: &mut CPlayer) {
        unsafe {
            if player.vely >= MAXVELY {
                if !player.is_invincible() && !player.is_shielded() {
                    self.starttimer += 1;
                    if self.starttimer >= 20 {
                        if self.starttimer == 20 {
                            if_sound_on_play(&mut rm.sfx_burnup);
                        }

                        self.timer += 1;
                        if self.timer > 80 {
                            player.kill_player_map_hazard(true, KillStyle::Environment, false, -1);
                        } else {
                            eyecandy[0].emplace(EC_SingleAnimation::new(
                                Ptr::from_mut(&mut rm.spr_burnup),
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
}
