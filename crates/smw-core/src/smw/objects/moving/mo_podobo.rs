//! Port of src/smw/objects/moving/MO_Podobo.cpp

use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{object_moving, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::mo_bullet_bill::MO_BulletBill;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class podobo (for survival mode)
//------------------------------------------------------------------------------
pub struct MO_Podobo {
    pub io_moving_object: IO_MovingObject,

    pub iColorOffsetY: i16,

    pub fIsSpawned: bool,
    pub iHiddenPlane: i16,
}
impl_base!(MO_Podobo => io_moving_object: IO_MovingObject);

impl MO_Podobo {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, dVelY: f32, playerid: i16, teamid: i16, colorid: i16, isSpawned: bool) -> Self {
        let mut o = MO_Podobo {
            io_moving_object: IO_MovingObject::new(nspr, pos, 4, 6, -1, -1, -1, -1, -1, -1, -1, -1),
            iColorOffsetY: 0,
            fIsSpawned: false,
            iHiddenPlane: 0,
        };

        o.fIsSpawned = isSpawned;
        o.iHiddenPlane = pos.y;

        o.objectType = object_moving;
        o.movingObjectType = movingobject_podobo;
        o.vely = dVelY;

        o.ih = 32;
        o.collisionHeight = o.ih;

        o.iPlayerID = playerid;
        o.iTeamID = teamid;
        o.iColorOffsetY = ((colorid as i32 + 1) * 64) as i16;

        o.fObjectCollidesWithMap = false;
        o
    }
}

impl CObjectTrait for MO_Podobo {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        // Special slow podobo gravity
        self.vely += 0.2f32;

        let (fx, velx) = (self.fx, self.velx);
        self.set_xf(fx + velx);
        let (fy, vely) = (self.fy, self.vely);
        self.set_yf(fy + vely);

        self.animate();

        if self.iy as i32 > App::screenHeight - 1 && self.vely > 0.0f32 {
            self.dead = true;
        }
    }

    fn draw(&mut self) {
        let srcRect = SDL_Rect {
            x: self.drawframe as i32,
            y: self.iColorOffsetY as i32 + if self.vely > 0.0f32 { 32 } else { 0 },
            w: self.iw as i32,
            h: self.ih as i32,
        };
        if self.fIsSpawned && self.vely < 0.0f32 {
            self.spr.draw_clip(self.ix as i32, self.iy as i32, &srcRect, ClipEdge::Bottom, self.iHiddenPlane as i32);
        } else {
            self.spr.draw_src(self.ix as i32, self.iy as i32, &srcRect);
        }
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        unsafe {
            if player.globalID != self.iPlayerID
                && (game_values.teamcollision == TeamCollisionStyle::On || self.iTeamID != player.teamID)
                && !player.is_invincible()
                && !player.is_shielded()
                && !player.shyguy
            {
                // Find the player that made this explosion so we can attribute a kill
                player_killed_player(self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Podobo, false, false);
                return true;
            }

            false
        }
    }

    fn collide_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        if self.iPlayerID == -1 {
            return;
        }

        let r#type = object.get_moving_object_type();

        if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_bulletbill {
            // Same team bullet bills don't kill each other
            if r#type == movingobject_bulletbill && object.as_any().downcast_mut::<MO_BulletBill>().unwrap().iTeamID == self.iTeamID {
                return;
            }

            // ((CO_Shell*)object)->Die() / ((CO_ThrowBlock*)object)->Die() / ((MO_BulletBill*)object)->Die() are all virtual calls
            object.die();

            unsafe { if_sound_on_play(&mut rm.sfx_kicksound) };
        }
    }
}

impl IO_MovingObjectTrait for MO_Podobo {
    crate::impl_io_moving_object_plumbing!();
}
