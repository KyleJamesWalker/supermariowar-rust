//! Port of src/common/IO_Block.h

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{CObject, CObjectTrait};
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;
use std::ops::{Deref, DerefMut};

pub struct IO_Block {
    pub cobject: CObject,

    pub iBumpPlayerID: i16,
    pub iBumpTeamID: i16,

    pub fposx: f32,
    pub fposy: f32,
    pub iposx: i16,
    pub iposy: i16,

    pub hidden: bool,
    pub ishiddentype: bool,
    pub iHiddenTimer: i16,

    pub col: i16,
    pub row: i16,
}
impl_base!(IO_Block => cobject: CObject);

impl IO_Block {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s) -> Self {
        crate::smw::objects::blocks::io_block::io_block_new(nspr, pos)
    }
}

/// Virtual interface of `IO_Block`. Overloads: `collide(CPlayer*, short, bool)` is `collide_player_dir`,
/// `collide(IO_MovingObject*, short)` is `collide_object_dir`, `hittop(CPlayer*, bool)` is `hittop_player`,
/// `hittop(IO_MovingObject*)` is `hittop_object` (same for the other sides).
/// Default bodies live in `smw::objects::blocks::io_block` (src/smw/objects/blocks/IO_Block.cpp).
pub trait IO_BlockTrait: CObjectTrait {
    fn block(&self) -> &IO_Block;
    fn block_mut(&mut self) -> &mut IO_Block;
    fn as_block_ptr(&mut self) -> Ptr<dyn IO_BlockTrait>;

    fn reset(&mut self) {
        crate::smw::objects::blocks::io_block::io_block_reset(self)
    }

    fn collide_player_dir(&mut self, player: Ptr<CPlayer>, direction: i16, useBehavior: bool) -> bool {
        crate::smw::objects::blocks::io_block::io_block_collide_player_dir(self, player, direction, useBehavior)
    }
    fn collide_object_dir(&mut self, object: Ptr<dyn IO_MovingObjectTrait>, direction: i16) -> bool {
        crate::smw::objects::blocks::io_block::io_block_collide_object_dir(self, object, direction)
    }

    fn is_transparent(&mut self) -> bool {
        false
    }
    fn is_hidden(&mut self) -> bool {
        self.block().hidden
    }

    fn hittop_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        crate::smw::objects::blocks::io_block::io_block_hittop_player(self, player, useBehavior)
    }
    fn hitbottom_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        crate::smw::objects::blocks::io_block::io_block_hitbottom_player(self, player, useBehavior)
    }
    fn hitright_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        crate::smw::objects::blocks::io_block::io_block_hitright_player(self, player, useBehavior)
    }
    fn hitleft_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        crate::smw::objects::blocks::io_block::io_block_hitleft_player(self, player, useBehavior)
    }

    fn hittop_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        crate::smw::objects::blocks::io_block::io_block_hittop_object(self, object)
    }
    fn hitbottom_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        crate::smw::objects::blocks::io_block::io_block_hitbottom_object(self, object)
    }
    fn hitright_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        crate::smw::objects::blocks::io_block::io_block_hitright_object(self, object)
    }
    fn hitleft_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        crate::smw::objects::blocks::io_block::io_block_hitleft_object(self, object)
    }

    fn trigger_behavior(&mut self) {}
}

impl Deref for dyn IO_BlockTrait {
    type Target = IO_Block;
    #[inline(always)]
    fn deref(&self) -> &IO_Block {
        self.block()
    }
}

impl DerefMut for dyn IO_BlockTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut IO_Block {
        self.block_mut()
    }
}
