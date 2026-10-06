//! Port of src/smw/objects/carriable/CO_Flag.cpp

use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_flag;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::mo_carried_object::{mo_carried_object_drop, mo_carried_object_move_to_owner, MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::mo_flag_base::MO_FlagBase;
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_map, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class flag (for Capture the Flag mode)
//------------------------------------------------------------------------------
pub struct CO_Flag {
    pub mo_carried_object: MO_CarriedObject,

    pub timer: i16,
    pub flagbase: Ptr<MO_FlagBase>,
    pub teamID: i16,
    pub fLastFlagDirection: bool,
    pub fInBase: bool,
    pub owner_throw: Ptr<CPlayer>,
    pub owner_throw_timer: i16,
    pub centerflag: bool,
}
impl_base!(CO_Flag => mo_carried_object: MO_CarriedObject);

impl CO_Flag {
    /// Returns the heap object: the constructor hands `this` to `flagbase->setFlag`, so it must not move afterwards.
    pub fn new(nspr: Ptr<gfxSprite>, base: Ptr<MO_FlagBase>, iTeamID: i16, iColorID: i16) -> Ptr<CO_Flag> {
        let mut o = Ptr::new_box(CO_Flag {
            mo_carried_object: MO_CarriedObject::new_anim(nspr, Vec2s::zero(), 4, 8, 30, 30, 1, 1, 0, ((iColorID as i32) << 6) as i16, 32, 32),
            timer: 0,
            flagbase: Ptr::null(),
            teamID: 0,
            fLastFlagDirection: false,
            fInBase: false,
            owner_throw: Ptr::null(),
            owner_throw_timer: 0,
            centerflag: false,
        });

        o.state = 1;
        o.movingObjectType = movingobject_flag;
        o.flagbase = base;
        o.teamID = iTeamID;
        o.fLastFlagDirection = false;
        o.owner_throw = Ptr::null();
        o.owner_throw_timer = 0;

        o.centerflag = o.teamID == -1;

        o.iOwnerRightOffset = (HALFPW - 31) as i16;
        o.iOwnerLeftOffset = (HALFPW + 1) as i16;
        o.iOwnerUpOffset = 38;

        o.fCarriedByKuriboShoe = true;

        o.place_flag();
        o
    }

    pub fn get_in_base(&self) -> bool {
        self.fInBase
    }
    pub fn get_team_id(&self) -> i16 {
        self.teamID
    }

    pub fn place_flag(&mut self) {
        if crate::smw::net_random::place_event(self.iNetworkID, 0) {
            return;
        }

        if self.centerflag {
            MO_CarriedObjectTrait::drop(self);
            self.fInBase = false;

            let mut iAttempts: i16 = 10;
            let (mut ix, mut iy) = (self.ix, self.iy);
            let (w, h) = (self.collisionWidth, self.collisionHeight);
            while !unsafe { g_map.findspawnpoint(5, &mut ix, &mut iy, w, h, false) } && {
                let t = iAttempts;
                iAttempts -= 1;
                t > 0
            } {}
            self.ix = ix;
            self.iy = iy;
            self.fx = self.ix as f32;
            self.fy = self.iy as f32;

            self.velx = 0.0;
            self.vely = 0.0;
            self.fLastFlagDirection = false;
        } else if !self.flagbase.is_null() {
            MO_CarriedObjectTrait::drop(self);
            self.fInBase = true;
            let (bx, by) = (self.flagbase.ix, self.flagbase.iy);
            self.set_xi(bx);
            self.set_yi(by);
            self.fLastFlagDirection = false;
            let this = Ptr::from_mut(self);
            self.flagbase.set_flag(this);
        }

        self.owner_throw = Ptr::null();
        self.owner_throw_timer = 0;

        self.timer = 0;
    }
}

impl CObjectTrait for CO_Flag {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        if self.owner.is_null() && player.isready() && (!self.fInBase || self.teamID != player.teamID) {
            if unsafe { game_values.gamemodesettings.flag.touchreturn } && self.teamID == player.teamID {
                self.place_flag();
                unsafe { if_sound_on_play(&mut rm.sfx_areatag) };
            } else if player.accept_item(self.as_carried_ptr()) {
                self.owner = player;
                self.owner_throw = player;

                if !self.flagbase.is_null() {
                    self.flagbase.set_flag(Ptr::null());
                }
            }
        }

        false
    }

    fn update(&mut self) {
        let autoreturn = unsafe { game_values.gamemodesettings.flag.autoreturn };
        if !self.owner.is_null() {
            self.move_to_owner();
            self.timer = 0;
            self.fInBase = false;
            self.owner_throw = self.owner;
        } else if self.fInBase {
            if !self.flagbase.is_null() {
                let (bx, by) = (self.flagbase.fx, self.flagbase.fy);
                self.set_xf(bx);
                self.set_yf(by);
            }

            self.owner_throw = Ptr::null();
        } else if autoreturn > 0 && {
            self.timer += 1;
            self.timer > autoreturn
        } {
            self.timer = 0;
            self.place_flag();
            self.owner_throw = Ptr::null();
        } else {
            if !self.owner_throw.is_null() && {
                self.owner_throw_timer -= 1;
                self.owner_throw_timer < 0
            } {
                self.owner_throw_timer = 0;
                self.owner_throw = Ptr::null();
            }

            self.applyfriction();

            // Collision detect map
            self.fOldX = self.fx;
            self.fOldY = self.fy;

            io_moving_object_collision_detection_map(self);
        }

        self.animate();

        if self.centerflag {
            if self.animationtimer % 2 == 0 {
                self.animationOffsetY += 64;
                if self.animationOffsetY > 192 {
                    self.animationOffsetY = 0;
                }
            }
        }
    }

    fn draw(&mut self) {
        let srcRect = SDL_Rect {
            x: self.drawframe as i32,
            y: self.animationOffsetY as i32 + if self.fLastFlagDirection { 32 } else { 0 },
            w: self.iw as i32,
            h: self.ih as i32,
        };
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        if !self.owner.is_null() {
            if self.centerflag {
                self.animationOffsetY = ((self.owner.colorID as i32) << 6) as i16;
            }

            if self.owner.iswarping() {
                let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(self.owner.get_warp_state() as i32) };
                self.spr.draw_clip(x, y, &srcRect, edge, self.owner.get_warp_plane() as i32);
            } else {
                self.spr.draw_src(x, y, &srcRect);
            }
        } else {
            self.spr.draw_src(x, y, &srcRect);
        }
    }
}

impl IO_MovingObjectTrait for CO_Flag {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }
}

impl MO_CarriedObjectTrait for CO_Flag {
    crate::impl_carried_object_plumbing!();

    fn move_to_owner(&mut self) {
        mo_carried_object_move_to_owner(self);

        if !self.owner.is_null() {
            self.fLastFlagDirection = self.owner.is_facing_right();
        }
    }

    fn drop(&mut self) {
        mo_carried_object_drop(self);
        self.owner_throw_timer = 62;
    }
}
