//! Shared base for powerups: rise out of a block, then fall and bounce under map physics.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_powerup;
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_map, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use std::ops::{Deref, DerefMut};

//------------------------------------------------------------------------------
// class powerup
//------------------------------------------------------------------------------
/// Base state for a powerup that grows out of a block, then moves under map physics.
#[derive(Clone)]
pub struct MO_Powerup {
    pub io_moving_object: IO_MovingObject,

    /// Y (pixels) where the rise out of the block ends; `state` is 0 until then.
    pub desty: f32,
}
impl_base!(MO_Powerup => io_moving_object: IO_MovingObject);

impl MO_Powerup {
    /// Collision arguments of `-1` use the sprite frame size with no offset.
    #[allow(clippy::too_many_arguments)]
    pub fn new(g: &mut Game, nspr: SpriteId,
        pos: Vec2s,
        iNumSpr: i16,
        aniSpeed: i16,
        iCollisionWidth: i16,
        iCollisionHeight: i16,
        iCollisionOffsetX: i16,
        iCollisionOffsetY: i16,
    ) -> Self {
        let mut o = MO_Powerup {
            io_moving_object: IO_MovingObject::new(g, nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY, -1, -1, -1, -1),
            desty: 0.0,
        };
        o.desty = o.fy - o.collisionHeight as f32;
        o.movingObjectType = movingobject_powerup;
        o
    }
}

/// Draws the current frame, clipped to the part already out of the block while rising.
pub fn mo_powerup_draw<T: MO_PowerupTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let o = this.pu();
    if o.state == 0 {
        let h = (o.ih as f32 - o.fy + o.desty) as i16;
        g.rm.sprite(o.spr).draw_src(g, (o.ix - o.collisionOffsetX) as i32, (o.iy - o.collisionOffsetY) as i32, &Rect { x: o.drawframe as i32, y: 0, w: o.iw as i32, h: h as i32 });
    } else {
        g.rm.sprite(o.spr).draw_src(g, (o.ix - o.collisionOffsetX) as i32, (o.iy - o.collisionOffsetY) as i32, &Rect { x: o.drawframe as i32, y: 0, w: o.iw as i32, h: o.ih as i32 });
    }
}

/// Rises out of the block while `state` is 0, then runs map collision and physics.
pub fn mo_powerup_update<T: MO_PowerupTrait + ?Sized>(g: &mut Game, this: &mut T) {
    // Have the powerup grow out of the powerup block
    if this.pu().state == 0 {
        let o = this.pu_mut();
        let fy = o.fy;
        o.set_yf(fy - vel(2.0));

        if o.fy <= o.desty {
            o.fy = o.desty;
            o.state = 1;
            o.vely = vel(1.0);
        }
    } else {
        // Then have it obey the physics
        {
            let o = this.pu_mut();
            o.fOldX = o.fx;
            o.fOldY = o.fy;
        }

        io_moving_object_collision_detection_map(g, this);
    }

    this.animate();
}

/// Default pickup: removes the powerup once it has finished rising.
pub fn mo_powerup_collide_player<T: MO_PowerupTrait + ?Sized>(this: &mut T, _player: PlayerId) -> bool {
    let o = this.pu_mut();
    if o.state > 0 {
        o.dead = true;
    }

    false
}

/// Skips the rise: places the powerup at `y` and pops it upward at half jump speed.
pub fn mo_powerup_nospawn<T: MO_PowerupTrait + ?Sized>(this: &mut T, y: i16) {
    let o = this.pu_mut();
    o.state = 1;
    o.set_yi(y);
    o.vely = (-VELJUMP as f64 / 2.0) as f32;
}

/// Virtual interface added by `MO_Powerup`. Implementors forward `CObjectTrait::draw`, `update` and
/// `collide_player` to `mo_powerup_*` unless they need their own behavior.
pub trait MO_PowerupTrait: IO_MovingObjectTrait {
    fn pu(&self) -> &MO_Powerup;
    fn pu_mut(&mut self) -> &mut MO_Powerup;

    /// Places the powerup at `y` (pixels) already out of its block, for spawns without a block.
    fn nospawn(&mut self, y: i16) {
        mo_powerup_nospawn(self, y)
    }
}

impl Deref for dyn MO_PowerupTrait {
    type Target = MO_Powerup;
    #[inline(always)]
    fn deref(&self) -> &MO_Powerup {
        self.pu()
    }
}

impl DerefMut for dyn MO_PowerupTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut MO_Powerup {
        self.pu_mut()
    }
}

/// Implements `pu`, `pu_mut` for a type that derefs to `MO_Powerup`.
#[macro_export]
macro_rules! impl_powerup_plumbing {
    () => {
        fn pu(&self) -> &$crate::smw::objects::powerup::powerup::MO_Powerup {
            self
        }
        fn pu_mut(&mut self) -> &mut $crate::smw::objects::powerup::powerup::MO_Powerup {
            self
        }
    };
}
