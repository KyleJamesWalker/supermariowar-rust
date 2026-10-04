//! Port of src/smw/objects/moving/MO_FlagBase.cpp

use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{object_moving, CObjectTrait};
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::net_random::{self, Ev};
use crate::smw::objects::carriable::co_flag::CO_Flag;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

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
    pub homeflag: Ptr<CO_Flag>,

    pub timer: i16,
}
impl_base!(MO_FlagBase => io_moving_object: IO_MovingObject);

impl MO_FlagBase {
    pub fn new(nspr: Ptr<gfxSprite>, iTeamID: i16, iColorID: i16) -> Self {
        let mut o = MO_FlagBase {
            // use 1280 and 960 so when placing base, it doesn't interfere (look in getClosestObject())
            io_moving_object: IO_MovingObject::new(nspr, Vec2s::new(1280, 960), 5, 0, -1, -1, -1, -1, -1, -1, -1, -1),
            teamID: 0,
            iGraphicOffsetX: 0,
            angle: 0.0,
            anglechange: 0.0,
            anglechangetimer: 0,
            speed: 0.0,
            homeflag: Ptr::null(),
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

        o.angle = RANDOM_INT(1000) as f32 * 0.00628f32;
        o.anglechange = RANDOM_INT(100) as f32 * 0.0002f32;
        o.anglechangetimer = (RANDOM_INT(50) + 100) as i16;

        o.velx = o.angle.sin();
        o.vely = o.angle.cos();

        o.homeflag = Ptr::null();

        o.place_flag_base(true);

        o.speed = unsafe { game_values.gamemodesettings.flag.speed } as f32 / 4.0f32;

        o.timer = 0;

        o.fObjectCollidesWithMap = false;
        o
    }

    pub fn place_flag_base(&mut self, fInit: bool) {
        if crate::smw::net_random::place_event(self.iNetworkID, fInit as i32) {
            return;
        }

        unsafe {
            self.timer = 0;
            let mut x: i16 = 0;
            let mut y: i16 = 0;

            if fInit && self.teamID < g_map.iNumFlagBases {
                x = g_map.flagbaselocations[self.teamID as usize].x;
                y = g_map.flagbaselocations[self.teamID as usize].y;
            } else {
                let mut iAttempts: i16 = 32;
                let (cw, ch) = (self.collisionWidth, self.collisionHeight);
                while (!g_map.findspawnpoint(5, &mut x, &mut y, cw, ch, false)
                    || objectcontainer[1].get_closest_moving_object(x, y, movingobject_flagbase) < 200.0f32)
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
    }

    pub fn score_flag(&mut self, mut flag: Ptr<CO_Flag>, mut player: Ptr<CPlayer>) {
        unsafe {
            if flag.teamID == self.teamID {
                flag.place_flag();
                if_sound_on_play(&mut rm.sfx_areatag);
            } else if !game_values.gamemodesettings.flag.homescore || !self.homeflag.is_null() || game_values.gamemodesettings.flag.centerflag {
                flag.place_flag();
                if !game_values.gamemode.gameover {
                    player.score().adjust_score(1);
                    game_values.gamemode.check_winner(player);
                }

                if_sound_on_play(&mut rm.sfx_racesound);

                if game_values.gamemodesettings.flag.pointmove {
                    // Set the values way outside the map so it will place the base correctly
                    self.ix = 1280;
                    self.iy = 960;
                    self.place_flag_base(false);
                }
            }
        }
    }

    pub fn change_angle(&mut self) {
        self.anglechange = (RANDOM_INT(101) - 50) as f32 * 0.0002f32;
        self.anglechangetimer = (RANDOM_INT(50) + 100) as i16;
    }

    pub fn set_flag(&mut self, flag: Ptr<CO_Flag>) {
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

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if self.teamID == player.teamID && !player.carriedItem.is_null() && player.carriedItem.get_moving_object_type() == movingobject_flag {
            let flag: Ptr<CO_Flag> = Ptr::from_mut(player.carriedItem.as_any().downcast_mut::<CO_Flag>().unwrap());
            self.score_flag(flag, player);
            self.timer = 0;
        }

        false
    }

    fn draw(&mut self) {
        self.spr.draw_src(
            self.ix as i32 - 8,
            self.iy as i32 - 8,
            &SDL_Rect { x: self.iGraphicOffsetX as i32, y: 0, w: 48, h: 48 },
        );
    }

    fn update(&mut self) {
        unsafe {
            if game_values.gamemodesettings.flag.speed > 0 {
                self.anglechangetimer -= 1;
                if self.anglechangetimer <= 0 && !net_random::object_event(Ev::Wander, &net_random::wander_args(self.iNetworkID, self.fx, self.fy, self.angle)) {
                    self.change_angle();
                }

                self.angle += self.anglechange;

                self.velx = self.speed * self.angle.sin();
                self.vely = self.speed * self.angle.cos();

                let (fx, velx) = (self.fx, self.velx);
                self.set_xf(fx + velx);
                let (fy, vely) = (self.fy, self.vely);
                self.set_yf(fy + vely);

                if self.ix < 0 {
                    self.velx = -self.velx;
                    self.ix = 0;
                    self.fx = self.ix as f32;

                    self.angle = self.velx.atan2(self.vely);
                } else if self.ix as i32 + self.collisionWidth as i32 >= App::screenWidth {
                    self.velx = -self.velx;
                    self.ix = (App::screenWidth - 1 - self.collisionWidth as i32) as i16;
                    self.fx = self.ix as f32;

                    self.angle = self.velx.atan2(self.vely);
                }

                if self.iy < 0 {
                    self.vely = -self.vely;
                    self.iy = 0;
                    self.fy = self.iy as f32;

                    self.angle = self.velx.atan2(self.vely);
                } else if self.iy as i32 + self.collisionHeight as i32 >= App::screenHeight {
                    self.vely = -self.vely;
                    self.iy = (App::screenHeight - 1 - self.collisionHeight as i32) as i16;
                    self.fy = self.iy as f32;

                    self.angle = self.velx.atan2(self.vely);
                }
            }

            if game_values.gamemodesettings.flag.speed == 0 && {
                let t = self.timer;
                self.timer += 1;
                t > 1000
            } {
                self.place_flag_base(false);
            }
        }
    }

    fn collide_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        if object.get_moving_object_type() == movingobject_flag {
            let flag: Ptr<CO_Flag> = Ptr::from_mut(object.as_any().downcast_mut::<CO_Flag>().unwrap());
            let player = flag.owner_throw;

            if !player.is_null() {
                if self.teamID == player.teamID {
                    self.score_flag(flag, player);
                }
            }
        }
    }
}

impl IO_MovingObjectTrait for MO_FlagBase {
    crate::impl_io_moving_object_plumbing!();
}
