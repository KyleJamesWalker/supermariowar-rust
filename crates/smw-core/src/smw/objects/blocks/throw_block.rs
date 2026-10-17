//! Port of src/smw/objects/blocks/ThrowBlock.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::EC_FallingObject;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::Ptr;
use crate::impl_base;

use crate::smw::objects::carriable::co_throw_block::CO_ThrowBlock;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::objects::throw_block_type::ThrowBlockType;
use crate::smw::player::CPlayer;

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
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, r#type: ThrowBlockType) -> Self {
        let mut b = B_ThrowBlock {
            io_block: IO_Block::new(g, nspr, pos),
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
        b.animationWidth = g.rm.sprite(nspr).get_width() as i16;
        b.iType = r#type;
        b
    }

    pub fn give_block_to_player(&mut self, g: &mut Game, mut player: PlayerId) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        let mut block = Ptr::new_box(CO_ThrowBlock::new(g, rm.spr_blueblock, Vec2s::new(self.ix, self.iy), self.iType));
        gp.objectcontainer[1].reserve(block);
        let item: Ptr<dyn MO_CarriedObjectTrait> = Ptr::from_mut(block.get() as &mut dyn MO_CarriedObjectTrait);
        if g.player(player).accept_item(item) {
            self.dead = true;
            g.g_map.blockdata[self.col as usize][self.row as usize] = crate::smw::object_container::ObjectId::NONE;
            g.update_tile_gap(self.col, self.row);

            block.owner = player;
            block.iPlayerID = g.player(player).globalID;
            gp.objectcontainer[1].add(g, block);
        } else {
            gp.objectcontainer[1].unreserve(block);
        }
    }
}

impl CObjectTrait for B_ThrowBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self, g: &mut Game) {
        g.rm.sprite(self.spr).draw_src(
            g,
            self.ix as i32,
            self.iy as i32,
            &Rect { x: self.drawFrame as i32, y: (self.iType as i16 * 32) as i32, w: self.iw as i32, h: self.ih as i32 },
        );
    }

    fn update(&mut self, g: &mut Game) {
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

    fn hittop_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior {
            g.player(player).set_yf((self.iposy as i32 - PH) as f32 - 0.2f32);
            g.player(player).fOldY = g.player(player).fy;
            g.player(player).inair = false;
            g.player(player).fallthrough = false;
            g.player(player).killsinrowinair = 0;
            g.player(player).extrajumps = 0;
            g.player(player).vely = GRAVITATION;

            if g.player(player).pressed_accept_item_key() && g.player(player).is_accepting_item() {
                self.give_block_to_player(g, player);
                return true;
            } else {
                return false;
            }
        }

        false
    }

    fn hitright_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior {
            g.player(player).set_xf((self.iposx as i32 + self.iw as i32) as f32 + 0.2f32);
            g.player(player).fOldX = g.player(player).fx;

            if g.player(player).velx < 0.0f32 {
                g.player(player).velx = 0.0f32;
            }

            if g.player(player).oldvelx < 0.0f32 {
                g.player(player).oldvelx = 0.0f32;
            }

            if g.player(player).is_accepting_item() {
                self.give_block_to_player(g, player);
                return true;
            } else {
                return false;
            }
        }

        true
    }

    fn hitleft_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior {
            g.player(player).set_xf((self.iposx as i32 - PW) as f32 - 0.2f32);
            g.player(player).fOldX = g.player(player).fx;

            if g.player(player).velx > 0.0f32 {
                g.player(player).velx = 0.0f32;
            }

            if g.player(player).oldvelx > 0.0f32 {
                g.player(player).oldvelx = 0.0f32;
            }

            if g.player(player).is_accepting_item() {
                self.give_block_to_player(g, player);
                return true;
            } else {
                return false;
            }
        }

        true
    }

    fn trigger_behavior(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.dead = true;
        g.g_map.blockdata[self.col as usize][self.row as usize] = crate::smw::object_container::ObjectId::NONE;
        g.update_tile_gap(self.col, self.row);

        let srcY = (self.iType as i32 * 16) as i16;
        let spr = g.rm.spr_brokenblueblock;
        let (ix, iy) = (self.ix, self.iy);
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix, iy, -1.5f32, -7.0f32, 6, 2, 0, srcY, 16, 16));
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix + 16, iy, 1.5f32, -7.0f32, 6, 2, 0, srcY, 16, 16));
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix, iy + 16, -1.5f32, -4.0f32, 6, 2, 0, srcY, 16, 16));
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix + 16, iy + 16, 1.5f32, -4.0f32, 6, 2, 0, srcY, 16, 16));

        if_sound_on_play(g, &mut gp.rm.sfx_breakblock);
    }
}
