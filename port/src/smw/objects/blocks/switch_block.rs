//! Port of src/smw/objects/blocks/SwitchBlock.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{cap_falling_velocity, CObjectTrait};
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::objects::switch_color::SwitchColor;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

pub struct B_SwitchBlock {
    pub io_block: IO_Block,

    pub iSrcX: i16,
}
impl_base!(B_SwitchBlock => io_block: IO_Block);

impl B_SwitchBlock {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, color: SwitchColor, iState: i16) -> Self {
        let mut b = B_SwitchBlock { io_block: IO_Block::new(nspr, pos), iSrcX: 0 };
        b.iw = (b.spr.get_width() as i16) >> 2;
        b.collisionWidth = b.iw;
        b.ih = (b.spr.get_height() as i16) >> 2;
        b.collisionHeight = b.ih;

        b.state = (1 - iState as i32) as i16;
        b.iSrcX = (color as i32 * 32) as i16;
        b
    }

    pub fn flip_state(&mut self, playerID: i16) {
        self.state = (1 - self.state as i32) as i16;
        unsafe {
            g_map.update_tile_gap(self.col, self.row);
        }

        if self.state == 0 {
            io_block_kill_players_and_objects_inside_block(self, playerID);
        }
    }
}

impl CObjectTrait for B_SwitchBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        self.spr.draw_src(
            self.ix as i32,
            self.iy as i32,
            &SDL_Rect { x: self.iSrcX as i32, y: if self.state == 0 { 64 } else { 96 }, w: self.iw as i32, h: self.ih as i32 },
        );
    }

    fn update(&mut self) {
        io_block_update(self);
    }
}

impl IO_BlockTrait for B_SwitchBlock {
    crate::impl_io_block_plumbing!();

    fn collide_player_dir(&mut self, player: Ptr<CPlayer>, direction: i16, useBehavior: bool) -> bool {
        if self.state != 0 || !useBehavior {
            return true;
        }

        let iposx = self.iposx as i32;
        let iposy = self.iposy as i32;
        let iw = self.iw as i32;
        let ih = self.ih as i32;

        if player.fOldY + PH as f32 <= iposy as f32 && direction == 2 {
            return self.hittop_player(player, useBehavior);
        } else if player.fOldY >= (iposy + ih) as f32 && direction == 0 {
            return self.hitbottom_player(player, useBehavior);
        } else if player.fOldX + PW as f32 <= iposx as f32 && direction == 1 {
            return self.hitleft_player(player, useBehavior);
        } else if player.fOldX >= (iposx + iw) as f32 && direction == 3 {
            return self.hitright_player(player, useBehavior);
        }

        true
    }

    fn is_transparent(&mut self) -> bool {
        self.state != 0
    }

    fn hittop_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        io_block_hittop_player(self, player, useBehavior);
        false
    }

    fn hitbottom_player(&mut self, mut player: Ptr<CPlayer>, _useBehavior: bool) -> bool {
        player.vely = cap_falling_velocity(-player.vely * BOUNCESTRENGTH);
        player.set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);
        self.vely = -VELBLOCKBOUNCE;
        false
    }

    fn hitright_player(&mut self, mut player: Ptr<CPlayer>, _useBehavior: bool) -> bool {
        player.set_xf((self.iposx as i32 + self.iw as i32) as f32 + 0.2f32);
        player.fOldX = player.fx;

        if player.velx < 0.0f32 {
            player.velx = 0.0f32;
        }

        if player.oldvelx < 0.0f32 {
            player.oldvelx = 0.0f32;
        }

        false
    }

    fn hitleft_player(&mut self, mut player: Ptr<CPlayer>, _useBehavior: bool) -> bool {
        player.set_xf((self.iposx as i32 - PW) as f32 - 0.2f32);
        player.fOldX = player.fx;

        if player.velx > 0.0f32 {
            player.velx = 0.0f32;
        }

        if player.oldvelx > 0.0f32 {
            player.oldvelx = 0.0f32;
        }

        false
    }

    fn hittop_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            let ch = object.collisionHeight;
            object.set_yf((self.iposy as i32 - ch as i32) as f32 - 0.2f32);
            object.fOldY = object.fy;
            object.vely = object.bottom_bounce();
        }

        true
    }

    fn hitbottom_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            object.set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);
            object.vely = -object.vely;
        }

        true
    }

    fn hitright_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            object.set_xf((self.iposx as i32 + self.iw as i32) as f32 + 0.2f32);

            if object.velx < 0.0f32 {
                object.velx = -object.velx;
            }
        }

        true
    }

    fn hitleft_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            let cw = object.collisionWidth;
            object.set_xf((self.iposx as i32 - cw as i32) as f32 - 0.2f32);

            if object.velx > 0.0f32 {
                object.velx = -object.velx;
            }
        }

        true
    }
}
