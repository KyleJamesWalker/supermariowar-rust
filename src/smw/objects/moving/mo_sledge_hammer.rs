//! Port of src/smw/objects/moving/MO_SledgeHammer.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::GRAVITATION;
use crate::common::math::trig::{cos, sin};
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::{eyecandy, objectcontainer};
use crate::smw::objects::moving::mo_explosion::MO_Explosion;
use crate::smw::objects::moving::mo_hammer::MO_Hammer;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, CPlayer};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class sledge hammer
//------------------------------------------------------------------------------
pub struct MO_SledgeHammer {
    pub io_moving_object: IO_MovingObject,

    pub playerID: i16,
    pub teamID: i16,
    pub colorOffset: i16,

    pub fSuper: bool,
}
impl_base!(MO_SledgeHammer => io_moving_object: IO_MovingObject);

impl MO_SledgeHammer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, vel: Vec2f, aniSpeed: i16, iGlobalID: i16, iTeamID: i16, iColorID: i16, superHammer: bool) -> Self {
        let iCollisionWidth = (nspr.get_width() as i16 as i32 / iNumSpr as i32) as i16;
        let iCollisionHeight = (nspr.get_height() as i16 as i32 / 5) as i16;
        let mut o = MO_SledgeHammer {
            io_moving_object: IO_MovingObject::new(nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, 0, 0, -1, -1, -1, -1),
            playerID: 0,
            teamID: 0,
            colorOffset: 0,
            fSuper: false,
        };

        o.ih = o.collisionHeight;

        o.playerID = iGlobalID;
        o.teamID = iTeamID;
        o.colorOffset = ((iColorID as i32 + 1) * 32) as i16;
        o.movingObjectType = movingobject_sledgehammer;
        o.state = 1;
        o.velx = vel.x;
        o.vely = vel.y;

        o.fSuper = superHammer;

        if o.velx > 0.0f32 {
            o.drawframe = 0;
        } else {
            o.drawframe = o.animationWidth - o.iw;
        }
        o
    }

    pub fn explode(&mut self) {
        unsafe {
            if self.fSuper {
                let pos = Vec2s::new((self.ix as i32 + (self.iw as i32 >> 2) - 96) as i16, (self.iy as i32 + (self.ih as i32 >> 2) - 64) as i16);
                objectcontainer[2].add(Ptr::new_box(MO_Explosion::new(Ptr::from_mut(&mut rm.spr_explosion), pos, 2, 4, -1, -1, KillStyle::Hammer)));
                if_sound_on_play(&mut rm.sfx_bobombsound);
            } else {
                let iCenterX: i16 = (self.ix as i32 + (self.iw as i32 >> 1) - 14) as i16;
                let iCenterY: i16 = (self.iy as i32 + (self.ih as i32 >> 1) - 14) as i16;
                let iColorID: i16 = (self.colorOffset as i32 / 32 - 1) as i16;

                for _iHammer in 0..3 {
                    let dAngle: f32 = RANDOM_INT(628) as f32 / 100.0f32;
                    let dVel: f32 = RANDOM_INT(5) as f32 / 2.0f32 + 3.0f32;
                    let dVelX: f32 = dVel * cos(dAngle);
                    let dVelY: f32 = dVel * sin(dAngle);
                    objectcontainer[2].add(Ptr::new_box(MO_Hammer::new(
                        Ptr::from_mut(&mut rm.spr_hammer),
                        Vec2s::new(iCenterX, iCenterY),
                        6,
                        Vec2f::new(dVelX, dVelY),
                        5,
                        self.playerID,
                        self.teamID,
                        iColorID,
                        true,
                    )));
                }

                let mut player: Ptr<CPlayer> = get_player_from_global_id(self.playerID);

                player.increase_projectiles_count(3);

                if_sound_on_play(&mut rm.sfx_cannon);
            }
        }
    }
}

impl CObjectTrait for MO_SledgeHammer {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        self.animationtimer += 1;
        if self.animationtimer == self.animationspeed {
            self.animationtimer = 0;

            if self.velx > 0.0 {
                self.drawframe += self.iw;
                if self.drawframe >= self.animationWidth {
                    self.drawframe = 0;
                }
            } else {
                self.drawframe -= self.iw;
                if self.drawframe < 0 {
                    self.drawframe = self.animationWidth - self.iw;
                }
            }
        }

        let (fx, velx) = (self.fx, self.velx);
        self.set_xf(fx + velx);
        let (fy, vely) = (self.fy, self.vely);
        self.set_yf(fy + vely);

        self.vely += GRAVITATION;

        if self.ix < 0 {
            let ix = (self.ix as i32 + App::screenWidth) as i16;
            self.set_xi(ix);
        } else if self.ix as i32 > App::screenWidth - 1 {
            let ix = (self.ix as i32 - App::screenWidth) as i16;
            self.set_xi(ix);
        }

        if self.iy as i32 >= App::screenHeight {
            self.dead = true;
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if !player.is_shielded() {
            self.dead = true;
            unsafe {
                let x = (self.ix as i32 + (self.iw as i32 >> 1) - 16) as i16;
                let y = (self.iy as i32 + (self.ih as i32 >> 1) - 16) as i16;
                eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), x, y, 3, 4));
                if_sound_on_play(&mut rm.sfx_hit);
            }

            if !player.is_invincible() && !player.shyguy {
                return player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
            }
        }

        false
    }

    fn draw(&mut self) {
        self.spr.draw_src(
            self.ix as i32 - self.collisionOffsetX as i32,
            self.iy as i32 - self.collisionOffsetY as i32,
            &SDL_Rect { x: self.drawframe as i32, y: self.colorOffset as i32, w: self.iw as i32, h: self.ih as i32 },
        );
    }
}

impl IO_MovingObjectTrait for MO_SledgeHammer {
    crate::impl_io_moving_object_plumbing!();
}
