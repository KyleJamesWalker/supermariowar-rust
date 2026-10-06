//! Port of src/smw/objects/overmap/WO_PipeBonus.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::GRAVITATION;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::object_base::{object_pipe_bonus, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::main::pipegamemode;
use crate::smw::objects::overmap::over_map_object::{io_over_map_object_update, IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class pipe powerup (for coin pipe minigame)
//------------------------------------------------------------------------------
pub struct OMO_PipeBonus {
    pub io_over_map_object: IO_OverMapObject,

    pub iType: i16,
    pub iDuration: i16,
    pub iUncollectableTime: i16,
}
impl_base!(OMO_PipeBonus => io_over_map_object: IO_OverMapObject);

impl OMO_PipeBonus {
    pub fn new(nspr: Ptr<gfxSprite>, vel: Vec2f, pos: Vec2s, r#type: i16, duration: i16, uncollectabletime: i16) -> Self {
        let mut this = OMO_PipeBonus {
            io_over_map_object: IO_OverMapObject::new(nspr, pos, 4, 8, 30, 30, 1, 1, 0, ((r#type as i32) << 5) as i16, 32, 32),
            iType: r#type,
            iDuration: duration,
            iUncollectableTime: 0,
        };
        this.state = 1;
        this.objectType = object_pipe_bonus;

        this.velx = vel.x;

        unsafe {
            if pipegamemode.is_slowdown() {
                this.vely = vel.y / 1.5f32;
            } else {
                this.vely = vel.y;
            }
        }

        this.iUncollectableTime = uncollectabletime;
        this
    }

    pub fn get_type(&self) -> i16 {
        self.iType
    }
}

impl CObjectTrait for OMO_PipeBonus {
    crate::impl_cobject_plumbing!();

    fn update(&mut self) {
        io_over_map_object_update(self);

        if self.iy as i32 >= App::screenHeight {
            self.dead = true;
        }

        unsafe {
            if pipegamemode.is_slowdown() {
                self.vely += GRAVITATION / 2.0f32;
            } else {
                self.vely += GRAVITATION;
            }
        }

        if self.iUncollectableTime > 0 {
            self.iUncollectableTime -= 1;
        }
    }

    fn draw(&mut self) {
        let srcRect = SDL_Rect { x: self.drawframe as i32, y: self.animationOffsetY as i32, w: self.iw as i32, h: self.ih as i32 };
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        if self.iUncollectableTime > 0 {
            self.spr.draw_clip(x, y, &srcRect, ClipEdge::Bottom, 256);
        } else {
            self.spr.draw_src(x, y, &srcRect);
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if self.iUncollectableTime > 0 {
            return false;
        }

        unsafe {
            // fireball
            if self.iType == 5 {
                if !player.is_shielded() {
                    self.dead = true;
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (self.ix as i32 - 1) as i16,
                        (self.iy as i32 - 1) as i16,
                        3,
                        8,
                    ));

                    if !player.is_invincible() {
                        return player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
                    }
                }

                return false;
            } else if !game_values.gamemode.gameover {
                pipegamemode.set_bonus((self.iType as i32 + 1) as i16, self.iDuration, player.get_team_id());
            }
        }

        self.dead = true;
        false
    }
}

impl IO_OverMapObjectTrait for OMO_PipeBonus {
    crate::impl_over_map_object_plumbing!();
}
