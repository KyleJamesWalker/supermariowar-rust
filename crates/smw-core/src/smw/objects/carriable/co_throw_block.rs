//! Port of src/smw/objects/carriable/CO_ThrowBlock.cpp

use crate::game::Game;
use crate::common::eyecandy::{EC_FallingObject, EC_SingleAnimation, Spotlight};
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::{gfxSprite, ClipEdge};
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::moving::mo_carried_object::{MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_checksides, io_moving_object_collision_detection_map, IO_MovingObjectTrait};
use crate::smw::objects::throw_block_type::ThrowBlockType;
use crate::smw::player::{player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

//------------------------------------------------------------------------------
// class throwable block projectile
//------------------------------------------------------------------------------
// State 1: Moving
// State 2: Holding
pub struct CO_ThrowBlock {
    pub mo_carried_object: MO_CarriedObject,

    pub iType: ThrowBlockType,

    pub iDeathTime: i16,
    pub fDieOnBounce: bool,
    pub fDieOnPlayerCollision: bool,
    pub iBounceCounter: i16,
    pub iNoOwnerKillTime: i16,

    pub frozen: bool,
    pub frozentimer: i16,
    pub frozenvelocity: f32,
    pub frozenanimationspeed: i16,

    pub sSpotlight: Ptr<Spotlight>,
}
impl_base!(CO_ThrowBlock => mo_carried_object: MO_CarriedObject);

impl CO_ThrowBlock {
    pub fn new(nspr: Ptr<gfxSprite>, pos: Vec2s, r#type: ThrowBlockType) -> Self {
        let mut o = CO_ThrowBlock {
            mo_carried_object: MO_CarriedObject::new(nspr, pos, 4, 2, 30, 30, 1, 1),
            iType: r#type,
            iDeathTime: 0,
            fDieOnBounce: false,
            fDieOnPlayerCollision: false,
            iBounceCounter: 0,
            iNoOwnerKillTime: 0,
            frozen: false,
            frozentimer: 0,
            frozenvelocity: 0.0,
            frozenanimationspeed: 0,
            sSpotlight: Ptr::null(),
        };

        o.state = 2;
        o.ih = 32;
        o.movingObjectType = movingobject_throwblock;
        o.iPlayerID = -1;
        o.iTeamID = -1;

        o.fDieOnBounce = r#type != ThrowBlockType::Red;
        o.fDieOnPlayerCollision = r#type == ThrowBlockType::Blue;

        o.iType = r#type;

        o.iDeathTime = 0;
        o.iBounceCounter = 0;
        o.iNoOwnerKillTime = 0;

        o.iOwnerRightOffset = 14;
        o.iOwnerLeftOffset = -22;
        o.iOwnerUpOffset = 32;

        o.frozen = false;
        o.frozentimer = 0;
        o.frozenvelocity = 0.0;
        o.frozenanimationspeed = 2;

        o.sSpotlight = Ptr::null();
        o
    }

    pub fn hit_top(&mut self, g: &mut Game, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        if player.is_invincible() || player.shyguy {
            self.die(g);
        } else if self.state == 1 {
            // moving
            return self.kill_player(g, player);
        } else if self.state == 2 {
            // Holding
            if player != self.owner {
                if !self.owner.is_null() {
                    self.owner.carriedItem = Ptr::null();
                }

                self.kick(g);

                player.set_yi((self.iy as i32 - PH - 1) as i16);
                player.bouncejump();
                let p = player.get();
                player.collisions.checktop(g, p);
                player.platform = Ptr::null();
            }
        }

        false
    }

    pub fn hit_other(&mut self, player: Ptr<CPlayer>) -> bool {
        if self.state == 1 {
            // Moving
            let mut flipx: i16 = 0;

            if (player.ix as i32 + PW) < 320 && self.ix > 320 {
                flipx = App::screenWidth as i16;
            } else if (self.ix as i32 + self.iw as i32) < 320 && player.ix > 320 {
                flipx = -(App::screenWidth as i16);
            }

            let px = player.ix as i32 + flipx as i32;
            if self.iNoOwnerKillTime == 0
                || player.globalID != self.iPlayerID
                || (px > self.ix as i32 + (self.iw as i32 >> 1) && self.velx > 0.0)
                || (px <= self.ix as i32 - (self.iw as i32 >> 1) && self.velx < 0.0)
            {
                return self.kill_player(game().get(), player);
            }
        } else if self.state == 2 {
            // Holding
            if player != self.owner {
                self.iPlayerID = self.owner.globalID;
                self.iTeamID = self.owner.teamID;
                return self.kill_player(game().get(), player);
            }
        }

        false
    }

    pub fn kill_player(&mut self, g: &mut Game, player: Ptr<CPlayer>) -> bool {
        if player.is_invincible() || player.shyguy {
            self.die(g);
            return false;
        }

        if player.is_shielded() {
            return false;
        }

        self.check_and_die(g);

        // Find the player that shot this shell so we can attribute a kill
        player_killed_player(g, self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::ThrowBlock, false, false);
        true
    }

    pub fn shatter_die(&mut self, g: &mut Game) {
        if self.dead {
            return;
        }

        unsafe {
            let spr = Ptr::from_mut(&mut g.rm.spr_brokeniceblock);
            let (ix, iy) = (self.ix as i32, self.iy as i32);
            g.eyecandy[2].emplace(EC_FallingObject::new(spr, ix as i16, iy as i16, -1.5, -7.0, 4, 2, 0, 0, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(spr, (ix + 16) as i16, iy as i16, 1.5, -7.0, 4, 2, 0, 0, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(spr, ix as i16, (iy + 16) as i16, -1.5, -4.0, 4, 2, 0, 0, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(spr, (ix + 16) as i16, (iy + 16) as i16, 1.5, -4.0, 4, 2, 0, 0, 16, 16));

            g.game_values.unlocksecret2part2 += 1;
        }

        self.die_helper(g);
    }

    fn die_helper(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.dead = true;
        unsafe { if_sound_on_play(g, &mut gp.rm.sfx_breakblock) };

        if !self.owner.is_null() {
            self.owner.carriedItem = Ptr::null();
            self.owner = Ptr::null();
        }
    }
}

impl CObjectTrait for CO_ThrowBlock {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, g: &mut Game, player: Ptr<CPlayer>) -> bool {
        if self.frozen {
            self.shatter_die(g);
            return false;
        }

        if player.fOldY + PH as f32 <= self.iy as f32 && player.iy as i32 + PH >= self.iy as i32 {
            self.hit_top(g, player)
        } else {
            self.hit_other(player)
        }
    }

    fn collide_object(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) {
        let mut object = object;
        if object.is_dead() {
            return;
        }

        removeifprojectile(g, object, false, false);

        let r#type = object.get_moving_object_type();

        if r#type == movingobject_throwblock || r#type == movingobject_throwbox {
            self.die(g);
            object.die(g);
        } else if r#type == movingobject_explosion {
            self.die(g);
        } else if r#type == movingobject_iceblast {
            self.frozenvelocity = self.velx;
            self.velx = 0.0;
            self.animationspeed = 0;

            self.frozen = true;
            self.frozentimer = 300;

            unsafe {
                g.eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut g.rm.spr_fireballexplosion),
                    (self.ix as i32 - self.collisionOffsetX as i32) as i16,
                    (self.iy as i32 - self.collisionOffsetY as i32) as i16,
                    3,
                    8,
                ));
            }
        }
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.iNoOwnerKillTime > 0 {
            self.iNoOwnerKillTime -= 1;
        }

        unsafe {
            if self.frozen {
                self.frozentimer -= 1;
                if self.frozentimer <= 0 {
                    self.frozentimer = 0;
                    self.frozen = false;

                    self.velx = self.frozenvelocity;
                    self.animationspeed = self.frozenanimationspeed;

                    g.eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut g.rm.spr_fireballexplosion),
                        (self.ix as i32 - self.collisionOffsetX as i32) as i16,
                        (self.iy as i32 - self.collisionOffsetY as i32) as i16,
                        3,
                        8,
                    ));
                }
            } else if g.game_values.blueblockttl > 0 && {
                self.iDeathTime += 1;
                self.iDeathTime >= g.game_values.blueblockttl
            } {
                g.eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut g.rm.spr_fireballexplosion), self.ix, self.iy, 3, 8));
                self.dead = true;

                if !self.owner.is_null() {
                    self.owner.carriedItem = Ptr::null();
                    self.owner = Ptr::null();
                }

                return;
            }
        }

        if !self.owner.is_null() {
            self.move_to_owner(g);
            self.inair = true;
        } else {
            if self.iBounceCounter > 0 {
                self.iBounceCounter -= 1;
            }

            self.fOldX = self.fx;
            self.fOldY = self.fy;

            io_moving_object_collision_detection_map(g, self);
        }

        self.animate();

        unsafe {
            if g.game_values.spotlights && self.state == 1 {
                let x = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
                let y = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
                if self.sSpotlight.is_null() {
                    self.sSpotlight = gp.spotlightManager.add_spotlight(g, x, y, 3);
                }

                if !self.sSpotlight.is_null() {
                    self.sSpotlight.update_position(x, y);
                }
            }
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let srcY: i32 = self.iType as i32 * 32;
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        let src = SDL_Rect { x: self.drawframe as i32, y: srcY, w: self.iw as i32, h: self.ih as i32 };
        if !self.owner.is_null() && self.owner.iswarping() {
            let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(self.owner.get_warp_state() as i32) };
            self.spr.draw_clip(g, x, y, &src, edge, self.owner.get_warp_plane() as i32);
        } else {
            self.spr.draw_src(g, x, y, &src);
        }

        unsafe {
            if self.frozen {
                gp.rm.spr_iceblock.draw_src(g, x, y, &SDL_Rect { x: 0, y: 0, w: 32, h: 32 });
            } else if self.fSmoking {
                g.eyecandy[0].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut g.rm.spr_burnup),
                    (x + (self.iw as i32 >> 1) - 16) as i16,
                    (y + (self.ih as i32 >> 1) - 16) as i16,
                    5,
                    3,
                ));
            }
        }
    }
}

