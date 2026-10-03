//! Port of src/smw/objects/carriable/CO_Egg.cpp

use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::{movingobject_egg, movingobject_yoshi};
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::moving::mo_carried_object::{mo_carried_object_draw, mo_carried_object_drop, MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::mo_explosion::MO_Explosion;
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_map, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class egg (for egg mode)
//------------------------------------------------------------------------------
pub struct CO_Egg {
    pub mo_carried_object: MO_CarriedObject,

    pub relocatetimer: i16,
    pub explosiondrawframe: i16,
    pub explosiondrawtimer: i16,

    pub owner_throw: Ptr<CPlayer>,
    pub owner_throw_timer: i16,

    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,

    pub color: i16,

    pub egganimationrates: [i16; 6],
}
impl_base!(CO_Egg => mo_carried_object: MO_CarriedObject);

impl CO_Egg {
    pub fn new(nspr: Ptr<gfxSprite>, iColor: i16) -> Self {
        let mut o = CO_Egg {
            mo_carried_object: MO_CarriedObject::new_anim(nspr, Vec2s::zero(), 2, 16, 28, 30, 2, 1, 0, ((iColor as i32) << 5) as i16, 32, 32),
            relocatetimer: 0,
            explosiondrawframe: 0,
            explosiondrawtimer: 0,
            owner_throw: Ptr::null(),
            owner_throw_timer: 0,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
            color: 0,
            egganimationrates: [0; 6],
        };

        o.state = 1;
        o.movingObjectType = movingobject_egg;

        o.owner_throw = Ptr::null();
        o.owner_throw_timer = 0;

        o.sparkleanimationtimer = 0;
        o.sparkledrawframe = 0;

        o.color = iColor;

        o.egganimationrates[0] = 2;
        o.egganimationrates[1] = 4;
        o.egganimationrates[2] = 6;
        o.egganimationrates[3] = 8;
        o.egganimationrates[4] = 12;
        o.egganimationrates[5] = 16;

        o.iOwnerRightOffset = HALFPW as i16;
        o.iOwnerLeftOffset = (HALFPW - 28) as i16;
        o.iOwnerUpOffset = 32;

        o.fCarriedByKuriboShoe = true;

        o.place_egg();
        o
    }

    pub fn get_color(&self) -> i16 {
        self.color
    }

    pub fn place_egg(&mut self) {
        self.relocatetimer = 0;
        unsafe {
            if game_values.gamemodesettings.egg.explode > 0 {
                self.explosiondrawframe = game_values.gamemodesettings.egg.explode - 1;
                self.explosiondrawtimer = 62;

                if self.explosiondrawframe < 5 {
                    self.animationspeed = self.egganimationrates[self.explosiondrawframe as usize];
                } else {
                    self.animationspeed = self.egganimationrates[5];
                }
            } else {
                self.animationspeed = self.egganimationrates[5];
            }

            let mut x: i16 = 0;
            let mut y: i16 = 0;
            let mut iAttempts: i16 = 32;
            let (w, h) = (self.collisionWidth, self.collisionHeight);
            while (!g_map.findspawnpoint(5, &mut x, &mut y, w, h, false) || objectcontainer[1].get_closest_moving_object(x, y, movingobject_yoshi) < 250.0) && {
                let t = iAttempts;
                iAttempts -= 1;
                t > 0
            } {}

            self.set_xi(x);
            self.set_yi(y);
        }

        self.vely = GRAVITATION;
        self.velx = 0.0;

        self.owner_throw = Ptr::null();
        self.owner_throw_timer = 0;

        MO_CarriedObjectTrait::drop(self);
    }
}

impl CObjectTrait for CO_Egg {
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
                self.owner_throw = player;
            }
        }

        false
    }

    fn update(&mut self) {
        if !self.owner.is_null() {
            self.move_to_owner();
            self.relocatetimer = 0;
            self.owner_throw = self.owner;
        } else if {
            self.relocatetimer += 1;
            self.relocatetimer > 1500
        } {
            self.place_egg();
            self.owner_throw = Ptr::null();
        } else {
            if !self.owner_throw.is_null() && {
                self.owner_throw_timer -= 1;
                self.owner_throw_timer <= 0
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

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }

        // Explode
        unsafe {
            if game_values.gamemodesettings.egg.explode > 0 {
                self.explosiondrawtimer -= 1;
                if self.explosiondrawtimer <= 0 {
                    self.explosiondrawtimer = 62;
                    self.explosiondrawframe -= 1;
                    if self.explosiondrawframe < 0 {
                        objectcontainer[2].add(Ptr::new_box(MO_Explosion::new(
                            Ptr::from_mut(&mut rm.spr_explosion),
                            Vec2s::new((self.ix as i32 + (self.iw as i32 >> 1) - 96) as i16, (self.iy as i32 + (self.ih as i32 >> 1) - 64) as i16),
                            2,
                            4,
                            -1,
                            -1,
                            KillStyle::Bomb,
                        )));
                        self.place_egg();

                        if_sound_on_play(&mut rm.sfx_bobombsound);
                    } else if self.explosiondrawframe < 5 {
                        self.animationspeed = self.egganimationrates[self.explosiondrawframe as usize];
                    } else {
                        self.animationspeed = self.egganimationrates[5];
                    }
                }
            }
        }

        self.animate();
    }

    fn draw(&mut self) {
        mo_carried_object_draw(self);

        // Display explosion timer
        unsafe {
            if game_values.gamemodesettings.egg.explode > 0 && self.explosiondrawframe < 5 {
                let x = self.ix as i32 - self.collisionOffsetX as i32;
                let y = self.iy as i32 - self.collisionOffsetY as i32;
                let src = SDL_Rect { x: (self.explosiondrawframe as i32) << 5, y: (self.color as i32) << 5, w: 32, h: 32 };
                if !self.owner.is_null() && self.owner.iswarping() {
                    let edge: ClipEdge = std::mem::transmute::<i32, ClipEdge>(self.owner.get_warp_state() as i32);
                    rm.spr_eggnumbers.draw_clip(x, y, &src, edge, self.owner.get_warp_plane() as i32);
                } else {
                    rm.spr_eggnumbers.draw_src(x, y, &src);
                }
            }
        }
    }
}

impl IO_MovingObjectTrait for CO_Egg {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }
}

impl MO_CarriedObjectTrait for CO_Egg {
    crate::impl_carried_object_plumbing!();

    fn drop(&mut self) {
        mo_carried_object_drop(self);
        self.owner_throw_timer = 62;
    }
}
