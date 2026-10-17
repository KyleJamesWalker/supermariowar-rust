//! Port of src/smw/objects/blocks/WeaponBreakableBlock.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::EC_FallingObject;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::mo_attack_zone::MO_AttackZone;
use crate::smw::objects::moving::mo_spin_attack::MO_SpinAttack;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};

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

#[derive(Clone)]

pub struct B_WeaponBreakableBlock {
    pub io_block: IO_Block,

    pub iType: WeaponDamageType,
    pub iDrawOffsetX: i16,
}
impl_base!(B_WeaponBreakableBlock => io_block: IO_Block);

impl B_WeaponBreakableBlock {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, r#type: WeaponDamageType) -> Self {
        let mut b = B_WeaponBreakableBlock { io_block: IO_Block::new(g, nspr, pos), iType: r#type, iDrawOffsetX: (r#type as i16) * 32 };
        b.iw = TILESIZE as i16;
        b.ih = TILESIZE as i16;
        b
    }

    pub fn r#type(&self) -> WeaponDamageType {
        self.iType
    }

    pub fn trigger_behavior_player(&mut self, g: &mut Game, iPlayerID: i16, iTeamID: i16) {
        let mut gp = g.ptr();
        if self.state == 0 {
            let spr = g.rm.spr_brokengrayblock;
            let (ix, iy) = (self.ix, self.iy);
            g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix, iy, -2.2f32, -10.0f32, 4, 2, 0, 0, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix + 16, iy, 2.2f32, -10.0f32, 4, 2, 0, 0, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix, iy + 16, -2.2f32, -5.5f32, 4, 2, 0, 0, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix + 16, iy + 16, 2.2f32, -5.5f32, 4, 2, 0, 0, 16, 16));

            self.state = 1;
            if_sound_on_play(g, &mut gp.rm.sfx_breakblock);

            self.iBumpPlayerID = iPlayerID;
            self.iBumpTeamID = iTeamID;
        }
    }

    fn objecthitside(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
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

            self.trigger_behavior_player(g, iPlayerID, iTeamID);
            return false;
        } else if self.iType == WeaponDamageType::Fireball && r#type == movingobject_fireball {
            let (iPlayerID, iTeamID) = (object.iPlayerID, object.iTeamID);
            self.trigger_behavior_player(g, iPlayerID, iTeamID);
            removeifprojectile(g, object, false, true);
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
                self.trigger_behavior_player(g, iPlayerID, iTeamID);
                object.die(g);
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

    fn draw(&mut self, g: &mut Game) {
        if self.state == 0 {
            g.rm.sprite(self.spr).draw_src(
                g,
                self.ix as i32,
                self.iy as i32,
                &Rect { x: self.iDrawOffsetX as i32, y: 0, w: self.iw as i32, h: self.ih as i32 },
            );
        }
    }

    fn update(&mut self, g: &mut Game) {
        if self.state > 0 {
            if self.state == 1 {
                self.state = 2;
            } else if self.state == 2 {
                self.iBumpPlayerID = -1;
                self.dead = true;
                g.g_map.blockdata[self.col as usize][self.row as usize] = crate::smw::object_container::ObjectId::NONE;
                g.update_tile_gap(self.col, self.row);
            }
        }
    }
}

impl IO_BlockTrait for B_WeaponBreakableBlock {
    crate::impl_io_block_plumbing!();

    fn hittop_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        io_block_hittop_player(g, self, player, useBehavior);

        if self.state == 1 || self.state == 2 {
            let iKillType = PlayerKillType::NonKill;
            if self.iBumpPlayerID >= 0
                && !g.player(player).is_invincible_on_bottom()
                && (g.player(player).teamID != self.iBumpTeamID || g.game_values.teamcollision == TeamCollisionStyle::On)
            {
                player_killed_player(g, self.iBumpPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Bounce, false, false);
            }

            if PlayerKillType::NonKill == iKillType {
                g.player(player).vely = -VELNOTEBLOCKREPEL;
            }
        } else if useBehavior {
            g.player(player).vely = GRAVITATION;

            //Save this for when we create a super stomp destroyable block
            if self.iType == WeaponDamageType::KuriboShoe && g.player(player).is_super_stomping() && self.state == 0 {
                self.trigger_behavior_player(g, g.player(player).globalID, g.player(player).teamID);
                return false;
            } else if self.iType == WeaponDamageType::Star && g.player(player).is_invincible() {
                self.trigger_behavior_player(g, g.player(player).globalID, g.player(player).teamID);
                return false;
            }
        }

        false
    }

    fn hitbottom_player(&mut self, g: &mut Game, player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            //If the player has a cape and they used it for a feather destroyable block, then kill it
            let mut fTriggerBlock = false;
            if self.iType == WeaponDamageType::Feather && g.player(player).powerup == 3 && g.player(player).extrajumps > 0 {
                fTriggerBlock = true;
            } else if self.iType == WeaponDamageType::PWings && g.player(player).powerup == 8 && g.player(player).flying {
                fTriggerBlock = true;
            } else if self.iType == WeaponDamageType::Star && g.player(player).is_invincible() {
                fTriggerBlock = true;
            }

            if fTriggerBlock {
                self.trigger_behavior_player(g, g.player(player).globalID, g.player(player).teamID);
            }

            return io_block_hitbottom_player(g, self, player, useBehavior);
        }

        false
    }

    fn hitleft_player(&mut self, g: &mut Game, player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            if self.iType == WeaponDamageType::Star && g.player(player).is_invincible() {
                self.trigger_behavior_player(g, g.player(player).globalID, g.player(player).teamID);
            }

            return io_block_hitleft_player(g, self, player, useBehavior);
        }

        false
    }

    fn hitright_player(&mut self, g: &mut Game, player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            if self.iType == WeaponDamageType::Star && g.player(player).is_invincible() {
                self.trigger_behavior_player(g, g.player(player).globalID, g.player(player).teamID);
            }

            return io_block_hitright_player(g, self, player, useBehavior);
        }

        false
    }

    fn hittop_object(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        io_block_hittop_object(g, self, object);

        if (self.state == 1 || self.state == 2) && object.bounce == GRAVITATION {
            io_block_bounce_moving_object(g, self, object);
            return false;
        }

        if self.state != 0 {
            return true;
        }

        let r#type = object.get_moving_object_type();

        if self.iType == WeaponDamageType::Fireball && r#type == movingobject_fireball {
            let (iPlayerID, iTeamID) = (object.iPlayerID, object.iTeamID);
            self.trigger_behavior_player(g, iPlayerID, iTeamID);
            removeifprojectile(g, object, false, true);
            return false;
        }

        true
    }

    fn hitbottom_object(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state != 0 {
            return true;
        }

        io_block_hitbottom_object(self, object);

        let r#type = object.get_moving_object_type();

        if self.iType == WeaponDamageType::Fireball && r#type == movingobject_fireball {
            let (iPlayerID, iTeamID) = (object.iPlayerID, object.iTeamID);
            self.trigger_behavior_player(g, iPlayerID, iTeamID);
            removeifprojectile(g, object, false, true);
            return false;
        }

        true
    }

    fn hitright_object(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state != 0 {
            return true;
        }

        io_block_hitright_object(self, object);

        self.objecthitside(g, object)
    }

    fn hitleft_object(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state != 0 {
            return true;
        }

        io_block_hitleft_object(self, object);

        self.objecthitside(g, object)
    }
}
