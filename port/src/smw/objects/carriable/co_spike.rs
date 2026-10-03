//! Port of src/smw/objects/carriable/CO_Spike.cpp

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
pub struct CO_Spike {
    pub co_spring: CO_Spring,
}
impl_base!(CO_Spike => co_spring: CO_Spring);

impl CO_Spike {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s) -> Self {
        let mut o = CO_Spike { co_spring: CO_Spring::new(nspr, pos, false) };

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
    fn update(&mut self) {
        co_spring_update(self)
    }
    fn draw(&mut self) {
        co_spring_draw(self)
    }
    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        co_spring_collide_player(self, player)
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

    fn hittop(&mut self, player: Ptr<CPlayer>) {
        let mut player = player;
        if player.isready() && !player.is_shielded() && !player.is_invincible() && !player.kuriboshoe.is_on() && !player.shyguy {
            player.kill_player_map_hazard(false, KillStyle::Environment, false, -1);
        }
    }
}
