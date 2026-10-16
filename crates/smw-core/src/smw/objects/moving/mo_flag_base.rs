//! Port of src/smw/objects/moving/MO_FlagBase.cpp

use crate::smw::object_container::ObjectId;
use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::trig::{atan2f, cosf, sinf};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{object_moving, CObjectTrait};
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::net_random::{self, Ev};
use crate::smw::objects::carriable::co_flag::CO_Flag;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class flag base (for CTF mode)
//------------------------------------------------------------------------------
pub struct MO_FlagBase {
    pub io_moving_object: IO_MovingObject,

    pub teamID: i16,
    pub iGraphicOffsetX: i16,
    pub angle: f32,
    pub anglechange: f32,
    pub anglechangetimer: i16,
    pub speed: f32,
    pub homeflag: ObjectId,

    pub timer: i16,
}
impl_base!(MO_FlagBase => io_moving_object: IO_MovingObject);

impl MO_FlagBase {
    pub fn new(g: &mut Game, nspr: SpriteId, iTeamID: i16, iColorID: i16) -> Self {
        let mut gp = g.ptr();
        let mut o = MO_FlagBase {
            // use 1280 and 960 so when placing base, it doesn't interfere (look in getClosestObject())
            io_moving_object: IO_MovingObject::new(g, nspr, Vec2s::new(1280, 960), 5, 0, -1, -1, -1, -1, -1, -1, -1, -1),
            teamID: 0,
            iGraphicOffsetX: 0,
            angle: 0.0,
            anglechange: 0.0,
            anglechangetimer: 0,
            speed: 0.0,
            homeflag: ObjectId::NONE,
            timer: 0,
        };

        o.state = 1;
        o.iw = 32;
        o.ih = 32;
        o.collisionWidth = 32;
        o.collisionHeight = 32;

        o.objectType = object_moving;
        o.movingObjectType = movingobject_flagbase;
        o.teamID = iTeamID;
        o.iGraphicOffsetX = (iColorID as i32 * 48) as i16;

        o.angle = RANDOM_INT(gp.get(), 1000) as f32 * 0.00628f32;
        o.anglechange = RANDOM_INT(gp.get(), 100) as f32 * 0.0002f32;
        o.anglechangetimer = (RANDOM_INT(gp.get(), 50) + 100) as i16;

        o.velx = sinf(o.angle);
        o.vely = cosf(o.angle);

        o.homeflag = ObjectId::NONE;

        o.place_flag_base(g, true);

        o.speed = g.game_values.gamemodesettings.flag.speed as f32 / 4.0f32;

        o.timer = 0;

        o.fObjectCollidesWithMap = false;
        o
    }

    pub fn place_flag_base(&mut self, g: &mut Game, fInit: bool) {
        let mut gp = g.ptr();
        if crate::smw::net_random::place_event(g, self.iNetworkID, fInit as i32) {
            return;
        }

        self.timer = 0;
        let mut x: i16 = 0;
        let mut y: i16 = 0;

        if fInit && self.teamID < g.g_map.iNumFlagBases {
            x = g.g_map.flagbaselocations[self.teamID as usize].x;
            y = g.g_map.flagbaselocations[self.teamID as usize].y;
        } else {
            let mut iAttempts: i16 = 32;
            let (cw, ch) = (self.collisionWidth, self.collisionHeight);
            while (!gp.g_map.findspawnpoint(g, 5, &mut x, &mut y, cw, ch, false)
                || g.objectcontainer[1].get_closest_moving_object(x, y, movingobject_flagbase) < 200.0f32)
                && {
                    let a = iAttempts;
                    iAttempts -= 1;
                    a > 0
                }
            {}
        }

        self.set_xi(x);
        self.set_yi(y);
    }

