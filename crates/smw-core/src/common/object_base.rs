//! Port of src/common/ObjectBase.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::g_map;
use crate::common::global_constants::*;
use crate::common::io_block::IO_BlockTrait;
use crate::common::math::vec2::Vec2s;
use crate::globals::{Aliased, Ptr};
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;
use std::any::Any;
use std::ops::{Deref, DerefMut};

pub type ObjectType = i32;
pub const object_none: ObjectType = 0;
pub const object_block: ObjectType = 1;
pub const object_moving: ObjectType = 2;
pub const object_overmap: ObjectType = 3;
pub const object_area: ObjectType = 4;
pub const object_frenzycard: ObjectType = 5;
pub const object_race_goal: ObjectType = 6;
pub const object_thwomp: ObjectType = 7;
pub const object_kingofthehill_area: ObjectType = 8;
pub const object_bowserfire: ObjectType = 9;
pub const object_orbithazard: ObjectType = 10;
pub const object_bulletbillcannon: ObjectType = 11;
pub const object_flamecannon: ObjectType = 12;
pub const object_pathhazard: ObjectType = 13;
pub const object_pipe_coin: ObjectType = 14;
pub const object_pipe_bonus: ObjectType = 15;
pub const object_phanto: ObjectType = 16;

pub fn cap_falling_velocity(vel: f32) -> f32 {
    if vel > MAXVELY {
        return MAXVELY;
    }

    vel
}

pub fn cap_side_velocity(vel: f32) -> f32 {
    if vel < -MAXSIDEVELY {
        return -MAXSIDEVELY;
    }

    if vel > MAXSIDEVELY {
        return MAXSIDEVELY;
    }

    vel
}

pub struct CObject {
    pub objectType: ObjectType,

    pub ix: i16,
    pub iy: i16,
    pub iw: i16,
    pub ih: i16,
    pub fx: f32,
    pub fy: f32,
    pub velx: f32,
    pub vely: f32,

    pub collisionWidth: i16,
    pub collisionHeight: i16,
    pub collisionOffsetX: i16,
    pub collisionOffsetY: i16,

    pub spr: Ptr<gfxSprite>,
    pub state: i16,
    pub dead: bool,

    pub iNetworkID: i32,

    pub _alias: Aliased,
}

/// Not in the C++ (whose `iNetworkID` is unused): while nonzero, new objects get `context << 16 | n`, so a net
/// game's clients give the objects an event makes the same IDs (smw/net_random.rs).
pub static mut g_networkIDContext: u32 = 0;
static mut g_networkIDCount: u32 = 0;

pub fn set_network_id_context(context: u32) {
    unsafe {
        g_networkIDContext = context;
        g_networkIDCount = 0;
    }
}

/// The ID context and the count of objects made in it, for checkpoints.
pub fn network_id_state() -> (&'static mut u32, &'static mut u32) {
    unsafe { (&mut *(&raw mut g_networkIDContext), &mut *(&raw mut g_networkIDCount)) }
}

fn next_network_id() -> i32 {
    unsafe {
        if g_networkIDContext == 0 {
            return 0;
        }
        g_networkIDCount += 1;
        (g_networkIDContext << 16 | (g_networkIDCount & 0xFFFF)) as i32
    }
}

impl CObject {
    pub fn new(nspr1: Ptr<gfxSprite>, pos: Vec2s) -> Self {
        let mut o = CObject {
            objectType: object_none,
            ix: 0,
            iy: 0,
            iw: 0,
            ih: 0,
            fx: 0.0,
            fy: 0.0,
            velx: 0.0,
            vely: 0.0,
            collisionWidth: 0,
            collisionHeight: 0,
            collisionOffsetX: 0,
            collisionOffsetY: 0,
            spr: nspr1,
            state: 0,
            dead: false,
            iNetworkID: next_network_id(),
            _alias: Aliased::new(),
        };

        o.set_xi(pos.x);
        o.set_yi(pos.y);

        if !o.spr.is_null() {
            o.iw = o.spr.get_width() as i16;
            o.ih = o.spr.get_height() as i16;
        }

        o.collisionWidth = o.iw;
        o.collisionHeight = o.ih;
        o.collisionOffsetX = 0;
        o.collisionOffsetY = 0;
        o
    }

