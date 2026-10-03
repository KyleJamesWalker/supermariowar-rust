//! Port of src/smw/objects/overmap/WO_Area.cpp

use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_area, CObjectTrait};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::players;
use crate::smw::objects::overmap::over_map_object::{IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class area (for Domination mode)
//------------------------------------------------------------------------------
pub struct OMO_Area {
    pub io_over_map_object: IO_OverMapObject,

    pub numareas: i16,

    pub iPlayerID: i16,
    pub iTeamID: i16,

    pub colorID: i16,
    pub scoretimer: i16,
    pub frame: i16,
    pub relocatetimer: i16,
    pub totalTouchingPlayers: i16,
    pub touchingPlayer: Ptr<CPlayer>,
}
impl_base!(OMO_Area => io_over_map_object: IO_OverMapObject);

impl OMO_Area {
    pub fn new(nspr: Ptr<gfxSprite>, iNumAreas: i16) -> Self {
        let mut this = OMO_Area {
            io_over_map_object: IO_OverMapObject::new(nspr, Vec2s::new(1280, 960), 5, 0, -1, -1, -1, -1, -1, -1, -1, -1),
            numareas: iNumAreas,
            iPlayerID: -1,
            iTeamID: -1,
            colorID: -1,
            scoretimer: 0,
            frame: 0,
            relocatetimer: 0,
            totalTouchingPlayers: 0,
            touchingPlayer: Ptr::null(),
        };
        this.iw = (this.spr.get_width() as i16 as i32 / 5) as i16;
        this.collisionWidth = this.iw;

        this.objectType = object_area;

        this.place_area();
        this
    }

    pub fn place_area(&mut self) {
        unsafe {
            let mut x: i16 = 0;
            let mut y: i16 = 0;
            let mut iAttempts: i16 = 32;
            while (!g_map.findspawnpoint(5, &mut x, &mut y, self.collisionWidth, self.collisionHeight, false)
                || objectcontainer[0].get_closest_object(x, y, object_area) <= (200.0f32 - ((self.numareas as i32 - 3) as f32 * 25.0f32)))
                && {
                    let a = iAttempts;
                    iAttempts -= 1;
                    a > 0
                }
            {}

            self.set_xi(x);
            self.set_yi(y);
        }
    }

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

    pub fn set_owner(&mut self, player: Ptr<CPlayer>) {
        if self.colorID != player.colorID {
            self.iPlayerID = player.localID;
            self.iTeamID = player.teamID;
            self.colorID = player.colorID;

            self.frame = ((self.colorID as i32 + 1) * self.iw as i32) as i16;
            unsafe {
                if_sound_on_play(&mut rm.sfx_areatag);
            }
        }
    }
}

impl CObjectTrait for OMO_Area {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self) {
        self.spr.draw_src(self.ix as i32, self.iy as i32, &SDL_Rect { x: self.frame as i32, y: 0, w: self.iw as i32, h: self.ih as i32 });
    }

    fn update(&mut self) {
        unsafe {
            if !self.touchingPlayer.is_null() {
                let p = self.touchingPlayer;
                self.set_owner(p);
            }

            if self.iPlayerID != -1 && !game_values.gamemode.gameover {
                self.scoretimer += 1;
                if self.scoretimer as i32 >= ((game_values.pointspeed as i32) << 1) {
                    self.scoretimer = 0;
                    players[self.iPlayerID as usize].score().adjust_score(1);
                    game_values.gamemode.check_winner(players[self.iPlayerID as usize]);
                }
            }

            if game_values.gamemodesettings.domination.relocationfrequency > 0 {
                self.relocatetimer += 1;
                if self.relocatetimer >= game_values.gamemodesettings.domination.relocationfrequency {
                    self.relocatetimer = 0;
                    self.place_area();
                }
            }

            self.totalTouchingPlayers = 0;
            self.touchingPlayer = Ptr::null();
        }
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        if player.tanookisuit.not_statue() && !player.isdead() {
            self.totalTouchingPlayers += 1;

            if self.totalTouchingPlayers == 1 {
                self.touchingPlayer = player;
            } else {
                self.touchingPlayer = Ptr::null();
                self.reset();
            }
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_Area {
    crate::impl_over_map_object_plumbing!();
}
