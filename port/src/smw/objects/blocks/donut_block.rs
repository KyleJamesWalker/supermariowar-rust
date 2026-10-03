//! Port of src/smw/objects/blocks/DonutBlock.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::{g_map, g_tilesetmanager};
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

pub struct B_DonutBlock {
    pub io_block: IO_Block,

    pub counter: i16,
    pub jigglex: i16,
    pub jigglecounter: i16,
}
impl_base!(B_DonutBlock => io_block: IO_Block);

impl B_DonutBlock {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s) -> Self {
        B_DonutBlock { io_block: IO_Block::new(nspr, pos), counter: 0, jigglex: 0, jigglecounter: 0 }
    }

    pub fn trigger_behavior_player(&mut self, iPlayerId: i16) {
        //eyecandy[2].emplace<EC_FallingObject>(&rm->spr_donutblock, ix, iy, 0.0f, 0, 0, 0, 0);

        unsafe {
            let tile = TilesetTile { iID: g_tilesetmanager.classic_tileset_index() as i16, iCol: 29, iRow: 15 };

            let r#type = TileType::Solid;

            let path = Box::new(FallingPath::new(Vec2f::new(self.ix as f32 + 16.0f32, self.iy as f32 + 15.8f32)));
            let mut platform = MovingPlatform::new(vec![tile], vec![r#type], 1, 1, 2, path, false);
            platform.set_player_id(iPlayerId);

            g_map.add_temporary_platform(platform);

            self.dead = true;
            g_map.blockdata[self.col as usize][self.row as usize] = Ptr::null();
            g_map.update_tile_gap(self.col, self.row);
        }
    }
}

impl CObjectTrait for B_DonutBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self) {
        self.spr.draw(self.ix as i32 + self.jigglex as i32, self.iy as i32);
    }

    fn update(&mut self) {
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
            if self.counter > 50 {
                let iPlayerId = (self.state as i32 - 1) as i16;
                self.trigger_behavior_player(iPlayerId);
            }
        }

        self.state = 0;
    }
}

impl IO_BlockTrait for B_DonutBlock {
    crate::impl_io_block_plumbing!();

    fn hittop_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        io_block_hittop_player(self, player, useBehavior);

        if useBehavior {
            self.state = (player.globalID as i32 + 1) as i16;
        }

        false
    }
}
