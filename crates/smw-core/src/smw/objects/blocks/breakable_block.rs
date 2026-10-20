//! Breakable brick: shatters when hit from below, super-stomped, or struck by a thrown object.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::eyecandy::EC_FallingObject;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::io_block::{IO_Block, IO_BlockTrait};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{cap_falling_velocity, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::objectgame::check_secret;
use crate::smw::objects::blocks::io_block::*;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};

//------------------------------------------------------------------------------
// class breakable block
//------------------------------------------------------------------------------
/// Brick whose `state` runs 0 intact, 1 just broken (bumps what stands on it), 2 removed on the next tick.
#[derive(Clone)]
pub struct B_BreakableBlock {
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
}
impl_base!(B_BreakableBlock => io_block: IO_Block);

impl B_BreakableBlock {
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, iNumSpr: i16, aniSpeed: i16) -> Self {
        let mut b = B_BreakableBlock {
            io_block: IO_Block::new(g, nspr, pos),
            iNumSprites: 0,
            animationSpeed: 0,
            drawFrame: 0,
            animationTimer: 0,
            animationWidth: 0,
        };
        b.iw = (g.rm.sprite(nspr).get_width() as i16) >> 2;
        b.iNumSprites = iNumSpr;
        b.animationSpeed = aniSpeed;
        b.animationTimer = 0;
        b.animationWidth = g.rm.sprite(nspr).get_width() as i16;
        b.drawFrame = 0;
        b
    }
}

impl CObjectTrait for B_BreakableBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        Some(self)
    }

    fn draw(&mut self, g: &mut Game) {
        if self.state == 0 {
            g.rm.sprite(self.spr).draw_src(
                g,
                self.ix as i32,
                self.iy as i32,
                &Rect { x: self.drawFrame as i32, y: 0, w: self.iw as i32, h: self.ih as i32 },
            );
        }
    }

    fn update(&mut self, g: &mut Game) {
        if self.state > 0 {
            if self.state == 1 {
                self.state = 2;
            } else if self.state == 2 {
                self.iBumpPlayerID = -1;
                self.dead = true;
                g.g_map.blockdata[self.col as usize][self.row as usize] = crate::smw::object_container::ObjectId::NONE;
                g.update_tile_gap(self.col, self.row);
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

impl IO_BlockTrait for B_BreakableBlock {
    crate::impl_io_block_plumbing!();

    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.block().harness_hash(h);
        h.i16s(&[self.drawFrame, self.animationTimer]);
    }

    fn hittop_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        io_block_hittop_player(g, self, player, useBehavior);

        if self.state == 1 || self.state == 2 {
            let mut iKillType = PlayerKillType::NonKill;
            if self.iBumpPlayerID >= 0
                && !g.player(player).is_invincible_on_bottom()
                && (g.player(player).teamID != self.iBumpTeamID || g.game_values.teamcollision == TeamCollisionStyle::On)
            {
                iKillType = player_killed_player(g, self.iBumpPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Bounce, false, false);
            }

            if PlayerKillType::NonKill == iKillType {
                g.player(player).vely = -VELNOTEBLOCKREPEL;
            }
        } else if useBehavior {
            g.player(player).vely = GRAVITATION;

            if g.player(player).is_super_stomping() && self.state == 0 {
                self.trigger_behavior(g);
                return true;
            }
        }

        false
    }

    fn hitbottom_player(&mut self, g: &mut Game, mut player: PlayerId, useBehavior: bool) -> bool {
        if useBehavior && self.state == 0 {
            self.trigger_behavior(g);

            g.player(player).vely = cap_falling_velocity(-g.player(player).vely * BOUNCESTRENGTH);
            g.player(player).set_yf((self.iposy as i32 + self.ih as i32) as f32 + 0.2f32);

            self.iBumpPlayerID = g.player(player).globalID;
            self.iBumpTeamID = g.player(player).teamID;
        }

        false
    }

    fn hittop_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        let ch = object.collisionHeight;
        object.set_yf((self.iposy as i32 - ch as i32) as f32 - 0.2f32);
        object.fOldY = object.fy;

        if self.state == 0 {
            let r#type = object.get_moving_object_type();
            if r#type == movingobject_throwbox && object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                self.trigger_behavior(g);
                object.vely = object.bottom_bounce(g);
                return true;
            }
        }

        if (self.state == 1 || self.state == 2) && object.bounce == GRAVITATION {
            io_block_bounce_moving_object(g, self, object);
            return false;
        } else {
            object.vely = object.bottom_bounce(g);
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

            let r#type = object.get_moving_object_type();
            if (r#type == movingobject_shell && object.state == 1)
                || r#type == movingobject_throwblock
                || (r#type == movingobject_throwbox && object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity())
                || r#type == movingobject_attackzone
            {
                self.trigger_behavior(g);
                return true;
            }
        }

        false
    }

    fn hitleft_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        if self.state == 0 {
            let cw = object.collisionWidth;
            object.set_xf((self.iposx as i32 - cw as i32) as f32 - 0.2f32);
            object.fOldX = object.fx;

            if object.velx > 0.0f32 {
                object.velx = -object.velx;
            }

            let r#type = object.get_moving_object_type();
            if (r#type == movingobject_shell && object.state == 1)
                || r#type == movingobject_throwblock
                || (r#type == movingobject_throwbox && object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity())
                || r#type == movingobject_attackzone
            {
                self.trigger_behavior(g);
                return true;
            }
        }

        false
    }

    fn trigger_behavior(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.state == 0 {
            let spr = g.rm.spr_brokenyellowblock;
            let (ix, iy) = (self.ix, self.iy);
            g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix, iy, vel(-2.2), vel(-10.0), 4, 2, 0, 0, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix + 16, iy, vel(2.2), vel(-10.0), 4, 2, 0, 0, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix, iy + 16, vel(-2.2), vel(-5.5), 4, 2, 0, 0, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, ix + 16, iy + 16, vel(2.2), vel(-5.5), 4, 2, 0, 0, 16, 16));

            self.state = 1;
            if_sound_on_play(g, &mut gp.rm.sfx_breakblock);
        }

        g.game_values.unlocksecret1part2 += 1;
        check_secret(g, 0);
    }
}
