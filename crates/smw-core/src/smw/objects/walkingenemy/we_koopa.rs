//! Port of src/smw/objects/walkingenemy/WE_Koopa.cpp

use crate::game::Game;
use crate::common::eyecandy::EC_FallingObject;
use crate::common::game_mode::game_mode_stomp;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_koopa;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;

use crate::smw::objects::carriable::co_shell::{ShellType, CO_Shell};
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::walkingenemy::walking_enemy::*;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class koopa
//------------------------------------------------------------------------------
pub struct MO_Koopa {
    pub mo_walking_enemy: MO_WalkingEnemy,

    pub fRed: bool,
}
impl_base!(MO_Koopa => mo_walking_enemy: MO_WalkingEnemy);

impl MO_Koopa {
    pub fn new(nspr: Ptr<gfxSprite>, moveToRight: bool, red: bool, fBouncing: bool, bFallOffLedges: bool) -> Self {
        let mut o = MO_Koopa {
            mo_walking_enemy: MO_WalkingEnemy::new(nspr, 2, 8, 30, 28, 1, 25, 0, if moveToRight { 0 } else { 54 }, 54, 32, moveToRight, true, fBouncing, bFallOffLedges),
            fRed: false,
        };
        o.fRed = red;
        o.movingObjectType = movingobject_koopa;
        o.iSpawnIconOffset = if o.fRed { 144 } else { 112 };
        o.killStyle = KillStyle::Koopa;
        o
    }
}

impl CObjectTrait for MO_Koopa {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        // if frozen, just draw shell, not entire koopa
        if self.frozen {
            unsafe {
                let x = self.ix as i32 - self.collisionOffsetX as i32 + self.iw as i32 - 32;
                let y = self.iy as i32 - self.collisionOffsetY as i32 + self.ih as i32 - 32;
                gp.rm.spr_shell.draw_src(g, x, y, &SDL_Rect { x: 0, y: if self.fRed { 32 } else { 0 }, w: 32, h: 32 });
                gp.rm.spr_iceblock.draw_src(g, x, y, &SDL_Rect { x: 0, y: 0, w: 32, h: 32 });
            }
        } else {
            mo_walking_enemy_draw(g, self);
        }
    }

    fn update(&mut self, g: &mut Game) {
        if self.velx < 0.0f32 {
            self.animationOffsetY = 54;
        } else {
            self.animationOffsetY = 0;
        }

        mo_walking_enemy_update(g, self);
    }

    fn collide_player(&mut self, g: &mut Game, player: Ptr<CPlayer>) -> bool {
        mo_walking_enemy_collide_player(g, self, player)
    }

    fn collide_object(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) {
        mo_walking_enemy_collide_object(g, self, object)
    }
}

impl IO_MovingObjectTrait for MO_Koopa {
    crate::impl_io_moving_object_plumbing!();
    fn as_walking_enemy(&mut self) -> Option<&mut dyn MO_WalkingEnemyTrait> {
        Some(self)
    }

    fn die(&mut self, g: &mut Game) {
        if self.frozen {
            self.shatter_die(g);
            return;
        }

        self.dead = true;
        unsafe {
            g.eyecandy[2].emplace(EC_FallingObject::new(Ptr::from_mut(&mut g.rm.spr_shelldead), self.ix, self.iy, 0.0f32, -VELJUMP / 2.0f32, 1, 0, if self.fRed { 32 } else { 0 }, 0, 32, 32));
        }
    }
}

impl MO_WalkingEnemyTrait for MO_Koopa {
    crate::impl_walking_enemy_plumbing!();

    fn hittop_player(&mut self, g: &mut Game, player: Ptr<CPlayer>) -> bool {
        let mut gp = g.ptr();
        let mut player = player;
        unsafe {
            player.set_yi((self.iy as i32 - PH - 1) as i16);
            player.bouncejump();
            let p = player;
            player.collisions.checktop(g, p.get());
            player.platform = Ptr::null();

            if g.game_values.gamemode.gamemode == game_mode_stomp && !g.game_values.gamemode.gameover {
                player.score().adjust_score(g, 1);
            }

            if self.fBouncing {
                self.fBouncing = false;
                self.bounce = GRAVITATION;

                if self.vely < GRAVITATION {
                    self.vely = GRAVITATION;
                }

                self.spr = if self.fRed { Ptr::from_mut(&mut g.rm.spr_redkoopa) } else { Ptr::from_mut(&mut g.rm.spr_koopa) };
            } else {
                self.dead = true;

                player.add_killer_award(g, Ptr::null(), KillStyle::Koopa);

                self.drop_shell(g, false, false);
            }

            if_sound_on_play(g, &mut gp.rm.sfx_mip);
        }

        false
    }

    fn drop_shell(&mut self, g: &mut Game, fBounce: bool, fFlip: bool) {
        // Give the shell a state 2 so it is already spawned but sitting
        let pos = Vec2s::new((self.ix as i32 - 1) as i16, (self.iy as i32 + 8) as i16);
        let r#type = if self.fRed { ShellType::Red } else { ShellType::Green };
        let mut shell = Ptr::new_box(CO_Shell::new(g, r#type, pos, false, true, true, false));

        shell.nospawn((self.iy as i32 + 8) as i16, fBounce);

        if fFlip {
            shell.flip(g);
        }

        unsafe {
            g.objectcontainer[1].add(shell);
        }
    }
}
