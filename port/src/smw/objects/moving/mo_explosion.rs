//! Port of src/smw/objects/moving/MO_Explosion.cpp

use crate::common::game::App;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::blocks::weapon_breakable_block::{B_WeaponBreakableBlock, WeaponDamageType};
use crate::smw::objects::moving::moving_object::{io_moving_object_draw, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};

//------------------------------------------------------------------------------
// class explosion (for bob-omb mode)
//------------------------------------------------------------------------------
pub struct MO_Explosion {
    pub io_moving_object: IO_MovingObject,

    pub timer: i16,
    pub iStyle: KillStyle,
}
impl_base!(MO_Explosion => io_moving_object: IO_MovingObject);

impl MO_Explosion {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, playerid: i16, teamid: i16, style: KillStyle) -> Self {
        let mut o = MO_Explosion {
            io_moving_object: IO_MovingObject::new(nspr, pos, iNumSpr, aniSpeed, -1, -1, -1, -1, -1, -1, -1, -1),
            timer: 0,
            iStyle: KillStyle::Stomp,
        };

        o.state = 1;

        o.iPlayerID = playerid;
        o.iTeamID = teamid;
        o.timer = 0;
        o.movingObjectType = movingobject_explosion;
        o.iStyle = style;

        o.fObjectCollidesWithMap = false;
        o
    }
}

impl CObjectTrait for MO_Explosion {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        io_moving_object_draw(self)
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        unsafe {
            if player.globalID != self.iPlayerID
                && (game_values.teamcollision == TeamCollisionStyle::On || self.iTeamID != player.teamID)
                && !player.is_invincible()
                && !player.is_shielded()
                && !player.shyguy
            {
                // Find the player that made this explosion so we can attribute a kill
                player_killed_player(self.iPlayerID, player, PlayerDeathStyle::Jump, self.iStyle, false, false);
                return true;
            }
        }

        false
    }

    fn update(&mut self) {
        self.animate();

        // If this is the first frame, look for blocks to kill
        if self.timer == 0 {
            let mut iTestY: i16 = self.iy;

            for _iRow in 0..5i16 {
                let mut iTestX: i16 = self.ix;

                if iTestX < 0 {
                    iTestX = (iTestX as i32 + App::screenWidth) as i16;
                }

                if iTestY >= 0 && (iTestY as i32) < App::screenHeight {
                    let iTestRow: i16 = (iTestY as i32 / TILESIZE) as i16;
                    for _iCol in 0..7i16 {
                        let mut block = unsafe { g_map.block((iTestX as i32 / TILESIZE) as i16, iTestRow) };
                        if !block.is_null() {
                            if let Some(weaponbreakableblock) = block.as_any().downcast_mut::<B_WeaponBreakableBlock>() {
                                if weaponbreakableblock.r#type() == WeaponDamageType::Bomb {
                                    weaponbreakableblock.trigger_behavior_player(self.iPlayerID, self.iTeamID);
                                }
                            }
                        }

                        iTestX = (iTestX as i32 + TILESIZE) as i16;

                        if iTestX as i32 >= App::screenWidth {
                            iTestX = (iTestX as i32 - App::screenWidth) as i16;
                        }
                    }
                }

                iTestY = (iTestY as i32 + TILESIZE) as i16;

                if iTestY as i32 >= App::screenHeight {
                    break;
                }
            }
        }

        // RFC: why 48?
        self.timer += 1;
        if self.timer >= 48 {
            self.dead = true;
        }
    }
}

impl IO_MovingObjectTrait for MO_Explosion {
    crate::impl_io_moving_object_plumbing!();
}
