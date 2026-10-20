//! Koopa, spiny and buzzy shells: carried, kicked across the floor, and deadly while moving.

use crate::smw::players::PlayerId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::map::PlatformId;
use crate::common::eyecandy::{EC_FallingObject, EC_SingleAnimation, SpotlightId};
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
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::moving::mo_carried_object::{MO_CarriedObject, MO_CarriedObjectTrait};
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_checksides, io_moving_object_update, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, player_killed_player, CPlayer, PlayerDeathStyle};

/// Shell kind; also selects the 32-pixel sprite row.
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
/// Shell that can be held, kicked, stomped to a stop, flipped or frozen.
#[derive(Clone)]
pub struct CO_Shell {
    pub mo_carried_object: MO_CarriedObject,

    pub iShellType: ShellType,

    /// Ticks after a kick from a stomp during which another stomp does not stop the shell.
    pub iIgnoreBounceTimer: i16,
    /// Y (pixels) where the rise out of the block ends.
    pub iDestY: i16,

    /// Breaks after hitting a player while moving, and loses shell-on-shell hits.
    pub fDieOnMovingPlayerCollision: bool,
    /// Breaks when its holder bumps a player with it; otherwise that has a 1 in 5 chance.
    pub fDieOnHoldingPlayerCollision: bool,
    /// Fireballs, hammers and boomerangs break it.
    pub fDieOnFire: bool,
    /// Kills players who stomp it unless it is flipped (spiny shells).
    pub fKillBouncePlayer: bool,

    /// Ticks spent moving; the shell disappears at the shell lifetime setting.
    pub iDeathTime: i16,
    /// Ticks after a kick during which the shell spares its kicker unless moving toward them.
    pub iNoOwnerKillTime: i16,

    /// Sprite row offset (pixels) for the shell kind.
    pub iColorOffsetY: i16,
    /// Ticks before another wall bounce shows the bounce effect.
    pub iBounceCounter: i16,
    /// Players killed in a row by this moving shell, for kill-streak awards.
    pub iKillCounter: i16,

    /// Turned upside down by a spin attack: safe to stomp, and breaks when its holder bumps a player.
    pub fFlipped: bool,
    /// Sprite row offset (pixels) for the flipped shell: 128 when flipped.
    pub iFlippedOffset: i16,

    /// Frozen in an ice block; touching it shatters it.
    pub frozen: bool,
    /// Ticks left frozen.
    pub frozentimer: i16,
    /// Horizontal speed restored when the ice thaws.
    pub frozenvelocity: f32,
    /// Animation speed restored when the ice thaws.
    pub frozenanimationspeed: i16,

    /// Spotlight that follows a moving shell while spotlights are on.
    pub sSpotlight: SpotlightId,
}
impl_base!(CO_Shell => mo_carried_object: MO_CarriedObject);

