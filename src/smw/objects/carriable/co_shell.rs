//! Port of src/smw/objects/carriable/CO_Shell.cpp

use crate::common::eyecandy::{EC_FallingObject, EC_SingleAnimation, Spotlight};
use crate::common::eyecandy_styles::AwardStyle;
use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::gfx::gfx_sprite::ClipEdge;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::player_kill_styles::KillStyle;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::{eyecandy, spotlightManager};
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::moving::mo_carried_object::{MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_checksides, io_moving_object_update, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, player_killed_player, CPlayer, PlayerDeathStyle};
use sdl2::sys::SDL_Rect;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ShellType {
    Green,
    Red,
    Spiny,
    Buzzy,
}

//------------------------------------------------------------------------------
// class shell projectile
//------------------------------------------------------------------------------
// state 0: Shell is being spawned
// state 1: Shell is moving
// state 2: Shell is waiting to be picked up
// state 3: Shell is being held
pub struct CO_Shell {
    pub mo_carried_object: MO_CarriedObject,

    pub iShellType: ShellType,

    pub iIgnoreBounceTimer: i16,
    pub iDestY: i16,

    pub fDieOnMovingPlayerCollision: bool,
    pub fDieOnHoldingPlayerCollision: bool,
    pub fDieOnFire: bool,
    pub fKillBouncePlayer: bool,

    pub iDeathTime: i16,
    pub iNoOwnerKillTime: i16,

    pub iColorOffsetY: i16,
    pub iBounceCounter: i16,
    pub iKillCounter: i16,

    pub fFlipped: bool,
    pub iFlippedOffset: i16,

    pub frozen: bool,
    pub frozentimer: i16,
    pub frozenvelocity: f32,
    pub frozenanimationspeed: i16,

    pub sSpotlight: Ptr<Spotlight>,
}
impl_base!(CO_Shell => mo_carried_object: MO_CarriedObject);

