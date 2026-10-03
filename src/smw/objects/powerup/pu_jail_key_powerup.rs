//! Port of src/smw/objects/powerup/PU_JailKeyPowerup.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objectgame::PowerupType;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class special jail key powerup for jail mode
//------------------------------------------------------------------------------
pub struct PU_JailKeyPowerup {
    pub mo_powerup: MO_Powerup,
}
impl_base!(PU_JailKeyPowerup => mo_powerup: MO_Powerup);

impl PU_JailKeyPowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s) -> Self {
        let mut o = PU_JailKeyPowerup { mo_powerup: MO_Powerup::new(nspr, pos, 1, 0, 30, 30, 1, 1) };
        o.velx = 0.0;
        o
    }
}

impl CObjectTrait for PU_JailKeyPowerup {
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
            self.dead = true;
            player.set_stored_powerup(PowerupType::JailKey as i16);
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_JailKeyPowerup {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_JailKeyPowerup {
    crate::impl_powerup_plumbing!();
}
