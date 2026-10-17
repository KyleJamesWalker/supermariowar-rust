//! Port of src/smw/objects/overmap/WO_BowserFire.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
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

//------------------------------------------------------------------------------
// class Bowser Fire (for survival mode)
//------------------------------------------------------------------------------
#[derive(Clone)]
pub struct OMO_BowserFire {
    pub io_over_map_object: IO_OverMapObject,

    pub iPlayerID: i16,
    pub iTeamID: i16,
    pub iColorOffsetY: i16,
}
impl_base!(OMO_BowserFire => io_over_map_object: IO_OverMapObject);

impl OMO_BowserFire {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, vel: Vec2f, id: i16, teamid: i16, colorid: i16) -> Self {
        let mut this = OMO_BowserFire {
            io_over_map_object: IO_OverMapObject::new(g, nspr, pos, 3, 6, -1, -1, -1, -1, -1, -1, -1, -1),
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

    fn update(&mut self, g: &mut Game) {
        io_over_map_object_update(self);

        if (self.velx < 0.0 && (self.ix as i32) < -(self.iw as i32)) || (self.velx > 0.0 && self.ix as i32 > App::screenWidth) {
            let mut player: PlayerId = get_player_from_global_id(g, self.iPlayerID);

            if !player.is_none() {
                g.player(player).decrease_projectiles_count();
            }

            self.dead = true;
        }
    }

    fn draw(&mut self, g: &mut Game) {
        g.rm.sprite(self.spr).draw_src(
            g,
            self.ix as i32,
            self.iy as i32,
            &Rect { x: self.drawframe as i32, y: (if self.velx > 0.0f32 { 32 } else { 0 }) + self.iColorOffsetY as i32, w: self.iw as i32, h: self.ih as i32 },
        );
    }

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let ix = self.ix as i32;
        let iw = self.iw as i32;
        let pix = g.player(player).ix as i32;

        // if the fire is off the screen, don't wrap it to collide
        if (ix < 0 && self.velx < 0.0f32 && pix > ix + iw && pix + PW < App::screenWidth) || (ix + iw >= App::screenWidth && self.velx > 0.0f32 && pix + PW < ix && pix >= 0) {
            return false;
        }

        if g.player(player).globalID != self.iPlayerID && (g.game_values.teamcollision == TeamCollisionStyle::On || self.iTeamID != g.player(player).teamID) && !g.player(player).is_invincible() && !g.player(player).is_shielded() {
            // Find the player that made this explosion so we can attribute a kill
            player_killed_player(g, self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Fireball, false, false);
            return true;
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_BowserFire {
    crate::impl_over_map_object_plumbing!();
}
