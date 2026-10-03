//! Port of src/smw/objects/powerup/PU_IceWandPowerup.cpp

use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class ice wand powerup
//------------------------------------------------------------------------------
pub struct PU_IceWandPowerup {
    pub mo_powerup: MO_Powerup,

    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,
}
impl_base!(PU_IceWandPowerup => mo_powerup: MO_Powerup);

impl PU_IceWandPowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, iCollisionWidth: i16, iCollisionHeight: i16, iCollisionOffsetX: i16, iCollisionOffsetY: i16) -> Self {
        let mut o = PU_IceWandPowerup {
            mo_powerup: MO_Powerup::new(nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY),
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
        };
        o.velx = 0.0;
        o
    }
}

impl CObjectTrait for PU_IceWandPowerup {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        mo_powerup_update(self);

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }
    }

    fn draw(&mut self) {
        mo_powerup_draw(self);

        // Draw sparkles
        if self.state == 1 {
            unsafe {
                rm.spr_shinesparkle.draw_src(
                    self.ix as i32 - self.collisionOffsetX as i32,
                    self.iy as i32 - self.collisionOffsetY as i32,
                    &SDL_Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 },
                );
            }
        }
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        if self.state > 0 {
            player.set_powerup(5);
            self.dead = true;
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_IceWandPowerup {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_IceWandPowerup {
    crate::impl_powerup_plumbing!();
}
