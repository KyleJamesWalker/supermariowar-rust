//! Port of src/smw/objects/blocks/FlipBlock.cpp

use crate::common::eyecandy::EC_FallingObject;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{cap_falling_velocity, CObjectTrait};
use crate::globals::{rm, Ptr};
use crate::impl_base;
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class flip block
// Known Bug:  If you have two flip blocks vertically aligned like this:
//
// ['']  or  ['']['']  etc.
// ['']      ['']['']
//
// And you hit both of them from the bottom, then keep jumping so that the bottom one
// stops to support you, and then the top one stops spinning, you can "hide" under the
// top block.  I was thinking about fixing this, but I don't know what the correct
// behavior should be if you're in a flip block when it stops spinning.  Plus, it is kind
// of cool to hide under these blocks.  You are somewhat invincible, but someone can come
// along and hit your support out from under you and you'll fall out and if they have star
// power, you're dead.  Plus fireballs have a large enough collision box that you can be hit
// with them too.
//------------------------------------------------------------------------------
pub struct B_FlipBlock {
    pub io_block: IO_Block,

    pub counter: i16,
    pub frame: i16,
    pub timer: i16,
    pub animationWidth: i16,
}
impl_base!(B_FlipBlock => io_block: IO_Block);

impl B_FlipBlock {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, fHidden: bool) -> Self {
        let mut b = B_FlipBlock { io_block: IO_Block::new(nspr, pos), counter: 0, frame: 0, timer: 0, animationWidth: 0 };
        b.iw = (b.spr.get_width() as i16) >> 2;
        b.collisionWidth = b.iw;

        b.hidden = fHidden;
        b.ishiddentype = fHidden;

        b.counter = 0;
        b.frame = 0;
        b.timer = 0;
        b.animationWidth = b.spr.get_width() as i16;
        b
    }

    fn explode(&mut self) {
        unsafe {
            let spr = Ptr::from_mut(&mut rm.spr_brokenflipblock);
            let (ix, iy) = (self.ix, self.iy);
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix, iy, -2.2f32, -10.0f32, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix + 16, iy, 2.2f32, -10.0f32, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix, iy + 16, -2.2f32, -5.5f32, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix + 16, iy + 16, 2.2f32, -5.5f32, 4, 2, 0, 0, 16, 16));

            if_sound_on_play(&mut rm.sfx_breakblock);
        }
    }

    fn object_hit_side(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
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
            self.explode();
        } else if r#type == movingobject_attackzone {
            self.iBumpPlayerID = object.iPlayerID;
            self.iBumpTeamID = object.iTeamID;

            if self.hidden {
                self.hidden = false;
                let iBumpPlayerID = self.iBumpPlayerID;
                io_block_kill_players_and_objects_inside_block(self, iBumpPlayerID);
            }

            unsafe {
                g_map.update_tile_gap(self.col, self.row);
            }

            self.trigger_behavior();
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

    fn draw(&mut self) {
        if self.hidden {
            return;
        }

        if self.state == 0 || self.state == 1 {
            self.spr.draw_src(
                self.ix as i32,
                self.iy as i32,
                &SDL_Rect { x: self.frame as i32, y: 0, w: self.iw as i32, h: self.ih as i32 },
            );
        }
    }

    fn update(&mut self) {
        io_block_update(self);

        if self.state == 1 {
            self.counter += 1;
            if self.counter >= 10 {
                self.counter = 0;
                self.frame += self.iw;

                if self.frame >= self.animationWidth {
                    self.frame = 0;
                }
            }

            self.timer += 1;
            if self.timer >= 240 {
                self.reset();
                unsafe {
                    g_map.update_tile_gap(self.col, self.row);
                }

                let iBumpPlayerID = self.iBumpPlayerID;
                io_block_kill_players_and_objects_inside_block(self, iBumpPlayerID);
                self.iBumpPlayerID = -1;
            }
        } else if self.state == 2 {
            self.state = 3;
        } else if self.state == 3 {
            self.dead = true;
            unsafe {
                g_map.blockdata[self.col as usize][self.row as usize] = Ptr::null();
                g_map.update_tile_gap(self.col, self.row);
            }
        }
    }
}

