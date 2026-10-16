//! Port of src/smw/objects/carriable/CO_Spring.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
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
    pub fn new(g: &mut Game, nspr: SpriteId, pos: Vec2s, fsuper: bool) -> Self {
        let mut o = CO_Spring { mo_carried_object: MO_CarriedObject::new(g, nspr, pos, 4, 4, 30, 31, 1, 0), fSuper: false, iOffsetY: 0 };

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

pub fn co_spring_collide_player<T: CO_SpringTrait + ?Sized>(g: &mut Game, this: &mut T, player: PlayerId) -> bool {
    let (owner_null, fOldY, iy, state) = {
        let o = this.spring();
        (o.owner.is_none(), o.fOldY, o.iy, o.state)
    };
    if owner_null {
        if g.player(player).fOldY + PH as f32 <= fOldY && g.player(player).iy as i32 + PH >= iy as i32 {
            this.hittop(g, player);
        } else if state == 1 {
            co_spring_hitother(g, this, player);
        }
    }

    false
}

pub fn co_spring_hittop<T: CO_SpringTrait + ?Sized>(g: &mut Game, this: &mut T, player: PlayerId) {
        let mut gp = g.ptr();
    let mut player = player;
    let (iy, fSuper) = {
        let o = this.spring_mut();
        o.state = 2;
        o.drawframe += o.iw;
        (o.iy, o.fSuper)
    };

    g.player(player).set_yi((iy as i32 - PH - 1) as i16);
    let p = g.player(player);
    g.player(player).collisions.checktop(g, p);
    g.player(player).platform = Ptr::null();
    g.player(player).inair = false;
    g.player(player).fallthrough = false;
    g.player(player).killsinrowinair = 0;
    g.player(player).extrajumps = 0;

    g.player(player).superjumptimer = 4;
    g.player(player).superjumptype = if fSuper { 2 } else { 1 };
    g.player(player).vely = -VELNOTEBLOCKREPEL;

    if_sound_on_play(g, &mut gp.rm.sfx_bump);
}

pub fn co_spring_hitother<T: CO_SpringTrait + ?Sized>(g: &mut Game, this: &mut T, player: PlayerId) {
    let mut player = player;
    if this.spring().owner.is_none() && g.player(player).isready() {
        let item = this.as_carried_ptr();
        if g.player(player).accept_item(item) {
            this.spring_mut().owner = player;
        }
    }
}

pub fn co_spring_update<T: CO_SpringTrait + ?Sized>(g: &mut Game, this: &mut T) {
    if !this.spring().owner.is_none() {
        this.move_to_owner(g);
    } else {
        {
            let o = this.spring_mut();
            o.applyfriction();

            // Collision detect map
            o.fOldX = o.fx;
            o.fOldY = o.fy;
        }

        io_moving_object_collision_detection_map(g, this);
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

pub fn co_spring_draw<T: CO_SpringTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let o = this.spring();
    let x = o.ix as i32 - o.collisionOffsetX as i32;
    let y = o.iy as i32 - o.collisionOffsetY as i32;
    if !o.owner.is_none() {
        let src = Rect { x: o.animationOffsetX as i32, y: o.iOffsetY as i32, w: 32, h: 32 };
        if g.player(o.owner).iswarping() {
            let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(g.player(o.owner).get_warp_state() as i32) };
            g.rm.sprite(o.spr).draw_clip(g, x, y, &src, edge, g.player(o.owner).get_warp_plane() as i32);
        } else {
            g.rm.sprite(o.spr).draw_src(g, x, y, &src);
        }
    } else {
        g.rm.sprite(o.spr).draw_src(g, x, y, &Rect { x: o.animationOffsetX as i32 + o.drawframe as i32, y: o.iOffsetY as i32, w: 32, h: 32 });
    }
}

pub fn co_spring_place<T: CO_SpringTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let mut gp = g.ptr();
    {
        let o = this.spring_mut();
        let mut iAttempts: i16 = 10;
        let (mut ix, mut iy) = (o.ix, o.iy);
        let (w, h) = (o.collisionWidth, o.collisionHeight);
        while !gp.g_map.findspawnpoint(g, 5, &mut ix, &mut iy, w, h, false) && {
            let t = iAttempts;
            iAttempts -= 1;
            t > 0
        } {}
        o.ix = ix;
        o.iy = iy;
        o.fx = o.ix as f32;
        o.fy = o.iy as f32;
    }

    MO_CarriedObjectTrait::drop(this, g);
}

/// Virtuals added by `CO_Spring` (`hittop`), plus the non-virtual `place` so it can dispatch `Drop()`.
pub trait CO_SpringTrait: MO_CarriedObjectTrait {
    fn spring(&self) -> &CO_Spring;
    fn spring_mut(&mut self) -> &mut CO_Spring;

    fn hittop(&mut self, g: &mut Game, player: PlayerId) {
        co_spring_hittop(g, self, player)
    }

    fn place(&mut self, g: &mut Game) {
        co_spring_place(g, self)
    }
}

impl CObjectTrait for CO_Spring {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }
    fn update(&mut self, g: &mut Game) {
        co_spring_update(g, self)
    }
    fn draw(&mut self, g: &mut Game) {
        co_spring_draw(g, self)
    }
    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        co_spring_collide_player(g, self, player)
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
