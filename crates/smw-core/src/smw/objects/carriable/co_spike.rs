//! Carriable spike block: kills players who land on it.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::game::Game;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_carried;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::carriable::co_spring::{co_spring_collide_player, co_spring_draw, co_spring_update, CO_Spring, CO_SpringTrait};
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class spike
//------------------------------------------------------------------------------
#[derive(Clone)]
pub struct CO_Spike {
    pub co_spring: CO_Spring,
}
impl_base!(CO_Spike => co_spring: CO_Spring);

impl CO_Spike {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s) -> Self {
        let mut o = CO_Spike { co_spring: CO_Spring::new(g, nspr, pos, false) };

        o.iw = 32;
        o.ih = 32;

        o.movingObjectType = movingobject_carried;
        o
    }
}

impl CObjectTrait for CO_Spike {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }
    fn update(&mut self, g: &mut Game) {
        co_spring_update(g, self)
    }
    fn draw(&mut self, g: &mut Game) {
        co_spring_draw(g, self)
    }
    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        co_spring_collide_player(g, self, player)
    }
}

impl IO_MovingObjectTrait for CO_Spike {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }
}

impl MO_CarriedObjectTrait for CO_Spike {
    crate::impl_carried_object_plumbing!();
}

impl CO_SpringTrait for CO_Spike {
    fn spring(&self) -> &CO_Spring {
        self
    }
    fn spring_mut(&mut self) -> &mut CO_Spring {
        self
    }

    fn hittop(&mut self, g: &mut Game, player: PlayerId) {
        let mut player = player;
        if g.player(player).isready() && !g.player(player).is_shielded() && !g.player(player).is_invincible() && !g.player(player).kuriboshoe.is_on() && !g.player(player).shyguy {
            g.player(player).kill_player_map_hazard(g, false, KillStyle::Environment, false, -1);
        }
    }
}
