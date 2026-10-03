//! Port of src/smw/objects/blocks/BreakableBlock.cpp

use crate::common::eyecandy::EC_FallingObject;
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
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objectgame::check_secret;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class breakable block
//------------------------------------------------------------------------------
pub struct B_BreakableBlock {
    pub io_block: IO_Block,

    pub iNumSprites: i16,
    pub animationSpeed: i16,
    pub drawFrame: i16,
    pub animationTimer: i16,
    pub animationWidth: i16,
}
impl_base!(B_BreakableBlock => io_block: IO_Block);

impl B_BreakableBlock {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, aniSpeed: i16) -> Self {
        let mut b = B_BreakableBlock {
            io_block: IO_Block::new(nspr, pos),
            iNumSprites: 0,
            animationSpeed: 0,
            drawFrame: 0,
            animationTimer: 0,
            animationWidth: 0,
        };
        b.iw = (nspr.get_width() as i16) >> 2;
        b.iNumSprites = iNumSpr;
        b.animationSpeed = aniSpeed;
        b.animationTimer = 0;
        b.animationWidth = nspr.get_width() as i16;
        b.drawFrame = 0;
        b
    }
}

impl CObjectTrait for B_BreakableBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        if self.state == 0 {
            self.spr.draw_src(
                self.ix as i32,
                self.iy as i32,
                &SDL_Rect { x: self.drawFrame as i32, y: 0, w: self.iw as i32, h: self.ih as i32 },
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

        self.animationTimer += 1;
        if self.animationTimer >= self.animationSpeed {
            self.animationTimer = 0;

            self.drawFrame += self.iw;
            if self.drawFrame >= self.animationWidth {
                self.drawFrame = 0;
            }
        }
    }
}

impl IO_BlockTrait for B_BreakableBlock {
    crate::impl_io_block_plumbing!();

    fn hittop_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        io_block_hittop_player(self, player, useBehavior);

        unsafe {
            if self.state == 1 || self.state == 2 {
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

                if player.is_super_stomping() && self.state == 0 {
                    self.trigger_behavior();
                    return true;
                }
            }
        }

        false
    }

    fn hitbottom_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            self.trigger_behavior();

            //When breaking a block, you smash through with a small velocity, but this allows for breaking two blocks at once
            /*
            if (player->vely < -VELMAXBREAKBLOCK)
                player->vely = -VELMAXBREAKBLOCK;
            */
            player.vely = cap_falling_velocity(-player.vely * BOUNCESTRENGTH);
            player.set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);

            self.iBumpPlayerID = player.globalID;
            self.iBumpTeamID = player.teamID;
        }

        false
    }

    fn hittop_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        let ch = object.collisionHeight;
        object.set_yf((self.iposy as i32 - ch as i32) as f32 - 0.2f32);
        object.fOldY = object.fy;

        if self.state == 0 {
            let r#type = object.get_moving_object_type();
            if r#type == movingobject_throwbox && object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                self.trigger_behavior();
                object.vely = object.bottom_bounce();
                return true;
            }
        }

        if (self.state == 1 || self.state == 2) && object.bounce == GRAVITATION {
            io_block_bounce_moving_object(self, object);
            return false;
        } else {
            object.vely = object.bottom_bounce();
        }

        true
    }

    fn hitright_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            object.set_xf((self.iposx as i32 + self.iw as i32) as f32 + 0.2f32);
            object.fOldX = object.fx;

            if object.velx < 0.0f32 {
                object.velx = -object.velx;
            }

            let r#type = object.get_moving_object_type();
            if (r#type == movingobject_shell && object.state == 1)
                || r#type == movingobject_throwblock
                || (r#type == movingobject_throwbox && object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity())
                || r#type == movingobject_attackzone
            {
                self.trigger_behavior();
                return true;
            }
        }

        false
    }

    fn hitleft_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            let cw = object.collisionWidth;
            object.set_xf((self.iposx as i32 - cw as i32) as f32 - 0.2f32);
            object.fOldX = object.fx;

            if object.velx > 0.0f32 {
                object.velx = -object.velx;
            }

            let r#type = object.get_moving_object_type();
            if (r#type == movingobject_shell && object.state == 1)
                || r#type == movingobject_throwblock
                || (r#type == movingobject_throwbox && object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity())
                || r#type == movingobject_attackzone
            {
                self.trigger_behavior();
                return true;
            }
        }

        false
    }

    fn trigger_behavior(&mut self) {
        unsafe {
            if self.state == 0 {
                let spr = Ptr::from_mut(&mut rm.spr_brokenyellowblock);
                let (ix, iy) = (self.ix, self.iy);
                eyecandy[2].emplace(EC_FallingObject::new(spr, ix, iy, -2.2f32, -10.0f32, 4, 2, 0, 0, 16, 16));
                eyecandy[2].emplace(EC_FallingObject::new(spr, ix + 16, iy, 2.2f32, -10.0f32, 4, 2, 0, 0, 16, 16));
                eyecandy[2].emplace(EC_FallingObject::new(spr, ix, iy + 16, -2.2f32, -5.5f32, 4, 2, 0, 0, 16, 16));
                eyecandy[2].emplace(EC_FallingObject::new(spr, ix + 16, iy + 16, 2.2f32, -5.5f32, 4, 2, 0, 0, 16, 16));

                self.state = 1;
                if_sound_on_play(&mut rm.sfx_breakblock);
            }

            game_values.unlocksecret1part2 += 1;
            check_secret(0);
        }
    }
}
