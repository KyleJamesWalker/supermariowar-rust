//! Port of src/smw/objects/overmap/WO_Thwomp.cpp

use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_thwomp, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::overmap::over_map_object::{io_over_map_object_draw, io_over_map_object_update, IO_OverMapObject, IO_OverMapObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class thwomp (for thwomp mode)
//------------------------------------------------------------------------------
pub struct OMO_Thwomp {
    pub io_over_map_object: IO_OverMapObject,
}
impl_base!(OMO_Thwomp => io_over_map_object: IO_OverMapObject);

impl OMO_Thwomp {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, nspeed: f32) -> Self {
        let mut this = OMO_Thwomp {
            io_over_map_object: IO_OverMapObject::new(nspr, Vec2s::new(x, -nspr.get_height() as i16), 1, 0, -1, -1, -1, -1, -1, -1, -1, -1),
        };
        this.objectType = object_thwomp;
        this.vely = nspeed;
        this
    }
}

impl CObjectTrait for OMO_Thwomp {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self) {
        io_over_map_object_draw(self);
    }

    fn update(&mut self) {
        io_over_map_object_update(self);

        if self.iy as i32 > App::screenHeight - 1 {
            self.dead = true;
        }
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            if !player.is_invincible() && !player.is_shielded() && (player.score().score > 0 || game_values.gamemode.goal == -1) {
                return player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill;
            }
        }

        false
    }
}

impl IO_OverMapObjectTrait for OMO_Thwomp {
    crate::impl_over_map_object_plumbing!();
}
