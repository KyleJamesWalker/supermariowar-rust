//! Port of src/smw/objects/overmap/WO_OrbitHazard.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::TWO_PI;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::object_base::{object_orbithazard, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::overmap::over_map_object::{io_over_map_object_draw, IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class OMO Orbit Hazard - component of the fireball string or rotodisc
//------------------------------------------------------------------------------
pub struct OMO_OrbitHazard {
    pub io_over_map_object: IO_OverMapObject,

    pub dAngle: f32,
    pub dVel: f32,
    pub dRadius: f32,
    pub dCenter: Vec2f,
}
impl_base!(OMO_OrbitHazard => io_over_map_object: IO_OverMapObject);

impl OMO_OrbitHazard {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        nspr: Ptr<gfxSprite>,
        pos: Vec2s,
        radius: f32,
        vel: f32,
        angle: f32,
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
        let mut this = OMO_OrbitHazard {
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
            dRadius: 0.0,
            dCenter: Vec2f::zero(),
        };
        this.objectType = object_orbithazard;

        this.dRadius = radius;
        this.dVel = vel;
        this.dAngle = angle;
        this.dCenter = Vec2f::new(pos.x as f32, pos.y as f32);

        this.calculate_position();
        this
    }

    fn calculate_position(&mut self) {
        let x = self.dCenter.x + self.dRadius * self.dAngle.cos() - self.iw as f32 / 2.0f32;
        self.set_xf(x);
        let y = self.dCenter.y + self.dRadius * self.dAngle.sin() - self.ih as f32 / 2.0f32;
        self.set_yf(y);
    }
}

impl CObjectTrait for OMO_OrbitHazard {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self) {
        io_over_map_object_draw(self);
    }

    fn update(&mut self) {
        self.animate();

        self.dAngle += self.dVel;

        if self.dAngle < 0.0f32 {
            self.dAngle += TWO_PI;
        } else if self.dAngle >= TWO_PI {
            self.dAngle -= TWO_PI;
        }

        self.calculate_position();
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if !player.is_invincible() && !player.is_shielded() && !player.shyguy {
            return player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_OrbitHazard {
    crate::impl_over_map_object_plumbing!();
}
