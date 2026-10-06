//! Port of src/smw/objects/blocks/WeaponBreakableBlock.cpp

use crate::common::eyecandy::EC_FallingObject;
use crate::common::game_values::{game_values, if_sound_on_play};
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::{rm, Ptr};
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::mo_attack_zone::MO_AttackZone;
use crate::smw::objects::moving::mo_spin_attack::MO_SpinAttack;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum WeaponDamageType {
    #[default]
    Fireball,
    Feather,
    Shell,
    Bomb,
    Boomerang,
    Hammer,
    KuriboShoe,
    PWings,
    Star,
    Leaf,
}

crate::enum_from_u8!(WeaponDamageType, 10);

pub struct B_WeaponBreakableBlock {
    pub io_block: IO_Block,

    pub iType: WeaponDamageType,
    pub iDrawOffsetX: i16,
}
impl_base!(B_WeaponBreakableBlock => io_block: IO_Block);

impl B_WeaponBreakableBlock {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, r#type: WeaponDamageType) -> Self {
        let mut b = B_WeaponBreakableBlock { io_block: IO_Block::new(nspr, pos), iType: r#type, iDrawOffsetX: (r#type as i16) * 32 };
        b.iw = TILESIZE as i16;
        b.ih = TILESIZE as i16;
        b
    }

    pub fn r#type(&self) -> WeaponDamageType {
        self.iType
    }

    pub fn trigger_behavior_player(&mut self, iPlayerID: i16, iTeamID: i16) {
        if self.state == 0 {
            unsafe {
                let spr = Ptr::from_mut(&mut rm.spr_brokengrayblock);
                let (ix, iy) = (self.ix, self.iy);
                eyecandy[2].emplace(EC_FallingObject::new(spr, ix, iy, -2.2f32, -10.0f32, 4, 2, 0, 0, 16, 16));
                eyecandy[2].emplace(EC_FallingObject::new(spr, ix + 16, iy, 2.2f32, -10.0f32, 4, 2, 0, 0, 16, 16));
                eyecandy[2].emplace(EC_FallingObject::new(spr, ix, iy + 16, -2.2f32, -5.5f32, 4, 2, 0, 0, 16, 16));
                eyecandy[2].emplace(EC_FallingObject::new(spr, ix + 16, iy + 16, 2.2f32, -5.5f32, 4, 2, 0, 0, 16, 16));

                self.state = 1;
                if_sound_on_play(&mut rm.sfx_breakblock);
            }

            self.iBumpPlayerID = iPlayerID;
            self.iBumpTeamID = iTeamID;
        }
    }

    fn objecthitside(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        let r#type = object.get_moving_object_type();

        if self.iType == WeaponDamageType::Shell
            && ((r#type == movingobject_shell && object.state == 1)
                || r#type == movingobject_throwblock
                || (r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity()))
        {
            let mut iPlayerID: i16 = -1;
            let mut iTeamID: i16 = -1;
            if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox {
                iPlayerID = object.iPlayerID;
                iTeamID = object.iTeamID;
            }

            self.trigger_behavior_player(iPlayerID, iTeamID);
            return false;
        } else if self.iType == WeaponDamageType::Fireball && r#type == movingobject_fireball {
            let (iPlayerID, iTeamID) = (object.iPlayerID, object.iTeamID);
            self.trigger_behavior_player(iPlayerID, iTeamID);
            removeifprojectile(object, false, true);
            return false;
        } else if r#type == movingobject_attackzone {
            let any = object.as_any();
            let iStyle = if let Some(zone) = any.downcast_mut::<MO_AttackZone>() {
                zone.iStyle
            } else {
                any.downcast_mut::<MO_SpinAttack>().unwrap().mo_attack_zone.iStyle
            };

            if (iStyle == KillStyle::Leaf && self.iType == WeaponDamageType::Leaf) || (iStyle == KillStyle::Feather && self.iType == WeaponDamageType::Feather) {
                let (iPlayerID, iTeamID) = (object.iPlayerID, object.iTeamID);
                self.trigger_behavior_player(iPlayerID, iTeamID);
                object.die();
                return false;
            }
        }

        true
    }
}

