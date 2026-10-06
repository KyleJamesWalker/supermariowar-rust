//! Port of src/smw/objects/overmap/WO_BowserFire.cpp

use crate::common::game::App;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::PW;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::object_base::{object_bowserfire, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::overmap::over_map_object::{io_over_map_object_update, IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::{get_player_from_global_id, player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class Bowser Fire (for survival mode)
//------------------------------------------------------------------------------
pub struct OMO_BowserFire {
    pub io_over_map_object: IO_OverMapObject,

    pub iPlayerID: i16,
    pub iTeamID: i16,
    pub iColorOffsetY: i16,
}
impl_base!(OMO_BowserFire => io_over_map_object: IO_OverMapObject);

impl OMO_BowserFire {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, vel: Vec2f, id: i16, teamid: i16, colorid: i16) -> Self {
        let mut this = OMO_BowserFire {
            io_over_map_object: IO_OverMapObject::new(nspr, pos, 3, 6, -1, -1, -1, -1, -1, -1, -1, -1),
            iPlayerID: id,
            iTeamID: teamid,
            iColorOffsetY: ((colorid as i32 + 1) * 64) as i16,
        };
        this.objectType = object_bowserfire;
        this.velx = vel.x;
        this.vely = vel.y;

        this.ih = 32;
        this.collisionHeight = this.ih;
        this
    }
}

impl CObjectTrait for OMO_BowserFire {
    crate::impl_cobject_plumbing!();

    fn update(&mut self) {
        io_over_map_object_update(self);

        if (self.velx < 0.0 && (self.ix as i32) < -(self.iw as i32)) || (self.velx > 0.0 && self.ix as i32 > App::screenWidth) {
            let mut player: Ptr<CPlayer> = get_player_from_global_id(self.iPlayerID);

            if !player.is_null() {
                player.decrease_projectiles_count();
            }

            self.dead = true;
        }
    }

    fn draw(&mut self) {
        self.spr.draw_src(
            self.ix as i32,
            self.iy as i32,
            &SDL_Rect { x: self.drawframe as i32, y: (if self.velx > 0.0f32 { 32 } else { 0 }) + self.iColorOffsetY as i32, w: self.iw as i32, h: self.ih as i32 },
        );
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let ix = self.ix as i32;
        let iw = self.iw as i32;
        let pix = player.ix as i32;

        // if the fire is off the screen, don't wrap it to collide
        if (ix < 0 && self.velx < 0.0f32 && pix > ix + iw && pix + PW < App::screenWidth) || (ix + iw >= App::screenWidth && self.velx > 0.0f32 && pix + PW < ix && pix >= 0) {
            return false;
        }

        unsafe {
            if player.globalID != self.iPlayerID && (game_values.teamcollision == TeamCollisionStyle::On || self.iTeamID != player.teamID) && !player.is_invincible() && !player.is_shielded() {
                // Find the player that made this explosion so we can attribute a kill
                player_killed_player(self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Fireball, false, false);
                return true;
            }
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_BowserFire {
    crate::impl_over_map_object_plumbing!();
}
