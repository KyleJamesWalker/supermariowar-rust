//! Port of src/smw/objects/blocks/OnOffSwitchBlock.cpp

use crate::common::game_values::{game_values, if_sound_on_play};
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{cap_falling_velocity, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::{rm, Ptr};
use crate::impl_base;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::blocks::switch_block::B_SwitchBlock;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::switch_color::SwitchColor;
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

pub struct B_OnOffSwitchBlock {
    pub io_block: IO_Block,

    pub m_color: SwitchColor,
    pub iSrcX: i16,
}
impl_base!(B_OnOffSwitchBlock => io_block: IO_Block);

impl B_OnOffSwitchBlock {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, color: SwitchColor, iState: i16) -> Self {
        let mut b = B_OnOffSwitchBlock { io_block: IO_Block::new(nspr, pos), m_color: SwitchColor::Red, iSrcX: 0 };
        b.iw = (b.spr.get_width() as i16) >> 2;
        b.collisionWidth = b.iw;
        b.ih = (b.spr.get_height() as i16) >> 2;
        b.collisionHeight = b.ih;

        b.m_color = color;
        b.iSrcX = (b.m_color as i16) * 32;

        b.state = if iState == 0 { 3 } else { 0 };
        b
    }

    pub fn flip_state(&mut self) {
        self.state = if self.state < 3 { self.state + 3 } else { self.state - 3 };
    }

    pub fn trigger_behavior_player(&mut self, playerID: i16) {
        if self.state == 0 || self.state == 3 {
            unsafe {
                if_sound_on_play(&mut rm.sfx_switchpress);
            }
            self.vely = -VELBLOCKBOUNCE;

            self.state += 1;

            let colorIdx = self.m_color as usize;

            unsafe {
                //Switch all the switch blocks and all the on/off blocks of the same color
                let mut i = 0;
                while i < g_map.switchBlocks[colorIdx].len() {
                    let mut block = g_map.switchBlocks[colorIdx][i];
                    block.as_any().downcast_mut::<B_OnOffSwitchBlock>().unwrap().flip_state();
                    i += 1;
                }

                //Switch all the switch blocks
                let mut i = 0;
                while i < g_map.switchBlocks[colorIdx + 4].len() {
                    let mut block = g_map.switchBlocks[colorIdx + 4][i];
                    block.as_any().downcast_mut::<B_SwitchBlock>().unwrap().flip_state(playerID);
                    i += 1;
                }
            }
        }
    }
}

impl CObjectTrait for B_OnOffSwitchBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn update(&mut self) {
        if self.state != 0 && self.state != 3 {
            let fy = self.fy + self.vely;
            self.set_yf(fy);

            if (self.state == 1 || self.state == 4) && (self.fposy - self.fy).abs() > 10.0f32 {
                self.iBumpPlayerID = -1;
                self.vely = -self.vely;
                self.state += 1;
            } else if (self.state == 2 || self.state == 5) && (self.fposy - self.fy).abs() < VELBLOCKBOUNCE {
                self.vely = 0.0f32;
                self.state -= 2;
                let fposy = self.fposy;
                self.set_yf(fposy);
            }
        }
    }

    fn draw(&mut self) {
        self.spr.draw_src(
            self.ix as i32,
            self.iy as i32,
            &SDL_Rect { x: self.iSrcX as i32, y: if self.state == 0 { 0 } else { 32 }, w: self.iw as i32, h: self.ih as i32 },
        );
    }
}

impl IO_BlockTrait for B_OnOffSwitchBlock {
    crate::impl_io_block_plumbing!();

    fn hittop_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        io_block_hittop_player(self, player, useBehavior);

        unsafe {
            if self.state == 1 || self.state == 4 {
                let mut iKillType = PlayerKillType::NonKill;
                if self.iBumpPlayerID >= 0
                    && !player.is_invincible_on_bottom()
                    && (player.teamID != self.iBumpTeamID || game_values.teamcollision == TeamCollisionStyle::On)
                {
                    iKillType = player_killed_player(self.iBumpPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Bounce, false, false);
                }

                if PlayerKillType::NonKill == iKillType {
                    player.vely = -VELNOTEBLOCKREPEL;
                }
            } else if useBehavior {
                player.vely = GRAVITATION;

                if (self.state == 0 || self.state == 3) && player.is_super_stomping() {
                    self.trigger_behavior_player(player.globalID);
                }
            }
        }

        false
    }

    fn hitbottom_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        //Player bounces off
        if useBehavior {
            player.vely = cap_falling_velocity(-player.vely * BOUNCESTRENGTH);
            player.set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);

            if self.state == 0 || self.state == 3 {
                self.iBumpPlayerID = player.globalID;
                self.iBumpTeamID = player.teamID;

                let iBumpPlayerID = self.iBumpPlayerID;
                self.trigger_behavior_player(iBumpPlayerID);
            }
        }

        false
    }

    fn hittop_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        let ch = object.collisionHeight;
        object.set_yf((self.iposy as i32 - ch as i32) as f32 - 0.2f32);
        object.fOldY = object.fy;

        let r#type = object.get_moving_object_type();
        if r#type == movingobject_throwbox && object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
            if self.state == 0 || self.state == 3 {
                self.iBumpPlayerID = -1;
                self.trigger_behavior_player(object.iPlayerID);
            }

            object.vely = object.bottom_bounce();

            return true;
        } else if (self.state == 1 || self.state == 4) && object.bounce == GRAVITATION {
            io_block_bounce_moving_object(self, object);
            return false;
        } else {
            object.vely = object.bottom_bounce();
        }

        true
    }

    fn hitright_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        //Object bounces off
        object.set_xf((self.iposx as i32 + self.iw as i32) as f32 + 0.2f32);
        object.fOldX = object.fx;

        if object.velx < 0.0f32 {
            object.velx = -object.velx;
        }

        let r#type = object.get_moving_object_type();
        if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox || r#type == movingobject_attackzone {
            if r#type == movingobject_shell {
                if object.state != 1 {
                    return false;
                }
            }

            if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                return false;
            }

            if self.state == 0 || self.state == 3 {
                self.iBumpPlayerID = -1;
                self.trigger_behavior_player(object.iPlayerID);
            } else {
                unsafe {
                    if_sound_on_play(&mut rm.sfx_bump);
                }
            }

            return true;
        }

        false
    }

    fn hitleft_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        //Object bounces off
        let cw = object.collisionWidth;
        object.set_xf((self.iposx as i32 - cw as i32) as f32 - 0.2f32);
        object.fOldX = object.fx;

        if object.velx > 0.0f32 {
            object.velx = -object.velx;
        }

        let r#type = object.get_moving_object_type();
        if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox || r#type == movingobject_attackzone {
            if r#type == movingobject_shell {
                if object.state != 1 {
                    return false;
                }
            }

            if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                return false;
            }

            if self.state == 0 || self.state == 3 {
                self.iBumpPlayerID = -1;
                self.trigger_behavior_player(object.iPlayerID);
            } else {
                unsafe {
                    if_sound_on_play(&mut rm.sfx_bump);
                }
            }

            return true;
        }

        false
    }
}