impl IO_BlockTrait for B_FlipBlock {
    crate::impl_io_block_plumbing!();

    fn reset(&mut self) {
        self.frame = 0;
        self.counter = 0;
        self.timer = 0;
        self.state = 0;
    }

    fn collide_player_dir(&mut self, player: Ptr<CPlayer>, direction: i16, useBehavior: bool) -> bool {
        let iposx = self.iposx as i32;
        let iposy = self.iposy as i32;
        let iw = self.iw as i32;
        let ih = self.ih as i32;

        if self.hidden {
            if player.fOldY >= (iposy + ih) as f32 && direction == 0 {
                return self.hitbottom_player(player, useBehavior);
            }

            return true;
        }

        if (player.fOldY + PH as f32 <= iposy as f32 || self.state > 1) && direction == 2 {
            return self.hittop_player(player, useBehavior);
        } else if (player.fOldY >= (iposy + ih) as f32 || self.state > 1) && direction == 0 {
            return self.hitbottom_player(player, useBehavior);
        } else if (player.fOldX + PW as f32 <= iposx as f32 || self.state > 1) && direction == 1 {
            return self.hitleft_player(player, useBehavior);
        } else if (player.fOldX >= (iposx + iw) as f32 || self.state > 1) && direction == 3 {
            return self.hitright_player(player, useBehavior);
        }

        true
    }

    fn is_transparent(&mut self) -> bool {
        self.state == 1
    }

    fn hittop_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if self.state == 2 || self.state == 3 {
            io_block_hittop_player(self, player, useBehavior);

            player.vely = -VELNOTEBLOCKREPEL;
            return false;
        } else if self.state == 0 {
            io_block_hittop_player(self, player, useBehavior);

            if player.is_super_stomping() {
                self.state = 2;
                self.explode();
                return true;
            }

            return false;
        }

        true
    }

    fn hitbottom_player(&mut self, mut player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            player.vely = cap_falling_velocity(-player.vely * BOUNCESTRENGTH);
            player.set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);

            self.iBumpPlayerID = player.globalID;
            self.iBumpTeamID = player.teamID;

            if self.hidden {
                self.hidden = false;
                let iBumpPlayerID = self.iBumpPlayerID;
                io_block_kill_players_and_objects_inside_block(self, iBumpPlayerID);
            }

            unsafe {
                g_map.update_tile_gap(self.col, self.row);
            }

            self.trigger_behavior();
            return false;
        }

        true
    }

    fn hitright_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            io_block_hitright_player(self, player, useBehavior);
        }

        true
    }

    fn hitleft_player(&mut self, player: Ptr<CPlayer>, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            io_block_hitleft_player(self, player, useBehavior);
        }

        true
    }

    fn collide_object_dir(&mut self, object: Ptr<dyn IO_MovingObjectTrait>, direction: i16) -> bool {
        if self.hidden {
            return true;
        }

        io_block_collide_object_dir(self, object, direction)
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
            object.fOldX = object.fx;

            if object.velx < 0.0f32 {
                object.velx = -object.velx;
            }

            return self.object_hit_side(object);
        }

        true
    }

    fn hitleft_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            let cw = object.collisionWidth;
            object.set_xf((self.iposx as i32 - cw as i32) as f32 - 0.2f32);
            object.fOldX = object.fx;

            if object.velx > 0.0f32 {
                object.velx = -object.velx;
            }

            return self.object_hit_side(object);
        }

        true
    }

    fn trigger_behavior(&mut self) {
        if self.state == 0 {
            self.state = 1;
            self.frame = self.iw;

            unsafe {
                g_map.update_tile_gap(self.col, self.row);
            }
        }
    }
}
