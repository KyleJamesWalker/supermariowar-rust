//! Port of src/smw/objects/moving/MovingObject.cpp
// PLACEHOLDER(foundation): struct, trait and the small methods are ported; the methods ending in
// `todo!` belong to the objects porter.

use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::movingplatform::MovingPlatform;
use crate::common::object_base::{object_moving, CObject, CObjectTrait};
use crate::globals::Ptr;
use crate::impl_base;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;
use std::ops::{Deref, DerefMut};

pub struct IO_MovingObject {
    pub cobject: CObject,

    pub iPlayerID: i16,
    pub iTeamID: i16,

    pub fOldX: f32,
    pub fOldY: f32,
    pub fPrecalculatedY: f32,

    pub iNumSprites: i16,
    pub drawframe: i16,
    pub animationtimer: i16,

    pub bounce: f32,

    pub animationspeed: i16,
    pub animationWidth: i16,
    pub animationOffsetX: i16,
    pub animationOffsetY: i16,

    pub inair: bool,
    pub onice: bool,

    pub movingObjectType: MovingObjectType,

    pub platform: Ptr<MovingPlatform>,
    pub iHorizontalPlatformCollision: i16,
    pub iVerticalPlatformCollision: i16,

    pub fObjectDiesOnSuperDeathTiles: bool,
    pub fObjectCollidesWithMap: bool,
}
impl_base!(IO_MovingObject => cobject: CObject);

impl IO_MovingObject {
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
        let mut o = IO_MovingObject {
            cobject: CObject::new(nspr, pos),
            iPlayerID: 0,
            iTeamID: 0,
            fOldX: 0.0,
            fOldY: 0.0,
            fPrecalculatedY: 0.0,
            iNumSprites: 0,
            drawframe: 0,
            animationtimer: 0,
            bounce: 0.0,
            animationspeed: 0,
            animationWidth: 0,
            animationOffsetX: 0,
            animationOffsetY: 0,
            inair: false,
            onice: false,
            movingObjectType: movingobject_none,
            platform: Ptr::null(),
            iHorizontalPlatformCollision: 0,
            iVerticalPlatformCollision: 0,
            fObjectDiesOnSuperDeathTiles: false,
            fObjectCollidesWithMap: false,
        };

        o.iNumSprites = iNumSpr;

        if iAnimationWidth > -1 {
            o.iw = iAnimationWidth;
        } else if !o.spr.is_null() {
            o.iw = (o.spr.get_width() as i16) / o.iNumSprites;
        }

        if iAnimationHeight > -1 {
            o.ih = iAnimationHeight;
        }

        o.animationtimer = 0;

        if !o.spr.is_null() {
            o.animationWidth = o.spr.get_width() as i16;
        }

        o.fOldX = o.fx;
        o.fOldY = o.fy - o.ih as f32;

        o.animationspeed = aniSpeed;

        o.objectType = object_moving;
        o.movingObjectType = movingobject_none;

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

        o.inair = false;
        o.onice = false;

        o.platform = Ptr::null();
        o.iHorizontalPlatformCollision = -1;
        o.iVerticalPlatformCollision = -1;

        o.fObjectDiesOnSuperDeathTiles = true;
        o.fObjectCollidesWithMap = true;

        o.bounce = GRAVITATION;
        o
    }

    pub fn get_moving_object_type(&self) -> MovingObjectType {
        self.movingObjectType
    }

    pub fn collides_with_map(&self) -> bool {
        self.fObjectCollidesWithMap
    }

    pub fn applyfriction(&mut self) {
        if self.velx > 0.0 {
            if self.inair {
                self.velx -= VELAIRFRICTION;
            } else if self.onice {
                self.velx -= VELICEFRICTION;
            } else {
                self.velx -= VELMOVINGFRICTION;
            }

            if self.velx < 0.0 {
                self.velx = 0.0;
            }
        } else if self.velx < 0.0 {
            if self.inair {
                self.velx += VELAIRFRICTION;
            } else if self.onice {
                self.velx += VELICEFRICTION;
            } else {
                self.velx += VELMOVINGFRICTION;
            }

            if self.velx > 0.0 {
                self.velx = 0.0;
            }
        }
    }

    pub fn flipsidesifneeded(&mut self) {
        if self.ix < 0 || self.fx < 0.0 {
            let fx = self.fx;
            self.set_xf(fx + App::screenWidth as f32);
            self.fOldX += App::screenWidth as f32;
        } else if self.ix as i32 >= App::screenWidth || self.fx >= App::screenWidth as f32 {
            let fx = self.fx;
            self.set_xf(fx - App::screenWidth as f32);
            self.fOldX -= App::screenWidth as f32;
        }
    }
}

