//! Port of src/smw/objects/carriable/CO_ThrowBox.cpp

use crate::common::eyecandy::{EC_FallingObject, EC_SingleAnimation, Spotlight};
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::{eyecandy, spotlightManager};
use crate::smw::objectgame::{createpowerup, removeifprojectile};
use crate::smw::objects::moving::mo_carried_object::{mo_carried_object_drop, mo_carried_object_kick, MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_checksides, io_moving_object_collision_detection_map, IO_MovingObjectTrait};
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class throwable box - can be used as a shield, thrown at a player, or holds items
//------------------------------------------------------------------------------
pub struct CO_ThrowBox {
    pub mo_carried_object: MO_CarriedObject,

    pub iItem: i16,

    pub frozen: bool,
    pub frozentimer: i16,
    pub frozenanimationspeed: i16,

    pub sSpotlight: Ptr<Spotlight>,
}
impl_base!(CO_ThrowBox => mo_carried_object: MO_CarriedObject);

impl CO_ThrowBox {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, item: i16) -> Self {
        let mut o = CO_ThrowBox {
            mo_carried_object: MO_CarriedObject::new(nspr, pos, 4, 8, 30, 30, 1, 1),
            iItem: 0,
            frozen: false,
            frozentimer: 0,
            frozenanimationspeed: 0,
            sSpotlight: Ptr::null(),
        };

        o.state = 1;
        o.ih = 32;
        o.iw = 32;

        o.movingObjectType = movingobject_throwbox;

        o.iPlayerID = -1;
        o.iTeamID = -1;

        o.iItem = item;

        o.iOwnerRightOffset = 14;
        o.iOwnerLeftOffset = -22;
        o.iOwnerUpOffset = 32;

        o.frozen = false;
        o.frozentimer = 0;
        o.frozenanimationspeed = 8;

        o.sSpotlight = Ptr::null();
        o
    }

    pub fn kill_player(&mut self, player: Ptr<CPlayer>) -> bool {
        if player.is_invincible() || player.shyguy {
            self.die();
            return false;
        }

        if player.is_shielded() {
            return false;
        }

        self.die();

        // Find the player that shot this shell so we can attribute a kill
        player_killed_player(self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::ThrowBlock, false, false);
        true
    }

    pub fn shatter_die(&mut self) {
        if self.dead {
            return;
        }

        unsafe {
            let spr = Ptr::from_mut(&mut rm.spr_brokeniceblock);
            let (ix, iy) = (self.ix as i32, self.iy as i32);
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix as i16, iy as i16, -1.5, -7.0, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, (ix + 16) as i16, iy as i16, 1.5, -7.0, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix as i16, (iy + 16) as i16, -1.5, -4.0, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, (ix + 16) as i16, (iy + 16) as i16, 1.5, -4.0, 4, 2, 0, 0, 16, 16));

            game_values.unlocksecret2part2 += 1;
        }

        self.die_helper();
    }

    fn die_helper(&mut self) {
        self.dead = true;
        unsafe { if_sound_on_play(&mut rm.sfx_breakblock) };

        if !self.owner.is_null() {
            self.owner.carriedItem = Ptr::null();
            self.owner = Ptr::null();
        }

        // Check to see if we should spawn an item here
        if self.iItem as i32 != NO_POWERUP {
            createpowerup(self.iItem, Vec2s::new(self.ix, self.iy), self.velx < 0.0, false);
        }
    }

    pub fn has_kill_velocity(&self) -> bool {
        self.velx < -0.01 || self.velx > 0.01 || self.vely < -0.01 || self.vely > 2.0
    }
}

impl CObjectTrait for CO_ThrowBox {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        // Kill the player if it is moving

        if self.frozen {
            self.shatter_die();
            return false;
        }

        if self.has_kill_velocity() && player.globalID != self.iPlayerID {
            return self.kill_player(player);
        }

        /*
        //Kill player when another player is holding the box
        else
        {
            if (owner && player != owner && (game_values.teamcollision == TeamCollisionStyle::On || player->teamID != owner->teamID))
            {
                    iPlayerID = owner->globalID;
                    iTeamID = owner->teamID;
                    return KillPlayer(player);
            }
        }*/

        // Otherwise allow them to pick this box up
        if self.owner.is_null() && player.isready() {
            if player.accept_item(self.as_carried_ptr()) {
                self.owner = player;
            }
        }

