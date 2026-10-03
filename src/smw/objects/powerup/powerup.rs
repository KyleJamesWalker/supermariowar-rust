//! Port of src/smw/objects/powerup/Powerup.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_powerup;
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_map, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;
use std::ops::{Deref, DerefMut};

//------------------------------------------------------------------------------
// class powerup
//------------------------------------------------------------------------------
pub struct MO_Powerup {
    pub io_moving_object: IO_MovingObject,

    pub desty: f32,
}
impl_base!(MO_Powerup => io_moving_object: IO_MovingObject);

impl MO_Powerup {
    /// Pass `-1` for the C++ default arguments.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        nspr: Ptr<gfxSprite>,
        pos: Vec2s,
        iNumSpr: i16,
        aniSpeed: i16,
        iCollisionWidth: i16,
        iCollisionHeight: i16,
        iCollisionOffsetX: i16,
        iCollisionOffsetY: i16,
    ) -> Self {
        let mut o = MO_Powerup {
            io_moving_object: IO_MovingObject::new(nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY, -1, -1, -1, -1),
            desty: 0.0,
        };
        o.desty = o.fy - o.collisionHeight as f32;
        o.movingObjectType = movingobject_powerup;
        o
    }
}

pub fn mo_powerup_draw<T: MO_PowerupTrait + ?Sized>(this: &mut T) {
    let o = this.pu();
    if o.state == 0 {
        let h = (o.ih as f32 - o.fy + o.desty) as i16;
        o.spr.draw_src((o.ix - o.collisionOffsetX) as i32, (o.iy - o.collisionOffsetY) as i32, &SDL_Rect { x: o.drawframe as i32, y: 0, w: o.iw as i32, h: h as i32 });
    } else {
        o.spr.draw_src((o.ix - o.collisionOffsetX) as i32, (o.iy - o.collisionOffsetY) as i32, &SDL_Rect { x: o.drawframe as i32, y: 0, w: o.iw as i32, h: o.ih as i32 });
    }
}

pub fn mo_powerup_update<T: MO_PowerupTrait + ?Sized>(this: &mut T) {
    // Have the powerup grow out of the powerup block
    if this.pu().state == 0 {
        let o = this.pu_mut();
        let fy = o.fy;
        o.set_yf(fy - 2.0);

        if o.fy <= o.desty {
            o.fy = o.desty;
            o.state = 1;
            o.vely = 1.0;
        }
    } else {
        // Then have it obey the physics
        {
            let o = this.pu_mut();
            o.fOldX = o.fx;
            o.fOldY = o.fy;
        }

        io_moving_object_collision_detection_map(this);
    }

    this.animate();
}

pub fn mo_powerup_collide_player<T: MO_PowerupTrait + ?Sized>(this: &mut T, _player: Ptr<CPlayer>) -> bool {
    let o = this.pu_mut();
    if o.state > 0 {
        o.dead = true;
    }

    false
}

pub fn mo_powerup_nospawn<T: MO_PowerupTrait + ?Sized>(this: &mut T, y: i16) {
    let o = this.pu_mut();
    o.state = 1;
    o.set_yi(y);
    o.vely = (-VELJUMP as f64 / 2.0) as f32;
}

/// Virtual interface added by `MO_Powerup`. Implementors forward `CObjectTrait::draw`, `update` and
/// `collide_player` to `mo_powerup_*` unless the C++ class overrides them.
pub trait MO_PowerupTrait: IO_MovingObjectTrait {
    fn pu(&self) -> &MO_Powerup;
    fn pu_mut(&mut self) -> &mut MO_Powerup;

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