pub fn io_moving_object_draw<T: IO_MovingObjectTrait + ?Sized>(this: &mut T) {
    let o = this.mo();
    o.spr.draw_src(
        (o.ix - o.collisionOffsetX) as i32,
        (o.iy - o.collisionOffsetY) as i32,
        &SDL_Rect { x: o.drawframe as i32, y: o.animationOffsetY as i32, w: o.iw as i32, h: o.ih as i32 },
    );
}

pub fn io_moving_object_update<T: IO_MovingObjectTrait + ?Sized>(this: &mut T) {
    {
        let o = this.mo_mut();
        o.fOldX = o.fx;
        o.fOldY = o.fy;
    }

    io_moving_object_collision_detection_map(this);

    this.animate();
}

pub fn io_moving_object_animate(o: &mut IO_MovingObject) {
    if o.animationspeed > 0 && {
        o.animationtimer += 1;
        o.animationtimer >= o.animationspeed
    } {
        o.animationtimer = 0;

        o.drawframe += o.iw;
        if o.drawframe >= o.animationWidth {
            o.drawframe = o.animationOffsetX;
        }
    }
}

pub fn io_moving_object_collide_player<T: IO_MovingObjectTrait + ?Sized>(this: &mut T, _player: Ptr<CPlayer>) -> bool {
    this.mo_mut().dead = true;
    false
}

pub fn io_moving_object_collision_detection_map<T: IO_MovingObjectTrait + ?Sized>(_this: &mut T) {
    todo!("port IO_MovingObject::collision_detection_map")
}

pub fn io_moving_object_collision_detection_checksides<T: IO_MovingObjectTrait + ?Sized>(_this: &mut T) -> bool {
    todo!("port IO_MovingObject::collision_detection_checksides")
}

/// `KillObjectMapHazard(short playerID = -1)`
pub fn io_moving_object_kill_object_map_hazard<T: IO_MovingObjectTrait + ?Sized>(_this: &mut T, _playerID: i16) {
    todo!("port IO_MovingObject::KillObjectMapHazard")
}

/// Virtual interface added by `IO_MovingObject` (`draw`, `update`, `collide` are on `CObjectTrait`).
pub trait IO_MovingObjectTrait: CObjectTrait {
    fn mo(&self) -> &IO_MovingObject;
    fn mo_mut(&mut self) -> &mut IO_MovingObject;
    fn as_mo_ptr(&mut self) -> Ptr<dyn IO_MovingObjectTrait>;

    fn animate(&mut self) {
        io_moving_object_animate(self.mo_mut())
    }

    fn side_bounce(&mut self, _fRightSide: bool) {}
    fn bottom_bounce(&mut self) -> f32 {
        self.mo().bounce
    }

    fn check_and_die(&mut self) {
        self.mo_mut().dead = true;
    }
    fn die(&mut self) {
        self.mo_mut().dead = true;
    }

    /// Non-virtual in C++; on the trait so it can dispatch to `die()` / downcasts.
    fn kill_object_map_hazard(&mut self, playerID: i16) {
        io_moving_object_kill_object_map_hazard(self, playerID)
    }
}

impl Deref for dyn IO_MovingObjectTrait {
    type Target = IO_MovingObject;
    #[inline(always)]
    fn deref(&self) -> &IO_MovingObject {
        self.mo()
    }
}

impl DerefMut for dyn IO_MovingObjectTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut IO_MovingObject {
        self.mo_mut()
    }
}

/// Implements `mo`, `mo_mut`, `as_mo_ptr` for a type that derefs to `IO_MovingObject`.
#[macro_export]
macro_rules! impl_io_moving_object_plumbing {
    () => {
        fn mo(&self) -> &$crate::smw::objects::moving::moving_object::IO_MovingObject {
            self
        }
        fn mo_mut(&mut self) -> &mut $crate::smw::objects::moving::moving_object::IO_MovingObject {
            self
        }
        fn as_mo_ptr(&mut self) -> $crate::globals::Ptr<dyn $crate::smw::objects::moving::moving_object::IO_MovingObjectTrait> {
            $crate::globals::Ptr::from_mut(self as &mut dyn $crate::smw::objects::moving::moving_object::IO_MovingObjectTrait)
        }
    };
}
