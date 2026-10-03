//! Port of src/smw/objects/blocks/ThrowBlock.cpp

use crate::common::eyecandy::EC_FallingObject;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::{rm, Ptr};
use crate::impl_base;
use crate::smw::gs_gameplay::{eyecandy, objectcontainer};
use crate::smw::objects::carriable::co_throw_block::CO_ThrowBlock;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::objects::throw_block_type::ThrowBlockType;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

pub struct B_ThrowBlock {
    pub io_block: IO_Block,

    pub iNumSprites: i16,
    pub animationSpeed: i16,
    pub drawFrame: i16,
    pub animationTimer: i16,
    pub animationWidth: i16,
    pub iType: ThrowBlockType,
}
impl_base!(B_ThrowBlock => io_block: IO_Block);

impl B_ThrowBlock {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, r#type: ThrowBlockType) -> Self {
        let mut b = B_ThrowBlock {
            io_block: IO_Block::new(nspr, pos),
            iNumSprites: 0,
            animationSpeed: 0,
            drawFrame: 0,
            animationTimer: 0,
            animationWidth: 0,
            iType: ThrowBlockType::Blue,
        };
        b.iw = 32;
        b.ih = 32;
        b.iNumSprites = iNumSpr;
        b.animationSpeed = aniSpeed;
        b.animationTimer = 0;
        b.drawFrame = 0;
        b.animationWidth = nspr.get_width() as i16;
        b.iType = r#type;
        b
    }

    pub fn give_block_to_player(&mut self, mut player: Ptr<CPlayer>) {
        unsafe {
            let mut block = Ptr::new_box(CO_ThrowBlock::new(Ptr::from_mut(&mut rm.spr_blueblock), Vec2s::new(self.ix, self.iy), self.iType));
            let item: Ptr<dyn MO_CarriedObjectTrait> = Ptr::from_mut(block.get() as &mut dyn MO_CarriedObjectTrait);
            if player.accept_item(item) {
                self.dead = true;
                g_map.blockdata[self.col as usize][self.row as usize] = Ptr::null();
                g_map.update_tile_gap(self.col, self.row);

                block.owner = player;
                block.iPlayerID = player.globalID;
                objectcontainer[1].add(block);
            } else {
                block.delete();
            }
        }
    }
}

impl CObjectTrait for B_ThrowBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        self.spr.draw_src(
            self.ix as i32,
            self.iy as i32,
            &SDL_Rect { x: self.drawFrame as i32, y: (self.iType as i16 * 32) as i32, w: self.iw as i32, h: self.ih as i32 },
        );
    }

    fn update(&mut self) {
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

impl IO_BlockTrait for B_ThrowBlock {
    crate::impl_io_block_plumbing!();

    fn hittop_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior {
            player.set_yf((self.iposy as i32 - PH) as f32 - 0.2f32);
            player.fOldY = player.fy;
            player.inair = false;
            player.fallthrough = false;
            player.killsinrowinair = 0;
            player.extrajumps = 0;
            player.vely = GRAVITATION;

            if player.pressed_accept_item_key() && player.is_accepting_item() {
                self.give_block_to_player(player);
                return true;
            } else {
                return false;
            }
        }

        false
    }

    fn hitright_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior {
            player.set_xf((self.iposx as i32 + self.iw as i32) as f32 + 0.2f32);
            player.fOldX = player.fx;

            if player.velx < 0.0f32 {
                player.velx = 0.0f32;
            }

            if player.oldvelx < 0.0f32 {
                player.oldvelx = 0.0f32;
            }

            if player.is_accepting_item() {
                self.give_block_to_player(player);
                return true;
            } else {
                return false;
            }
        }

        true
    }

    fn hitleft_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior {
            player.set_xf((self.iposx as i32 - PW) as f32 - 0.2f32);
            player.fOldX = player.fx;

            if player.velx > 0.0f32 {
                player.velx = 0.0f32;
            }

            if player.oldvelx > 0.0f32 {
                player.oldvelx = 0.0f32;
            }

            if player.is_accepting_item() {
                self.give_block_to_player(player);
                return true;
            } else {
                return false;
            }
        }

        true
    }

    fn trigger_behavior(&mut self) {
        unsafe {
            self.dead = true;
            g_map.blockdata[self.col as usize][self.row as usize] = Ptr::null();
            g_map.update_tile_gap(self.col, self.row);

            let srcY = (self.iType as i32 * 16) as i16;
            let spr = Ptr::from_mut(&mut rm.spr_brokenblueblock);
            let (ix, iy) = (self.ix, self.iy);
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix, iy, -1.5f32, -7.0f32, 6, 2, 0, srcY, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix + 16, iy, 1.5f32, -7.0f32, 6, 2, 0, srcY, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix, iy + 16, -1.5f32, -4.0f32, 6, 2, 0, srcY, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix + 16, iy + 16, 1.5f32, -4.0f32, 6, 2, 0, srcY, 16, 16));

            if_sound_on_play(&mut rm.sfx_breakblock);
        }
    }
}