impl CO_Shell {
    pub fn new(r#type: ShellType, pos: Vec2s, dieOnMovingPlayerCollision: bool, dieOnHoldingPlayerCollision: bool, dieOnFire: bool, killBouncePlayer: bool) -> Self {
        let mut o = CO_Shell {
            mo_carried_object: MO_CarriedObject::new_anim(unsafe { Ptr::from_mut(&mut rm.spr_shell) }, pos, 4, 4, 30, 20, 1, 11, 0, (r#type as i32 * 32) as i16, 32, 32),
            iShellType: r#type,
            iIgnoreBounceTimer: 0,
            iDestY: 0,
            fDieOnMovingPlayerCollision: false,
            fDieOnHoldingPlayerCollision: false,
            fDieOnFire: false,
            fKillBouncePlayer: false,
            iDeathTime: 0,
            iNoOwnerKillTime: 0,
            iColorOffsetY: 0,
            iBounceCounter: 0,
            iKillCounter: 0,
            fFlipped: false,
            iFlippedOffset: 0,
            frozen: false,
            frozentimer: 0,
            frozenvelocity: 0.0,
            frozenanimationspeed: 0,
            sSpotlight: Ptr::null(),
        };

        o.iShellType = r#type;

        o.state = 0;

        o.movingObjectType = movingobject_shell;

        o.iPlayerID = -1;
        o.iTeamID = -1;

        o.iIgnoreBounceTimer = 0;
        o.iBounceCounter = 0;

        o.fDieOnMovingPlayerCollision = dieOnMovingPlayerCollision;
        o.fDieOnHoldingPlayerCollision = dieOnHoldingPlayerCollision;
        o.fDieOnFire = dieOnFire;
        o.fKillBouncePlayer = killBouncePlayer;

        o.iDeathTime = 0;

        o.iDestY = (o.iy as i32 - o.collisionHeight as i32) as i16;
        o.fy = o.iDestY as f32 + 32.0;
        o.iColorOffsetY = (r#type as i32 * 32) as i16;

        o.iKillCounter = 0;
        o.iNoOwnerKillTime = 0;

        o.fFlipped = false;
        o.iFlippedOffset = 0;

        o.iOwnerRightOffset = 14;
        o.iOwnerLeftOffset = -22;
        o.iOwnerUpOffset = 32;

        o.frozen = false;
        o.frozentimer = 0;
        o.frozenvelocity = 0.0;
        o.frozenanimationspeed = 4;

        o.sSpotlight = Ptr::null();
        o
    }

    fn flipx_for(&self, player: Ptr<CPlayer>) -> i16 {
        let mut flipx: i16 = 0;

        if (player.ix as i32 + PW) < 320 && self.ix > 320 {
            flipx = App::screenWidth as i16;
        } else if (self.ix as i32 + self.iw as i32) < 320 && player.ix > 320 {
            flipx = -(App::screenWidth as i16);
        }

        flipx
    }

    pub fn hit_top(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        if player.is_invincible() || player.kuriboshoe.is_on() || player.shyguy {
            self.die();
            self.fSmoking = false;
            return false;
        }

        if self.fKillBouncePlayer && !self.fFlipped {
            self.kill_player(player);
        } else if self.state == 2 {
            // Sitting
            self.owner = player;
            self.kick();
            self.fSmoking = false;
            if (player.ix as i32 + HALFPW) < self.ix as i32 + (self.iw as i32 >> 1) {
                self.velx = 5.0;
            } else {
                self.velx = -5.0;
            }

            self.iIgnoreBounceTimer = 10;
        } else if self.state == 1 && self.iIgnoreBounceTimer == 0 {
            // Moving
            self.stop();

            player.set_yi((self.iy as i32 - PH - 1) as i16);
            player.bouncejump();
            let p = player.get();
            player.collisions.checktop(p);
            player.platform = Ptr::null();
        } else if self.state == 3 {
            // Holding
            if player != self.owner && (unsafe { game_values.teamcollision } == TeamCollisionStyle::On || player.teamID != self.owner.teamID) {
                if !self.owner.is_null() {
                    self.owner.carriedItem = Ptr::null();
                }

                self.kick();
                self.fSmoking = false;

                player.set_yi((self.iy as i32 - PH - 1) as i16);
                player.bouncejump();
                let p = player.get();
                player.collisions.checktop(p);
                player.platform = Ptr::null();
            }
        }

        false
    }

    pub fn hit_other(&mut self, player: Ptr<CPlayer>) -> bool {
        let mut player = player;
        if self.state == 2 {
            // Sitting
            if self.owner.is_null() && player.isready() {
                if player.accept_item(self.as_carried_ptr()) {
                    self.owner = player;
                    self.iPlayerID = self.owner.globalID;
                    self.iTeamID = self.owner.teamID;
                    self.state = 3;
                } else {
                    let flipx = self.flipx_for(player);

                    self.owner = player;
                    self.kick();
                    if (player.ix as i32 + HALFPW + flipx as i32) < self.ix as i32 + (self.iw as i32 >> 1) {
                        self.velx = 5.0;
                    } else {
                        self.velx = -5.0;
                    }
                }
            }
        } else if self.state == 1 {
            // Moving
            let flipx = self.flipx_for(player);

            let px = player.ix as i32 + HALFPW + flipx as i32;
            let cx = self.ix as i32 + (self.iw as i32 >> 1);
            if self.iNoOwnerKillTime == 0 || player.globalID != self.iPlayerID || (px >= cx && self.velx > 0.0) || (px < cx && self.velx < 0.0) {
                return self.kill_player(player);
            }
        } else if self.state == 3 {
            // Holding
            if player != self.owner && (unsafe { game_values.teamcollision } == TeamCollisionStyle::On || player.teamID != self.owner.teamID) {
                self.iPlayerID = self.owner.globalID;
                self.iTeamID = self.owner.teamID;
                return self.kill_player(player);
            }
        }

        false
    }

    pub fn used_as_stored_powerup(&mut self, player: Ptr<CPlayer>) {
        let mut player = player;
        self.owner = player;
        self.move_to_owner();

        if player.accept_item(self.as_carried_ptr()) {
            self.state = 3;
        } else {
            self.kick();
        }
    }

    pub fn kill_player(&mut self, player: Ptr<CPlayer>) -> bool {
        if player.is_shielded() || player.is_invincible() || player.shyguy {
            return false;
        }

        self.check_and_die();

        // Find the player that shot this shell so we can attribute a kill
        player_killed_player(self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Shell, false, false);

        let killer = get_player_from_global_id(self.iPlayerID);
        if !killer.is_null() && self.iPlayerID != player.globalID {
            self.add_moving_kill(killer);
        }

        true
    }

    pub fn add_moving_kill(&mut self, killer: Ptr<CPlayer>) {
        let mut killer = killer;
        if self.state == 1 && unsafe { game_values.awardstyle } != AwardStyle::None {
            // If the shell is moving, the keep track of how many people we kill in a row with it
            self.iKillCounter += 1;
            if self.iKillCounter > 1 {
                killer.add_kills_in_row_in_air_award();
            }
        }
    }

    pub fn shatter_die(&mut self) {
        self.dead = true;
        unsafe { if_sound_on_play(&mut rm.sfx_breakblock) };
        self.iKillCounter = 0;

        if !self.owner.is_null() {
            self.owner.carriedItem = Ptr::null();
            self.owner = Ptr::null();
        }

        let iBrokenIceX = (self.ix as i32 - self.collisionOffsetX as i32) as i16;
        let iBrokenIceY = (self.iy as i32 - self.collisionOffsetY as i32) as i16;
        unsafe {
            let spr = Ptr::from_mut(&mut rm.spr_brokeniceblock);
            eyecandy[2].emplace(EC_FallingObject::new(spr, iBrokenIceX, iBrokenIceY, -1.5, -7.0, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, (iBrokenIceX as i32 + 16) as i16, iBrokenIceY, 1.5, -7.0, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, iBrokenIceX, (iBrokenIceY as i32 + 16) as i16, -1.5, -4.0, 4, 2, 0, 0, 16, 16));
            eyecandy[2].emplace(EC_FallingObject::new(spr, (iBrokenIceX as i32 + 16) as i16, (iBrokenIceY as i32 + 16) as i16, 1.5, -4.0, 4, 2, 0, 0, 16, 16));
        }
    }

    pub fn is_threat(&self) -> bool {
        self.state == 1 || self.state == 3
    }

    pub fn flip(&mut self) {
        if self.frozen {
            self.shatter_die();
            return;
        }

        if !self.owner.is_null() {
            self.die();
            return;
        }

        if !self.fFlipped {
            self.fFlipped = true;
            self.iFlippedOffset = 128;
        }

        self.stop();
        self.vely = (-VELJUMP as f64 / 2.0) as f32;
    }

    fn stop(&mut self) {
        self.owner = Ptr::null();
        self.velx = 0.0;
        self.state = 2;
        self.fSmoking = false;
        unsafe { if_sound_on_play(&mut rm.sfx_kicksound) };
        self.iKillCounter = 0;
    }

    pub fn nospawn(&mut self, y: i16, fBounce: bool) {
        self.state = 2;
        self.set_yi(y);

        if fBounce {
            self.vely = (-VELJUMP as f64 / 2.0) as f32;
        }
    }
}

impl CObjectTrait for CO_Shell {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        if player.is_invincible() || player.shyguy || self.frozen {
            if self.frozen {
                self.shatter_die();
                return false;
            } else if self.state == 0 || self.state == 2 {
                // If sitting or spawning then just die
                self.die();
                return false;
            } else if self.state == 3 {
                // if held, but not by us then die
                if self.owner != player {
                    self.die();
                    return false;
                }
            } else if self.state == 1 {
                // If moving, see if it is actually hitting us before we kill it
                let flipx = self.flipx_for(player);

                let px = player.ix as i32 + HALFPW + flipx as i32;
                let cx = self.ix as i32 + (self.iw as i32 >> 1);
                if (px >= cx && self.velx > 0.0) || (px < cx && self.velx < 0.0) {
                    self.die();
                    return false;
                }
            }
        }

        if player.tanookisuit.not_statue() {
            if player.fOldY + PH as f32 <= self.iy as f32 && player.iy as i32 + PH >= self.iy as i32 {
                return self.hit_top(player);
            } else {
                return self.hit_other(player);
            }
        }

        false
    }

    fn collide_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) {
        let mut object = object;
        if object.is_dead() {
            return;
        }

        // Don't allow shells to die if they are warping
        if !self.owner.is_null() && self.owner.iswarping() {
            return;
        }

        removeifprojectile(object, false, false);

        let r#type = object.get_moving_object_type();

        if r#type == movingobject_shell {
            let mut shell: Ptr<CO_Shell> = Ptr::from_mut(object.as_any().downcast_mut::<CO_Shell>().unwrap());

            // Green shells should die on collision, other shells should not,
            // except if they also hit a non dead on collision shell

            if self.frozen || shell.frozen {
                self.die();
                shell.die();
            }
            if shell.fSmoking && !self.fSmoking {
                self.die();
            } else if !shell.fSmoking && self.fSmoking {
                shell.die();
            } else {
                if self.fDieOnMovingPlayerCollision || self.state == 2 || (!shell.fDieOnMovingPlayerCollision && shell.state != 2) {
                    self.die();
                }

                if shell.fDieOnMovingPlayerCollision || shell.state == 2 || (!self.fDieOnMovingPlayerCollision && self.state != 2) {
                    shell.die();
                }
            }
        } else if r#type == movingobject_throwblock || r#type == movingobject_throwbox {
            self.die();
            object.die();
        } else if r#type == movingobject_explosion || r#type == movingobject_sledgehammer || r#type == movingobject_superfireball {
            self.die();
        } else if r#type == movingobject_fireball || r#type == movingobject_hammer || r#type == movingobject_boomerang {
            if self.fDieOnFire {
                self.die();
            }
        } else if r#type == movingobject_iceblast {
            self.frozenvelocity = self.velx;
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

                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (self.ix as i32 - self.collisionOffsetX as i32) as i16,
                        (self.iy as i32 - self.collisionOffsetY as i32) as i16,
                        3,
                        8,
                    ));
                }
            } else if self.state == 1 {
                if game_values.shellttl > 0 && {
                    self.iDeathTime += 1;
                    self.iDeathTime >= game_values.shellttl
                } {
                    eyecandy[2].emplace(EC_SingleAnimation::new(Ptr::from_mut(&mut rm.spr_fireballexplosion), self.ix, self.iy, 3, 8));
                    self.dead = true;
                    if_sound_on_play(&mut rm.sfx_kicksound);

                    if !self.owner.is_null() {
                        self.owner.carriedItem = Ptr::null();
                        self.owner = Ptr::null();
                    }

                    return;
                }
            } else {
                self.iDeathTime = 0;
            }
        }

        // Have the powerup grow out of the powerup block
        if self.state == 0 {
            let fy = self.fy;
            self.set_yf(fy - 2.0);

            if self.fy <= self.iDestY as f32 {
                self.state = 2;
                self.vely = GRAVITATION;
                let iDestY = self.iDestY;
                self.set_yf(iDestY as f32);
            }

            return;
        }

        if self.iIgnoreBounceTimer > 0 {
            self.iIgnoreBounceTimer -= 1;
        }

        if self.iBounceCounter > 0 {
            self.iBounceCounter -= 1;
        }

        if !self.owner.is_null() {
            self.move_to_owner();
            self.inair = true;
        } else {
            io_moving_object_update(self);
        }

        unsafe {
            if game_values.spotlights {
                if self.state == 1 {
                    let x = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
                    let y = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
                    if self.sSpotlight.is_null() {
                        self.sSpotlight = spotlightManager.add_spotlight(x, y, 3);
                    }

                    if !self.sSpotlight.is_null() {
                        self.sSpotlight.update_position(x, y);
                    }
                } else {
                    self.sSpotlight = Ptr::null();
                }
            }
        }
    }

    fn draw(&mut self) {
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        if self.state == 0 {
            let h = (self.ih as f32 - self.fy + self.iDestY as f32) as i16;
            self.spr.draw_src(x, y, &SDL_Rect { x: 0, y: self.iColorOffsetY as i32, w: self.iw as i32, h: h as i32 });
        } else if !self.owner.is_null() {
            let src = SDL_Rect { x: 0, y: self.iColorOffsetY as i32 + self.iFlippedOffset as i32, w: self.iw as i32, h: self.ih as i32 };
            if self.owner.iswarping() {
                let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(self.owner.get_warp_state() as i32) };
                self.spr.draw_clip(x, y, &src, edge, self.owner.get_warp_plane() as i32);
            } else {
                self.spr.draw_src(x, y, &src);
            }
        } else if self.state == 2 {
            self.spr.draw_src(x, y, &SDL_Rect { x: 0, y: self.iColorOffsetY as i32 + self.iFlippedOffset as i32, w: self.iw as i32, h: self.ih as i32 });
        } else if self.state == 1 {
            self.spr.draw_src(x, y, &SDL_Rect { x: self.drawframe as i32, y: self.iColorOffsetY as i32 + self.iFlippedOffset as i32, w: self.iw as i32, h: self.ih as i32 });
        }

        unsafe {
            if self.frozen {
                rm.spr_iceblock.draw_src(x, y, &SDL_Rect { x: 0, y: 0, w: 32, h: 32 });
            } else if self.fSmoking {
                eyecandy[0].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_burnup),
                    (x + (self.iw as i32 >> 1) - 16) as i16,
                    (y + (self.ih as i32 >> 1) - 16) as i16,
                    5,
                    3,
                ));
            }
        }
    }
}

