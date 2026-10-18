//! Port of src/smw/objects/powerup/PU_PoisonPowerup.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::game::Game;
use crate::common::global_constants::vel;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::common::moving_object_types::movingobject_poisonpowerup;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class poison powerup
//------------------------------------------------------------------------------
#[derive(Clone)]
pub struct PU_PoisonPowerup {
    pub mo_powerup: MO_Powerup,
}
impl_base!(PU_PoisonPowerup => mo_powerup: MO_Powerup);

impl PU_PoisonPowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, iNumSpr: i16, moveToRight: bool, aniSpeed: i16, iCollisionWidth: i16, iCollisionHeight: i16, iCollisionOffsetX: i16, iCollisionOffsetY: i16) -> Self {
        let mut o = PU_PoisonPowerup { mo_powerup: MO_Powerup::new(g, nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY) };
        if moveToRight {
            o.velx = vel(2.0);
        } else {
            o.velx = vel(-2.0);
        }

        o.movingObjectType = movingobject_poisonpowerup;
        o
    }
}

impl CObjectTrait for PU_PoisonPowerup {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }
    fn draw(&mut self, g: &mut Game) {
        mo_powerup_draw(g, self)
    }
    fn update(&mut self, g: &mut Game) {
        mo_powerup_update(g, self)
    }
    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut player = player;
        if self.state > 0 {
            if g.player(player).is_invincible() || g.player(player).shyguy {
                self.dead = true;
                return false;
            }

            if g.player(player).is_shielded() {
                return false;
            }

            self.dead = true;

            return g.player(player).kill_player_map_hazard(g, false, KillStyle::PoisonMushroom, false, -1) != PlayerKillType::NonKill;
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_PoisonPowerup {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_PoisonPowerup {
    crate::impl_powerup_plumbing!();
}
