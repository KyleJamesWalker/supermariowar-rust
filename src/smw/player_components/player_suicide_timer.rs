//! Port of src/smw/player_components/PlayerSuicideTimer.cpp

use crate::common::global_constants::*;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

#[derive(Default)]
pub struct PlayerSuicideTimer {
    pub timer: i16,
    pub counttimer: i16,
    pub displaytimer: i16,
    pub _alias: Aliased,
}

impl PlayerSuicideTimer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.timer = 0;
        self.counttimer = 0;
        self.displaytimer = 2;
    }

    pub fn update(&mut self, player: &mut CPlayer) {
        unsafe {
            if game_values.gamemode.gameover || game_values.singleplayermode >= 0 {
                self.reset();
            }

            if !player.is_invincible()
                && !player.is_frozen()
                && game_values.suicidetime > 0
                && {
                    self.timer = self.timer.wrapping_add(1);
                    self.timer > game_values.suicidetime
                }
            {
                self.counttimer = self.counttimer.wrapping_add(1);
                if self.counttimer > 62 {
                    self.counttimer = 0;

                    self.displaytimer = self.displaytimer.wrapping_sub(1);
                    if self.displaytimer < 0 {
                        player.kill_player_map_hazard(true, KillStyle::Environment, false, -1);
                    }
                }
            }
        }
    }

    pub fn draw(&mut self, player: &mut CPlayer) {
        unsafe {
            if self.timer > game_values.suicidetime {
                rm.spr_awardkillsinrow.draw_src(
                    player.left_x() as i32 - PWOFFSET + 8,
                    player.top_y() as i32 - PHOFFSET + 8,
                    &SDL_Rect { x: (self.displaytimer as i32) << 4, y: (player.get_color_id() as i32) << 4, w: 16, h: 16 },
                );
            }
        }
    }
}
