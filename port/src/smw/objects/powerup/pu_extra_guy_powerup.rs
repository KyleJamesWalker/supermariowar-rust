//! Port of src/smw/objects/powerup/PU_ExtraGuyPowerup.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::common::eyecandy::EC_FloatingObject;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::*;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class 1up powerup
//------------------------------------------------------------------------------
pub struct PU_ExtraGuyPowerup {
    pub mo_powerup: MO_Powerup,

    pub iType: i16,
}
impl_base!(PU_ExtraGuyPowerup => mo_powerup: MO_Powerup);

impl PU_ExtraGuyPowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, moveToRight: bool, aniSpeed: i16, iCollisionWidth: i16, iCollisionHeight: i16, iCollisionOffsetX: i16, iCollisionOffsetY: i16, r#type: i16) -> Self {
        let mut o = PU_ExtraGuyPowerup { mo_powerup: MO_Powerup::new(nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY), iType: r#type };
        if moveToRight {
            o.velx = 1.0f32 + r#type as f32;
        } else {
            o.velx = -1.0f32 - r#type as f32;
        }
        o
    }
}

impl CObjectTrait for PU_ExtraGuyPowerup {
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
        unsafe {
            if self.state > 0 {
                game_values.gamemode.playerextraguy(player, self.iType);
                if_sound_on_play(&mut rm.sfx_extraguysound);

                let iSrcY = ((if self.iType == 5 { 3 } else { self.iType as i32 - 1 }) * 16) as i16;
                eyecandy[2].emplace(EC_FloatingObject::new(
                    Ptr::from_mut(&mut rm.spr_extralife),
                    (player.ix as i32 + HALFPW - 19) as i16,
                    (player.iy as i32 - 16) as i16,
                    0.0f32,
                    -1.5f32,
                    62,
                    (player.colorID as i32 * 38) as i16,
                    iSrcY,
                    38,
                    16,
                ));

                self.dead = true;
            }
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_ExtraGuyPowerup {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_ExtraGuyPowerup {
    crate::impl_powerup_plumbing!();
}
