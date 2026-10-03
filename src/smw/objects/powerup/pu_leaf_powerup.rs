//! Port of src/smw/objects/powerup/PU_LeafPowerup.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::pu_feather_powerup::{pu_feather_powerup_draw, pu_feather_powerup_update, PU_FeatherPowerup, PU_FeatherPowerupTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class leaf powerup
//------------------------------------------------------------------------------
pub struct PU_LeafPowerup {
    pub pu_feather_powerup: PU_FeatherPowerup,
}
impl_base!(PU_LeafPowerup => pu_feather_powerup: PU_FeatherPowerup);

impl PU_LeafPowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, iCollisionWidth: i16, iCollisionHeight: i16, iCollisionOffsetX: i16, iCollisionOffsetY: i16) -> Self {
        PU_LeafPowerup {
            pu_feather_powerup: PU_FeatherPowerup::new(nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY),
        }
    }
}

impl CObjectTrait for PU_LeafPowerup {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }
    fn draw(&mut self) {
        pu_feather_powerup_draw(self)
    }
    fn update(&mut self) {
        pu_feather_powerup_update(self)
    }
    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        if self.state > 0 {
            player.set_powerup(7);
            self.dead = true;
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_LeafPowerup {
    crate::impl_io_moving_object_plumbing!();
}

impl PU_FeatherPowerupTrait for PU_LeafPowerup {
    crate::impl_feather_powerup_plumbing!();
}