impl IO_MovingObjectTrait for CO_Shell {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }

    fn check_and_die(&mut self) {
        if (self.fDieOnMovingPlayerCollision && self.state == 1) || ((self.fDieOnHoldingPlayerCollision || self.fFlipped) && self.state == 3) {
            self.die();
        } else if !self.fDieOnHoldingPlayerCollision && self.state == 3 && RANDOM_INT(5) == 0 {
            self.die();
        }
    }

    fn die(&mut self) {
        if self.frozen {
            self.shatter_die();
            return;
        }

        unsafe {
            eyecandy[2].emplace(EC_FallingObject::new(
                Ptr::from_mut(&mut rm.spr_shelldead),
                self.ix,
                self.iy,
                -self.velx / 4.0,
                -VELJUMP / 2.0,
                1,
                0,
                (self.iShellType as i32 * 32) as i16,
                0,
                32,
                32,
            ));
            self.dead = true;
            if_sound_on_play(&mut rm.sfx_kicksound);
        }
        self.iKillCounter = 0;

        if !self.owner.is_null() {
            self.owner.carriedItem = Ptr::null();
            self.owner = Ptr::null();
        }
    }

    fn side_bounce(&mut self, _fRightSide: bool) {
        if self.state == 1 {
            if self.iBounceCounter == 0 {
                unsafe {
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_shellbounce),
                        (self.ix as i32 + (if self.velx > 0.0 { 0 } else { self.collisionWidth as i32 }) - 21) as i16,
                        (self.iy as i32 + (self.collisionHeight as i32 >> 1) - 20) as i16,
                        4,
                        4,
                    ));
                    if_sound_on_play(&mut rm.sfx_bump);
                }

                self.iBounceCounter = 7; // Allow bounce stars to show on each bounce on a 2x wide pit
            }
        }
    }
}

