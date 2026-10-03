//! Port of src/smw/objects/overmap/WO_Phanto.cpp

use crate::common::game::App;
use crate::common::game_mode::game_mode_chase;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::CRUNCHMAX;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_phanto, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::chase::CGM_Chase;
use crate::smw::objects::overmap::over_map_object::{io_over_map_object_draw, IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class Phanto (for phanto mode)
//------------------------------------------------------------------------------
pub struct OMO_Phanto {
    pub io_over_map_object: IO_OverMapObject,

    pub iType: i16,

    pub dMaxSpeedX: f32,
    pub dMaxSpeedY: f32,
    pub dReactionSpeed: f32,
    pub dSpeedRatio: f32,
    pub iSpeedTimer: i16,
}
impl_base!(OMO_Phanto => io_over_map_object: IO_OverMapObject);

impl OMO_Phanto {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, dVelX: f32, dVelY: f32, r#type: i16) -> Self {
        let mut this = OMO_Phanto {
            io_over_map_object: IO_OverMapObject::new(nspr, pos, 1, 0, 30, 32, 1, 0, ((r#type as i32) << 5) as i16, 0, 32, 32),
            iType: r#type,
            dMaxSpeedX: 0.0,
            dMaxSpeedY: 0.0,
            dReactionSpeed: 0.0,
            dSpeedRatio: 0.0,
            iSpeedTimer: 0,
        };
        this.objectType = object_phanto;
        this.velx = dVelX;
        this.vely = dVelY;

        unsafe {
            this.dMaxSpeedY = game_values.gamemodesettings.chase.phantospeed as f32 / 2.0f32;
            this.dMaxSpeedX = this.dMaxSpeedY + 1.0f32;
            this.dSpeedRatio = game_values.gamemodesettings.chase.phantospeed as f32 / 6.0f32;
        }

        this.dReactionSpeed = 0.2f32;
        this
    }

    pub fn get_type(&self) -> i16 {
        self.iType
    }
}

impl CObjectTrait for OMO_Phanto {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self) {
        io_over_map_object_draw(self);
    }

    fn update(&mut self) {
        let x = self.fx + self.velx;
        self.set_xf(x);
        let y = self.fy + self.vely;
        self.set_yf(y);

        if self.fx < 0.0f32 {
            let x = self.fx + App::screenWidth as f32;
            self.set_xf(x);
        } else if self.fx + self.iw as f32 >= App::screenWidth as f32 {
            let x = self.fx - App::screenWidth as f32;
            self.set_xf(x);
        }

        self.iSpeedTimer += 1;
        if self.iSpeedTimer > 62 {
            self.iSpeedTimer = 0;
            self.dReactionSpeed = (0.05f32 + RANDOM_INT(20) as f32 / 100.0f32) * self.dSpeedRatio;
        }

        unsafe {
            // Chase player or move off screen if there is no player holding the key
            if game_values.gamemode.gamemode == game_mode_chase {
                let chasemode: &mut CGM_Chase = game_values.gamemode.as_any().downcast_mut::<CGM_Chase>().unwrap();

                let player: Ptr<CPlayer> = chasemode.get_key_holder();

                if !game_values.gamemode.gameover && !player.is_null() {
                    // Chase the player
                    let mut fWrap = false;
                    if (player.ix as i32 - self.ix as i32).abs() > 320 {
                        fWrap = true;
                    }

                    // Chase wrapped around edge of screen
                    if (player.ix < self.ix && !fWrap) || (player.ix > self.ix && fWrap) {
                        self.velx -= self.dReactionSpeed;

                        if self.velx < -self.dMaxSpeedX {
                            self.velx = -self.dMaxSpeedX;
                        }
                    } else {
                        self.velx += self.dReactionSpeed;

                        if self.velx > self.dMaxSpeedX {
                            self.velx = self.dMaxSpeedX;
                        }
                    }

                    if player.iy < self.iy {
                        self.vely -= self.dReactionSpeed;

                        if self.vely < -self.dMaxSpeedY {
                            self.vely = -self.dMaxSpeedY;
                        }
                    } else {
                        self.vely += self.dReactionSpeed;

                        if self.vely > self.dMaxSpeedY {
                            self.vely = self.dMaxSpeedY;
                        }
                    }
                } else {
                    // Wander off screen
                    if self.iy > 240 {
                        self.vely += self.dReactionSpeed;

                        if self.vely > self.dMaxSpeedY {
                            self.vely = self.dMaxSpeedY;
                        }
                    } else {
                        self.vely -= self.dReactionSpeed;

                        if self.vely < -self.dMaxSpeedY {
                            self.vely = -self.dMaxSpeedY;
                        }
                    }

                    if self.iy as i32 >= App::screenHeight || (self.iy as i32 + self.ih as i32) < -CRUNCHMAX {
                        self.vely = 0.0f32;
                        self.velx = 0.0f32;

                        // Randomly position phanto off screen
                        self.set_xi(RANDOM_INT(App::screenWidth) as i16);
                        let yi = if RANDOM_BOOL() { -(self.ih as i32) - CRUNCHMAX } else { App::screenHeight };
                        self.set_yi(yi as i16);
                    }
                }
            }
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if self.iy as i32 <= -(self.ih as i32) || self.iy as i32 >= App::screenHeight {
            return false;
        }

        unsafe {
            // If the player is holding the key, kill him
            if !player.is_invincible() && !player.is_shielded() {
                if self.iType == 2 {
                    player.kill_player_map_hazard(false, KillStyle::Phanto, false, -1);
                    return true;
                } else if game_values.gamemode.gamemode == game_mode_chase {
                    let chasemode: &mut CGM_Chase = game_values.gamemode.as_any().downcast_mut::<CGM_Chase>().unwrap();
                    let keyholder: Ptr<CPlayer> = chasemode.get_key_holder();

                    if keyholder == player {
                        player.kill_player_map_hazard(false, KillStyle::Phanto, false, -1);

                        if !game_values.gamemode.gameover && self.iType == 1 {
                            player.score().adjust_score(-10);
                            if_sound_on_play(&mut rm.sfx_stun);
                        }

                        return true;
                    }
                }
            }
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_Phanto {
    crate::impl_over_map_object_plumbing!();
}