        false
    }

    fn collide_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) {
        let mut object = object;
        if object.is_dead() {
            return;
        }

        removeifprojectile(object, false, false);

        let r#type = object.get_moving_object_type();

        if r#type == movingobject_throwbox {
            let mut r#box: Ptr<CO_ThrowBox> = Ptr::from_mut(object.as_any().downcast_mut::<CO_ThrowBox>().unwrap());
            if self.frozen || r#box.frozen || self.has_kill_velocity() || r#box.has_kill_velocity() {
                self.die();
                r#box.die();
            }
        } else if r#type == movingobject_explosion
            || r#type == movingobject_fireball
            || r#type == movingobject_hammer
            || r#type == movingobject_boomerang
            || r#type == movingobject_superfireball
            || r#type == movingobject_sledgehammer
        {
            self.die();
        } else if r#type == movingobject_iceblast {
            self.velx = 0.0;
            self.animationspeed = 0;

            self.frozen = true;
            self.frozentimer = 300;

            unsafe {
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (self.ix as i32 - self.collisionOffsetX as i32) as i16,
                    (self.iy as i32 - self.collisionOffsetY as i32) as i16,
                    3,
                    8,
                ));
            }
        }
    }

    fn update(&mut self) {
        if self.frozen {
            self.frozentimer -= 1;
            if self.frozentimer <= 0 {
                self.frozentimer = 0;
                self.frozen = false;

                self.animationspeed = self.frozenanimationspeed;

                unsafe {
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (self.ix as i32 - self.collisionOffsetX as i32) as i16,
                        (self.iy as i32 - self.collisionOffsetY as i32) as i16,
                        3,
                        8,
                    ));
                }
            }
        }

        self.fOldX = self.fx;
        self.fOldY = self.fy;

        if !self.owner.is_null() {
            self.move_to_owner();
            self.inair = true;
        } else {
            self.applyfriction();
            io_moving_object_collision_detection_map(self);
        }

        self.animate();

        unsafe {
            if game_values.spotlights && self.has_kill_velocity() {
                let x = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
                let y = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
                if self.sSpotlight.is_null() {
                    self.sSpotlight = spotlightManager.add_spotlight(x, y, 3);
                }

                if !self.sSpotlight.is_null() {
                    self.sSpotlight.update_position(x, y);
                }
            }
        }
    }

    fn draw(&mut self) {
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        let src = SDL_Rect { x: self.drawframe as i32, y: 0, w: self.iw as i32, h: self.ih as i32 };
        if !self.owner.is_null() && self.owner.iswarping() {
            let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(self.owner.get_warp_state() as i32) };
            self.spr.draw_clip(x, y, &src, edge, self.owner.get_warp_plane() as i32);
        } else {
            self.spr.draw_src(x, y, &src);
        }

        if self.frozen {
            unsafe { rm.spr_iceblock.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: 32, h: 32 }) };
        }
    }
}

impl IO_MovingObjectTrait for CO_ThrowBox {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }

    fn die(&mut self) {
        if self.frozen {
            self.shatter_die();
            return;
        }

        if self.dead {
            return;
        }

        unsafe {
            let spr = Ptr::from_mut(&mut rm.spr_brokenyellowblock);
            let (ix, iy) = (self.ix as i32, self.iy as i32);
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix as i16, iy as i16, -2.2, -10.0, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, (ix + 16) as i16, iy as i16, 2.2, -10.0, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, ix as i16, (iy + 16) as i16, -2.2, -5.5, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, (ix + 16) as i16, (iy + 16) as i16, 2.2, -5.5, 4, 2, 0, 0, 16, 16));
        }

        self.die_helper();
    }

    fn side_bounce(&mut self, _fRightSide: bool) {
        if self.dead {
            return;
        }

        if self.has_kill_velocity() {
            if self.frozen {
                self.shatter_die();
            } else {
                self.die();
            }
        }
    }

    fn bottom_bounce(&mut self) -> f32 {
        if self.dead {
            return self.bounce;
        }

        if self.has_kill_velocity() {
            if self.frozen {
                self.shatter_die();
            } else {
                self.die();
            }
        }

        self.bounce
    }
}

impl MO_CarriedObjectTrait for CO_ThrowBox {
    crate::impl_carried_object_plumbing!();

    fn drop(&mut self) {
        if !self.owner.is_null() {
            self.iPlayerID = self.owner.globalID;
            self.iTeamID = self.owner.teamID;
        }

        if io_moving_object_collision_detection_checksides(self) {
            self.die();
        } else {
            mo_carried_object_drop(self);
        }
    }

    fn kick(&mut self) {
        if !self.owner.is_null() {
            self.iPlayerID = self.owner.globalID;
            self.iTeamID = self.owner.teamID;
        }

        if io_moving_object_collision_detection_checksides(self) {
            self.die();
        } else {
            mo_carried_object_kick(self);
        }
    }
}
