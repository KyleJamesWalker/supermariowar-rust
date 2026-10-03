//! Port of src/smw/objects/powerup/PU_SecretPowerup.cpp

use crate::common::eyecandy::EC_Snow;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// secret powerup
//------------------------------------------------------------------------------
pub struct PU_SecretPowerup {
    pub mo_powerup: MO_Powerup,

    pub itemtype: i16, // TODO: enum
    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,
}
impl_base!(PU_SecretPowerup => mo_powerup: MO_Powerup);

impl PU_SecretPowerup {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, r#type: i16) -> Self {
        let mut o = PU_SecretPowerup {
            mo_powerup: MO_Powerup::new(nspr, pos, 4, 8, 30, 30, 1, 1),
            itemtype: r#type,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
        };
        o.place();
        o
    }

    pub fn place(&mut self) {
        let mut ix = self.ix;
        let mut iy = self.iy;
        let collisionWidth = self.collisionWidth;
        let collisionHeight = self.collisionHeight;
        let mut iAttempts: i16 = 10;
        unsafe {
            while !g_map.findspawnpoint(5, &mut ix, &mut iy, collisionWidth, collisionHeight, false) && {
                let a = iAttempts;
                iAttempts -= 1;
                a > 0
            } {}
        }
        self.ix = ix;
        self.iy = iy;

        self.fx = self.ix as f32;
        self.fy = self.iy as f32;
    }
}

impl CObjectTrait for PU_SecretPowerup {
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
        unsafe {
            rm.spr_shinesparkle.draw_src(
                self.ix as i32 - self.collisionOffsetX as i32,
                self.iy as i32 - self.collisionOffsetY as i32,
                &SDL_Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 },
            );
        }
    }

    fn collide_player(&mut self, _player: Ptr<CPlayer>) -> bool {
        unsafe {
            if self.itemtype == 0 {
                game_values.windaffectsplayers = true;

                for _i in 0..15i16 {
                    let x = RANDOM_INT(App::screenWidth) as f32;
                    let y = RANDOM_INT(App::screenHeight) as f32;
                    let t = (RANDOM_INT(4) + 1) as i16;
                    eyecandy[2].emplace(EC_Snow::new(Ptr::from_mut(&mut rm.spr_snow), x, y, t));
                }
            } else if self.itemtype == 1 {
                game_values.spinscreen = true;
            } else if self.itemtype == 2 {
                game_values.reversewalk = true;
            } else if self.itemtype == 3 {
                game_values.spotlights = true;
            }

            if_sound_on_play(&mut rm.sfx_pickup);
        }

        self.dead = true;
        false
    }
}

impl IO_MovingObjectTrait for PU_SecretPowerup {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_SecretPowerup {
    crate::impl_powerup_plumbing!();
}
