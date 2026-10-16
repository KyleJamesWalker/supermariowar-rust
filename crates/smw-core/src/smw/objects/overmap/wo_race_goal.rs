//! Port of src/smw/objects/overmap/WO_RaceGoal.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game::App;
use crate::common::game_mode::game_mode_race;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::trig::{atan2f, cosf, sinf};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_race_goal, CObjectTrait};
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::race::CGM_Race;
use crate::smw::net_random::{self, Ev};
use crate::smw::objects::overmap::over_map_object::{io_over_map_object_update, IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class race goal (for Race mode)
//------------------------------------------------------------------------------
pub static flagpositions: [[[i16; 2]; 4]; 3] = [
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
    pub fn new(g: &mut Game, nspr: SpriteId, id: i16) -> Self {
        let mut gp = g.ptr();
        let mut this = OMO_RaceGoal {
            io_over_map_object: IO_OverMapObject::new(g, nspr, Vec2s::zero(), 2, 8, -1, -1, -1, -1, -1, -1, -1, -1),
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
        this.iw = (g.rm.sprite(this.spr).get_width() as i16 as i32 >> 1) as i16;
        this.ih = (g.rm.sprite(this.spr).get_height() as i16 as i32 >> 1) as i16;
        this.collisionWidth = 36;
        this.collisionHeight = 36;
        this.collisionOffsetX = 16;
        this.collisionOffsetY = 18;

        this.objectType = object_race_goal;
        this.state = 1;

        for k in 0..4usize {
            this.tagged[k] = -1;
        }

        this.angle = RANDOM_INT(gp.get(), 1000) as f32 * 0.00628f32;
        this.anglechange = RANDOM_INT(gp.get(), 100) as f32 * 0.0002f32;
        this.anglechangetimer = (RANDOM_INT(gp.get(), 50) + 100) as i16;

        this.velx = sinf(this.angle);
        this.vely = cosf(this.angle);

        this.place_race_goal(g);

        this.speed = g.game_values.gamemodesettings.race.speed as f32 / 4.0f32;
        this.quantity = g.game_values.gamemodesettings.race.quantity;
        this.isfinishline = this.goalID as i32 == this.quantity as i32 - 1;
        this
    }

    pub fn change_angle(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.anglechange = (RANDOM_INT(gp.get(), 101) - 50) as f32 * 0.0002f32;
        self.anglechangetimer = (RANDOM_INT(gp.get(), 50) + 100) as i16;
    }

    pub fn place_race_goal(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut x: i16 = 0;
        let mut y: i16 = 0;

        if self.goalID < g.g_map.iNumRaceGoals {
            x = g.g_map.racegoallocations[self.goalID as usize].x;
            y = g.g_map.racegoallocations[self.goalID as usize].y;
        } else {
            let mut tries: i16 = 0;
            loop {
                tries += 1;
                if tries > 32 {
                    break;
                }

                x = RANDOM_INT(gp.get(), App::screenWidth - self.collisionWidth as i32) as i16;
                y = RANDOM_INT(gp.get(), App::screenHeight - self.collisionHeight as i32) as i16;

                if !(g.objectcontainer[2].get_closest_object(x, y, object_race_goal) <= 250.0f32 - (self.quantity as f32 * 25.0f32)) {
                    break;
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

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;

        if self.isfinishline {
            g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: self.drawframe as i32, y: 54, w: self.iw as i32, h: self.ih as i32 });
        } else {
            g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: self.drawframe as i32, y: 0, w: self.iw as i32, h: self.ih as i32 });

            for k in 0..g.score_cnt {
                if self.tagged[k as usize] > -1 {
                    let fp = &flagpositions[(g.score_cnt - 2) as usize][k as usize];
                    gp.rm.sprite(gp.rm.spr_bonus).draw_src(g, x + fp[0] as i32, y + fp[1] as i32, &Rect { x: 0, y: self.tagged[k as usize] as i32 * 16, w: 16, h: 16 });
                }
            }
        }

        gp.rm.sprite(gp.rm.spr_racetext).draw_src(g, x + 26, y, &Rect { x: (self.goalID as i32 + 1) * 16, y: 0, w: 16, h: 16 });
    }

    fn update(&mut self, g: &mut Game) {
        self.anglechangetimer -= 1;
        if self.anglechangetimer <= 0 && !net_random::object_event(g, Ev::Wander, &net_random::wander_args(self.iNetworkID, self.fx, self.fy, self.angle)) {
            self.change_angle(g);
        }

        self.angle += self.anglechange;

        self.velx = self.speed * sinf(self.angle);
        self.vely = self.speed * cosf(self.angle);

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

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut gp = g.ptr();
        if g.game_values.gamemode.getgamemode() == game_mode_race && g.player(player).tanookisuit.not_statue() {
            let gamemode: &mut CGM_Race = g.game_values.gamemode.get().as_any().downcast_mut::<CGM_Race>().unwrap();

            if self.tagged[g.player(player).teamID as usize] != g.player(player).colorID && gamemode.get_next_goal(g.player(player).teamID) >= self.goalID {
                self.tagged[g.player(player).teamID as usize] = g.player(player).colorID;

                if self.isfinishline {
                    if_sound_on_play(g, &mut gp.rm.sfx_racesound);
                } else {
                    if_sound_on_play(g, &mut gp.rm.sfx_areatag);
                }
            }

            if gamemode.get_next_goal(g.player(player).teamID) == self.goalID {
                gamemode.set_next_goal(g, g.player(player).teamID);
            }
        }
        false
    }
}

impl IO_OverMapObjectTrait for OMO_RaceGoal {
    crate::impl_over_map_object_plumbing!();
}
