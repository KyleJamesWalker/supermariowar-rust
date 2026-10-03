//! Port of src/smw/objects/powerup/PU_StarPowerup.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::common::global_constants::VELPOWERUPBOUNCE;
use crate::smw::objectgame::PowerupType;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class star powerup
//------------------------------------------------------------------------------
pub struct PU_StarPowerup {
    pub mo_powerup: MO_Powerup,
}
impl_base!(PU_StarPowerup => mo_powerup: MO_Powerup);

impl PU_StarPowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, moveToRight: bool, aniSpeed: i16, iCollisionWidth: i16, iCollisionHeight: i16, iCollisionOffsetX: i16, iCollisionOffsetY: i16) -> Self {
        let mut o = PU_StarPowerup { mo_powerup: MO_Powerup::new(nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY) };
        if moveToRight {
            o.velx = 2.0f32;
        } else {
            o.velx = -2.0f32;
        }

        o.bounce = -VELPOWERUPBOUNCE;
        o
    }
}

impl CObjectTrait for PU_StarPowerup {
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

            if !player.is_invincible() && !player.shyguy {
                player.makeinvincible();
            } else {
                player.set_stored_powerup(PowerupType::Star as i16);
            }
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_StarPowerup {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_StarPowerup {
    crate::impl_powerup_plumbing!();
}
