//! Port of src/smw/objects/walkingenemy/WE_Spiny.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::game::Game;
use crate::common::map::PlatformId;
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

use crate::smw::objects::carriable::co_shell::{ShellType, CO_Shell};
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::walkingenemy::walking_enemy::*;
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class spiny
//------------------------------------------------------------------------------
#[derive(Clone)]
pub struct MO_Spiny {
    pub mo_walking_enemy: MO_WalkingEnemy,
}
impl_base!(MO_Spiny => mo_walking_enemy: MO_WalkingEnemy);

impl MO_Spiny {
    pub fn new(g: &mut Game, nspr: SpriteId, moveToRight: bool) -> Self {
        let mut gp = g.ptr();
        let mut o = MO_Spiny {
            mo_walking_enemy: MO_WalkingEnemy::new(gp.get(), nspr, 2, 8, 30, 20, 1, 11, 0, if moveToRight { 0 } else { 32 }, 32, 32, moveToRight, true, false, true),
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

    fn draw(&mut self, g: &mut Game) {
        mo_walking_enemy_draw(g, self);
    }

    fn update(&mut self, g: &mut Game) {
        if self.velx < 0.0f32 {
            self.animationOffsetY = 32;
        } else {
            self.animationOffsetY = 0;
        }

        mo_walking_enemy_update(g, self);
    }

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        mo_walking_enemy_collide_player(g, self, player)
    }

    fn collide_object(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) {
        mo_walking_enemy_collide_object(g, self, object)
    }
}

impl IO_MovingObjectTrait for MO_Spiny {
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
        g.eyecandy[2].emplace(EC_FallingObject::new(&g.rm, g.rm.spr_shelldead, self.ix, self.iy, 0.0f32, -VELJUMP / 2.0f32, 1, 0, 64, 0, 32, 32));
    }
}

impl MO_WalkingEnemyTrait for MO_Spiny {
    crate::impl_walking_enemy_plumbing!();

    fn hittop_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut gp = g.ptr();
        let mut player = player;
        // Kill player here
        if g.player(player).isready() && !g.player(player).is_shielded() && !g.player(player).is_invincible() && !g.player(player).kuriboshoe.is_on() {
            return g.player(player).kill_player_map_hazard(g, false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
        }

        if g.player(player).kuriboshoe.is_on() {
            g.player(player).set_yi((self.iy as i32 - PH - 1) as i16);
            g.player(player).bouncejump(g);
            let p = player;
            g.player(player).collisions.checktop(g, g.player(p));
            g.player(player).platform = PlatformId::NONE;

            self.dead = true;

            g.player(player).add_killer_award(g, PlayerId::NONE, KillStyle::Spiny);

            if g.mode().gamemode == game_mode_stomp && !g.mode().gameover {
                g.player(player).score(g).adjust_score(g, 1);
            }

            self.drop_shell(g, false, false);

            if_sound_on_play(g, &mut gp.rm.sfx_mip);
        }

        false
    }

    fn drop_shell(&mut self, g: &mut Game, fBounce: bool, fFlip: bool) {
        let mut gp = g.ptr();
        // Give the shell a state 2 so it is already spawned but sitting
        let mut shell = Ptr::new_box(CO_Shell::new(g, ShellType::Spiny, Vec2s::new((self.ix as i32 - 1) as i16, self.iy), false, true, false, false));
        shell.nospawn(self.iy, fBounce);

        if fFlip {
            shell.flip(g);
        }

        gp.objectcontainer[1].add(g, shell);
    }
}