    pub fn set_xf(&mut self, xf: f32) {
        self.fx = xf;
        self.ix = self.fx as i16;
    }
    pub fn set_xi(&mut self, xi: i16) {
        self.ix = xi;
        self.fx = self.ix as f32;
    }
    pub fn set_yf(&mut self, yf: f32) {
        self.fy = yf;
        self.iy = (if self.fy < 0.0 { self.fy - 1.0 } else { self.fy }) as i16;
    }
    pub fn set_yi(&mut self, yi: i16) {
        self.iy = yi;
        self.fy = self.iy as f32;
    }

    pub fn x(&self) -> i32 {
        self.ix as i32
    }
    pub fn y(&self) -> i32 {
        self.iy as i32
    }
    pub fn w(&self) -> i32 {
        self.iw as i32
    }
    pub fn h(&self) -> i32 {
        self.ih as i32
    }

    pub fn vel_x(&self) -> f32 {
        self.velx
    }
    pub fn vel_y(&self) -> f32 {
        self.vely
    }

    pub fn collision_rect_w(&self) -> i16 {
        self.collisionWidth
    }
    pub fn collision_rect_h(&self) -> i16 {
        self.collisionHeight
    }

    pub fn get_state(&self) -> i16 {
        self.state
    }
    pub fn is_dead(&self) -> bool {
        self.dead
    }

    pub fn network_id(&self) -> i32 {
        self.iNetworkID
    }

    pub fn get_wrap(&self) -> bool {
        if !self.spr.is_null() {
            self.spr.is_wrapping()
        } else {
            true
        }
    }

    pub fn get_collision_blocks(&self) -> [Ptr<dyn IO_BlockTrait>; 4] {
        let ix = self.ix as i32;
        let iy = self.iy as i32;
        let iw = self.iw as i32;
        let ih = self.ih as i32;

        let xl: i16 = if ix < 0 { ((ix + 640) / TILESIZE) as i16 } else { (ix / TILESIZE) as i16 };

        let xr: i16 = if ix + iw >= 640 { ((ix + iw - 640) / TILESIZE) as i16 } else { ((ix + iw) / TILESIZE) as i16 };

        let mut blocks: [Ptr<dyn IO_BlockTrait>; 4] = [Ptr::null(); 4];

        unsafe {
            if iy >= 0 && iy < 480 {
                let yt = (iy / TILESIZE) as i16;
                blocks[0] = g_map.block(xl, yt);
                blocks[1] = g_map.block(xr, yt);
            }

            if iy + ih >= 0 && iy + ih < 480 {
                let yb = ((iy + ih) / TILESIZE) as i16;
                blocks[2] = g_map.block(xl, yb);
                blocks[3] = g_map.block(xr, yb);
            }
        }

        blocks
    }
}

/// Virtual interface of `CObject`. Overloads: `collide(CPlayer*)` is `collide_player`, `collide(IO_MovingObject*)` is `collide_object`.
pub trait CObjectTrait: Any {
    fn obj(&self) -> &CObject;
    fn obj_mut(&mut self) -> &mut CObject;
    fn as_any(&mut self) -> &mut dyn Any;
    fn as_object_ptr(&mut self) -> Ptr<dyn CObjectTrait>;

    fn draw(&mut self) {}
    fn update(&mut self);
    fn collide_player(&mut self, _player: Ptr<CPlayer>) -> bool {
        false
    }
    fn collide_object(&mut self, _object: Ptr<dyn IO_MovingObjectTrait>) {}

    fn get_object_type(&mut self) -> ObjectType {
        self.obj().objectType
    }

    fn as_io_block(&mut self) -> Option<&mut dyn IO_BlockTrait> {
        None
    }
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        None
    }
}

impl Deref for dyn CObjectTrait {
    type Target = CObject;
    #[inline(always)]
    fn deref(&self) -> &CObject {
        self.obj()
    }
}

impl DerefMut for dyn CObjectTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut CObject {
        self.obj_mut()
    }
}

/// Implements the `CObjectTrait` plumbing (`obj`, `obj_mut`, `as_any`, `as_object_ptr`) for a type that derefs to `CObject`.
#[macro_export]
macro_rules! impl_cobject_plumbing {
    () => {
        fn obj(&self) -> &$crate::common::object_base::CObject {
            self
        }
        fn obj_mut(&mut self) -> &mut $crate::common::object_base::CObject {
            self
        }
        fn as_any(&mut self) -> &mut dyn ::std::any::Any {
            self
        }
        fn as_object_ptr(&mut self) -> $crate::globals::Ptr<dyn $crate::common::object_base::CObjectTrait> {
            $crate::globals::Ptr::from_mut(self as &mut dyn $crate::common::object_base::CObjectTrait)
        }
    };
}
