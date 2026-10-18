//! Port of src/smw/objects/overmap/WO_OrbitHazard.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::game::Game;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::{vel, TWO_PI};
use crate::common::math::trig::{cosf, sinf};
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
#[derive(Clone)]
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
    pub fn new(g: &mut Game, nspr: SpriteId,
        pos: Vec2s,
        radius: f32,
        velocity: f32,
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
                g,
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
        this.dVel = vel(velocity);
        this.dAngle = angle;
        this.dCenter = Vec2f::new(pos.x as f32, pos.y as f32);

        this.calculate_position();
        this
    }

    fn calculate_position(&mut self) {
        let x = self.dCenter.x + self.dRadius * cosf(self.dAngle) - self.iw as f32 / 2.0f32;
        self.set_xf(x);
        let y = self.dCenter.y + self.dRadius * sinf(self.dAngle) - self.ih as f32 / 2.0f32;
        self.set_yf(y);
    }
}

impl CObjectTrait for OMO_OrbitHazard {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self, g: &mut Game) {
        io_over_map_object_draw(g, self);
    }

    fn update(&mut self, g: &mut Game) {
        self.animate();

        self.dAngle += self.dVel;

        if self.dAngle < 0.0f32 {
            self.dAngle += TWO_PI;
        } else if self.dAngle >= TWO_PI {
            self.dAngle -= TWO_PI;
        }

        self.calculate_position();
    }

    fn collide_player(&mut self, g: &mut Game, mut player: PlayerId) -> bool {
        if !g.player(player).is_invincible() && !g.player(player).is_shielded() && !g.player(player).shyguy {
            return g.player(player).kill_player_map_hazard(g, false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_OrbitHazard {
    crate::impl_over_map_object_plumbing!();
}
