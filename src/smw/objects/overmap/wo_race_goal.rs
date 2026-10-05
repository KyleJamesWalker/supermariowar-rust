//! Port of src/smw/objects/overmap/WO_RaceGoal.cpp

use crate::common::game::App;
use crate::common::game_mode::game_mode_race;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::trig::{atan2f, cos, sin};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_race_goal, CObjectTrait};
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::race::CGM_Race;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::score_cnt;
use crate::smw::net_random::{self, Ev};
use crate::smw::objects::overmap::over_map_object::{io_over_map_object_update, IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class race goal (for Race mode)
//------------------------------------------------------------------------------
pub static mut flagpositions: [[[i16; 2]; 4]; 3] = [
    [[18, 20], [34, 20], [0, 0], [0, 0]],
    [[18, 20], [34, 20], [26, 36], [0, 0]],
    [[18, 20], [34, 20], [18, 36], [34, 36]],
];

pub struct OMO_RaceGoal {
    pub io_over_map_object: IO_OverMapObject,

    pub goalID: i16,
    pub tagged: [i16; 4],
    pub angle: f32,
    pub anglechange: f32,
    pub anglechangetimer: i16,
    pub speed: f32,
    pub quantity: i16,
    pub isfinishline: bool,
}
impl_base!(OMO_RaceGoal => io_over_map_object: IO_OverMapObject);

impl OMO_RaceGoal {
    pub fn new(nspr: Ptr<gfxSprite>, id: i16) -> Self {
        let mut this = OMO_RaceGoal {
            io_over_map_object: IO_OverMapObject::new(nspr, Vec2s::zero(), 2, 8, -1, -1, -1, -1, -1, -1, -1, -1),
            goalID: id,
            tagged: [0; 4],
            angle: 0.0,
            anglechange: 0.0,
            anglechangetimer: 0,
            speed: 0.0,
            // C++ leaves this uninitialized when placeRaceGoal() reads it below.
            quantity: 0,
            isfinishline: false,
        };
        this.iw = (this.spr.get_width() as i16 as i32 >> 1) as i16;
        this.ih = (this.spr.get_height() as i16 as i32 >> 1) as i16;
        this.collisionWidth = 36;
        this.collisionHeight = 36;
        this.collisionOffsetX = 16;
        this.collisionOffsetY = 18;

        this.objectType = object_race_goal;
        this.state = 1;

        for k in 0..4usize {
            this.tagged[k] = -1;
        }

        this.angle = RANDOM_INT(1000) as f32 * 0.00628f32;
        this.anglechange = RANDOM_INT(100) as f32 * 0.0002f32;
        this.anglechangetimer = (RANDOM_INT(50) + 100) as i16;

        this.velx = sin(this.angle);
        this.vely = cos(this.angle);

        this.place_race_goal();

        unsafe {
            this.speed = game_values.gamemodesettings.race.speed as f32 / 4.0f32;
            this.quantity = game_values.gamemodesettings.race.quantity;
        }
        this.isfinishline = this.goalID as i32 == this.quantity as i32 - 1;
        this
    }

    pub fn change_angle(&mut self) {
        self.anglechange = (RANDOM_INT(101) - 50) as f32 * 0.0002f32;
        self.anglechangetimer = (RANDOM_INT(50) + 100) as i16;
    }

    pub fn place_race_goal(&mut self) {
        let mut x: i16 = 0;
        let mut y: i16 = 0;

        unsafe {
            if self.goalID < g_map.iNumRaceGoals {
                x = g_map.racegoallocations[self.goalID as usize].x;
                y = g_map.racegoallocations[self.goalID as usize].y;
            } else {
                let mut tries: i16 = 0;
                loop {
                    tries += 1;
                    if tries > 32 {
                        break;
                    }

                    x = RANDOM_INT(App::screenWidth - self.collisionWidth as i32) as i16;
                    y = RANDOM_INT(App::screenHeight - self.collisionHeight as i32) as i16;

                    if !(objectcontainer[2].get_closest_object(x, y, object_race_goal) <= 250.0f32 - (self.quantity as f32 * 25.0f32)) {
                        break;
                    }
                }
            }
        }

        self.set_xi(x);
        self.set_yi(y);
    }

    pub fn reset(&mut self, teamID: i16) {
        self.tagged[teamID as usize] = -1;
    }

    pub fn is_tagged(&self, teamID: i16) -> i16 {
        self.tagged[teamID as usize]
    }

    pub fn get_goal_id(&self) -> i16 {
        self.goalID
    }
}

impl CObjectTrait for OMO_RaceGoal {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self) {
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;

        unsafe {
            if self.isfinishline {
                self.spr.draw_src(x, y, &SDL_Rect { x: self.drawframe as i32, y: 54, w: self.iw as i32, h: self.ih as i32 });
            } else {
                self.spr.draw_src(x, y, &SDL_Rect { x: self.drawframe as i32, y: 0, w: self.iw as i32, h: self.ih as i32 });

                for k in 0..score_cnt {
                    if self.tagged[k as usize] > -1 {
                        let fp = &flagpositions[(score_cnt - 2) as usize][k as usize];
                        rm.spr_bonus.draw_src(x + fp[0] as i32, y + fp[1] as i32, &SDL_Rect { x: 0, y: self.tagged[k as usize] as i32 * 16, w: 16, h: 16 });
                    }
                }
            }

            rm.spr_racetext.draw_src(x + 26, y, &SDL_Rect { x: (self.goalID as i32 + 1) * 16, y: 0, w: 16, h: 16 });
        }
    }

    fn update(&mut self) {
        self.anglechangetimer -= 1;
        if self.anglechangetimer <= 0 && !net_random::object_event(Ev::Wander, &net_random::wander_args(self.iNetworkID, self.fx, self.fy, self.angle)) {
            self.change_angle();
        }

        self.angle += self.anglechange;

        self.velx = self.speed * sin(self.angle);
        self.vely = self.speed * cos(self.angle);

        io_over_map_object_update(self);

        self.ix = self.fx as i16;
        self.iy = self.fy as i16;

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

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        unsafe {
            if game_values.gamemode.getgamemode() == game_mode_race && player.tanookisuit.not_statue() {
                let gamemode: &mut CGM_Race = game_values.gamemode.as_any().downcast_mut::<CGM_Race>().unwrap();

                if self.tagged[player.teamID as usize] != player.colorID && gamemode.get_next_goal(player.teamID) >= self.goalID {
                    self.tagged[player.teamID as usize] = player.colorID;

                    if self.isfinishline {
                        if_sound_on_play(&mut rm.sfx_racesound);
                    } else {
                        if_sound_on_play(&mut rm.sfx_areatag);
                    }
                }

                if gamemode.get_next_goal(player.teamID) == self.goalID {
                    gamemode.set_next_goal(player.teamID);
                }
            }
        }
        false
    }
}

impl IO_OverMapObjectTrait for OMO_RaceGoal {
    crate::impl_over_map_object_plumbing!();
}