    pub fn score_flag(&mut self, g: &mut Game, mut flag: Ptr<CO_Flag>, mut player: PlayerId) {
        let mut gp = g.ptr();
        if flag.teamID == self.teamID {
            flag.place_flag(g);
            if_sound_on_play(g, &mut gp.rm.sfx_areatag);
        } else if !g.game_values.gamemodesettings.flag.homescore || !self.homeflag.is_none() || g.game_values.gamemodesettings.flag.centerflag {
            flag.place_flag(g);
            if !g.game_values.gamemode.gameover {
                g.player(player).score().adjust_score(g, 1);
                gp.game_values.gamemode.check_winner(g, player);
            }

            if_sound_on_play(g, &mut gp.rm.sfx_racesound);

            if g.game_values.gamemodesettings.flag.pointmove {
                // Set the values way outside the map so it will place the base correctly
                self.ix = 1280;
                self.iy = 960;
                self.place_flag_base(g, false);
            }
        }
    }

    pub fn change_angle(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.anglechange = (RANDOM_INT(gp.get(), 101) - 50) as f32 * 0.0002f32;
        self.anglechangetimer = (RANDOM_INT(gp.get(), 50) + 100) as i16;
    }

    pub fn set_flag(&mut self, flag: ObjectId) {
        self.homeflag = flag;
    }

    pub fn get_team_id(&self) -> i16 {
        self.teamID
    }
}

impl CObjectTrait for MO_FlagBase {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, g: &mut Game, mut player: PlayerId) -> bool {
        if self.teamID == g.player(player).teamID && !g.player(player).carriedItem.is_null() && g.player(player).carriedItem.get_moving_object_type() == movingobject_flag {
            let flag: Ptr<CO_Flag> = Ptr::from_mut(g.player(player).carriedItem.as_any().downcast_mut::<CO_Flag>().unwrap());
            self.score_flag(g, flag, player);
            self.timer = 0;
        }

        false
    }

    fn draw(&mut self, g: &mut Game) {
        g.rm.sprite(self.spr).draw_src(
            g,
            self.ix as i32 - 8,
            self.iy as i32 - 8,
            &Rect { x: self.iGraphicOffsetX as i32, y: 0, w: 48, h: 48 },
        );
    }

    fn update(&mut self, g: &mut Game) {
        if g.game_values.gamemodesettings.flag.speed > 0 {
            self.anglechangetimer -= 1;
            if self.anglechangetimer <= 0 && !net_random::object_event(g, Ev::Wander, &net_random::wander_args(self.iNetworkID, self.fx, self.fy, self.angle)) {
                self.change_angle(g);
            }

            self.angle += self.anglechange;

            self.velx = self.speed * sinf(self.angle);
            self.vely = self.speed * cosf(self.angle);

            let (fx, velx) = (self.fx, self.velx);
            self.set_xf(fx + velx);
            let (fy, vely) = (self.fy, self.vely);
            self.set_yf(fy + vely);

            if self.ix < 0 {
                self.velx = -self.velx;
                self.ix = 0;
                self.fx = self.ix as f32;

                self.angle = atan2f(self.velx, self.vely);
            } else if self.ix as i32 + self.collisionWidth as i32 >= App::screenWidth {
                self.velx = -self.velx;
                self.ix = (App::screenWidth - 1 - self.collisionWidth as i32) as i16;
                self.fx = self.ix as f32;

                self.angle = atan2f(self.velx, self.vely);
            }

            if self.iy < 0 {
                self.vely = -self.vely;
                self.iy = 0;
                self.fy = self.iy as f32;

                self.angle = atan2f(self.velx, self.vely);
            } else if self.iy as i32 + self.collisionHeight as i32 >= App::screenHeight {
                self.vely = -self.vely;
                self.iy = (App::screenHeight - 1 - self.collisionHeight as i32) as i16;
                self.fy = self.iy as f32;

                self.angle = atan2f(self.velx, self.vely);
            }
        }

        if g.game_values.gamemodesettings.flag.speed == 0 && {
            let t = self.timer;
            self.timer += 1;
            t > 1000
        } {
            self.place_flag_base(g, false);
        }
    }

    fn collide_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        if object.get_moving_object_type() == movingobject_flag {
            let flag: Ptr<CO_Flag> = Ptr::from_mut(object.as_any().downcast_mut::<CO_Flag>().unwrap());
            let player = flag.owner_throw;

            if !player.is_none() {
                if self.teamID == g.player(player).teamID {
                    self.score_flag(g, flag, player);
                }
            }
        }
    }
}

impl IO_MovingObjectTrait for MO_FlagBase {
    crate::impl_io_moving_object_plumbing!();
}
