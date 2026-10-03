//! Port of src/smw/objects/blocks/NoteBlock.cpp

use crate::common::game_values::{game_values, if_sound_on_play};
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::{rm, Ptr};
use crate::impl_base;
use crate::smw::objectgame::check_secret;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

/// NOTE: The elements are ordered!
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum NoteBlockType {
    #[default]
    Blue,
    Gray,
    Red,
}

crate::enum_from_u8!(NoteBlockType, 3);

pub struct B_NoteBlock {
    pub io_block: IO_Block,

    pub iNumSprites: i16,
    pub animationSpeed: i16,
    pub drawFrame: i16,
    pub animationTimer: i16,
    pub animationWidth: i16,
    pub iType: NoteBlockType,
    pub iTypeOffsetY: i16,
}
impl_base!(B_NoteBlock => io_block: IO_Block);

impl B_NoteBlock {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, r#type: NoteBlockType, fHidden: bool) -> Self {
        let mut b = B_NoteBlock {
            io_block: IO_Block::new(nspr, pos),
            iNumSprites: 0,
            animationSpeed: 0,
            drawFrame: 0,
            animationTimer: 0,
            animationWidth: 0,
            iType: NoteBlockType::Blue,
            iTypeOffsetY: 0,
        };
        b.iw = (nspr.get_width() as i16) >> 2;
        b.ih = TILESIZE as i16;
        b.iNumSprites = iNumSpr;
        b.animationSpeed = aniSpeed;
        b.animationTimer = 0;
        b.drawFrame = 0;
        b.animationWidth = b.spr.get_width() as i16;

        b.hidden = fHidden;
        b.ishiddentype = fHidden;

        b.iType = r#type;
        b.iTypeOffsetY = (b.iType as i16) * 32;
        b
    }
}

impl CObjectTrait for B_NoteBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        if self.hidden {
            return;
        }

        self.spr.draw_src(
            self.ix as i32,
            self.iy as i32,
            &SDL_Rect { x: self.drawFrame as i32, y: self.iTypeOffsetY as i32, w: self.iw as i32, h: self.ih as i32 },
        );
    }

    fn update(&mut self) {
        io_block_update(self);

        if self.state > 0 {
            let fx = self.fx + self.velx;
            self.set_xf(fx);
            let fy = self.fy + self.vely;
            self.set_yf(fy);

            if self.state == 1 && (self.fposx - self.fx).abs() > 10.0f32 {
                self.velx = -self.velx;
                self.state = 2;
            } else if self.state == 2 && (self.fposx - self.fx).abs() < VELNOTEBLOCKBOUNCE {
                self.reset();
            } else if self.state == 3 && (self.fposy - self.fy).abs() > 10.0f32 {
                self.vely = -self.vely;
                self.state = 4;
                self.iBumpPlayerID = -1;
            } else if self.state == 4 && (self.fposy - self.fy).abs() < VELNOTEBLOCKBOUNCE {
                self.reset();
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

impl IO_BlockTrait for B_NoteBlock {
    crate::impl_io_block_plumbing!();

    fn reset(&mut self) {
        self.velx = 0.0f32;
        self.vely = 0.0f32;
        self.state = 0;
        let (fposx, fposy) = (self.fposx, self.fposy);
        self.set_xf(fposx);
        self.set_yf(fposy);
    }

    fn collide_player_dir(&mut self, player: Ptr<CPlayer>, direction: i16, useBehavior: bool) -> bool {
        if self.hidden {
            if (player.fOldY >= (self.iposy as i32 + self.ih as i32) as f32 || self.state > 1) && direction == 0 {
                return self.hitbottom_player(player, useBehavior);
            }

            return true;
        }

        io_block_collide_player_dir(self, player, direction, useBehavior)
    }

    fn hittop_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        io_block_hittop_player(self, player, useBehavior);

        if useBehavior {
            player.superjumptimer = 4;
            player.superjumptype = self.iType as i16;
            player.vely = -VELNOTEBLOCKREPEL;

            if self.state == 0 {
                self.vely = VELNOTEBLOCKBOUNCE;
                self.state = 3;
            }

            unsafe {
                if_sound_on_play(&mut rm.sfx_bump);

                game_values.unlocksecret3part2[player.globalID as usize] += 2;
                check_secret(2);
            }
        }

        false
    }

    fn hitbottom_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior {
            player.set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);
            player.vely = VELNOTEBLOCKREPEL;

            if self.state == 0 {
                self.iBumpPlayerID = player.globalID;
                self.iBumpTeamID = player.teamID;

                self.vely = -VELNOTEBLOCKBOUNCE;
                self.state = 3;

                unsafe {
                    if_sound_on_play(&mut rm.sfx_bump);
                }
            }

            if self.hidden {
                self.hidden = false;
                io_block_kill_players_and_objects_inside_block(self, player.globalID);
            }

            unsafe {
                g_map.update_tile_gap(self.col, self.row);
            }
        }

        false
    }

    fn hitright_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior {
            player.set_xf((self.iposx as i32 + self.iw as i32) as f32 + 0.2f32);
            player.fOldX = player.fx;
            player.velx = VELNOTEBLOCKREPEL;
            player.oldvelx = VELNOTEBLOCKREPEL;

            if self.state == 0 {
                self.velx = -VELNOTEBLOCKBOUNCE;
                self.state = 1;
            }

            unsafe {
                if_sound_on_play(&mut rm.sfx_bump);
            }
        }

        false
    }

    fn hitleft_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior {
            player.set_xf((self.iposx as i32 - PW) as f32 - 0.2f32);
            player.fOldX = player.fx;
            player.velx = -VELNOTEBLOCKREPEL;
            player.oldvelx = -VELNOTEBLOCKREPEL;

            if self.state == 0 {
                self.velx = VELNOTEBLOCKBOUNCE;
                self.state = 1;
            }

            unsafe {
                if_sound_on_play(&mut rm.sfx_bump);
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

        if self.state == 3 && object.bounce == GRAVITATION {
            io_block_bounce_moving_object(self, object);
            return false;
        } else {
            object.vely = object.bottom_bounce();
        }

        true
    }
}