impl CO_Shell {
    pub fn new(g: &mut Game, r#type: ShellType, pos: Vec2s, dieOnMovingPlayerCollision: bool, dieOnHoldingPlayerCollision: bool, dieOnFire: bool, killBouncePlayer: bool) -> Self {
        let mut rm = g.rm;
        let mut o = CO_Shell {
            mo_carried_object: MO_CarriedObject::new_anim(g, rm.spr_shell, pos, 4, 4, 30, 20, 1, 11, 0, (r#type as i32 * 32) as i16, 32, 32),
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
            sSpotlight: SpotlightId::NONE,
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

        o.sSpotlight = SpotlightId::NONE;
        o
    }

    /// Screen-width offset that puts the shell on the same side of the wrap as `player`, for direction checks.
    fn flipx_for(&self, g: &Game, player: PlayerId) -> i16 {
        let mut flipx: i16 = 0;

        if (g.player(player).ix as i32 + PW) < 320 && self.ix > 320 {
            flipx = App::screenWidth as i16;
        } else if (self.ix as i32 + self.iw as i32) < 320 && g.player(player).ix > 320 {
            flipx = -(App::screenWidth as i16);
        }

        flipx
    }

    /// A player landed on the shell: kick it, stop it, knock it out of a holder's hands, or die on a spiny.
    pub fn hit_top(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut player = player;
        if g.player(player).is_invincible() || g.player(player).kuriboshoe.is_on() || g.player(player).shyguy {
            self.die(g);
            self.fSmoking = false;
            return false;
        }

        if self.fKillBouncePlayer && !self.fFlipped {
            self.kill_player(g, player);
        } else if self.state == 2 {
            // Sitting
            self.owner = player;
            self.kick(g);
            self.fSmoking = false;
            if (g.player(player).ix as i32 + HALFPW) < self.ix as i32 + (self.iw as i32 >> 1) {
                self.velx = vel(5.0);
            } else {
                self.velx = vel(-5.0);
            }

            self.iIgnoreBounceTimer = ticks(10) as i16;
        } else if self.state == 1 && self.iIgnoreBounceTimer == 0 {
            // Moving
            self.stop(g);

            g.player(player).set_yi((self.iy as i32 - PH - 1) as i16);
            g.player(player).bouncejump(g);
            let p = g.player(player);
            g.player(player).collisions.checktop(g, p);
            g.player(player).platform = PlatformId::NONE;
        } else if self.state == 3 {
            // Holding
            if player != self.owner && (g.game_values.teamcollision == TeamCollisionStyle::On || g.player(player).teamID != g.player(self.owner).teamID) {
                if !self.owner.is_none() {
                    g.player(self.owner).carriedItem = crate::smw::object_container::ObjectId::NONE;
                }

                self.kick(g);
                self.fSmoking = false;

                g.player(player).set_yi((self.iy as i32 - PH - 1) as i16);
                g.player(player).bouncejump(g);
                let p = g.player(player);
                g.player(player).collisions.checktop(g, p);
                g.player(player).platform = PlatformId::NONE;
            }
        }

        false
    }

    /// Side contact: pick up or kick a sitting shell; a moving or held shell kills the player.
    pub fn hit_other(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut player = player;
        if self.state == 2 {
            // Sitting
            if self.owner.is_none() && g.player(player).isready() {
                if g.player(player).accept_item(self.as_carried_ptr()) {
                    self.owner = player;
                    self.iPlayerID = g.player(self.owner).globalID;
                    self.iTeamID = g.player(self.owner).teamID;
                    self.state = 3;
                } else {
                    let flipx = self.flipx_for(g, player);

                    self.owner = player;
                    self.kick(g);
                    if (g.player(player).ix as i32 + HALFPW + flipx as i32) < self.ix as i32 + (self.iw as i32 >> 1) {
                        self.velx = vel(5.0);
                    } else {
                        self.velx = vel(-5.0);
                    }
                }
            }
        } else if self.state == 1 {
            // Moving
            let flipx = self.flipx_for(g, player);

            let px = g.player(player).ix as i32 + HALFPW + flipx as i32;
            let cx = self.ix as i32 + (self.iw as i32 >> 1);
            if self.iNoOwnerKillTime == 0 || g.player(player).globalID != self.iPlayerID || (px >= cx && self.velx > 0.0) || (px < cx && self.velx < 0.0) {
                return self.kill_player(g, player);
            }
        } else if self.state == 3 {
            // Holding
            if player != self.owner && (g.game_values.teamcollision == TeamCollisionStyle::On || g.player(player).teamID != g.player(self.owner).teamID) {
                self.iPlayerID = g.player(self.owner).globalID;
                self.iTeamID = g.player(self.owner).teamID;
                return self.kill_player(g, player);
            }
        }

        false
    }

    /// Puts the shell in `player`'s hands, or kicks it if they cannot hold it.
    pub fn used_as_stored_powerup(&mut self, g: &mut Game, player: PlayerId) {
        let mut player = player;
        self.owner = player;
        self.move_to_owner(g);

        if g.player(player).accept_item(self.as_carried_ptr()) {
            self.state = 3;
        } else {
            self.kick(g);
        }
    }

    /// Kills an unprotected player and credits the shell's owner; returns true on a kill.
    pub fn kill_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        if g.player(player).is_shielded() || g.player(player).is_invincible() || g.player(player).shyguy {
            return false;
        }

        self.check_and_die(g);

        // Find the player that shot this shell so we can attribute a kill
        player_killed_player(g, self.iPlayerID, player, PlayerDeathStyle::Jump, KillStyle::Shell, false, false);

        let killer = get_player_from_global_id(g, self.iPlayerID);
        if !killer.is_none() && self.iPlayerID != g.player(player).globalID {
            self.add_moving_kill(g, killer);
        }

        true
    }

    /// Counts a kill by a moving shell toward `killer`'s kill-streak award.
    pub fn add_moving_kill(&mut self, g: &mut Game, killer: PlayerId) {
        let mut killer = killer;
        if self.state == 1 && g.game_values.awardstyle != AwardStyle::None {
            // If the shell is moving, the keep track of how many people we kill in a row with it
            self.iKillCounter += 1;
            if self.iKillCounter > 1 {
                g.player(killer).add_kills_in_row_in_air_award(g);
            }
        }
    }

    /// Breaks a frozen shell into ice fragments and frees its holder.
    pub fn shatter_die(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.dead = true;
        if_sound_on_play(g, &mut gp.rm.sfx_breakblock);
        self.iKillCounter = 0;

        if !self.owner.is_none() {
            g.player(self.owner).carriedItem = crate::smw::object_container::ObjectId::NONE;
            self.owner = PlayerId::NONE;
        }

        let iBrokenIceX = (self.ix as i32 - self.collisionOffsetX as i32) as i16;
        let iBrokenIceY = (self.iy as i32 - self.collisionOffsetY as i32) as i16;
        let spr = g.rm.spr_brokeniceblock;
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, iBrokenIceX, iBrokenIceY, vel(-1.5), vel(-7.0), 4, 2, 0, 0, 16, 16));
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, (iBrokenIceX as i32 + 16) as i16, iBrokenIceY, vel(1.5), vel(-7.0), 4, 2, 0, 0, 16, 16));
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, iBrokenIceX, (iBrokenIceY as i32 + 16) as i16, vel(-1.5), vel(-4.0), 4, 2, 0, 0, 16, 16));
        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, (iBrokenIceX as i32 + 16) as i16, (iBrokenIceY as i32 + 16) as i16, vel(1.5), vel(-4.0), 4, 2, 0, 0, 16, 16));
    }

    /// The shell is moving or held, so it can kill.
    pub fn is_threat(&self) -> bool {
        self.state == 1 || self.state == 3
    }

    /// Turns the shell upside down and pops it up; a held shell breaks and a frozen one shatters.
    pub fn flip(&mut self, g: &mut Game) {
        if self.frozen {
            self.shatter_die(g);
            return;
        }

        if !self.owner.is_none() {
            self.die(g);
            return;
        }

        if !self.fFlipped {
            self.fFlipped = true;
            self.iFlippedOffset = 128;
        }

        self.stop(g);
        self.vely = (-VELJUMP as f64 / 2.0) as f32;
    }

    /// Stops the shell where it is, sitting and free to pick up.
    fn stop(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.owner = PlayerId::NONE;
        self.velx = 0.0;
        self.state = 2;
        self.fSmoking = false;
        if_sound_on_play(g, &mut gp.rm.sfx_kicksound);
        self.iKillCounter = 0;
    }

    /// Skips the rise out of a block: the shell sits at `y`, popping up if `fBounce`.
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

    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        if g.player(player).is_invincible() || g.player(player).shyguy || self.frozen {
            if self.frozen {
                self.shatter_die(g);
                return false;
            } else if self.state == 0 || self.state == 2 {
                // If sitting or spawning then just die
                self.die(g);
                return false;
            } else if self.state == 3 {
                // if held, but not by us then die
                if self.owner != player {
                    self.die(g);
                    return false;
                }
            } else if self.state == 1 {
                // If moving, see if it is actually hitting us before we kill it
                let flipx = self.flipx_for(g, player);

                let px = g.player(player).ix as i32 + HALFPW + flipx as i32;
                let cx = self.ix as i32 + (self.iw as i32 >> 1);
                if (px >= cx && self.velx > 0.0) || (px < cx && self.velx < 0.0) {
                    self.die(g);
                    return false;
                }
            }
        }

        if g.player(player).tanookisuit.not_statue() {
            if g.player(player).fOldY + PH as f32 <= self.iy as f32 && g.player(player).iy as i32 + PH >= self.iy as i32 {
                return self.hit_top(g, player);
            } else {
                return self.hit_other(g, player);
            }
        }

        false
    }

    fn collide_object(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) {
        let mut object = object;
        if object.is_dead() {
            return;
        }

        // Don't allow shells to die if they are warping
        if !self.owner.is_none() && g.player(self.owner).iswarping() {
            return;
        }

        removeifprojectile(g, object, false, false);

        let r#type = object.get_moving_object_type();

        if r#type == movingobject_shell {
            let mut shell: Ptr<CO_Shell> = Ptr::from_mut(object.as_any().downcast_mut::<CO_Shell>().unwrap());

            // Green shells should die on collision, other shells should not,
            // except if they also hit a non dead on collision shell

            if self.frozen || shell.frozen {
                self.die(g);
                shell.die(g);
            } else if shell.fSmoking && !self.fSmoking {
                self.die(g);
            } else if !shell.fSmoking && self.fSmoking {
                shell.die(g);
            } else {
                if self.fDieOnMovingPlayerCollision || self.state == 2 || (!shell.fDieOnMovingPlayerCollision && shell.state != 2) {
                    self.die(g);
                }

                if shell.fDieOnMovingPlayerCollision || shell.state == 2 || (!self.fDieOnMovingPlayerCollision && self.state != 2) {
                    shell.die(g);
                }
            }
        } else if r#type == movingobject_throwblock || r#type == movingobject_throwbox {
            self.die(g);
            object.die(g);
        } else if r#type == movingobject_explosion || r#type == movingobject_sledgehammer || r#type == movingobject_superfireball {
            self.die(g);
        } else if r#type == movingobject_fireball || r#type == movingobject_hammer || r#type == movingobject_boomerang {
            if self.fDieOnFire {
                self.die(g);
            }
        } else if r#type == movingobject_iceblast {
            self.frozenvelocity = self.velx;
            self.velx = 0.0;
            self.animationspeed = 0;

            self.frozen = true;
            self.frozentimer = ticks(300) as i16;

            g.eyecandy[2].emplace(EC_SingleAnimation::new(&g.rm, 
                g.rm.spr_fireballexplosion,
                (self.ix as i32 - self.collisionOffsetX as i32) as i16,
                (self.iy as i32 - self.collisionOffsetY as i32) as i16,
                3,
                8,
            ));
        }
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.iNoOwnerKillTime > 0 {
            self.iNoOwnerKillTime -= 1;
        }

        if self.frozen {
            self.frozentimer -= 1;
            if self.frozentimer <= 0 {
                self.frozentimer = 0;
                self.frozen = false;

                self.velx = self.frozenvelocity;
                self.animationspeed = self.frozenanimationspeed;

                g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                    g.rm.spr_fireballexplosion,
                    (self.ix as i32 - self.collisionOffsetX as i32) as i16,
                    (self.iy as i32 - self.collisionOffsetY as i32) as i16,
                    3,
                    8,
                ));
            }
        } else if self.state == 1 {
            if g.game_values.shellttl > 0 && {
                self.iDeathTime += 1;
                self.iDeathTime >= g.game_values.shellttl
            } {
                g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, g.rm.spr_fireballexplosion, self.ix, self.iy, 3, 8));
                self.dead = true;
                if_sound_on_play(g, &mut gp.rm.sfx_kicksound);

                if !self.owner.is_none() {
                    g.player(self.owner).carriedItem = crate::smw::object_container::ObjectId::NONE;
                    self.owner = PlayerId::NONE;
                }

                return;
            }
        } else {
            self.iDeathTime = 0;
        }

        // Have the powerup grow out of the powerup block
        if self.state == 0 {
            let fy = self.fy;
            self.set_yf(fy - vel(2.0));

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

        if !self.owner.is_none() {
            self.move_to_owner(g);
            self.inair = true;
        } else {
            io_moving_object_update(g, self);
        }

        if g.game_values.spotlights {
            if self.state == 1 {
                let x = (self.ix as i32 - self.collisionOffsetX as i32 + (self.iw as i32 >> 1)) as i16;
                let y = (self.iy as i32 - self.collisionOffsetY as i32 + (self.ih as i32 >> 1)) as i16;
                if self.sSpotlight.is_none() {
                    self.sSpotlight = gp.spotlightManager.add_spotlight(g, x, y, 3);
                }

                if !self.sSpotlight.is_none() {
                    g.spotlightManager.update_position(self.sSpotlight, x, y);
                }
            } else {
                self.sSpotlight = SpotlightId::NONE;
            }
        }
    }

    fn draw(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let x = self.ix as i32 - self.collisionOffsetX as i32;
        let y = self.iy as i32 - self.collisionOffsetY as i32;
        if self.state == 0 {
            let h = (self.ih as f32 - self.fy + self.iDestY as f32) as i16;
            g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: 0, y: self.iColorOffsetY as i32, w: self.iw as i32, h: h as i32 });
        } else if !self.owner.is_none() {
            let src = Rect { x: 0, y: self.iColorOffsetY as i32 + self.iFlippedOffset as i32, w: self.iw as i32, h: self.ih as i32 };
            if g.player(self.owner).iswarping() {
                let edge: ClipEdge = unsafe { std::mem::transmute::<i32, ClipEdge>(g.player(self.owner).get_warp_state() as i32) };
                g.rm.sprite(self.spr).draw_clip(g, x, y, &src, edge, g.player(self.owner).get_warp_plane() as i32);
            } else {
                g.rm.sprite(self.spr).draw_src(g, x, y, &src);
            }
        } else if self.state == 2 {
            g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: 0, y: self.iColorOffsetY as i32 + self.iFlippedOffset as i32, w: self.iw as i32, h: self.ih as i32 });
        } else if self.state == 1 {
            g.rm.sprite(self.spr).draw_src(g, x, y, &Rect { x: self.drawframe as i32, y: self.iColorOffsetY as i32 + self.iFlippedOffset as i32, w: self.iw as i32, h: self.ih as i32 });
        }

        if self.frozen {
            gp.rm.sprite(gp.rm.spr_iceblock).draw_src(g, x, y, &Rect { x: 0, y: 0, w: 32, h: 32 });
        } else if self.fSmoking {
            g.eyecandy[0].emplace(EC_SingleAnimation::new(&gp.rm, 
                g.rm.spr_burnup,
                (x + (self.iw as i32 >> 1) - 16) as i16,
                (y + (self.ih as i32 >> 1) - 16) as i16,
                5,
                3,
            ));
        }
    }
}