impl IO_MovingObjectTrait for CO_ThrowBlock {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }

    fn check_and_die(&mut self, g: &mut Game) {
        if self.fDieOnPlayerCollision {
            self.die(g);
        }
    }

    fn die(&mut self, g: &mut Game) {
        if self.frozen {
            self.shatter_die(g);
            return;
        }

        if self.dead {
            return;
        }

        let srcY = (self.iType as i32 * 16) as i16;
        unsafe {
            let spr = Ptr::from_mut(&mut g.rm.spr_brokenblueblock);
            let (ix, iy) = (self.ix as i32, self.iy as i32);
            g.eyecandy[2].emplace(EC_FallingObject::new(spr, ix as i16, iy as i16, -1.5, -7.0, 6, 2, 0, srcY, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(spr, (ix + 16) as i16, iy as i16, 1.5, -7.0, 6, 2, 0, srcY, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(spr, ix as i16, (iy + 16) as i16, -1.5, -4.0, 6, 2, 0, srcY, 16, 16));
            g.eyecandy[2].emplace(EC_FallingObject::new(spr, (ix + 16) as i16, (iy + 16) as i16, 1.5, -4.0, 6, 2, 0, srcY, 16, 16));
        }

        self.die_helper(g);
    }

    fn side_bounce(&mut self, g: &mut Game, _fRightSide: bool) {
        let mut gp = g.ptr();
        if self.fDieOnBounce {
            self.die(g);
        } else if self.state == 1 {
            if self.iBounceCounter == 0 {
                unsafe {
                    g.eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut g.rm.spr_shellbounce),
                        (self.ix as i32 + (if self.velx > 0.0 { 0 } else { self.collisionWidth as i32 }) - 21) as i16,
                        (self.iy as i32 + (self.collisionHeight as i32 >> 1) - 20) as i16,
                        4,
                        4,
                    ));
                    if_sound_on_play(g, &mut gp.rm.sfx_bump);
                }

                self.iBounceCounter = 7; // Allow bounce stars to show on each bounce on a 2x wide pit
            }
        }
    }
}

