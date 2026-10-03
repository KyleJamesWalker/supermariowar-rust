//! Port of src/smw/objects/powerup/PU_ExtraTimePowerup.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::common::game_mode::{game_mode_star, game_mode_timelimit};
use crate::common::game_values::if_sound_on_play;
use crate::smw::gamemodes::star::CGM_Star;
use crate::smw::gamemodes::time_limit::CGM_TimeLimit;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::powerup::powerup::{mo_powerup_draw, mo_powerup_update, MO_Powerup, MO_PowerupTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class special extra time powerup for timed or star mode
//------------------------------------------------------------------------------
pub struct PU_ExtraTimePowerup {
    pub mo_powerup: MO_Powerup,
}
impl_base!(PU_ExtraTimePowerup => mo_powerup: MO_Powerup);

impl PU_ExtraTimePowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s) -> Self {
        let mut o = PU_ExtraTimePowerup { mo_powerup: MO_Powerup::new(nspr, pos, 1, 0, 30, 30, 1, 1) };
        o.velx = 0.0;
        o
    }
}

impl CObjectTrait for PU_ExtraTimePowerup {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }
    fn draw(&mut self) {
        mo_powerup_draw(self)
    }
    fn update(&mut self) {
        mo_powerup_update(self)
    }
    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        unsafe {
            if self.state > 0 {
                if game_values.gamemode.gamemode == game_mode_timelimit || game_values.gamemode.gamemode == game_mode_star {
                    let mut gamemode = game_values.gamemode;
                    let timelimitmode: &mut CGM_TimeLimit = if gamemode.as_any().is::<CGM_Star>() {
                        gamemode.as_any().downcast_mut::<CGM_Star>().unwrap()
                    } else {
                        gamemode.as_any().downcast_mut::<CGM_TimeLimit>().unwrap()
                    };
                    let goal = timelimitmode.goal;
                    timelimitmode.addtime(goal / 5);
                }

                if_sound_on_play(&mut rm.sfx_collectpowerup);
                self.dead = true;
            }
        }

        false
    }
}

impl IO_MovingObjectTrait for PU_ExtraTimePowerup {
    crate::impl_io_moving_object_plumbing!();
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        Some(self)
    }
}

impl MO_PowerupTrait for PU_ExtraTimePowerup {
    crate::impl_powerup_plumbing!();
}
