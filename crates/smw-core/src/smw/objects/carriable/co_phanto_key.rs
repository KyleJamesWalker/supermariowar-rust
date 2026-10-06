//! Port of src/smw/objects/carriable/CO_PhantoKey.cpp

use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::movingobject_phantokey;
use crate::common::object_base::CObjectTrait;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::mo_carried_object::{mo_carried_object_draw, MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_map, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class phanto key (for chase mode)
//------------------------------------------------------------------------------
pub struct CO_PhantoKey {
    pub mo_carried_object: MO_CarriedObject,

    pub relocatetimer: i16,

    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,
}
impl_base!(CO_PhantoKey => mo_carried_object: MO_CarriedObject);

impl CO_PhantoKey {
    pub fn new(nspr: Ptr<gfxSprite>) -> Self {
        let mut o = CO_PhantoKey {
            mo_carried_object: MO_CarriedObject::new_anim(nspr, Vec2s::zero(), 1, 0, 30, 30, 1, 1, 0, 0, 32, 32),
            relocatetimer: 0,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
        };

        o.state = 1;
        o.movingObjectType = movingobject_phantokey;

        o.iOwnerRightOffset = 12;
        o.iOwnerLeftOffset = -20;
        o.iOwnerUpOffset = 32;

        o.sparkleanimationtimer = 0;
        o.sparkledrawframe = 0;

        o.fCarriedByKuriboShoe = true;

        o.place_key();
        o
    }

    pub fn place_key(&mut self) {
        if crate::smw::net_random::place_event(self.iNetworkID, 0) {
            return;
        }

        self.relocatetimer = 0;

        let mut x: i16 = 0;
        let mut y: i16 = 0;
        let mut iAttempts: i16 = 10;
        let (w, h) = (self.collisionWidth, self.collisionHeight);
        while !unsafe { g_map.findspawnpoint(5, &mut x, &mut y, w, h, false) } && {
            let t = iAttempts;
            iAttempts -= 1;
            t > 0
        } {}

        self.set_xi(x);
        self.set_yi(y);

        self.vely = GRAVITATION;
        self.velx = 0.0;

        MO_CarriedObjectTrait::drop(self);
    }
}

impl CObjectTrait for CO_PhantoKey {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        if self.owner.is_null() && player.isready() {
            let item = self.as_carried_ptr();
            if player.accept_item(item) {
                self.owner = player;
            }
        }

        false
    }

    fn update(&mut self) {
        if !self.owner.is_null() {
            self.move_to_owner();
            self.relocatetimer = 0;
        } else if {
            self.relocatetimer += 1;
            self.relocatetimer > 1000
        } {
            self.place_key();
        } else {
            self.applyfriction();

            // Collision detect map
            self.fOldX = self.fx;
            self.fOldY = self.fy;

            io_moving_object_collision_detection_map(self);
        }

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }
    }

    fn draw(&mut self) {
        mo_carried_object_draw(self);

        unsafe {
            rm.spr_shinesparkle.draw_src(
                self.ix as i32 - self.collisionOffsetX as i32,
                self.iy as i32 - self.collisionOffsetY as i32,
                &SDL_Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 },
            );
        }
    }
}

impl IO_MovingObjectTrait for CO_PhantoKey {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }
}

impl MO_CarriedObjectTrait for CO_PhantoKey {
    crate::impl_carried_object_plumbing!();
}
