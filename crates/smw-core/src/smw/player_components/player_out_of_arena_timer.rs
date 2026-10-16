//! Port of src/smw/player_components/PlayerOutOfArenaTimer.cpp

use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::global_constants::PWOFFSET;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::player::CPlayer;

#[derive(Default)]
pub struct PlayerOutOfArenaTimer {
    pub timer: i16,
    pub displaytimer: i16,
    pub _alias: Aliased,
}

impl PlayerOutOfArenaTimer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self, g: &mut Game) {
        unsafe {
            self.timer = 0;
            self.displaytimer = (g.game_values.outofboundstime as i32 - 1) as i16;
        }
    }

    pub fn update(&mut self, g: &mut Game, player: &mut CPlayer) {
        unsafe {
            if player.iy < 0 {
                if (player.bottom_y() < -1 || (player.bottom_y() <= 1 && player.vely <= 0.8f32))
                    && g.game_values.outofboundstime > 0
                    && !player.is_invincible()
                {
                    self.timer += 1;
                    if self.timer > 62 {
                        // 62 = fps
                        self.timer = 0;

                        self.displaytimer -= 1;
                        if self.displaytimer < 0 {
                            player.kill_player_map_hazard(g, false, KillStyle::Environment, false, -1);
                        }
                    }
                }
            }
        }
    }

    pub fn draw(&mut self, g: &mut Game, player: &mut CPlayer) {
        let mut gp = g.ptr();
        unsafe {
            if !player.isready() {
                return;
            }

            if player.top_y() < 0 {
                if player.bottom_y() < -1 || (player.bottom_y() <= 0 && player.vely <= 1.0f32) {
                    gp.rm.spr_abovearrows.draw_src(
                        g,
                        player.left_x() as i32 - PWOFFSET,
                        0,
                        &Rect { x: player.get_color_id() as i32 * 32, y: 0, w: 32, h: 26 },
                    );

                    //This displays the out of arena timer before the player is killed
                    if g.game_values.outofboundstime > 0 && self.displaytimer >= 0 {
                        gp.rm.spr_awardkillsinrow.draw_src(
                            g,
                            player.left_x() as i32 - PWOFFSET + 8,
                            18,
                            &Rect { x: (self.displaytimer as i32) << 4, y: (player.get_color_id() as i32) << 4, w: 16, h: 16 },
                        );
                    }
                }
            }
        }
    }
}
