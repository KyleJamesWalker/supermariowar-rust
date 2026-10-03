//! Port of src/smw/objects/carriable/CO_KuriboShoe.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_carried;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objects::carriable::co_spring::{co_spring_collide_player, co_spring_draw, co_spring_update, CO_Spring, CO_SpringTrait};
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;
use crate::smw::player_components::player_kuribo_shoe::{NORMAL, STICKY};

//------------------------------------------------------------------------------
// class kuribo's shoe
//------------------------------------------------------------------------------
pub struct CO_KuriboShoe {
    pub co_spring: CO_Spring,

    pub fSticky: bool,
}
impl_base!(CO_KuriboShoe => co_spring: CO_Spring);

impl CO_KuriboShoe {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, sticky: bool) -> Self {
        let mut o = CO_KuriboShoe { co_spring: CO_Spring::new(nspr, pos + Vec2s::new(0, 15), false), fSticky: false };

        o.iw = 32;
        o.ih = 32;

        o.collisionOffsetY = 15;
        o.collisionHeight = 16;

        o.animationOffsetX = if sticky { 64 } else { 0 };

        o.movingObjectType = movingobject_carried;

        o.fSticky = sticky;
        o
    }
}

impl CObjectTrait for CO_KuriboShoe {
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

impl IO_MovingObjectTrait for CO_KuriboShoe {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }
}

impl MO_CarriedObjectTrait for CO_KuriboShoe {
    crate::impl_carried_object_plumbing!();
}

impl CO_SpringTrait for CO_KuriboShoe {
    fn spring(&self) -> &CO_Spring {
        self
    }
    fn spring_mut(&mut self) -> &mut CO_Spring {
        self
    }

    fn hittop(&mut self, player: Ptr<CPlayer>) {
        let mut player = player;
        if !player.kuriboshoe.is_on() && player.tanookisuit.not_statue() {
            self.dead = true;
            player.set_kuribo_shoe(if self.fSticky { STICKY } else { NORMAL });
            unsafe {
                if_sound_on_play(&mut rm.sfx_transform);
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (player.ix as i32 + HALFPW - 16) as i16,
                    (player.iy as i32 + HALFPH - 16) as i16,
                    3,
                    8,
                ));
            }
        }
    }
}
