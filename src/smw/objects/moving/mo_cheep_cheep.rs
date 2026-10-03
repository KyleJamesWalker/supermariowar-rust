//! Port of src/smw/objects/moving/MO_CheepCheep.cpp

use crate::common::eyecandy::{EC_FallingObject, EC_SingleAnimation};
use crate::common::game::App;
use crate::common::game_mode::game_mode_stomp;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::carriable::co_shell::CO_Shell;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, CPlayer};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class cheep cheep
//------------------------------------------------------------------------------
pub struct MO_CheepCheep {
    pub io_moving_object: IO_MovingObject,

    pub iColorOffsetY: i16,
    pub frozen: bool,
}
impl_base!(MO_CheepCheep => io_moving_object: IO_MovingObject);

impl MO_CheepCheep {
    pub fn new(nspr: Ptr<gfxSprite>) -> Self {
        let mut o = MO_CheepCheep {
            io_moving_object: IO_MovingObject::new(nspr, Vec2s::new(0, App::screenHeight as i16), 2, 8, 30, 28, 1, 3, -1, -1, -1, -1),
            iColorOffsetY: 0,
            frozen: false,
        };

        o.ih = 32;
        o.set_xi(RANDOM_INT(608) as i16);

        o.velx = 0.0;
        while o.velx == 0.0 {
            o.velx = (RANDOM_INT(19) - 9) as f32 / 2.0f32;
        }

        // Cheep cheep up velocity is between 9.0 and 13.0 in 0.5 increments
        o.vely = -(RANDOM_INT(11) as f32 / 2.0f32) - 9.0f32;

        o.movingObjectType = movingobject_cheepcheep;
        o.state = 1;

        o.iColorOffsetY = ((RANDOM_INT(3) as i16) as i32 * 64) as i16;

        if o.velx > 0.0 {
            o.iColorOffsetY += 32;
        }

        o.fObjectCollidesWithMap = false;
        o.frozen = false;
        o
    }

    pub fn hittop(&mut self, player: Ptr<CPlayer>) -> bool {
        unsafe {
            let mut p = player;
            p.set_yi((self.iy as i32 - PH - 1) as i16);
            p.bouncejump();
            p.get().collisions.checktop(player.get());
            p.platform = Ptr::null();

            p.add_killer_award(Ptr::null(), KillStyle::CheepCheep);

            if game_values.gamemode.gamemode == game_mode_stomp && !game_values.gamemode.gameover {
                p.score().adjust_score(1);
            }

            if_sound_on_play(&mut rm.sfx_mip);
        }

        self.die();

        false
    }

    pub fn hitother(&mut self, mut player: Ptr<CPlayer>) -> bool {
        if player.is_shielded() {
            return false;
        }

        player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill
    }

    pub fn shatter_die(&mut self) {
        unsafe {
            if_sound_on_play(&mut rm.sfx_breakblock);
            self.dead = true;

            let iBrokenIceX: i16 = (self.ix as i32 - self.collisionOffsetX as i32) as i16;
            let iBrokenIceY: i16 = (self.iy as i32 - self.collisionOffsetY as i32) as i16;
            let spr = Ptr::from_mut(&mut rm.spr_brokeniceblock);
            eyecandy[2].emplace(EC_FallingObject::new(spr, iBrokenIceX, iBrokenIceY, -1.5f32, -7.0f32, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, (iBrokenIceX as i32 + 16) as i16, iBrokenIceY, 1.5f32, -7.0f32, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, iBrokenIceX, (iBrokenIceY as i32 + 16) as i16, -1.5f32, -4.0f32, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, (iBrokenIceX as i32 + 16) as i16, (iBrokenIceY as i32 + 16) as i16, 1.5f32, -4.0f32, 4, 2, 0, 0, 16, 16));

            game_values.unlocksecret2part2 += 1;
        }
    }
}

impl CObjectTrait for MO_CheepCheep {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        self.fOldX = self.fx;
        self.fOldY = self.fy;

        let fx = self.fx + self.velx;
        self.set_xf(fx);
        let fy = self.fy + self.vely;
        self.set_yf(fy);

