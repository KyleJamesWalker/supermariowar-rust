//! Port of src/smw/objects/moving/MO_CarriedObject.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_checksides, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use std::ops::{Deref, DerefMut};

//------------------------------------------------------------------------------
// class CarriedObject - all objects players can carry inheirit from this class
//------------------------------------------------------------------------------
pub struct MO_CarriedObject {
    pub io_moving_object: IO_MovingObject,

    pub fCarriedByKuriboShoe: bool,

    pub owner: PlayerId,
    pub fSmoking: bool,

    pub dKickX: f32,
    pub dKickY: f32,
    pub iOwnerLeftOffset: i16,
    pub iOwnerRightOffset: i16,
    pub iOwnerUpOffset: i16,
    /// For `drop`, which clears the owner's `carriedItem`.
    game: Ptr<Game>,
}
impl_base!(MO_CarriedObject => io_moving_object: IO_MovingObject);

impl MO_CarriedObject {
    /// The 8-argument constructor.
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
        Self::new_anim(g, nspr, pos, iNumSpr, aniSpeed, iCollisionWidth, iCollisionHeight, iCollisionOffsetX, iCollisionOffsetY, -1, -1, -1, -1)
    }

    /// The 12-argument constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn new_anim(g: &mut Game, nspr: SpriteId,
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
        let mut o = MO_CarriedObject {
            io_moving_object: IO_MovingObject::new(
                g,
                nspr,
                pos,
                iNumSpr,
                aniSpeed,
                iCollisionWidth,
                iCollisionHeight,
                iCollisionOffsetX,
                iCollisionOffsetY,
                iAnimationOffsetX,
                iAnimationOffsetY,
                iAnimationHeight,
                iAnimationWidth,
            ),
            fCarriedByKuriboShoe: false,
            owner: PlayerId::NONE,
            fSmoking: false,
            dKickX: 0.0,
            dKickY: 0.0,
            iOwnerLeftOffset: 0,
            iOwnerRightOffset: 0,
            iOwnerUpOffset: 0,
            game: g.ptr(),
        };
        o.init();
        o
    }

    fn init(&mut self) {
        self.owner = PlayerId::NONE;
        self.fSmoking = false;

        self.dKickX = 2.0;
        self.dKickY = 4.0;

        self.iOwnerRightOffset = HALFPW as i16;
        self.iOwnerLeftOffset = (HALFPW - 32) as i16;
        self.iOwnerUpOffset = 32;

        self.fCarriedByKuriboShoe = false;
    }

    pub fn has_owner(&self) -> bool {
        !self.owner.is_none()
    }
    pub fn is_carried_by_kuribo_shoe(&self) -> bool {
        self.fCarriedByKuriboShoe
    }
}

impl Drop for MO_CarriedObject {
    fn drop(&mut self) {
        if let Some(owner) = self.game.players.get(self.owner) {
            owner.carriedItem = crate::smw::object_container::ObjectId::NONE;
        }
    }
}

pub fn mo_carried_object_draw<T: MO_CarriedObjectTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let o = this.carried();
    let src = Rect { x: o.drawframe as i32, y: o.animationOffsetY as i32, w: o.iw as i32, h: o.ih as i32 };
    if !o.owner.is_none() && g.player(o.owner).iswarping() {
        let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(g.player(o.owner).get_warp_state() as i32) };
        g.rm.sprite(o.spr).draw_clip(g, (o.ix - o.collisionOffsetX) as i32, (o.iy - o.collisionOffsetY) as i32, &src, edge, g.player(o.owner).get_warp_plane() as i32);
    } else {
        g.rm.sprite(o.spr).draw_src(g, (o.ix - o.collisionOffsetX) as i32, (o.iy - o.collisionOffsetY) as i32, &src);
    }
}

pub fn mo_carried_object_move_to_owner<T: MO_CarriedObjectTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let o = this.carried_mut();
    if !o.owner.is_none() {
        let owner = o.owner;
        let xi = (g.player(owner).ix as i32 + if g.player(owner).is_facing_right(g) { o.iOwnerRightOffset as i32 } else { o.iOwnerLeftOffset as i32 }) as i16;
        o.set_xi(xi);
        let yi = (g.player(owner).iy as i32 + PH - o.iOwnerUpOffset as i32 + o.collisionOffsetY as i32) as i16;
        o.set_yi(yi);
    }
}

pub fn mo_carried_object_drop<T: MO_CarriedObjectTrait + ?Sized>(g: &mut Game, this: &mut T) {
    {
        let o = this.carried_mut();
        if !o.owner.is_none() {
            g.player(o.owner).carriedItem = crate::smw::object_container::ObjectId::NONE;
        }

        o.owner = PlayerId::NONE;
    }

    io_moving_object_collision_detection_checksides(g, this);
}

pub fn mo_carried_object_kick<T: MO_CarriedObjectTrait + ?Sized>(g: &mut Game, this: &mut T) {
        let mut gp = g.ptr();
    {
        let o = this.carried_mut();
        if !o.owner.is_none() {
            let owner = o.owner;
            o.velx = g.player(owner).velx + if g.player(owner).is_facing_right(g) { o.dKickX } else { -o.dKickX };
            o.vely = -o.dKickY;
            if_sound_on_play(g, &mut gp.rm.sfx_kicksound);
        }
    }

    this.drop(g);
}

/// Virtual interface added by `MO_CarriedObject`. Implementors also forward `CObjectTrait::draw`
/// to `mo_carried_object_draw` (unless overridden) and keep `collide_player` returning `false`.
pub trait MO_CarriedObjectTrait: IO_MovingObjectTrait {
    fn carried(&self) -> &MO_CarriedObject;
    fn carried_mut(&mut self) -> &mut MO_CarriedObject;
    fn as_carried_ptr(&mut self) -> Ptr<dyn MO_CarriedObjectTrait>;

    fn move_to_owner(&mut self, g: &mut Game) {
        mo_carried_object_move_to_owner(g, self)
    }

    fn drop(&mut self, g: &mut Game) {
        mo_carried_object_drop(g, self)
    }
    fn kick(&mut self, g: &mut Game) {
        mo_carried_object_kick(g, self)
    }
}

impl Deref for dyn MO_CarriedObjectTrait {
    type Target = MO_CarriedObject;
    #[inline(always)]
    fn deref(&self) -> &MO_CarriedObject {
        self.carried()
    }
}

impl DerefMut for dyn MO_CarriedObjectTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut MO_CarriedObject {
        self.carried_mut()
    }
}

/// Implements `carried`, `carried_mut`, `as_carried_ptr` for a type that derefs to `MO_CarriedObject`.
#[macro_export]
macro_rules! impl_carried_object_plumbing {
    () => {
        fn carried(&self) -> &$crate::smw::objects::moving::mo_carried_object::MO_CarriedObject {
            self
        }
        fn carried_mut(&mut self) -> &mut $crate::smw::objects::moving::mo_carried_object::MO_CarriedObject {
            self
        }
        fn as_carried_ptr(&mut self) -> $crate::globals::Ptr<dyn $crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait> {
            $crate::globals::Ptr::from_mut(self as &mut dyn $crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait)
        }
    };
}