impl CObjectTrait for B_WeaponBreakableBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        if self.state == 0 {
            self.spr.draw_src(
                self.ix as i32,
                self.iy as i32,
                &SDL_Rect { x: self.iDrawOffsetX as i32, y: 0, w: self.iw as i32, h: self.ih as i32 },
            );
        }
    }

    fn update(&mut self) {
        if self.state > 0 {
            if self.state == 1 {
                self.state = 2;
            } else if self.state == 2 {
                self.iBumpPlayerID = -1;
                self.dead = true;
                unsafe {
                    g_map.blockdata[self.col as usize][self.row as usize] = Ptr::null();
                    g_map.update_tile_gap(self.col, self.row);
                }
            }
        }
    }
}

impl IO_BlockTrait for B_WeaponBreakableBlock {
    crate::impl_io_block_plumbing!();

    fn hittop_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        io_block_hittop_player(self, player, useBehavior);

        unsafe {
            if self.state == 1 || self.state == 2 {
                let iKillType = PlayerKillType::NonKill;
                if self.iBumpPlayerID >= 0
                    && !player.is_invincible_on_bottom()
                    && (player.teamID != self.iBumpTeamID || game_values.teamcollision == TeamCollisionStyle::On)
                {
                    player_killed_player(self.iBumpPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Bounce, false, false);
                }

                if PlayerKillType::NonKill == iKillType {
                    player.vely = -VELNOTEBLOCKREPEL;
                }
            } else if useBehavior {
                player.vely = GRAVITATION;

                //Save this for when we create a super stomp destroyable block
                if self.iType == WeaponDamageType::KuriboShoe && player.is_super_stomping() && self.state == 0 {
                    self.trigger_behavior_player(player.globalID, player.teamID);
                    return false;
                } else if self.iType == WeaponDamageType::Star && player.is_invincible() {
                    self.trigger_behavior_player(player.globalID, player.teamID);
                    return false;
                }
            }
        }

        false
    }

    fn hitbottom_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            //If the player has a cape and they used it for a feather destroyable block, then kill it
            let mut fTriggerBlock = false;
            if self.iType == WeaponDamageType::Feather && player.powerup == 3 && player.extrajumps > 0 {
                fTriggerBlock = true;
            } else if self.iType == WeaponDamageType::PWings && player.powerup == 8 && player.flying {
                fTriggerBlock = true;
            } else if self.iType == WeaponDamageType::Star && player.is_invincible() {
                fTriggerBlock = true;
            }

            if fTriggerBlock {
                self.trigger_behavior_player(player.globalID, player.teamID);
            }

            return io_block_hitbottom_player(self, player, useBehavior);
        }

        false
    }

    fn hitleft_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            if self.iType == WeaponDamageType::Star && player.is_invincible() {
                self.trigger_behavior_player(player.globalID, player.teamID);
            }

            return io_block_hitleft_player(self, player, useBehavior);
        }

        false
    }

    fn hitright_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            if self.iType == WeaponDamageType::Star && player.is_invincible() {
                self.trigger_behavior_player(player.globalID, player.teamID);
            }

            return io_block_hitright_player(self, player, useBehavior);
        }

        false
    }

    fn hittop_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        io_block_hittop_object(self, object);

        if (self.state == 1 || self.state == 2) && object.bounce == GRAVITATION {
            io_block_bounce_moving_object(self, object);
            return false;
        }

        if self.state != 0 {
            return true;
        }

        let r#type = object.get_moving_object_type();

        if self.iType == WeaponDamageType::Fireball && r#type == movingobject_fireball {
            let (iPlayerID, iTeamID) = (object.iPlayerID, object.iTeamID);
            self.trigger_behavior_player(iPlayerID, iTeamID);
            removeifprojectile(object, false, true);
            return false;
        }

        true
    }

    fn hitbottom_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state != 0 {
            return true;
        }

        io_block_hitbottom_object(self, object);

        let r#type = object.get_moving_object_type();

        if self.iType == WeaponDamageType::Fireball && r#type == movingobject_fireball {
            let (iPlayerID, iTeamID) = (object.iPlayerID, object.iTeamID);
            self.trigger_behavior_player(iPlayerID, iTeamID);
            removeifprojectile(object, false, true);
            return false;
        }

        true
    }

    fn hitright_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state != 0 {
            return true;
        }

        io_block_hitright_object(self, object);

        self.objecthitside(object)
    }

    fn hitleft_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state != 0 {
            return true;
        }

        io_block_hitleft_object(self, object);

        self.objecthitside(object)
    }
}