impl MO_CarriedObjectTrait for CO_ThrowBlock {
    crate::impl_carried_object_plumbing!();

    fn drop(&mut self, g: &mut Game) {
        self.kick(g);
    }

    fn kick(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        /*
        if (superkick)
        {
            vel = 12.0f;
            fSmoking = true;
            ifSoundOnPlay(game().rm->sfx_cannon);
        }
        */

        self.iDeathTime = 0;

        let mut fVel: f32;
        let fPlayerBonusVel: f32 = self.owner.velx / 2.0;
        if self.owner.is_facing_right(g) {
            fVel = 6.5;
            if fPlayerBonusVel > 0.0 {
                fVel += fPlayerBonusVel;
            }

            // if (fVel >= 9.0f)
            //	fSmoking = true;
        } else {
            fVel = -6.5;
            if fPlayerBonusVel < 0.0 {
                fVel += fPlayerBonusVel;
            }

            // if (fVel <= -9.0f)
            //	fSmoking = true;
        }

        self.velx = fVel;
        self.vely = 0.0;

        self.iPlayerID = self.owner.globalID;
        self.iTeamID = self.owner.teamID;

        self.owner = Ptr::null();
        self.iNoOwnerKillTime = 30;

        self.state = 1;

        if io_moving_object_collision_detection_checksides(g, self) {
            self.die(g);
        } else {
            unsafe { if_sound_on_play(g, &mut gp.rm.sfx_kicksound) };
        }
    }
}
