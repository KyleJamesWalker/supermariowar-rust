//! Port of src/smw/player_components/PlayerOutOfArenaTimer.cpp

use crate::common::global_constants::PWOFFSET;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

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

    pub fn reset(&mut self) {
        unsafe {
            self.timer = 0;
            self.displaytimer = (game_values.outofboundstime as i32 - 1) as i16;
        }
    }

    pub fn update(&mut self, player: &mut CPlayer) {
        unsafe {
            if player.iy < 0 {
                if (player.bottom_y() < -1 || (player.bottom_y() <= 1 && player.vely <= 0.8f32))
                    && game_values.outofboundstime > 0
                    && !player.is_invincible()
                {
                    self.timer += 1;
                    if self.timer > 62 {
                        // 62 = fps
                        self.timer = 0;

                        self.displaytimer -= 1;
                        if self.displaytimer < 0 {
                            player.kill_player_map_hazard(false, KillStyle::Environment, false, -1);
                        }
                    }
                }
            }
        }
    }

    pub fn draw(&mut self, player: &mut CPlayer) {
        unsafe {
            if !player.isready() {
                return;
            }

            if player.top_y() < 0 {
                if player.bottom_y() < -1 || (player.bottom_y() <= 0 && player.vely <= 1.0f32) {
                    rm.spr_abovearrows.draw_src(
                        player.left_x() as i32 - PWOFFSET,
                        0,
                        &SDL_Rect { x: player.get_color_id() as i32 * 32, y: 0, w: 32, h: 26 },
                    );

                    //This displays the out of arena timer before the player is killed
                    if game_values.outofboundstime > 0 && self.displaytimer >= 0 {
                        rm.spr_awardkillsinrow.draw_src(
                            player.left_x() as i32 - PWOFFSET + 8,
                            18,
                            &SDL_Rect { x: (self.displaytimer as i32) << 4, y: (player.get_color_id() as i32) << 4, w: 16, h: 16 },
                        );
                    }
                }
            }
        }
    }
}
