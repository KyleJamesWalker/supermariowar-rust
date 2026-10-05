//! Port of src/smw/objects/powerup/PU_FeatherPowerup.cpp

use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::trig::{cosf, sinf};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_powerup;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;
use std::ops::{Deref, DerefMut};

//------------------------------------------------------------------------------
// class feather powerup
//------------------------------------------------------------------------------
pub struct PU_FeatherPowerup {
    pub io_moving_object: IO_MovingObject,

    pub fFloatDirectionRight: bool,
    pub dFloatAngle: f32,
    pub dFloatCenterX: f32,
    pub dFloatCenterY: f32,
    pub desty: f32,
}
impl_base!(PU_FeatherPowerup => io_moving_object: IO_MovingObject);

impl PU_FeatherPowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, iCollisionWidth: i16, iCollisionHeight: i16, iCollisionOffsetX: i16, iCollisionOffsetY: i16) -> Self {
        let mut o = PU_FeatherPowerup {
            io_moving_object: IO_MovingObject::new(nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY, -1, -1, -1, -1),
            fFloatDirectionRight: true,
            dFloatAngle: HALF_PI,
            dFloatCenterX: 0.0f32,
            dFloatCenterY: 0.0f32,
            desty: 0.0,
        };
        o.desty = o.fy - o.collisionHeight as f32;

        o.movingObjectType = movingobject_powerup;

        o.iw = (nspr.get_width() as i16) >> 1;
        o.velx = 0.0f32;

        o.fObjectCollidesWithMap = false;
        o
    }

    pub fn nospawn(&mut self, y: i16) {
        self.state = 1;
        self.desty = y as f32;
        let collisionHeight = self.collisionHeight;
        self.set_yi((y as i32 + TILESIZE - collisionHeight as i32) as i16);
    }
}

pub fn pu_feather_powerup_draw<T: PU_FeatherPowerupTrait + ?Sized>(this: &mut T) {
    let o = this.fp();
    let x = o.ix as i32 - o.collisionOffsetX as i32;
    let y = o.iy as i32 - o.collisionOffsetY as i32;
    if o.state == 0 {
        o.spr.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: o.iw as i32, h: (o.ih as f32 - o.fy + o.desty) as i16 as i32 });
    } else if o.state == 1 {
        o.spr.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: o.iw as i32, h: o.ih as i32 });
    } else {
        o.spr.draw_src(x, y, &SDL_Rect { x: if o.fFloatDirectionRight { 0 } else { 32 }, y: 0, w: o.iw as i32, h: o.ih as i32 });
    }
}

pub fn pu_feather_powerup_update<T: PU_FeatherPowerupTrait + ?Sized>(this: &mut T) {
    let o = this.fp_mut();
    // Have the powerup grow out of the powerup block
    if o.state == 0 {
        let fy = o.fy;
        o.set_yf(fy - 4.0f32);

        if o.fy <= o.desty {
            o.state = 1;
        }
    } else if o.state == 1 {
        o.fOldX = o.fx;
        o.fOldY = o.fy;

        let fy = o.fy;
        o.set_yf(fy - 4.0f32);

        if o.fy <= o.desty - 128.0f32 {
            o.state = 2;
            o.dFloatCenterY = o.fy - 64.0f32;
            o.dFloatCenterX = o.fx;
        }
    } else {
        if !o.fFloatDirectionRight {
            o.dFloatAngle += 0.035f32;

            if o.dFloatAngle >= THREE_QUARTER_PI {
                o.dFloatAngle = THREE_QUARTER_PI;
                o.fFloatDirectionRight = true;
            }
        } else {
            o.dFloatAngle -= 0.035f32;

            if o.dFloatAngle <= QUARTER_PI {
                o.dFloatAngle = QUARTER_PI;
                o.fFloatDirectionRight = false;
            }
        }

        o.dFloatCenterY += 1.0f32;

        let x = 64.0f32 * cosf(o.dFloatAngle) + o.dFloatCenterX;
        o.set_xf(x);
        let y = 64.0f32 * sinf(o.dFloatAngle) + o.dFloatCenterY;
        o.set_yf(y);

        if o.fy >= App::screenHeight as f32 {
            o.dead = true;
        }
    }
}

pub fn pu_feather_powerup_collide_player<T: PU_FeatherPowerupTrait + ?Sized>(this: &mut T, player: Ptr<CPlayer>) -> bool {
    let mut player = player;
    let o = this.fp_mut();
    if o.state > 0 {
        player.set_powerup(3);
        o.dead = true;
    }

    false
}

/// `PU_FeatherPowerup` as a base class (of `PU_LeafPowerup`). Implementors forward
/// `CObjectTrait::draw`, `update` and `collide_player` to `pu_feather_powerup_*` unless overridden.
pub trait PU_FeatherPowerupTrait: IO_MovingObjectTrait {
    fn fp(&self) -> &PU_FeatherPowerup;
    fn fp_mut(&mut self) -> &mut PU_FeatherPowerup;
}

impl Deref for dyn PU_FeatherPowerupTrait {
    type Target = PU_FeatherPowerup;
    #[inline(always)]
    fn deref(&self) -> &PU_FeatherPowerup {
        self.fp()
    }
}

impl DerefMut for dyn PU_FeatherPowerupTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut PU_FeatherPowerup {
        self.fp_mut()
    }
}

/// Implements `fp`, `fp_mut` for a type that derefs to `PU_FeatherPowerup`.
#[macro_export]
macro_rules! impl_feather_powerup_plumbing {
    () => {
        fn fp(&self) -> &$crate::smw::objects::powerup::pu_feather_powerup::PU_FeatherPowerup {
            self
        }
        fn fp_mut(&mut self) -> &mut $crate::smw::objects::powerup::pu_feather_powerup::PU_FeatherPowerup {
            self
        }
    };
}

impl CObjectTrait for PU_FeatherPowerup {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }
    fn draw(&mut self) {
        pu_feather_powerup_draw(self)
    }
    fn update(&mut self) {
        pu_feather_powerup_update(self)
    }
    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        pu_feather_powerup_collide_player(self, player)
    }
}

impl IO_MovingObjectTrait for PU_FeatherPowerup {
    crate::impl_io_moving_object_plumbing!();
}

impl PU_FeatherPowerupTrait for PU_FeatherPowerup {
    crate::impl_feather_powerup_plumbing!();
}
