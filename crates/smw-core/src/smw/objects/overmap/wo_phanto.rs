//! Phanto for chase mode: a mask that chases the player holding the key.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::game::Game;
use crate::common::game::App;
use crate::common::game_mode::game_mode_chase;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::{acc, vel, CRUNCHMAX, TICKS_PER_SECOND};
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
/// Chasing mask; it drifts off screen and waits while nobody holds the key.
#[derive(Clone)]
pub struct OMO_Phanto {
    pub io_over_map_object: IO_OverMapObject,

    /// 0 kills the key holder, 1 also costs them 10 points, 2 kills anyone it touches.
    pub iType: i16,

    /// Top speed (pixels per tick) from the phanto speed setting; x is one faster than y.
    pub dMaxSpeedX: f32,
    pub dMaxSpeedY: f32,
    /// Acceleration toward the target (pixels per tick squared), rerolled every second.
    pub dReactionSpeed: f32,
    /// Phanto speed setting divided by 6; scales `dReactionSpeed`.
    pub dSpeedRatio: f32,
    /// Ticks toward the next `dReactionSpeed` reroll.
    pub iSpeedTimer: i16,
}
impl_base!(OMO_Phanto => io_over_map_object: IO_OverMapObject);

impl OMO_Phanto {
    /// Rerolls the acceleration.
    pub fn change_speed(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.iSpeedTimer = 0;
        self.dReactionSpeed = acc((0.05f32 + RANDOM_INT(gp.get(), 20) as f32 / 100.0f32) * self.dSpeedRatio);
    }

    /// Moves the phanto to a random x just above or below the screen.
    pub fn return_offscreen(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.set_xi(RANDOM_INT(gp.get(), App::screenWidth) as i16);
        let yi = if RANDOM_BOOL(gp.get()) { -(self.ih as i32) - CRUNCHMAX } else { App::screenHeight };
        self.set_yi(yi as i16);
    }

    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, dVelX: f32, dVelY: f32, r#type: i16) -> Self {
        let mut this = OMO_Phanto {
            io_over_map_object: IO_OverMapObject::new(g, nspr, pos, 1, 0, 30, 32, 1, 0, ((r#type as i32) << 5) as i16, 0, 32, 32),
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

        this.dMaxSpeedY = vel(g.game_values.gamemodesettings.chase.phantospeed as f32 / 2.0f32);
        this.dMaxSpeedX = this.dMaxSpeedY + vel(1.0);
        this.dSpeedRatio = g.game_values.gamemodesettings.chase.phantospeed as f32 / 6.0f32;

        this.dReactionSpeed = acc(0.2);
        this
    }

    pub fn get_type(&self) -> i16 {
        self.iType
    }
}

impl CObjectTrait for OMO_Phanto {
    fn harness_hash_object(&mut self, h: &mut crate::smw::rollback::Fnv) {
        h.i16s(&[self.iType, self.iSpeedTimer]);
        h.f32s(&[self.dMaxSpeedX, self.dMaxSpeedY, self.dReactionSpeed, self.dSpeedRatio]);
    }
    crate::impl_cobject_plumbing!();

    fn draw(&mut self, g: &mut Game) {
        io_over_map_object_draw(g, self);
    }

    fn update(&mut self, g: &mut Game) {
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
        if self.iSpeedTimer as i32 > TICKS_PER_SECOND as i32 {
            self.change_speed(g);
        }

        // Chase player or move off screen if there is no player holding the key
        if g.mode().gamemode == game_mode_chase {
            let chasemode: &mut CGM_Chase = g.mode().get().as_any().downcast_mut::<CGM_Chase>().unwrap();

            let player: PlayerId = chasemode.get_key_holder(g, );

            if !g.mode().gameover && !player.is_none() {
                // Chase the player
                let mut fWrap = false;
                if (g.player(player).ix as i32 - self.ix as i32).abs() > 320 {
                    fWrap = true;
                }

                // Chase wrapped around edge of screen
                if (g.player(player).ix < self.ix && !fWrap) || (g.player(player).ix > self.ix && fWrap) {
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

                if g.player(player).iy < self.iy {
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
                    self.return_offscreen(g);
                }
            }
        }
    }

    fn collide_player(&mut self, g: &mut Game, mut player: PlayerId) -> bool {
        let mut gp = g.ptr();
        if self.iy as i32 <= -(self.ih as i32) || self.iy as i32 >= App::screenHeight {
            return false;
        }

        // Kills the key holder; type 2 kills anyone.
        if !g.player(player).is_invincible() && !g.player(player).is_shielded() {
            if self.iType == 2 {
                g.player(player).kill_player_map_hazard(g, false, KillStyle::Phanto, false, -1);
                return true;
            } else if g.mode().gamemode == game_mode_chase {
                let chasemode: &mut CGM_Chase = g.mode().get().as_any().downcast_mut::<CGM_Chase>().unwrap();
                let keyholder: PlayerId = chasemode.get_key_holder(g, );

                if keyholder == player {
                    g.player(player).kill_player_map_hazard(g, false, KillStyle::Phanto, false, -1);

                    if !g.mode().gameover && self.iType == 1 {
                        g.player(player).score(g).adjust_score(g, -10);
                        if_sound_on_play(g, &mut gp.rm.sfx_stun);
                    }

                    return true;
                }
            }
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_Phanto {
    crate::impl_over_map_object_plumbing!();
}
