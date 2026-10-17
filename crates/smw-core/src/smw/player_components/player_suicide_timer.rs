//! Port of src/smw/player_components/PlayerSuicideTimer.cpp

use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::global_constants::*;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::player::CPlayer;

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

    pub fn update(&mut self, g: &mut Game, player: &mut CPlayer) {
        if g.mode().gameover || g.game_values.singleplayermode >= 0 {
            self.reset();
        }

        if !player.is_invincible()
            && !player.is_frozen()
            && g.game_values.suicidetime > 0
            && {
                self.timer = self.timer.wrapping_add(1);
                self.timer > g.game_values.suicidetime
            }
        {
            self.counttimer = self.counttimer.wrapping_add(1);
            if self.counttimer > 62 {
                self.counttimer = 0;

                self.displaytimer = self.displaytimer.wrapping_sub(1);
                if self.displaytimer < 0 {
                    player.kill_player_map_hazard(g, true, KillStyle::Environment, false, -1);
                }
            }
        }
    }

    pub fn draw(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
        if self.timer > g.game_values.suicidetime {
            gp.rm.sprite(gp.rm.spr_awardkillsinrow).draw_src(
                g,
                player.left_x() as i32 - PWOFFSET + 8,
                player.top_y() as i32 - PHOFFSET + 8,
                &Rect { x: (self.displaytimer as i32) << 4, y: (player.get_color_id() as i32) << 4, w: 16, h: 16 },
            );
        }
    }
}
