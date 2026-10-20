//! Base for over-map objects: sprite animation and straight-line motion, with no map or object collision.

use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::gfx::gfx_drawpreview;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_overmap, CObject, CObjectTrait};
use crate::globals::Ptr;
use crate::impl_base;
use std::ops::{Deref, DerefMut};

//------------------------------------------------------------------------------
// class OverMapObject - moving objects that don't collide with map or objects, just player
//------------------------------------------------------------------------------
/// Base state for over-map objects; positions and sizes are in pixels.
#[derive(Clone)]
pub struct IO_OverMapObject {
    pub cobject: CObject,

    /// Number of animation frames in the sprite strip.
    pub iNumSprites: i16,
    /// X offset (pixels) of the current animation frame.
    pub drawframe: i16,
    /// Tick counter toward the next animation frame.
    pub animationtimer: i16,
    /// Width (pixels) of the frame strip; `drawframe` wraps back to `animationOffsetX` here.
    pub animationWidth: i16,

    /// Source offset (pixels) of the first frame in the sprite.
    pub animationOffsetX: i16,
    pub animationOffsetY: i16,
    /// Ticks per animation frame; 0 stops the animation.
    pub animationspeed: i16,
}
impl_base!(IO_OverMapObject => cobject: CObject);

impl IO_OverMapObject {
    /// Collision and animation arguments of `-1` use the sprite frame size and zero offsets.
    #[allow(clippy::too_many_arguments)]
    pub fn new(g: &mut Game, nspr: SpriteId,
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
        let mut gp = g.ptr();
        let mut o = IO_OverMapObject {
            cobject: CObject::new(gp.get(), nspr, pos),
            iNumSprites: 0,
            drawframe: 0,
            animationtimer: 0,
            animationWidth: 0,
            animationOffsetX: 0,
            animationOffsetY: 0,
            animationspeed: 0,
        };
        o.objectType = object_overmap;

        o.iNumSprites = iNumSpr;

        if iAnimationWidth > -1 {
            o.iw = iAnimationWidth;
        } else if !o.spr.is_none() {
            o.iw = (g.rm.sprite(o.spr).get_width() as i16) / o.iNumSprites;
        }

        if iAnimationHeight > -1 {
            o.ih = iAnimationHeight;
        }

        o.animationspeed = aniSpeed;
        o.animationtimer = 0;

        if !o.spr.is_none() {
            o.animationWidth = g.rm.sprite(o.spr).get_width() as i16;
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

/// Draws the current animation frame, offset from the collision box.
pub fn io_over_map_object_draw<T: IO_OverMapObjectTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let o = this.omo();
    g.rm.sprite(o.spr).draw_src(
        g,
        (o.ix - o.collisionOffsetX) as i32,
        (o.iy - o.collisionOffsetY) as i32,
        &Rect { x: o.drawframe as i32, y: o.animationOffsetY as i32, w: o.iw as i32, h: o.ih as i32 },
    );
}

/// Draws at half scale for the map preview, offset by `iOffsetX`, `iOffsetY` (pixels).
pub fn io_over_map_object_draw_offset<T: IO_OverMapObjectTrait + ?Sized>(g: &mut Game, this: &mut T, iOffsetX: i16, iOffsetY: i16) {
    let o = this.omo();
    let rm = g.rm;
    gfx_drawpreview(
        g,
        rm.sprite(o.spr),
        (((o.ix - o.collisionOffsetX) as i32 >> 1) + iOffsetX as i32) as i16,
        (((o.iy - o.collisionOffsetY) as i32 >> 1) + iOffsetY as i32) as i16,
        o.drawframe >> 1,
        o.animationOffsetY >> 1,
        o.iw >> 1,
        o.ih >> 1,
        &Rect { x: iOffsetX as i32, y: iOffsetY as i32, w: 320, h: 240 },
        true,
        None,
    );
}

/// Moves by the current velocity, then animates.
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

/// Advances `drawframe` every `animationspeed` ticks.
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
/// `update` to `io_over_map_object_*` unless they need their own behavior; `collide_*` stay no-ops.
pub trait IO_OverMapObjectTrait: CObjectTrait {
    fn omo(&self) -> &IO_OverMapObject;
    fn omo_mut(&mut self) -> &mut IO_OverMapObject;

    /// Draws at half scale for the map preview.
    fn draw_offset(&mut self, g: &mut Game, iOffsetX: i16, iOffsetY: i16) {
        let mut gp = g.ptr();
        io_over_map_object_draw_offset(gp.get(), self, iOffsetX, iOffsetY)
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
