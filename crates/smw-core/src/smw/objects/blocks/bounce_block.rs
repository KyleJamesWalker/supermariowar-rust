//! Port of src/smw/objects/blocks/BounceBlock.cpp

use crate::common::game_values::{game_values, if_sound_on_play};
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{cap_falling_velocity, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::{rm, Ptr};
use crate::impl_base;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};

pub struct B_BounceBlock {
    pub io_block: IO_Block,
}
impl_base!(B_BounceBlock => io_block: IO_Block);

impl B_BounceBlock {
    pub fn new(nspr1: Ptr<gfxSprite>, pos: Vec2s, fHidden: bool) -> Self {
        let mut b = B_BounceBlock { io_block: IO_Block::new(nspr1, pos) };
        b.hidden = fHidden;
        b.ishiddentype = fHidden;
        b
    }
}

impl CObjectTrait for B_BounceBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        if self.hidden {
            return;
        }

        io_block_draw(self);
    }

    fn update(&mut self) {
        io_block_update(self);

        if self.state > 0 {
            let fy = self.fy + self.vely;
            self.set_yf(fy);

            if self.state == 1 && (self.fposy - self.fy).abs() > 10.0f32 {
                self.iBumpPlayerID = -1;
                self.vely = -self.vely;
                self.state = 2;
            } else if self.state == 2 && (self.fposy - self.fy).abs() < VELBLOCKBOUNCE {
                self.reset();
            }
        }
    }
}

impl IO_BlockTrait for B_BounceBlock {
    crate::impl_io_block_plumbing!();

    fn reset(&mut self) {
        self.vely = 0.0f32;
        self.state = 0;
        let fposy = self.fposy;
        self.set_yf(fposy);
    }

    fn collide_player_dir(&mut self, player: Ptr<CPlayer>, direction: i16, useBehavior: bool) -> bool {
        if self.hidden {
            if player.fOldY >= (self.iposy as i32 + self.ih as i32) as f32 && direction == 0 {
                return self.hitbottom_player(player, useBehavior);
            }

            return true;
        }

        io_block_collide_player_dir(self, player, direction, useBehavior)
    }

    fn hittop_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        io_block_hittop_player(self, player, useBehavior);

        unsafe {
            if self.state == 1 {
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
            }
        }

        false
    }

    fn hitbottom_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        //Player bounces off
        if useBehavior {
            player.vely = cap_falling_velocity(-player.vely * BOUNCESTRENGTH);
            player.set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);

            self.iBumpPlayerID = player.globalID;
            self.iBumpTeamID = player.teamID;

            self.trigger_behavior();

            if self.hidden {
                self.hidden = false;
                let iBumpPlayerID = self.iBumpPlayerID;
                io_block_kill_players_and_objects_inside_block(self, iBumpPlayerID);
            }

            unsafe {
                g_map.update_tile_gap(self.col, self.row);
            }
        }

        false
    }

    fn collide_object_dir(&mut self, object: Ptr<dyn IO_MovingObjectTrait>, direction: i16) -> bool {
        if self.hidden {
            return true;
        }

        io_block_collide_object_dir(self, object, direction)
    }

    fn hittop_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        let ch = object.collisionHeight;
        object.set_yf((self.iposy as i32 - ch as i32) as f32 - 0.2f32);
        object.fOldY = object.fy;

        if self.state == 1 && object.bounce == GRAVITATION {
            io_block_bounce_moving_object(self, object);
            return false;
        } else {
            object.vely = object.bottom_bounce();
        }

        true
    }

    fn trigger_behavior(&mut self) {
        if self.state == 0 {
            self.vely = -VELBLOCKBOUNCE;
            self.state = 1;
            unsafe {
                if_sound_on_play(&mut rm.sfx_bump);
            }
        }
    }
}
