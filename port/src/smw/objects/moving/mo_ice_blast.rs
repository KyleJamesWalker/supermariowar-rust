//! Port of src/smw/objects/moving/MO_IceBlast.cpp

use crate::common::eyecandy::Spotlight;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::spotlightManager;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::moving::moving_object::{io_moving_object_draw, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class Ice Blast
//------------------------------------------------------------------------------
pub struct MO_IceBlast {
    pub io_moving_object: IO_MovingObject,

    pub ttl: i16,
    pub sSpotlight: Ptr<Spotlight>,
}
impl_base!(MO_IceBlast => io_moving_object: IO_MovingObject);

impl MO_IceBlast {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, fVelyX: f32, iGlobalID: i16, teamID: i16, iColorID: i16) -> Self {
        let mut o = MO_IceBlast {
            io_moving_object: IO_MovingObject::new(nspr, pos, 4, 8, 32, 32, 0, 0, 0, ((iColorID as i32 + 1) << 5) as i16, 32, 32),
            ttl: 0,
            sSpotlight: Ptr::null(),
        };

        o.iPlayerID = iGlobalID;
        o.iTeamID = teamID;

        o.movingObjectType = movingobject_iceblast;

        o.state = 1;

        o.velx = fVelyX;
        o.vely = 0.0f32;

        if o.velx > 0.0f32 {
            o.drawframe = 0;
        } else {
            o.drawframe = o.animationWidth - o.iw;
        }

        o.ttl = 120;

        o.fObjectCollidesWithMap = false;

        o.sSpotlight = Ptr::null();
        o
    }
}

impl CObjectTrait for MO_IceBlast {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        unsafe {
            self.animationtimer += 1;
            if self.animationtimer == self.animationspeed {
                self.animationtimer = 0;

                if self.velx > 0.0 {
                    self.drawframe += self.iw;
                    if self.drawframe >= self.animationWidth {
                        self.drawframe = 0;
                    }
                } else {
                    self.drawframe -= self.iw;
                    if self.drawframe < 0 {
                        self.drawframe = self.animationWidth - self.iw;
                    }
                }
            }

            let (fx, velx) = (self.fx, self.velx);
            self.set_xf(fx + velx);

            self.ttl -= 1;
            if self.ttl <= 0 {
                removeifprojectile(self.as_mo_ptr(), false, true);
            } else if game_values.spotlights {
                let sx = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
                let sy = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
                if self.sSpotlight.is_null() {
                    self.sSpotlight = spotlightManager.add_spotlight(sx, sy, 3);
                }

                if !self.sSpotlight.is_null() {
                    self.sSpotlight.update_position(sx, sy);
                }
            }
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            if self.iPlayerID != player.globalID && (game_values.teamcollision == TeamCollisionStyle::On || self.iTeamID != player.teamID) {
                if !player.is_shielded() && !player.is_invincible() && !player.shyguy {
                    player.makefrozen(game_values.wandfreezetime);
                    removeifprojectile(self.as_mo_ptr(), false, true);
                }
            }

            false
        }
    }

    fn draw(&mut self) {
        io_moving_object_draw(self);
    }
}

impl IO_MovingObjectTrait for MO_IceBlast {
    crate::impl_io_moving_object_plumbing!();
}
