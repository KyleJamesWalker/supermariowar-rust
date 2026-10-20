//! Donut block: jiggles while stood on, then falls as a temporary platform.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::game::Game;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::{ticks, TILESETNONE};
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::map::TilesetTile;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_platform_paths::FallingPath;
use crate::common::movingplatform::MovingPlatform;
use crate::common::object_base::CObjectTrait;
use crate::common::tile_types::TileType;
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::player::CPlayer;

/// Block that falls after a player stands on it for `ticks(50)`.
#[derive(Clone)]

pub struct B_DonutBlock {
    pub io_block: IO_Block,

    /// Ticks stood on without a break.
    pub counter: i16,
    /// Horizontal draw offset (pixels) of the jiggle, +2 or -2.
    pub jigglex: i16,
    /// Tick counter between jiggle steps.
    pub jigglecounter: i16,
}
impl_base!(B_DonutBlock => io_block: IO_Block);

impl B_DonutBlock {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s) -> Self {
        B_DonutBlock { io_block: IO_Block::new(g, nspr, pos), counter: 0, jigglex: 0, jigglecounter: 0 }
    }

    /// Turns the block into a falling platform credited to `iPlayerId`.
    pub fn trigger_behavior_player(&mut self, g: &mut Game, iPlayerId: i16) {
        //The falling block uses this block's own graphic instead of a tileset tile
        let tile = TilesetTile { iID: TILESETNONE as i16, iCol: 0, iRow: 0 };

        let r#type = TileType::Solid;

        let path = Box::new(FallingPath::new(Vec2f::new(self.ix as f32 + 16.0f32, self.iy as f32 + 15.8f32)));
        let mut platform = MovingPlatform::new(g, vec![tile], vec![r#type], 1, 1, 2, path, false);
        platform.set_player_id(iPlayerId);
        let spr = self.spr;
        platform.paint_sprite_at(g.rm.sprite(spr), 0, 0);

        g.g_map.add_temporary_platform(platform);

        self.dead = true;
        g.g_map.blockdata[self.col as usize][self.row as usize] = crate::smw::object_container::ObjectId::NONE;
        g.update_tile_gap(self.col, self.row);
    }
}

impl CObjectTrait for B_DonutBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self, g: &mut Game) {
        g.rm.sprite(self.spr).draw(g, self.ix as i32 + self.jigglex as i32, self.iy as i32);
    }

    fn update(&mut self, g: &mut Game) {
        //If a player is standing on us, jiggle and then fall
        if self.state == 0 {
            self.counter = 0;
            self.jigglex = 0;
            self.jigglecounter = 0;
        } else {
            self.jigglecounter += 1;
            if self.jigglecounter > 1 {
                self.jigglecounter = 0;

                if self.jigglex == 2 {
                    self.jigglex = -2;
                } else {
                    self.jigglex = 2;
                }
            }

            self.counter += 1;
            if self.counter as i32 > ticks(50) {
                let iPlayerId = (self.state as i32 - 1) as i16;
                self.trigger_behavior_player(g, iPlayerId);
            }
        }

        self.state = 0;
    }
}

impl IO_BlockTrait for B_DonutBlock {
    crate::impl_io_block_plumbing!();

    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.block().harness_hash(h);
        h.i16s(&[self.counter, self.jigglex, self.jigglecounter]);
    }

    // `state` holds the standing player's global ID plus 1; `update` clears it every tick.
    fn hittop_player(&mut self, g: &mut Game, player: PlayerId, useBehavior: bool) -> bool {
        io_block_hittop_player(g, self, player, useBehavior);

        if useBehavior {
            self.state = (g.player(player).globalID as i32 + 1) as i16;
        }

        false
    }
}
