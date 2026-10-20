//! Note block: springs players and objects away from whichever side they hit.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::CObjectTrait;
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::objectgame::check_secret;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;

/// Note block kind; the discriminant is the sprite row and the `superjumptype` given to players, so the order is fixed.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum NoteBlockType {
    #[default]
    Blue,
    Gray,
    Red,
}

crate::enum_from_u8!(NoteBlockType, 3);

/// Note block; `state` 1 and 2 are a sideways wobble, 3 and 4 a vertical one, each returning to 0.
#[derive(Clone)]

pub struct B_NoteBlock {
    pub io_block: IO_Block,

    /// Number of animation frames.
    pub iNumSprites: i16,
    /// Ticks per animation frame.
    pub animationSpeed: i16,
    /// X offset (pixels) of the current animation frame.
    pub drawFrame: i16,
    /// Tick counter toward the next animation frame.
    pub animationTimer: i16,
    /// Width (pixels) of the frame strip.
    pub animationWidth: i16,
    pub iType: NoteBlockType,
    /// Sprite row offset (pixels) for the kind.
    pub iTypeOffsetY: i16,
}
impl_base!(B_NoteBlock => io_block: IO_Block);

impl B_NoteBlock {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, r#type: NoteBlockType, fHidden: bool) -> Self {
        let mut b = B_NoteBlock {
            io_block: IO_Block::new(g, nspr, pos),
            iNumSprites: 0,
            animationSpeed: 0,
            drawFrame: 0,
            animationTimer: 0,
            animationWidth: 0,
            iType: NoteBlockType::Blue,
            iTypeOffsetY: 0,
        };
        b.iw = (g.rm.sprite(nspr).get_width() as i16) >> 2;
        b.ih = TILESIZE as i16;
        b.iNumSprites = iNumSpr;
        b.animationSpeed = aniSpeed;
        b.animationTimer = 0;
        b.drawFrame = 0;
        b.animationWidth = g.rm.sprite(b.spr).get_width() as i16;

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

    fn draw(&mut self, g: &mut Game) {
        if self.hidden {
            return;
        }

        g.rm.sprite(self.spr).draw_src(
            g,
            self.ix as i32,
            self.iy as i32,
            &Rect { x: self.drawFrame as i32, y: self.iTypeOffsetY as i32, w: self.iw as i32, h: self.ih as i32 },
        );
    }

    fn update(&mut self, g: &mut Game) {
        io_block_update(g, self);

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

    fn collide_player_dir(&mut self, g: &mut Game, player: PlayerId, direction: i16, useBehavior: bool) -> bool {
        if self.hidden {
            if (g.player(player).fOldY >= (self.iposy as i32 + self.ih as i32) as f32 || self.state > 1) && direction == 0 {
                return self.hitbottom_player(g, player, useBehavior);
            }

            return true;
        }

        io_block_collide_player_dir(g, self, player, direction, useBehavior)
    }

    fn hittop_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        let mut gp = g.ptr();
        io_block_hittop_player(g, self, player, useBehavior);

        if useBehavior {
            g.player(player).superjumptimer = 4;
            g.player(player).superjumptype = self.iType as i16;
            g.player(player).vely = -VELNOTEBLOCKREPEL;

            if self.state == 0 {
                self.vely = VELNOTEBLOCKBOUNCE;
                self.state = 3;
            }

            if_sound_on_play(g, &mut gp.rm.sfx_bump);

            let globalID = g.player(player).globalID as usize;
            g.game_values.unlocksecret3part2[globalID] += 2;
            check_secret(g, 2);
        }

        false
    }

    fn hitbottom_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        let mut gp = g.ptr();
        if useBehavior {
            g.player(player).set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);
            g.player(player).vely = VELNOTEBLOCKREPEL;

            if self.state == 0 {
                self.iBumpPlayerID = g.player(player).globalID;
                self.iBumpTeamID = g.player(player).teamID;

                self.vely = -VELNOTEBLOCKBOUNCE;
                self.state = 3;

                if_sound_on_play(g, &mut gp.rm.sfx_bump);
            }

            if self.hidden {
                self.hidden = false;
                io_block_kill_players_and_objects_inside_block(g, self, g.player(player).globalID);
            }

            g.update_tile_gap(self.col, self.row);
        }

        false
    }

    fn hitright_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        let mut gp = g.ptr();
        if useBehavior {
            g.player(player).set_xf((self.iposx as i32 + self.iw as i32) as f32 + 0.2f32);
            g.player(player).fOldX = g.player(player).fx;
            g.player(player).velx = VELNOTEBLOCKREPEL;
            g.player(player).oldvelx = VELNOTEBLOCKREPEL;

            if self.state == 0 {
                self.velx = -VELNOTEBLOCKBOUNCE;
                self.state = 1;
            }

            if_sound_on_play(g, &mut gp.rm.sfx_bump);
        }

        false
    }

    fn hitleft_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        let mut gp = g.ptr();
        if useBehavior {
            g.player(player).set_xf((self.iposx as i32 - PW) as f32 - 0.2f32);
            g.player(player).fOldX = g.player(player).fx;
            g.player(player).velx = -VELNOTEBLOCKREPEL;
            g.player(player).oldvelx = -VELNOTEBLOCKREPEL;

            if self.state == 0 {
                self.velx = VELNOTEBLOCKBOUNCE;
                self.state = 1;
            }

            if_sound_on_play(g, &mut gp.rm.sfx_bump);
        }

        false
    }

    fn collide_object_dir(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>, direction: i16) -> bool {
        if self.hidden {
            return true;
        }

        io_block_collide_object_dir(g, self, object, direction)
    }

    fn hittop_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        let ch = object.collisionHeight;
        object.set_yf((self.iposy as i32 - ch as i32) as f32 - 0.2f32);
        object.fOldY = object.fy;

        if self.state == 3 && object.bounce == GRAVITATION {
            io_block_bounce_moving_object(g, self, object);
            return false;
        } else {
            object.vely = object.bottom_bounce(g);
        }

        true
    }
}
