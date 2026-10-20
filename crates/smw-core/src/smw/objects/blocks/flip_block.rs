//! Flip block: spins and lets players pass for a while when hit from below; shells and super stomps break it.

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
use crate::common::moving_object_types::*;
use crate::common::object_base::{cap_falling_velocity, CObjectTrait};
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class flip block
// Trap: a player inside a flip block when it stops spinning stays there, half protected, until the block below is hit.
//------------------------------------------------------------------------------
/// Flip block whose `state` runs 0 solid, 1 spinning (passable), 2 broken, 3 removed.
#[derive(Clone)]
pub struct B_FlipBlock {
    pub io_block: IO_Block,

    /// Tick counter toward the next spin frame.
    pub counter: i16,
    /// X offset (pixels) of the current spin frame.
    pub frame: i16,
    /// Ticks spinning; the block turns solid again at `ticks(240)`.
    pub timer: i16,
    /// Width (pixels) of the frame strip.
    pub animationWidth: i16,
}
impl_base!(B_FlipBlock => io_block: IO_Block);

impl B_FlipBlock {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, fHidden: bool) -> Self {
        let mut b = B_FlipBlock { io_block: IO_Block::new(g, nspr, pos), counter: 0, frame: 0, timer: 0, animationWidth: 0 };
        b.iw = (g.rm.sprite(b.spr).get_width() as i16) >> 2;
        b.collisionWidth = b.iw;

        b.hidden = fHidden;
        b.ishiddentype = fHidden;

        b.counter = 0;
        b.frame = 0;
        b.timer = 0;
        b.animationWidth = g.rm.sprite(b.spr).get_width() as i16;
        b
    }

    /// Spawns the broken-block debris and sound.
    fn explode(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let spr = g.rm.spr_brokenflipblock;
        let (ix, iy) = (self.ix, self.iy);
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix, iy, vel(-2.2), vel(-10.0), 4, 2, 0, 0, 16, 16));
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix + 16, iy, vel(2.2), vel(-10.0), 4, 2, 0, 0, 16, 16));
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix, iy + 16, vel(-2.2), vel(-5.5), 4, 2, 0, 0, 16, 16));
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix + 16, iy + 16, vel(2.2), vel(-5.5), 4, 2, 0, 0, 16, 16));

        if_sound_on_play(g, &mut gp.rm.sfx_breakblock);
    }

    /// A moving shell, thrown block or fast box breaks the block; an attack zone starts it spinning.
    fn object_hit_side(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        let mut object = object;
        let r#type = object.get_moving_object_type();
        if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox {
            if r#type == movingobject_shell {
                if object.state != 1 {
                    return false;
                }
            }

            if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                return false;
            }

            self.state = 2;
            self.explode(g);
        } else if r#type == movingobject_attackzone {
            self.iBumpPlayerID = object.iPlayerID;
            self.iBumpTeamID = object.iTeamID;

            if self.hidden {
                self.hidden = false;
                let iBumpPlayerID = self.iBumpPlayerID;
                io_block_kill_players_and_objects_inside_block(g, self, iBumpPlayerID);
            }

            g.update_tile_gap(self.col, self.row);

            self.trigger_behavior(g);
            return false;
        }

        true
    }
}

impl CObjectTrait for B_FlipBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self, g: &mut Game) {
        if self.hidden {
            return;
        }

        if self.state == 0 || self.state == 1 {
            g.rm.sprite(self.spr).draw_src(
                g,
                self.ix as i32,
                self.iy as i32,
                &Rect { x: self.frame as i32, y: 0, w: self.iw as i32, h: self.ih as i32 },
            );
        }
    }

    fn update(&mut self, g: &mut Game) {
        io_block_update(g, self);

        if self.state == 1 {
            self.counter += 1;
            if self.counter as i32 >= ticks(10) {
                self.counter = 0;
                self.frame += self.iw;

                if self.frame >= self.animationWidth {
                    self.frame = 0;
                }
            }

            self.timer += 1;
            if self.timer as i32 >= ticks(240) {
                self.reset();
                g.update_tile_gap(self.col, self.row);

                let iBumpPlayerID = self.iBumpPlayerID;
                io_block_kill_players_and_objects_inside_block(g, self, iBumpPlayerID);
                self.iBumpPlayerID = -1;
            }
        } else if self.state == 2 {
            self.state = 3;
        } else if self.state == 3 {
            self.dead = true;
            g.g_map.blockdata[self.col as usize][self.row as usize] = crate::smw::object_container::ObjectId::NONE;
            g.update_tile_gap(self.col, self.row);
        }
    }
}

