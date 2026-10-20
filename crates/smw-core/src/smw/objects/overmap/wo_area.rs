//! Domination mode area: a player claims it by touching it alone, then scores over time.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::global_constants::ticks;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_area, CObjectTrait};
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::overmap::over_map_object::{IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class area (for Domination mode)
//------------------------------------------------------------------------------
/// Capturable area; two or more players touching at once leave it neutral.
#[derive(Clone)]
pub struct OMO_Area {
    pub io_over_map_object: IO_OverMapObject,

    /// Number of areas in play; more areas allow them to sit closer together.
    pub numareas: i16,

    /// Index into `players` of the owner, or -1.
    pub iPlayerID: i16,
    /// Owner's team, or -1.
    pub iTeamID: i16,

    /// Owner's color, or -1 when neutral.
    pub colorID: i16,
    /// Ticks toward the owner's next point, every `ticks(2 * pointspeed)`.
    pub scoretimer: i16,
    /// Sprite x offset (pixels) for the owner's color.
    pub frame: i16,
    /// Ticks at the current spot; the area moves at the relocation setting.
    pub relocatetimer: i16,
    /// Players touching the area this tick.
    pub totalTouchingPlayers: i16,
    /// The one player touching this tick, or `PlayerId::NONE` if none or contested.
    pub touchingPlayer: PlayerId,
}
impl_base!(OMO_Area => io_over_map_object: IO_OverMapObject);

impl OMO_Area {
    pub fn new(g: &mut Game, nspr: SpriteId, iNumAreas: i16) -> Self {
        let mut this = OMO_Area {
            io_over_map_object: IO_OverMapObject::new(g, nspr, Vec2s::new(1280, 960), 5, 0, -1, -1, -1, -1, -1, -1, -1, -1),
            numareas: iNumAreas,
            iPlayerID: -1,
            iTeamID: -1,
            colorID: -1,
            scoretimer: 0,
            frame: 0,
            relocatetimer: 0,
            totalTouchingPlayers: 0,
            touchingPlayer: PlayerId::NONE,
        };
        this.iw = (g.rm.sprite(this.spr).get_width() as i16 as i32 / 5) as i16;
        this.collisionWidth = this.iw;

        this.objectType = object_area;

        this.place_area(g);
        this
    }

    /// Moves the area to a random spawn point away from other areas, trying up to 33 times.
    pub fn place_area(&mut self, g: &mut Game) {
        let mut gp = g.ptr();

        let mut x: i16 = 0;
        let mut y: i16 = 0;
        let mut iAttempts: i16 = 32;
        while (!gp.g_map.findspawnpoint(g, 5, &mut x, &mut y, self.collisionWidth, self.collisionHeight, false)
            || g.objectcontainer[0].get_closest_object(x, y, object_area) <= (200.0f32 - ((self.numareas as i32 - 3) as f32 * 25.0f32)))
            && {
                let a = iAttempts;
                iAttempts -= 1;
                a > 0
            }
        {}

        self.set_xi(x);
        self.set_yi(y);
    }

    /// Makes the area neutral.
    pub fn reset(&mut self) {
        self.iPlayerID = -1;
        self.iTeamID = -1;

        self.colorID = -1;
        self.scoretimer = 0;
        self.frame = 0;
    }

    pub fn get_color_id(&self) -> i16 {
        self.colorID
    }

    /// Gives the area to `player` if a different color holds it.
    pub fn set_owner(&mut self, g: &mut Game, player: PlayerId) {
        let mut gp = g.ptr();
        if self.colorID != g.player(player).colorID {
            self.iPlayerID = g.player(player).localID;
            self.iTeamID = g.player(player).teamID;
            self.colorID = g.player(player).colorID;

            self.frame = ((self.colorID as i32 + 1) * self.iw as i32) as i16;
            if_sound_on_play(g, &mut gp.rm.sfx_areatag);
        }
    }
}

impl CObjectTrait for OMO_Area {
    fn harness_hash_object(&mut self, h: &mut crate::smw::rollback::Fnv) {
        h.i16s(&[self.numareas, self.iPlayerID, self.iTeamID, self.colorID, self.scoretimer, self.frame, self.relocatetimer, self.totalTouchingPlayers]);
        h.player(self.touchingPlayer);
    }
    crate::impl_cobject_plumbing!();

    fn draw(&mut self, g: &mut Game) {
        g.rm.sprite(self.spr).draw_src(g, self.ix as i32, self.iy as i32, &Rect { x: self.frame as i32, y: 0, w: self.iw as i32, h: self.ih as i32 });
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if !self.touchingPlayer.is_none() {
            let p = self.touchingPlayer;
            self.set_owner(g, p);
        }

        if self.iPlayerID != -1 && !g.mode().gameover {
            self.scoretimer += 1;
            if self.scoretimer as i32 >= ticks((g.game_values.pointspeed as i32) << 1) {
                self.scoretimer = 0;
                g.player(gp.players[self.iPlayerID as usize]).score(g).adjust_score(g, 1);
                gp.get().mode().check_winner(g, gp.get().players[self.iPlayerID as usize]);
            }
        }

        if g.game_values.gamemodesettings.domination.relocationfrequency > 0 {
            self.relocatetimer += 1;
            if self.relocatetimer >= g.game_values.gamemodesettings.domination.relocationfrequency {
                self.relocatetimer = 0;
                self.place_area(g);
            }
        }

        self.totalTouchingPlayers = 0;
        self.touchingPlayer = PlayerId::NONE;
    }

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        if g.player(player).tanookisuit.not_statue() && !g.player(player).isdead() {
            self.totalTouchingPlayers += 1;

            if self.totalTouchingPlayers == 1 {
                self.touchingPlayer = player;
            } else {
                self.touchingPlayer = PlayerId::NONE;
                self.reset();
            }
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_Area {
    crate::impl_over_map_object_plumbing!();
}
