//! Port of src/smw/objects/overmap/WO_StraightPathHazard.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::trig::{cos, sin};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_pathhazard, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objects::overmap::over_map_object::{io_over_map_object_draw, io_over_map_object_update, IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class OMO Straight Path Hazard - straight path fireball
//------------------------------------------------------------------------------
pub struct OMO_StraightPathHazard {
    pub io_over_map_object: IO_OverMapObject,

    pub dAngle: f32,
    pub dVel: f32,
}
impl_base!(OMO_StraightPathHazard => io_over_map_object: IO_OverMapObject);

impl OMO_StraightPathHazard {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        nspr: Ptr<gfxSprite>,
        pos: Vec2s,
        angle: f32,
        vel: f32,
        iNumSpr: i16,
        aniSpeed: i16,
        iCollisionWidth: i16,
        iCollisionHeight: i16,
        iCollisionOffsetX: i16,
        iCollisionOffsetY: i16,
        iAnimationOffsetX: i16,
        iAnimationOffsetY: i16,
        iAnimationHeight: i16,
        iAnimationWidth: i16,
    ) -> Self {
        let mut this = OMO_StraightPathHazard {
            io_over_map_object: IO_OverMapObject::new(
                nspr,
                pos,
                iNumSpr,
                aniSpeed,
                iCollisionWidth,
                iCollisionHeight,
                iCollisionOffsetX,
                iCollisionOffsetY,
                iAnimationOffsetX,
                iAnimationOffsetY,
                iAnimationHeight,
                iAnimationWidth,
            ),
            dAngle: 0.0,
            dVel: 0.0,
        };
        this.objectType = object_pathhazard;

        this.dVel = vel;
        this.dAngle = angle;

        this.velx = vel * cos(angle);
        this.vely = vel * sin(angle);
        this
    }
}

impl CObjectTrait for OMO_StraightPathHazard {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self) {
        io_over_map_object_draw(self);
    }

    fn update(&mut self) {
        io_over_map_object_update(self);

        if (self.iy as i32 + self.ih as i32) < 0 || self.iy as i32 >= App::screenHeight {
            self.dead = true;
        }

        // Wrap hazard if it is off the edge of the screen
        if self.ix < 0 {
            self.ix = (self.ix as i32 + App::screenWidth) as i16;
        } else if self.ix as i32 + self.iw as i32 >= App::screenWidth {
            self.ix = (self.ix as i32 - App::screenWidth) as i16;
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if !player.is_shielded() {
            unsafe {
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (self.ix as i32 + (self.iw as i32 >> 2) - 16) as i16,
                    (self.iy as i32 + (self.ih as i32 >> 2) - 16) as i16,
                    3,
                    8,
                ));
            }
            self.dead = true;

            if !player.is_invincible() && !player.shyguy {
                return player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
            }
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_StraightPathHazard {
    crate::impl_over_map_object_plumbing!();
}
