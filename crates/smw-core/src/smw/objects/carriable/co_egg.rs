//! Port of src/smw/objects/carriable/CO_Egg.cpp

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
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
use crate::smw::objects::moving::mo_carried_object::{mo_carried_object_draw, mo_carried_object_drop, MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::mo_explosion::MO_Explosion;
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_map, IO_MovingObjectTrait};
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class egg (for egg mode)
//------------------------------------------------------------------------------
#[derive(Clone)]
pub struct CO_Egg {
    pub mo_carried_object: MO_CarriedObject,

    pub relocatetimer: i16,
    pub explosiondrawframe: i16,
    pub explosiondrawtimer: i16,

    pub owner_throw: PlayerId,
    pub owner_throw_timer: i16,

    pub sparkleanimationtimer: i16,
    pub sparkledrawframe: i16,

    pub color: i16,

    pub egganimationrates: [i16; 6],
}
impl_base!(CO_Egg => mo_carried_object: MO_CarriedObject);

impl CO_Egg {
    pub fn new(g: &mut Game, nspr: SpriteId, iColor: i16) -> Self {
        let mut o = CO_Egg {
            mo_carried_object: MO_CarriedObject::new_anim(g, nspr, Vec2s::zero(), 2, ticks(16) as i16, 28, 30, 2, 1, 0, ((iColor as i32) << 5) as i16, 32, 32),
            relocatetimer: 0,
            explosiondrawframe: 0,
            explosiondrawtimer: 0,
            owner_throw: PlayerId::NONE,
            owner_throw_timer: 0,
            sparkleanimationtimer: 0,
            sparkledrawframe: 0,
            color: 0,
            egganimationrates: [0; 6],
        };

        o.state = 1;
        o.movingObjectType = movingobject_egg;

        o.owner_throw = PlayerId::NONE;
        o.owner_throw_timer = 0;

        o.sparkleanimationtimer = 0;
        o.sparkledrawframe = 0;

        o.color = iColor;

        o.egganimationrates[0] = 2;
        o.egganimationrates[1] = 4;
        o.egganimationrates[2] = 6;
        o.egganimationrates[3] = 8;
        o.egganimationrates[4] = ticks(12) as i16;
        o.egganimationrates[5] = ticks(16) as i16;

        o.iOwnerRightOffset = HALFPW as i16;
        o.iOwnerLeftOffset = (HALFPW - 28) as i16;
        o.iOwnerUpOffset = 32;

        o.fCarriedByKuriboShoe = true;

        o.place_egg(g);
        o
    }

    pub fn get_color(&self) -> i16 {
        self.color
    }

    pub fn place_egg(&mut self, g: &mut Game) {
        let mut gp = g.ptr();

        self.relocatetimer = 0;
        if g.game_values.gamemodesettings.egg.explode > 0 {
            self.explosiondrawframe = g.game_values.gamemodesettings.egg.explode - 1;
            self.explosiondrawtimer = TICKS_PER_SECOND as i16;

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
        while (!gp.g_map.findspawnpoint(g, 5, &mut x, &mut y, w, h, false) || g.objectcontainer[1].get_closest_moving_object(x, y, movingobject_yoshi) < 250.0) && {
            let t = iAttempts;
            iAttempts -= 1;
            t > 0
        } {}

        self.set_xi(x);
        self.set_yi(y);

        self.vely = GRAVITATION;
        self.velx = 0.0;

        self.owner_throw = PlayerId::NONE;
        self.owner_throw_timer = 0;

        MO_CarriedObjectTrait::drop(self, g);
    }
}

impl CObjectTrait for CO_Egg {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut player = player;
        if self.owner.is_none() && g.player(player).isready() {
            let item = self.as_carried_ptr();
            if g.player(player).accept_item(item) {
                self.owner = player;
                self.owner_throw = player;
            }
        }

        false
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        if !self.owner.is_none() {
            self.move_to_owner(g);
            self.relocatetimer = 0;
            self.owner_throw = self.owner;
        } else if {
            self.relocatetimer += 1;
            self.relocatetimer as i32 > ticks(1500)
        } {
            self.place_egg(g);
            self.owner_throw = PlayerId::NONE;
        } else {
            if !self.owner_throw.is_none() && {
                self.owner_throw_timer -= 1;
                self.owner_throw_timer <= 0
            } {
                self.owner_throw_timer = 0;
                self.owner_throw = PlayerId::NONE;
            }

            self.applyfriction();

            // Collision detect map
            self.fOldX = self.fx;
            self.fOldY = self.fy;

            io_moving_object_collision_detection_map(g, self);
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
        if g.game_values.gamemodesettings.egg.explode > 0 {
            self.explosiondrawtimer -= 1;
            if self.explosiondrawtimer <= 0 {
                self.explosiondrawtimer = TICKS_PER_SECOND as i16;
                self.explosiondrawframe -= 1;
                if self.explosiondrawframe < 0 {
                    let obj = Ptr::new_box(MO_Explosion::new(
                        g,
                        rm.spr_explosion,
                        Vec2s::new((self.ix as i32 + (self.iw as i32 >> 1) - 96) as i16, (self.iy as i32 + (self.ih as i32 >> 1) - 64) as i16),
                        2,
                        4,
                        -1,
                        -1,
                        KillStyle::Bomb,
                    ));
                    gp.objectcontainer[2].add(g, obj);
                    self.place_egg(g);

                    if_sound_on_play(g, &mut gp.rm.sfx_bobombsound);
                } else if self.explosiondrawframe < 5 {
                    self.animationspeed = self.egganimationrates[self.explosiondrawframe as usize];
                } else {
                    self.animationspeed = self.egganimationrates[5];
                }
            }
        }

        self.animate();
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        mo_carried_object_draw(g, self);

        // Display explosion timer
        unsafe {
            if g.game_values.gamemodesettings.egg.explode > 0 && self.explosiondrawframe < 5 {
                let x = self.ix as i32 - self.collisionOffsetX as i32;
                let y = self.iy as i32 - self.collisionOffsetY as i32;
                let src = Rect { x: (self.explosiondrawframe as i32) << 5, y: (self.color as i32) << 5, w: 32, h: 32 };
                if !self.owner.is_none() && g.player(self.owner).iswarping() {
                    let edge: ClipEdge = std::mem::transmute::<i32, ClipEdge>(g.player(self.owner).get_warp_state() as i32);
                    gp.rm.sprite(gp.rm.spr_eggnumbers).draw_clip(g, x, y, &src, edge, g.player(self.owner).get_warp_plane() as i32);
                } else {
                    gp.rm.sprite(gp.rm.spr_eggnumbers).draw_src(g, x, y, &src);
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

    fn drop(&mut self, g: &mut Game) {
        mo_carried_object_drop(g, self);
        self.owner_throw_timer = TICKS_PER_SECOND as i16;
    }
}
