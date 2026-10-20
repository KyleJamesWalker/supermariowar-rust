//! Feather powerup: rises from the block, then drifts down swinging side to side; gives the cape power.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
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
use std::ops::{Deref, DerefMut};

//------------------------------------------------------------------------------
// class feather powerup
//------------------------------------------------------------------------------
/// Feather that ignores the map: `state` 0 rises out of the block, 1 rises 128 pixels more, 2 floats down.
#[derive(Clone)]
pub struct PU_FeatherPowerup {
    pub io_moving_object: IO_MovingObject,

    /// Swing direction while floating; also picks the sprite frame.
    pub fFloatDirectionRight: bool,
    /// Angle (radians) on the swing arc, kept between a quarter and three quarters of pi.
    pub dFloatAngle: f32,
    /// Pivot of the swing arc (pixels); the feather hangs 64 pixels from it.
    pub dFloatCenterX: f32,
    pub dFloatCenterY: f32,
    /// Y (pixels) where the rise out of the block ends.
    pub desty: f32,
}
impl_base!(PU_FeatherPowerup => io_moving_object: IO_MovingObject);

impl PU_FeatherPowerup {
    #[allow(clippy::too_many_arguments)]
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, iNumSpr: i16, aniSpeed: i16, iCollisionWidth: i16, iCollisionHeight: i16, iCollisionOffsetX: i16, iCollisionOffsetY: i16) -> Self {
        let mut o = PU_FeatherPowerup {
            io_moving_object: IO_MovingObject::new(g, nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY, -1, -1, -1, -1),
            fFloatDirectionRight: true,
            dFloatAngle: HALF_PI,
            dFloatCenterX: 0.0f32,
            dFloatCenterY: 0.0f32,
            desty: 0.0,
        };
        o.desty = o.fy - o.collisionHeight as f32;

        o.movingObjectType = movingobject_powerup;

        o.iw = (g.rm.sprite(nspr).get_width() as i16) >> 1;
        o.velx = 0.0f32;

        o.fObjectCollidesWithMap = false;
        o
    }

    /// Skips the block rise and starts the second rise from tile row top `y` (pixels).
    pub fn nospawn(&mut self, y: i16) {
        self.state = 1;
        self.desty = y as f32;
        let collisionHeight = self.collisionHeight;
        self.set_yi((y as i32 + TILESIZE - collisionHeight as i32) as i16);
    }
}

pub fn pu_feather_powerup_draw<T: PU_FeatherPowerupTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let o = this.fp();
    let x = o.ix as i32 - o.collisionOffsetX as i32;
    let y = o.iy as i32 - o.collisionOffsetY as i32;
    if o.state == 0 {
        g.rm.sprite(o.spr).draw_src(g, x, y, &Rect { x: 0, y: 0, w: o.iw as i32, h: (o.ih as f32 - o.fy + o.desty) as i16 as i32 });
    } else if o.state == 1 {
        g.rm.sprite(o.spr).draw_src(g, x, y, &Rect { x: 0, y: 0, w: o.iw as i32, h: o.ih as i32 });
    } else {
        g.rm.sprite(o.spr).draw_src(g, x, y, &Rect { x: if o.fFloatDirectionRight { 0 } else { 32 }, y: 0, w: o.iw as i32, h: o.ih as i32 });
    }
}

/// Advances the rise or the swinging fall; removes the feather once it falls off the bottom of the screen.
pub fn pu_feather_powerup_update<T: PU_FeatherPowerupTrait + ?Sized>(this: &mut T) {
    let o = this.fp_mut();
    // Have the powerup grow out of the powerup block
    if o.state == 0 {
        let fy = o.fy;
        o.set_yf(fy - vel(4.0));

        if o.fy <= o.desty {
            o.state = 1;
        }
    } else if o.state == 1 {
        o.fOldX = o.fx;
        o.fOldY = o.fy;

        let fy = o.fy;
        o.set_yf(fy - vel(4.0));

        if o.fy <= o.desty - 128.0f32 {
            o.state = 2;
            o.dFloatCenterY = o.fy - 64.0f32;
            o.dFloatCenterX = o.fx;
        }
    } else {
        if !o.fFloatDirectionRight {
            o.dFloatAngle += vel(0.035);

            if o.dFloatAngle >= THREE_QUARTER_PI {
                o.dFloatAngle = THREE_QUARTER_PI;
                o.fFloatDirectionRight = true;
            }
        } else {
            o.dFloatAngle -= vel(0.035);

            if o.dFloatAngle <= QUARTER_PI {
                o.dFloatAngle = QUARTER_PI;
                o.fFloatDirectionRight = false;
            }
        }

        o.dFloatCenterY += vel(1.0);

        let x = 64.0f32 * cosf(o.dFloatAngle) + o.dFloatCenterX;
        o.set_xf(x);
        let y = 64.0f32 * sinf(o.dFloatAngle) + o.dFloatCenterY;
        o.set_yf(y);

        if o.fy >= App::screenHeight as f32 {
            o.dead = true;
        }
    }
}

/// Gives the cape power once the feather is out of the block.
pub fn pu_feather_powerup_collide_player<T: PU_FeatherPowerupTrait + ?Sized>(g: &mut Game, this: &mut T, player: PlayerId) -> bool {
    let mut player = player;
    let o = this.fp_mut();
    if o.state > 0 {
        g.player(player).set_powerup(g, 3);
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
    fn draw(&mut self, g: &mut Game) {
        pu_feather_powerup_draw(g, self)
    }
    fn update(&mut self, g: &mut Game) {
        pu_feather_powerup_update(self)
    }
    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        pu_feather_powerup_collide_player(g, self, player)
    }
}

impl IO_MovingObjectTrait for PU_FeatherPowerup {
    fn harness_hash(&mut self, h: &mut crate::smw::rollback::Fnv) {
        crate::smw::objects::moving::moving_object::mo_harness_hash(self, h);
        h.f32s(&[self.dFloatAngle, self.dFloatCenterX, self.dFloatCenterY, self.desty]);
        h.bools(&[self.fFloatDirectionRight]);
    }
    crate::impl_io_moving_object_plumbing!();
}

impl PU_FeatherPowerupTrait for PU_FeatherPowerup {
    crate::impl_feather_powerup_plumbing!();
}