impl MO_CarriedObjectTrait for CO_Shell {
    crate::impl_carried_object_plumbing!();

    fn drop(&mut self) {
        if !self.owner.is_null() {
            self.owner.carriedItem = Ptr::null();
            let xi = (self.owner.ix as i32 + if self.owner.is_facing_right() { PW + 1 } else { -31 }) as i16;
            self.set_xi(xi);
        }

        if io_moving_object_collision_detection_checksides(self) {
            // Move back to where it was before checking sides, then kill it
            let xi = (self.owner.ix as i32 + if self.owner.is_facing_right() { PW + 1 } else { -31 }) as i16;
            self.set_xi(xi);
            let yi = (self.owner.iy as i32 + PH - 32 + self.collisionOffsetY as i32) as i16;
            self.set_yi(yi);
            self.die();
        } else {
            self.owner = Ptr::null();
            self.state = 2;
        }
    }

    fn kick(&mut self) {
        /*
        if (superkick)
        {
            vel = 10.0f;
            fSmoking = true;
            ifSoundOnPlay(rm->sfx_cannon);
        }
        */

        let mut fVel: f32;
        let fPlayerBonusVel: f32 = self.owner.velx / 2.0;
        if self.owner.is_facing_right() {
            fVel = 5.0;
            if fPlayerBonusVel > 0.0 {
                fVel += fPlayerBonusVel;
            }

            // if (fVel >= 7.5f)
            //	fSmoking = true;
        } else {
            fVel = -5.0;
            if fPlayerBonusVel < 0.0 {
                fVel += fPlayerBonusVel;
            }

            // if (fVel <= -7.5f)
            //	fSmoking = true;
        }

        self.velx = fVel;
        self.vely = 0.0;

        self.iPlayerID = self.owner.globalID;
        self.iTeamID = self.owner.teamID;

        self.owner = Ptr::null();
        self.iNoOwnerKillTime = 30;

        self.state = 1;

        if io_moving_object_collision_detection_checksides(self) {
            self.die();
        } else {
            unsafe { if_sound_on_play(&mut rm.sfx_kicksound) };
        }
    }
}