impl CO_Shell {
    /// Breaks the shell with a 1 in 5 chance.
    fn maybe_die(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if RANDOM_INT(gp.get(), 5) == 0 {
            self.die(g);
        }
    }
}

impl IO_MovingObjectTrait for CO_Shell {
    crate::impl_io_moving_object_plumbing!();
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        Some(self)
    }

    fn check_and_die(&mut self, g: &mut Game) {
        if (self.fDieOnMovingPlayerCollision && self.state == 1) || ((self.fDieOnHoldingPlayerCollision || self.fFlipped) && self.state == 3) {
            self.die(g);
        } else if !self.fDieOnHoldingPlayerCollision && self.state == 3 {
            let holder = if self.owner.is_none() { -1 } else { g.player(self.owner).globalID as i32 };
            self.maybe_die(g);
        }
    }

    fn die(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.frozen {
            self.shatter_die(g);
            return;
        }

        g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, 
            g.rm.spr_shelldead,
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
        if_sound_on_play(g, &mut gp.rm.sfx_kicksound);
        self.iKillCounter = 0;

        if !self.owner.is_none() {
            g.player(self.owner).carriedItem = crate::smw::object_container::ObjectId::NONE;
            self.owner = PlayerId::NONE;
        }
    }

    fn side_bounce(&mut self, g: &mut Game, _fRightSide: bool) {
        let mut gp = g.ptr();
        if self.state == 1 {
            if self.iBounceCounter == 0 {
                g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                    g.rm.spr_shellbounce,
                    (self.ix as i32 + (if self.velx > 0.0 { 0 } else { self.collisionWidth as i32 }) - 21) as i16,
                    (self.iy as i32 + (self.collisionHeight as i32 >> 1) - 20) as i16,
                    4,
                    4,
                ));
                if_sound_on_play(g, &mut gp.rm.sfx_bump);

                self.iBounceCounter = 7; // Allow bounce stars to show on each bounce on a 2x wide pit
            }
        }
    }
}

