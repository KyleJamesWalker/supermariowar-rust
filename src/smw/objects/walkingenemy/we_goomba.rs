//! Port of src/smw/objects/walkingenemy/WE_Goomba.cpp

use crate::common::eyecandy::{EC_Corpse, EC_FallingObject};
use crate::common::game_mode::game_mode_stomp;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::moving_object_types::movingobject_goomba;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::walkingenemy::walking_enemy::*;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class goomba
//------------------------------------------------------------------------------
pub struct MO_Goomba {
    pub mo_walking_enemy: MO_WalkingEnemy,
}
impl_base!(MO_Goomba => mo_walking_enemy: MO_WalkingEnemy);

impl MO_Goomba {
    pub fn new(nspr: Ptr<gfxSprite>, moveToRight: bool, fBouncing: bool) -> Self {
        let mut o = MO_Goomba {
            mo_walking_enemy: MO_WalkingEnemy::new(nspr, 2, 8, 30, 20, 1, 11, 0, if moveToRight { 0 } else { 32 }, 32, 32, moveToRight, true, fBouncing, true),
        };
        o.movingObjectType = movingobject_goomba;
        o.iSpawnIconOffset = 64;
        o.killStyle = KillStyle::Goomba;

        if fBouncing {
            o.iw = 40;
            o.ih = 48;

            o.fOldY = o.fy - o.ih as f32;

            o.collisionOffsetX = 5;
            o.collisionOffsetY = 27;

            o.animationOffsetY = if moveToRight { 0 } else { o.ih };
        }
        o
    }
}

impl CObjectTrait for MO_Goomba {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        // if frozen, just draw shell, not entire koopa
        if self.frozen {
            unsafe {
                let x = self.ix as i32 - self.collisionOffsetX as i32 + self.iw as i32 - 32;
                let y = self.iy as i32 - self.collisionOffsetY as i32 + self.ih as i32 - 32;
                rm.spr_goomba.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: 32, h: 32 });
                rm.spr_iceblock.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: 32, h: 32 });
            }
        } else {
            mo_walking_enemy_draw(self);
        }
    }

    fn update(&mut self) {
        if self.velx < 0.0f32 {
            self.animationOffsetY = self.ih;
        } else {
            self.animationOffsetY = 0;
        }

        mo_walking_enemy_update(self);
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        mo_walking_enemy_collide_player(self, player)
    }

    fn collide_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) {
        mo_walking_enemy_collide_object(self, object)
    }
}

impl IO_MovingObjectTrait for MO_Goomba {
    crate::impl_io_moving_object_plumbing!();
    fn as_walking_enemy(&mut self) -> Option<&mut dyn MO_WalkingEnemyTrait> {
        Some(self)
    }

    fn die(&mut self) {
        if self.frozen {
            self.shatter_die();
            return;
        }

        self.dead = true;
        unsafe {
            eyecandy[2].emplace(EC_FallingObject::new(Ptr::from_mut(&mut rm.spr_goombadeadflying), self.ix, self.iy, 0.0f32, -VELJUMP / 2.0f32, 1, 0, 0, 0, 0, 0));
        }
    }
}

impl MO_WalkingEnemyTrait for MO_Goomba {
    crate::impl_walking_enemy_plumbing!();

    fn hittop_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        unsafe {
            player.set_yi((self.iy as i32 - PH - 1) as i16);
            player.bouncejump();
            let p = player;
            player.collisions.checktop(p.get());
            player.platform = Ptr::null();

            if game_values.gamemode.gamemode == game_mode_stomp && !game_values.gamemode.gameover {
                player.score().adjust_score(1);
            }

            if self.fBouncing {
                self.fBouncing = false;
                self.bounce = GRAVITATION;

                if self.vely < GRAVITATION {
                    self.vely = GRAVITATION;
                }

                self.iw = 32;
                self.ih = 32;

                self.collisionOffsetX = 1;
                self.collisionOffsetY = 11;

                self.animationWidth = 64;
                self.drawframe = 0;

                self.animationOffsetY = if self.velx > 0.0f32 { 0 } else { self.ih };
                self.spr = Ptr::from_mut(&mut rm.spr_goomba);
            } else {
                self.dead = true;

                let killStyle = self.killStyle;
                player.add_killer_award(Ptr::null(), killStyle);

                eyecandy[0].emplace(EC_Corpse::new(
                    Ptr::from_mut(&mut rm.spr_goombadead),
                    (self.ix as i32 - self.collisionOffsetX as i32) as f32,
                    (self.iy as i32 + self.collisionHeight as i32 - 32) as f32,
                    0,
                ));
            }

            if_sound_on_play(&mut rm.sfx_mip);
        }

        false
    }

    fn die_and_drop_shell(&mut self, _fBounce: bool, _fFlip: bool) {
        self.die();
    }
}
