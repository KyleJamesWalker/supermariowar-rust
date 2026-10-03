//! Port of src/smw/objects/overmap/WO_PipeCoin.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::GRAVITATION;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::object_base::{object_pipe_coin, CObjectTrait};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::main::pipegamemode;
use crate::smw::objects::overmap::over_map_object::{IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class pipe coin (for coin pipe minigame)
//------------------------------------------------------------------------------
pub struct OMO_PipeCoin {
    pub io_over_map_object: IO_OverMapObject,

    pub iTeamID: i16,
    pub iColorID: i16,

    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,

    pub iUncollectableTime: i16,
}
impl_base!(OMO_PipeCoin => io_over_map_object: IO_OverMapObject);

impl OMO_PipeCoin {
    pub fn new(nspr: Ptr<gfxSprite>, vel: Vec2f, pos: Vec2s, teamid: i16, colorid: i16, uncollectabletime: i16) -> Self {
        let mut this = OMO_PipeCoin {
            io_over_map_object: IO_OverMapObject::new(nspr, pos, 4, 8, 30, 30, 1, 1, 0, ((colorid as i32) << 5) as i16, 32, 32),
            iTeamID: teamid,
            iColorID: colorid,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
            iUncollectableTime: 0,
        };
        this.state = 1;
        this.objectType = object_pipe_coin;

        this.velx = vel.x;

        unsafe {
            if pipegamemode.is_slowdown() {
                this.vely = vel.y / 2.0f32;
            } else {
                this.vely = vel.y;
            }
        }

        this.iUncollectableTime = uncollectabletime;
        this
    }

    pub fn get_color(&self) -> i16 {
        self.iColorID
    }

    pub fn get_team(&self) -> i16 {
        self.iTeamID
    }
}

impl CObjectTrait for OMO_PipeCoin {
    crate::impl_cobject_plumbing!();

    fn update(&mut self) {
        let x = self.fx + self.velx;
        self.set_xf(x);
        let y = self.fy + self.vely;
        self.set_yf(y);

        if self.iTeamID == -1 {
            self.animate();
        }

        if self.iy as i32 >= App::screenHeight {
            self.dead = true;
        }

        unsafe {
            if pipegamemode.is_slowdown() {
                self.vely += GRAVITATION / 1.5f32;
            } else {
                self.vely += GRAVITATION;
            }
        }

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }

        if self.iUncollectableTime > 0 {
            self.iUncollectableTime -= 1;
        }
    }

    fn draw(&mut self) {
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        let srcRect = SDL_Rect { x: self.drawframe as i32, y: self.animationOffsetY as i32, w: self.iw as i32, h: self.ih as i32 };
        let sparkleRect = SDL_Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 };

        unsafe {
            if self.iUncollectableTime > 0 {
                self.spr.draw_clip(x, y, &srcRect, ClipEdge::Bottom, 256);

                // Draw sparkles
                if self.iTeamID == -1 {
                    rm.spr_shinesparkle.draw_clip(x, y, &sparkleRect, ClipEdge::Bottom, 256);
                }
            } else {
                self.spr.draw_src(x, y, &srcRect);

                // Draw sparkles
                if self.iTeamID == -1 {
                    rm.spr_shinesparkle.draw_src(x, y, &sparkleRect);
                }
            }
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if self.iUncollectableTime > 0 {
            return false;
        }

        unsafe {
            if !game_values.gamemode.gameover {
                if self.iTeamID != -1 {
                    if player.teamID == self.iTeamID {
                        player.score().adjust_score(1);
                        if_sound_on_play(&mut rm.sfx_coin);
                    }
                } else if self.iColorID == 2 {
                    player.score().adjust_score(1);
                    if_sound_on_play(&mut rm.sfx_coin);
                } else if self.iColorID == 0 {
                    player.score().adjust_score(-1);
                    if_sound_on_play(&mut rm.sfx_stun);
                } else if self.iColorID == 1 {
                    player.score().adjust_score(5);
                    if_sound_on_play(&mut rm.sfx_extraguysound);
                }

                game_values.gamemode.check_winner(player);
            }

            eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_coinsparkle), self.ix, self.iy, 7, 4));
        }

        self.dead = true;
        false
    }
}

impl IO_OverMapObjectTrait for OMO_PipeCoin {
    crate::impl_over_map_object_plumbing!();
}