        // Cheep cheep gravitation
        self.vely += 0.2f32;

        self.animate();

        // Remove if cheep cheep has fallen below bottom of screen
        if self.vely > 0.0 && self.iy as i32 > App::screenHeight {
            self.dead = true;
        }
    }

    fn draw(&mut self) {
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        self.spr.draw_src(x, y, &SDL_Rect { x: self.drawframe as i32, y: self.iColorOffsetY as i32, w: self.iw as i32, h: self.ih as i32 });

        if self.frozen {
            unsafe {
                rm.spr_iceblock.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: 32, h: 32 });
            }
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            if player.is_invincible() || self.frozen {
                player.add_killer_award(Ptr::null(), KillStyle::CheepCheep);

                if game_values.gamemode.gamemode == game_mode_stomp && !game_values.gamemode.gameover {
                    player.score().adjust_score(1);
                }

                if self.frozen {
                    self.shatter_die();
                } else {
                    if_sound_on_play(&mut rm.sfx_kicksound);
                    self.die();
                }
            } else {
                if player.fOldY + PH as f32 <= self.fOldY && player.iy as i32 + PH >= self.iy as i32 {
                    return self.hittop(player);
                } else {
                    return self.hitother(player);
                }
            }
        }

        false
    }

    fn collide_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        unsafe {
            if !object.is_dead() {
                removeifprojectile(object, false, false);

                let r#type: MovingObjectType = object.get_moving_object_type();

                if r#type == movingobject_fireball
                    || r#type == movingobject_hammer
                    || r#type == movingobject_boomerang
                    || r#type == movingobject_shell
                    || r#type == movingobject_throwblock
                    || r#type == movingobject_throwbox
                    || r#type == movingobject_bulletbill
                    || r#type == movingobject_podobo
                    || r#type == movingobject_attackzone
                    || r#type == movingobject_explosion
                    || r#type == movingobject_sledgehammer
                {
                    // Don't kill goombas with non-moving shells
                    if r#type == movingobject_shell && object.get_state() == 2 {
                        return;
                    }

                    if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                        return;
                    }

                    if game_values.gamemode.gamemode == game_mode_stomp && !game_values.gamemode.gameover {
                        // Find the player that shot this projectile so we can attribute a kill
                        let mut killer = get_player_from_global_id(object.iPlayerID);

                        if !killer.is_null() {
                            killer.add_killer_award(Ptr::null(), KillStyle::CheepCheep);
                            killer.score().adjust_score(1);

                            if r#type == movingobject_shell {
                                object.as_any().downcast_mut::<CO_Shell>().unwrap().add_moving_kill(killer);
                            }
                        }
                    }

                    if self.frozen {
                        self.shatter_die();
                    } else {
                        if_sound_on_play(&mut rm.sfx_kicksound);
                        self.die();
                    }

                    if r#type == movingobject_shell || r#type == movingobject_throwblock {
                        object.check_and_die();
                    } else if r#type == movingobject_bulletbill || r#type == movingobject_attackzone || r#type == movingobject_throwbox {
                        object.die();
                    }
                } else if r#type == movingobject_iceblast {
                    self.animationspeed = 0;
                    self.frozen = true;

                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (self.ix as i32 - self.collisionOffsetX as i32) as i16,
                        (self.iy as i32 - self.collisionOffsetY as i32) as i16,
                        3,
                        8,
                    ));
                }
            }
        }
    }
}

impl IO_MovingObjectTrait for MO_CheepCheep {
    crate::impl_io_moving_object_plumbing!();

    fn die(&mut self) {
        if self.frozen {
            self.shatter_die();
            return;
        }

        self.dead = true;
        unsafe {
            eyecandy[2].emplace(EC_FallingObject::new(
                Ptr::from_mut(&mut rm.spr_cheepcheepdead),
                self.ix,
                self.iy,
                0.0,
                -VELJUMP / 2.0f32,
                1,
                0,
                0,
                self.iColorOffsetY,
                32,
                32,
            ));
        }
    }
}