impl IO_BlockTrait for B_FlipBlock {
    crate::impl_io_block_plumbing!();

    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.block().harness_hash(h);
        h.i16s(&[self.counter, self.frame, self.timer]);
    }

    fn reset(&mut self) {
        self.frame = 0;
        self.counter = 0;
        self.timer = 0;
        self.state = 0;
    }

    fn collide_player_dir(&mut self, g: &mut Game, player: PlayerId, direction: i16, useBehavior: bool) -> bool {
        let iposx = self.iposx as i32;
        let iposy = self.iposy as i32;
        let iw = self.iw as i32;
        let ih = self.ih as i32;

        if self.hidden {
            if g.player(player).fOldY >= (iposy + ih) as f32 && direction == 0 {
                return self.hitbottom_player(g, player, useBehavior);
            }

            return true;
        }

        if (g.player(player).fOldY + PH as f32 <= iposy as f32 || self.state > 1) && direction == 2 {
            return self.hittop_player(g, player, useBehavior);
        } else if (g.player(player).fOldY >= (iposy + ih) as f32 || self.state > 1) && direction == 0 {
            return self.hitbottom_player(g, player, useBehavior);
        } else if (g.player(player).fOldX + PW as f32 <= iposx as f32 || self.state > 1) && direction == 1 {
            return self.hitleft_player(g, player, useBehavior);
        } else if (g.player(player).fOldX >= (iposx + iw) as f32 || self.state > 1) && direction == 3 {
            return self.hitright_player(g, player, useBehavior);
        }

        true
    }

    fn is_transparent(&mut self) -> bool {
        self.state == 1
    }

    fn hittop_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        if self.state == 2 || self.state == 3 {
            io_block_hittop_player(g, self, player, useBehavior);

            g.player(player).vely = -VELNOTEBLOCKREPEL;
            return false;
        } else if self.state == 0 {
            io_block_hittop_player(g, self, player, useBehavior);

            if g.player(player).is_super_stomping() {
                self.state = 2;
                self.explode(g);
                return true;
            }

            return false;
        }

        true
    }

    fn hitbottom_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            g.player(player).vely = cap_falling_velocity(-g.player(player).vely * BOUNCESTRENGTH);
            g.player(player).set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);

            self.iBumpPlayerID = g.player(player).globalID;
            self.iBumpTeamID = g.player(player).teamID;

            if self.hidden {
                self.hidden = false;
                let iBumpPlayerID = self.iBumpPlayerID;
                io_block_kill_players_and_objects_inside_block(g, self, iBumpPlayerID);
            }

            g.update_tile_gap(self.col, self.row);

            self.trigger_behavior(g);
            return false;
        }

        true
    }

    fn hitright_player(&mut self, g: &mut Game, player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            io_block_hitright_player(g, self, player, useBehavior);
        }

        true
    }

    fn hitleft_player(&mut self, g: &mut Game, player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            io_block_hitleft_player(g, self, player, useBehavior);
        }

        true
    }

    fn collide_object_dir(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>, direction: i16) -> bool {
        if self.hidden {
            return true;
        }

        io_block_collide_object_dir(g, self, object, direction)
    }

    fn hittop_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            let ch = object.collisionHeight;
            object.set_yf((self.iposy as i32 - ch as i32) as f32 - 0.2f32);
            object.fOldY = object.fy;
            object.vely = object.bottom_bounce(g);
        }

        true
    }

    fn hitbottom_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            object.set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);
            object.vely = -object.vely;
        }

        true
    }

    fn hitright_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            object.set_xf((self.iposx as i32 + self.iw as i32) as f32 + 0.2f32);
            object.fOldX = object.fx;

            if object.velx < 0.0f32 {
                object.velx = -object.velx;
            }

            return self.object_hit_side(g, object);
        }

        true
    }

    fn hitleft_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            let cw = object.collisionWidth;
            object.set_xf((self.iposx as i32 - cw as i32) as f32 - 0.2f32);
            object.fOldX = object.fx;

            if object.velx > 0.0f32 {
                object.velx = -object.velx;
            }

            return self.object_hit_side(g, object);
        }

        true
    }

    fn trigger_behavior(&mut self, g: &mut Game) {
        if self.state == 0 {
            self.state = 1;
            self.frame = self.iw;

            g.update_tile_gap(self.col, self.row);
        }
    }
}
