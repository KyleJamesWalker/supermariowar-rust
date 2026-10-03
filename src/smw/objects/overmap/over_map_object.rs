//! Port of src/smw/objects/overmap/OverMapObject.cpp

use crate::common::gfx::gfx_drawpreview;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_overmap, CObject, CObjectTrait};
use crate::globals::Ptr;
use crate::impl_base;
use sdl2::sys::SDL_Rect;
use std::ops::{Deref, DerefMut};

//------------------------------------------------------------------------------
// class OverMapObject - moving objects that don't collide with map or objects, just player
//------------------------------------------------------------------------------
pub struct IO_OverMapObject {
    pub cobject: CObject,

    pub iNumSprites: i16,
    pub drawframe: i16,
    pub animationtimer: i16,
    pub animationWidth: i16,

    pub animationOffsetX: i16,
    pub animationOffsetY: i16,
    pub animationspeed: i16,
}
impl_base!(IO_OverMapObject => cobject: CObject);

impl IO_OverMapObject {
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
        iAnimationOffsetX: i16,
        iAnimationOffsetY: i16,
        iAnimationHeight: i16,
        iAnimationWidth: i16,
    ) -> Self {
        let mut o = IO_OverMapObject {
            cobject: CObject::new(nspr, pos),
            iNumSprites: 0,
            drawframe: 0,
            animationtimer: 0,
            animationWidth: 0,
            animationOffsetX: 0,
            animationOffsetY: 0,
            animationspeed: 0,
        };
        o.objectType = object_overmap;
        // movingObjectType = movingobject_none;

        o.iNumSprites = iNumSpr;

        if iAnimationWidth > -1 {
            o.iw = iAnimationWidth;
        } else if !o.spr.is_null() {
            o.iw = (o.spr.get_width() as i16) / o.iNumSprites;
        }

        if iAnimationHeight > -1 {
            o.ih = iAnimationHeight;
        }

        o.animationspeed = aniSpeed;
        o.animationtimer = 0;

        if !o.spr.is_null() {
            o.animationWidth = o.spr.get_width() as i16;
        }

        if iCollisionWidth > -1 {
            o.collisionWidth = iCollisionWidth;
            o.collisionHeight = iCollisionHeight;
            o.collisionOffsetX = iCollisionOffsetX;
            o.collisionOffsetY = iCollisionOffsetY;
        } else {
            o.collisionWidth = o.iw;
            o.collisionHeight = o.ih;
            o.collisionOffsetX = 0;
            o.collisionOffsetY = 0;
        }

        if iAnimationOffsetX > -1 {
            o.animationOffsetX = iAnimationOffsetX;
            o.animationOffsetY = iAnimationOffsetY;
        } else {
            o.animationOffsetX = 0;
            o.animationOffsetY = 0;
        }

        o.drawframe = o.animationOffsetX;
        o
    }
}

pub fn io_over_map_object_draw<T: IO_OverMapObjectTrait + ?Sized>(this: &mut T) {
    let o = this.omo();
    o.spr.draw_src(
        (o.ix - o.collisionOffsetX) as i32,
        (o.iy - o.collisionOffsetY) as i32,
        &SDL_Rect { x: o.drawframe as i32, y: o.animationOffsetY as i32, w: o.iw as i32, h: o.ih as i32 },
    );
}

pub fn io_over_map_object_draw_offset<T: IO_OverMapObjectTrait + ?Sized>(this: &mut T, iOffsetX: i16, iOffsetY: i16) {
    let o = this.omo();
    gfx_drawpreview(
        o.spr.get_surface(),
        (((o.ix - o.collisionOffsetX) as i32 >> 1) + iOffsetX as i32) as i16,
        (((o.iy - o.collisionOffsetY) as i32 >> 1) + iOffsetY as i32) as i16,
        o.drawframe >> 1,
        o.animationOffsetY >> 1,
        o.iw >> 1,
        o.ih >> 1,
        iOffsetX,
        iOffsetY,
        320,
        240,
        true,
        None,
    );
}

pub fn io_over_map_object_update<T: IO_OverMapObjectTrait + ?Sized>(this: &mut T) {
    {
        let o = this.omo_mut();
        let x = o.fx + o.velx;
        o.set_xf(x);
        let y = o.fy + o.vely;
        o.set_yf(y);
    }

    this.animate();
}

pub fn io_over_map_object_animate(o: &mut IO_OverMapObject) {
    if o.animationspeed > 0 && {
        o.animationtimer += 1;
        o.animationtimer == o.animationspeed
    } {
        o.animationtimer = 0;
        o.drawframe += o.iw;
        if o.drawframe >= o.animationWidth {
            o.drawframe = o.animationOffsetX;
        }
    }
}

/// Virtual interface added by `IO_OverMapObject`. Implementors forward `CObjectTrait::draw` and
/// `update` to `io_over_map_object_*` unless the C++ class overrides them; `collide_*` stay no-ops.
/// `draw(short, short)` is `draw_offset`.
pub trait IO_OverMapObjectTrait: CObjectTrait {
    fn omo(&self) -> &IO_OverMapObject;
    fn omo_mut(&mut self) -> &mut IO_OverMapObject;

    fn draw_offset(&mut self, iOffsetX: i16, iOffsetY: i16) {
        io_over_map_object_draw_offset(self, iOffsetX, iOffsetY)
    }
    fn animate(&mut self) {
        io_over_map_object_animate(self.omo_mut())
    }
}

impl Deref for dyn IO_OverMapObjectTrait {
    type Target = IO_OverMapObject;
    #[inline(always)]
    fn deref(&self) -> &IO_OverMapObject {
        self.omo()
    }
}

impl DerefMut for dyn IO_OverMapObjectTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut IO_OverMapObject {
        self.omo_mut()
    }
}

/// Implements `omo`, `omo_mut` for a type that derefs to `IO_OverMapObject`.
#[macro_export]
macro_rules! impl_over_map_object_plumbing {
    () => {
        fn omo(&self) -> &$crate::smw::objects::overmap::over_map_object::IO_OverMapObject {
            self
        }
        fn omo_mut(&mut self) -> &mut $crate::smw::objects::overmap::over_map_object::IO_OverMapObject {
            self
        }
    };
}
