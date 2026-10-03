//! Port of src/smw/objects/moving/MO_SuperFireball.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class super fireball
//------------------------------------------------------------------------------
pub struct MO_SuperFireball {
    pub io_moving_object: IO_MovingObject,

    pub colorOffset: i16,
    pub directionOffset: i16,
    pub ttl: i16,
}
impl_base!(MO_SuperFireball => io_moving_object: IO_MovingObject);

impl MO_SuperFireball {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, vel: Vec2f, aniSpeed: i16, iGlobalID: i16, teamID: i16, iColorID: i16) -> Self {
        let mut o = MO_SuperFireball {
            io_moving_object: IO_MovingObject::new(
                nspr,
                pos,
                iNumSpr,
                aniSpeed,
                (nspr.get_width() / iNumSpr as i32) as i16,
                (nspr.get_height() / 10) as i16,
                0,
                0,
                -1,
                -1,
                -1,
                -1,
            ),
            colorOffset: 0,
            directionOffset: 0,
            ttl: 0,
        };

        o.ih /= 10;

        o.iPlayerID = iGlobalID;
        o.iTeamID = teamID;
        o.colorOffset = ((iColorID as i32 + 1) * 64) as i16;
        o.directionOffset = if o.velx < 0.0 { 0 } else { 32 };
        o.movingObjectType = movingobject_superfireball;

        o.state = 1;

        o.velx = vel.x;
        o.vely = vel.y;
        o.ttl = (RANDOM_INT(30) + 60) as i16;

        o.drawframe = 0;

        o.fObjectCollidesWithMap = false;
        o
    }
}

impl CObjectTrait for MO_SuperFireball {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        self.animate();

        let fx = self.fx + self.velx;
        self.set_xf(fx);
        let fy = self.fy + self.vely;
        self.set_yf(fy);

        if self.ix < 0 {
            let ix = (self.ix as i32 + App::screenWidth) as i16;
            self.set_xi(ix);
        } else if self.ix as i32 > App::screenWidth - 1 {
            let ix = (self.ix as i32 - App::screenWidth) as i16;
            self.set_xi(ix);
        }

        if self.iy as i32 > App::screenHeight || (self.iy as i32) < -(self.ih as i32) || {
            self.ttl -= 1;
            self.ttl <= 0
        } {
            removeifprojectile(self.as_mo_ptr(), false, true);
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            if !player.is_shielded() {
                self.dead = true;
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (self.ix as i32 + (self.iw as i32 >> 1) - 16) as i16,
                    (self.iy as i32 + (self.ih as i32 >> 1) - 16) as i16,
                    3,
                    4,
                ));
                if_sound_on_play(&mut rm.sfx_hit);

                if !player.is_invincible() && !player.shyguy {
                    return player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
                }
            }
        }

        false
    }

    fn draw(&mut self) {
        self.spr.draw_src(
            self.ix as i32 - self.collisionOffsetX as i32,
            self.iy as i32 - self.collisionOffsetY as i32,
            &SDL_Rect { x: self.drawframe as i32, y: self.colorOffset as i32 + self.directionOffset as i32, w: self.iw as i32, h: self.ih as i32 },
        );
    }
}

impl IO_MovingObjectTrait for MO_SuperFireball {
    crate::impl_io_moving_object_plumbing!();
}
