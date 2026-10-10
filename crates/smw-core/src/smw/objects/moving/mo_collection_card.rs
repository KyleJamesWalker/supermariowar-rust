//! Port of src/smw/objects/moving/MO_CollectionCard.cpp

use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::object_base::{object_moving, CObjectTrait};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::{eyecandy, objectcontainer};
use crate::smw::objects::moving::moving_object::{
    io_moving_object_collision_detection_checksides, io_moving_object_draw, io_moving_object_update, IO_MovingObject, IO_MovingObjectTrait,
};
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class collection card (for card collection mode)
//------------------------------------------------------------------------------
pub struct MO_CollectionCard {
    pub io_moving_object: IO_MovingObject,

    pub timer: i16,
    pub r#type: i16,
    pub value: i16,

    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,

    pub uncollectabletime: i16,
}
impl_base!(MO_CollectionCard => io_moving_object: IO_MovingObject);

impl MO_CollectionCard {
    pub fn new(nspr: Ptr<gfxSprite>, iType: i16, iValue: i16, iUncollectableTime: i16, vel: Vec2f, pos: Vec2s) -> Self {
        let mut o = MO_CollectionCard {
            io_moving_object: IO_MovingObject::new(nspr, pos, 6, 8, -1, -1, -1, -1, 0, 0, 32, 32),
            timer: 0,
            r#type: 0,
            value: 0,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
            uncollectabletime: 0,
        };

        o.state = 1;
        o.objectType = object_moving;
        o.movingObjectType = movingobject_collectioncard;

        o.sparkleanimationtimer = 0;
        o.sparkledrawframe = 0;

        o.r#type = iType;
        o.value = iValue;

        o.uncollectabletime = iUncollectableTime;
        o.velx = vel.x;
        o.vely = vel.y;

        if iType == 0 {
            o.place_card();
            o.fObjectCollidesWithMap = false;
        } else {
            io_moving_object_collision_detection_checksides(&mut o);
            o.animationOffsetY = ((o.value as i32 + 1) << 5) as i16;
        }
        o
    }

    pub fn get_type(&self) -> i16 {
        self.r#type
    }
    pub fn get_value(&self) -> i16 {
        self.value
    }

    pub fn place_card(&mut self) {
        if crate::smw::net_random::place_event(self.iNetworkID, 0) {
            return;
        }

        self.timer = 0;

        let mut x: i16 = 0;
        let mut y: i16 = 0;
        let mut iAttempts: i16 = 32;
        unsafe {
            while (!g_map.findspawnpoint(5, &mut x, &mut y, self.collisionWidth, self.collisionHeight, false)
                || objectcontainer[1].get_closest_moving_object(x, y, movingobject_collectioncard) <= 150.0f32)
                && {
                    let a = iAttempts;
                    iAttempts -= 1;
                    a > 0
                }
            {}
        }
        if let Some((hx, hy)) = crate::smw::net_random::spawn_position() {
            (x, y) = (hx, hy);
        }

        self.set_xi(x);
        self.set_yi(y);
    }
}

impl CObjectTrait for MO_CollectionCard {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, mut player: Ptr<CPlayer>) -> bool {
        // If it is not collectable, return
        if (self.r#type == 1 && self.uncollectabletime > 0) || self.state != 1 {
            return false;
        }

        unsafe {
            if_sound_on_play(&mut rm.sfx_areatag);
        }

        let value = self.value as i32;

        // Add this card to the team's score
        if player.score().subscore[0] < 3 {
            let shift = (player.score().subscore[0] as i32) << 1;
            player.score().subscore[1] = (player.score().subscore[1] as i32 | (value << shift)) as i16;
            player.score().subscore[0] += 1;
        } else {
            player.score().subscore[1] = (player.score().subscore[1] as i32 & !48) as i16; // Clear previous card in 3rd slot
            player.score().subscore[1] = (player.score().subscore[1] as i32 | (value << 4)) as i16; // Set card to newly collected one in 3rd slot
        }

        player.score().subscore[2] = 0;

        if self.r#type == 1 {
            self.dead = true;
        } else {
            self.state = 2;
            self.animationspeed = 4;
            self.animationtimer = 0;
            self.animationOffsetY = ((value + 1) << 5) as i16; // FIXME
            self.drawframe = 96;
        }

        self.timer = 0;

        false
    }

    fn update(&mut self) {
        if self.r#type == 1 || self.state < 3 {
            self.animate();
        }

        // Handle flipping over a card to reveal it's value
        if self.state == 2 && self.drawframe == 0 {
            self.state = 3;
            self.timer = 0;
        } else if self.state == 3 {
            self.timer += 1;
            if self.timer > 200 {
                self.dead = true;
                unsafe {
                    eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), self.ix, self.iy, 3, 8));
                }
            }
        }

        self.sparkleanimationtimer += 1;
        if self.sparkleanimationtimer >= 4 {
            self.sparkleanimationtimer = 0;
            self.sparkledrawframe += 32;
            if self.sparkledrawframe as i32 >= App::screenHeight {
                self.sparkledrawframe = 0;
            }
        }

        if self.r#type == 0 {
            self.timer += 1;
            if self.timer > 1500 {
                self.place_card();
            }
        } else {
            self.applyfriction();
            io_moving_object_update(self);

            self.uncollectabletime -= 1;
            unsafe {
                if (self.uncollectabletime as i32) < -(game_values.gamemodesettings.collection.cardlife as i32) {
                    eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), self.ix, self.iy, 3, 8));
                    self.dead = true;
                }
            }
        }
    }

    fn draw(&mut self) {
        io_moving_object_draw(self);

        // Draw sparkles
        unsafe {
            rm.spr_shinesparkle.draw_src(
                self.ix as i32 - self.collisionOffsetX as i32,
                self.iy as i32 - self.collisionOffsetY as i32,
                &SDL_Rect { x: self.sparkledrawframe as i32, y: 0, w: 32, h: 32 },
            );
        }
    }
}

impl IO_MovingObjectTrait for MO_CollectionCard {
    crate::impl_io_moving_object_plumbing!();
}
