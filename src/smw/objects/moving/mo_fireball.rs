//! Port of src/smw/objects/moving/MO_Fireball.cpp

use crate::common::eyecandy::Spotlight;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::spotlightManager;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::moving::moving_object::{io_moving_object_update, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class fireball
//------------------------------------------------------------------------------
pub struct MO_Fireball {
    pub io_moving_object: IO_MovingObject,

    pub colorOffset: i16,

    pub ttl: i16,
    pub sSpotlight: Ptr<Spotlight>,
}
impl_base!(MO_Fireball => io_moving_object: IO_MovingObject);

impl MO_Fireball {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, moveToRight: bool, aniSpeed: i16, iGlobalID: i16, teamID: i16, iColorID: i16) -> Self {
        let mut o = MO_Fireball {
            io_moving_object: IO_MovingObject::new(
                nspr,
                pos,
                iNumSpr,
                aniSpeed,
                (nspr.get_width() as i16) >> 2,
                (nspr.get_height() as i16) >> 3,
                0,
                0,
                -1,
                -1,
                -1,
                -1,
            ),
            colorOffset: 0,
            ttl: 0,
            sSpotlight: Ptr::null(),
        };

        if moveToRight {
            o.velx = 5.0;
        } else {
            o.velx = -5.0;
        }

        // fireball sprites have both right and left sprites in them
        o.ih = o.ih >> 3;

        o.bounce = -FIREBALLBOUNCE;

        o.iPlayerID = iGlobalID;
        o.iTeamID = teamID;

        o.colorOffset = (iColorID as i32 * 36) as i16;
        o.movingObjectType = movingobject_fireball;

        o.state = 1;

        o.ttl = unsafe { game_values.fireballttl };

        o.sSpotlight = Ptr::null();
        o
    }
}

impl CObjectTrait for MO_Fireball {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        io_moving_object_update(self);

        unsafe {
            self.ttl -= 1;
            if self.ttl <= 0 {
                removeifprojectile(self.as_mo_ptr(), true, true);
            } else if game_values.spotlights {
                if self.sSpotlight.is_null() {
                    self.sSpotlight = spotlightManager.add_spotlight(
                        (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16,
                        (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16,
                        3,
                    );
                }

                if !self.sSpotlight.is_null() {
                    let x = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
                    let y = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
                    self.sSpotlight.update_position(x, y);
                }
            }
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            if self.iPlayerID != player.get_global_id() && (game_values.teamcollision == TeamCollisionStyle::On || self.iTeamID != player.get_team_id()) {
                if !player.is_shielded() {
                    removeifprojectile(self.as_mo_ptr(), false, false);

                    if !player.is_invincible() && !player.shyguy {
                        // Find the player that shot this fireball so we can attribute a kill
                        player_killed_player(self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Fireball, false, false);
                        return true;
                    }
                }
            }
        }

        false
    }

    fn draw(&mut self) {
        self.spr.draw_src(
            self.ix as i32 - self.collisionOffsetX as i32,
            self.iy as i32 - self.collisionOffsetY as i32,
            &SDL_Rect {
                x: self.drawframe as i32,
                y: (if self.velx > 0.0 { 0 } else { 18 }) + self.colorOffset as i32,
                w: self.iw as i32,
                h: self.ih as i32,
            },
        );
    }
}

impl IO_MovingObjectTrait for MO_Fireball {
    crate::impl_io_moving_object_plumbing!();
}