impl MO_CarriedObjectTrait for CO_Shell {
    crate::impl_carried_object_plumbing!();

    fn drop(&mut self, g: &mut Game) {
        if !self.owner.is_none() {
            g.player(self.owner).carriedItem = crate::smw::object_container::ObjectId::NONE;
            let xi = (g.player(self.owner).ix as i32 + if g.player(self.owner).is_facing_right(g) { PW + 1 } else { -31 }) as i16;
            self.set_xi(xi);
        }

        if io_moving_object_collision_detection_checksides(g, self) {
            // Move back to where it was before checking sides, then kill it
            let xi = (g.player(self.owner).ix as i32 + if g.player(self.owner).is_facing_right(g) { PW + 1 } else { -31 }) as i16;
            self.set_xi(xi);
            let yi = (g.player(self.owner).iy as i32 + PH - 32 + self.collisionOffsetY as i32) as i16;
            self.set_yi(yi);
            self.die(g);
        } else {
            self.owner = PlayerId::NONE;
            self.state = 2;
        }
    }

    fn kick(&mut self, g: &mut Game) {
        let mut gp = g.ptr();

        let mut fVel: f32;
        let fPlayerBonusVel: f32 = g.player(self.owner).velx / 2.0;
        if g.player(self.owner).is_facing_right(g) {
            fVel = vel(5.0);
            if fPlayerBonusVel > 0.0 {
                fVel += fPlayerBonusVel;
            }
        } else {
            fVel = vel(-5.0);
            if fPlayerBonusVel < 0.0 {
                fVel += fPlayerBonusVel;
            }
        }

        self.velx = fVel;
        self.vely = 0.0;

        self.iPlayerID = g.player(self.owner).globalID;
        self.iTeamID = g.player(self.owner).teamID;

        self.owner = PlayerId::NONE;
        self.iNoOwnerKillTime = ticks(30) as i16;

        self.state = 1;

        if io_moving_object_collision_detection_checksides(g, self) {
            self.die(g);
        } else {
            if_sound_on_play(g, &mut gp.rm.sfx_kicksound);
        }
    }
}
