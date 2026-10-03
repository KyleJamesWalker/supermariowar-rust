//! Port of src/smw/objects/powerup/PU_PWingsPowerup.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class pwings
//------------------------------------------------------------------------------
pub struct PU_PWingsPowerup {
    pub mo_powerup: MO_Powerup,
}
impl_base!(PU_PWingsPowerup => mo_powerup: MO_Powerup);

impl PU_PWingsPowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s) -> Self {
        PU_PWingsPowerup { mo_powerup: MO_Powerup::new(nspr, pos, 1, 0, 30, 30, 1, 1) }
    }
}

impl CObjectTrait for PU_PWingsPowerup {
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
        player.set_powerup(8);
        self.dead = true;
        false
    }
}

impl IO_MovingObjectTrait for PU_PWingsPowerup {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_PWingsPowerup {
    crate::impl_powerup_plumbing!();
}
