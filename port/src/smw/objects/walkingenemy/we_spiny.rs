//! Port of src/smw/objects/walkingenemy/WE_Spiny.cpp

use crate::common::eyecandy::EC_FallingObject;
use crate::common::game_mode::game_mode_stomp;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_spiny;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::{eyecandy, objectcontainer};
use crate::smw::objects::carriable::co_shell::{ShellType, CO_Shell};
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::walkingenemy::walking_enemy::*;
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class spiny
//------------------------------------------------------------------------------
pub struct MO_Spiny {
    pub mo_walking_enemy: MO_WalkingEnemy,
}
impl_base!(MO_Spiny => mo_walking_enemy: MO_WalkingEnemy);

impl MO_Spiny {
    pub fn new(nspr: Ptr<gfxSprite>, moveToRight: bool) -> Self {
        let mut o = MO_Spiny {
            mo_walking_enemy: MO_WalkingEnemy::new(nspr, 2, 8, 30, 20, 1, 11, 0, if moveToRight { 0 } else { 32 }, 32, 32, moveToRight, true, false, true),
        };
        o.movingObjectType = movingobject_spiny;
        o.iSpawnIconOffset = 176;
        o.killStyle = KillStyle::Spiny;
        o
    }
}

impl CObjectTrait for MO_Spiny {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        mo_walking_enemy_draw(self);
    }

    fn update(&mut self) {
        if self.velx < 0.0f32 {
            self.animationOffsetY = 32;
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

impl IO_MovingObjectTrait for MO_Spiny {
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
            eyecandy[2].emplace(EC_FallingObject::new(Ptr::from_mut(&mut rm.spr_shelldead), self.ix, self.iy, 0.0f32, -VELJUMP / 2.0f32, 1, 0, 64, 0, 32, 32));
        }
    }
}

impl MO_WalkingEnemyTrait for MO_Spiny {
    crate::impl_walking_enemy_plumbing!();

    fn hittop_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        // Kill player here
        if player.isready() && !player.is_shielded() && !player.is_invincible() && !player.kuriboshoe.is_on() {
            return player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
        }

        if player.kuriboshoe.is_on() {
            unsafe {
                player.set_yi((self.iy as i32 - PH - 1) as i16);
                player.bouncejump();
                let p = player;
                player.collisions.checktop(p.get());
                player.platform = Ptr::null();

                self.dead = true;

                player.add_killer_award(Ptr::null(), KillStyle::Spiny);

                if game_values.gamemode.gamemode == game_mode_stomp && !game_values.gamemode.gameover {
                    player.score().adjust_score(1);
                }

                self.drop_shell(false, false);

                if_sound_on_play(&mut rm.sfx_mip);
            }
        }

        false
    }

    fn drop_shell(&mut self, fBounce: bool, fFlip: bool) {
        // Give the shell a state 2 so it is already spawned but sitting
        let mut shell = Ptr::new_box(CO_Shell::new(ShellType::Spiny, Vec2s::new((self.ix as i32 - 1) as i16, self.iy), false, true, false, false));
        shell.nospawn(self.iy, fBounce);

        if fFlip {
            shell.flip();
        }

        unsafe {
            objectcontainer[1].add(shell);
        }
    }
}
