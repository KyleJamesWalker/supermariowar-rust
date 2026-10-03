//! Port of src/smw/objects/carriable/CO_Spring.cpp

use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_carried;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::mo_carried_object::{MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_map, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class spring
//------------------------------------------------------------------------------
pub struct CO_Spring {
    pub mo_carried_object: MO_CarriedObject,

    pub fSuper: bool,
    pub iOffsetY: i16,
}
impl_base!(CO_Spring => mo_carried_object: MO_CarriedObject);

impl CO_Spring {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, fsuper: bool) -> Self {
        let mut o = CO_Spring { mo_carried_object: MO_CarriedObject::new(nspr, pos, 4, 4, 30, 31, 1, 0), fSuper: false, iOffsetY: 0 };

        o.fSuper = fsuper;
        o.iOffsetY = if o.fSuper { 32 } else { 0 };

        o.state = 1;
        o.movingObjectType = movingobject_carried;

        o.iOwnerRightOffset = 14;
        o.iOwnerLeftOffset = -22;
        o.iOwnerUpOffset = 32;
        o
    }
}

pub fn co_spring_collide_player<T: CO_SpringTrait + ?Sized>(this: &mut T, player: Ptr<CPlayer>) -> bool {
    let (owner_null, fOldY, iy, state) = {
        let o = this.spring();
        (o.owner.is_null(), o.fOldY, o.iy, o.state)
    };
    if owner_null {
        if player.fOldY + PH as f32 <= fOldY && player.iy as i32 + PH >= iy as i32 {
            this.hittop(player);
        } else if state == 1 {
            co_spring_hitother(this, player);
        }
    }

    false
}

pub fn co_spring_hittop<T: CO_SpringTrait + ?Sized>(this: &mut T, player: Ptr<CPlayer>) {
    let mut player = player;
    let (iy, fSuper) = {
        let o = this.spring_mut();
        o.state = 2;
        o.drawframe += o.iw;
        (o.iy, o.fSuper)
    };

    player.set_yi((iy as i32 - PH - 1) as i16);
    let p = player.get();
    player.collisions.checktop(p);
    player.platform = Ptr::null();
    player.inair = false;
    player.fallthrough = false;
    player.killsinrowinair = 0;
    player.extrajumps = 0;

    player.superjumptimer = 4;
    player.superjumptype = if fSuper { 2 } else { 1 };
    player.vely = -VELNOTEBLOCKREPEL;

    unsafe { if_sound_on_play(&mut rm.sfx_bump) };
}

pub fn co_spring_hitother<T: CO_SpringTrait + ?Sized>(this: &mut T, player: Ptr<CPlayer>) {
    let mut player = player;
    if this.spring().owner.is_null() && player.isready() {
        let item = this.as_carried_ptr();
        if player.accept_item(item) {
            this.spring_mut().owner = player;
        }
    }
}

pub fn co_spring_update<T: CO_SpringTrait + ?Sized>(this: &mut T) {
    if !this.spring().owner.is_null() {
        this.move_to_owner();
    } else {
        {
            let o = this.spring_mut();
            o.applyfriction();

            // Collision detect map
            o.fOldX = o.fx;
            o.fOldY = o.fy;
        }

        io_moving_object_collision_detection_map(this);
    }

    let o = this.spring_mut();
    if o.state == 2 {
        o.animationtimer += 1;
        if o.animationtimer == o.animationspeed {
            o.animationtimer = 0;

            o.drawframe += o.iw;
            if o.drawframe >= o.animationWidth {
                o.drawframe = 0;
                o.state = 1;
            }
        }
    }
}

pub fn co_spring_draw<T: CO_SpringTrait + ?Sized>(this: &mut T) {
    let o = this.spring();
    let x = o.ix as i32 - o.collisionOffsetX as i32;
    let y = o.iy as i32 - o.collisionOffsetY as i32;
    if !o.owner.is_null() {
        let src = SDL_Rect { x: o.animationOffsetX as i32, y: o.iOffsetY as i32, w: 32, h: 32 };
        if o.owner.iswarping() {
            let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(o.owner.get_warp_state() as i32) };
            o.spr.draw_clip(x, y, &src, edge, o.owner.get_warp_plane() as i32);
        } else {
            o.spr.draw_src(x, y, &src);
        }
    } else {
        o.spr.draw_src(x, y, &SDL_Rect { x: o.animationOffsetX as i32 + o.drawframe as i32, y: o.iOffsetY as i32, w: 32, h: 32 });
    }
}

pub fn co_spring_place<T: CO_SpringTrait + ?Sized>(this: &mut T) {
    {
        let o = this.spring_mut();
        let mut iAttempts: i16 = 10;
        let (mut ix, mut iy) = (o.ix, o.iy);
        let (w, h) = (o.collisionWidth, o.collisionHeight);
        while !unsafe { g_map.findspawnpoint(5, &mut ix, &mut iy, w, h, false) } && {
            let t = iAttempts;
            iAttempts -= 1;
            t > 0
        } {}
        o.ix = ix;
        o.iy = iy;
        o.fx = o.ix as f32;
        o.fy = o.iy as f32;
    }

    MO_CarriedObjectTrait::drop(this);
}

/// Virtuals added by `CO_Spring` (`hittop`), plus the non-virtual `place` so it can dispatch `Drop()`.
pub trait CO_SpringTrait: MO_CarriedObjectTrait {
    fn spring(&self) -> &CO_Spring;
    fn spring_mut(&mut self) -> &mut CO_Spring;

    fn hittop(&mut self, player: Ptr<CPlayer>) {
        co_spring_hittop(self, player)
    }

    fn place(&mut self) {
        co_spring_place(self)
    }
}

impl CObjectTrait for CO_Spring {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }
    fn update(&mut self) {
        co_spring_update(self)
    }
    fn draw(&mut self) {
        co_spring_draw(self)
    }
    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        co_spring_collide_player(self, player)
    }
}

impl IO_MovingObjectTrait for CO_Spring {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }
}

impl MO_CarriedObjectTrait for CO_Spring {
    crate::impl_carried_object_plumbing!();
}

impl CO_SpringTrait for CO_Spring {
    fn spring(&self) -> &CO_Spring {
        self
    }
    fn spring_mut(&mut self) -> &mut CO_Spring {
        self
    }
}
