//! Port of src/smw/objects/powerup/PU_Tanooki.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class tanooki suit
//------------------------------------------------------------------------------
pub struct PU_Tanooki {
    pub mo_powerup: MO_Powerup,
}
impl_base!(PU_Tanooki => mo_powerup: MO_Powerup);

impl PU_Tanooki {
    #[allow(clippy::too_many_arguments)]
    pub fn new(pos: Vec2s) -> Self {
        unsafe { PU_Tanooki { mo_powerup: MO_Powerup::new(Ptr::from_mut(&mut rm.spr_tanooki), pos, 1, 0, 30, 30, 1, 1) } }
    }
}

impl CObjectTrait for PU_Tanooki {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }
    fn draw(&mut self) {
        mo_powerup_draw(self)
    }
    fn update(&mut self) {
        mo_powerup_update(self)
    }
    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        if self.state > 0 {
            player.set_powerup(9);
            self.dead = true;
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_Tanooki {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_Tanooki {
    crate::impl_powerup_plumbing!();
}
