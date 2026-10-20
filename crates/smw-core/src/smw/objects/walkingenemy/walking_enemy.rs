//! Shared walking-enemy behavior: spawn-in effect, ledge turning, freezing, burnup and weapon hits.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::map::PlatformId;
use crate::common::eyecandy::{EC_FallingObject, EC_SingleAnimation};
use crate::common::game::App;
use crate::common::game_mode::game_mode_stomp;
use crate::common::game_values::if_sound_on_play;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::trig::{cosf, sinf};
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::tile_types::{tile_flag_nonsolid, tile_flag_solid, tile_flag_solid_on_top, tile_flag_super_death_top};
use crate::globals::*;
use crate::impl_base;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::carriable::co_shell::CO_Shell;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::{io_moving_object_draw, io_moving_object_update, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, CPlayer};
use std::ops::{Deref, DerefMut};

//------------------------------------------------------------------------------
// class walking enemy (base class for goomba and koopa)
//------------------------------------------------------------------------------
/// Base state for walking enemies. `state` 0 is the spawn-in effect (no collisions), 1 is active.
#[derive(Clone)]
pub struct MO_WalkingEnemy {
    pub io_moving_object: IO_MovingObject,

    /// Radius (pixels) of the ring of spawn icons; the enemy goes active once it shrinks below 10.
    pub spawnradius: f32,
    /// Rotation (radians) of the ring of spawn icons.
    pub spawnangle: f32,

    /// X offset (pixels) of this enemy's 16x16 icon in the spawn-effect sprite.
    pub iSpawnIconOffset: i16,
    /// Kill style credited to a player who kills this enemy.
    pub killStyle: KillStyle,

    /// Ticks spent at terminal fall speed; smokes after `ticks(20)` and dies after `ticks(80)`.
    pub burnuptimer: i16,
    /// Fireballs, hammers and boomerangs kill this enemy even when it is not frozen.
    pub fKillOnWeakWeapon: bool,
    /// Winged variant that hops; the first stomp only clips its wings.
    pub fBouncing: bool,
    /// Walks off ledges; when false the enemy turns around at gaps and deadly tiles.
    pub fFallOffLedges: bool,

    /// Frozen in an ice block by an ice blast; any player or weapon hit shatters it.
    pub frozen: bool,
    /// Ticks left frozen.
    pub frozentimer: i16,
    /// Horizontal speed restored when the ice thaws.
    pub frozenvelocity: f32,
    /// Animation speed restored when the ice thaws.
    pub frozenanimationspeed: i16,
}
impl_base!(MO_WalkingEnemy => io_moving_object: IO_MovingObject);

impl MO_WalkingEnemy {
    /// Places the enemy at a random enemy spawn point and starts the spawn-in effect.
    #[allow(clippy::too_many_arguments)]
    pub fn new(g: &mut Game, 
        nspr: SpriteId,
        iNumSpr: i16,
        aniSpeed: i16,
        iCollisionWidth: i16,
        iCollisionHeight: i16,
        iCollisionOffsetX: i16,
        iCollisionOffsetY: i16,
        iAnimationOffsetX: i16,
        iAnimationOffsetY: i16,
        iAnimationHeight: i16,
        iAnimationWidth: i16,
        moveToRight: bool,
        killOnWeakWeapon: bool,
        bouncing: bool,
        fallOffLedges: bool,
    ) -> Self {
        let mut gp = g.ptr();
        let mut o = MO_WalkingEnemy {
            io_moving_object: IO_MovingObject::new(
                g,
                nspr,
                Vec2s::zero(),
                iNumSpr,
                aniSpeed,
                iCollisionWidth,
                iCollisionHeight,
                iCollisionOffsetX,
                iCollisionOffsetY,
                iAnimationOffsetX,
                iAnimationOffsetY,
                iAnimationHeight,
                iAnimationWidth,
            ),
            spawnradius: 0.0,
            spawnangle: 0.0,
            iSpawnIconOffset: 0,
            killStyle: KillStyle::default(),
            burnuptimer: 0,
            fKillOnWeakWeapon: false,
            fBouncing: false,
            fFallOffLedges: false,
            frozen: false,
            frozentimer: 0,
            frozenvelocity: 0.0,
            frozenanimationspeed: 0,
        };

        if moveToRight {
            o.velx = vel(1.0);
        } else {
            o.velx = vel(-1.0);
        }

        o.movingObjectType = movingobject_none;

        o.fBouncing = bouncing;
        if o.fBouncing {
            o.bounce = -VELENEMYBOUNCE;
        } else {
            o.bounce = GRAVITATION;
        }

        o.spawnradius = 100.0f32;
        o.spawnangle = RANDOM_INT(gp.get(), 1000) as f32 * 0.00628f32;
        o.inair = true;

        o.iSpawnIconOffset = 0;

        o.burnuptimer = 0;

        o.fKillOnWeakWeapon = killOnWeakWeapon;

        o.frozen = false;
        o.frozentimer = 0;
        o.frozenvelocity = o.velx;
        o.frozenanimationspeed = aniSpeed;

        o.fFallOffLedges = fallOffLedges;

        // Always the base placement; a subclass `place` does not run here.
        mo_walking_enemy_place(g, &mut o);
        o
    }

    pub fn get_kill_style(&self) -> KillStyle {
        self.killStyle
    }
}

/// Draws the ring of spawn icons while spawning, else the sprite plus the ice block when frozen.
pub fn mo_walking_enemy_draw<T: MO_WalkingEnemyTrait + ?Sized>(g: &mut Game, this: &mut T) {
        let mut gp = g.ptr();
    if this.we().state == 0 {
        let o = this.we();
        let numeyecandy: i16 = 8;
        let addangle: f32 = TWO_PI / numeyecandy as f32;
        let mut displayangle: f32 = o.spawnangle;

        for _k in 0..numeyecandy {
            let spawnX: i16 = (o.ix as i32 + (o.collisionWidth as i32 >> 1) - 8 + (o.spawnradius * cosf(displayangle)) as i16 as i32) as i16;
            let spawnY: i16 = (o.iy as i32 + (o.collisionHeight as i32 >> 1) - 8 + (o.spawnradius * sinf(displayangle)) as i16 as i32) as i16;

            displayangle += addangle;

            gp.rm.sprite(gp.rm.spr_awardsouls).draw_src(g, spawnX as i32, spawnY as i32, &Rect { x: o.iSpawnIconOffset as i32, y: 0, w: 16, h: 16 });
        }
    } else {
        io_moving_object_draw(g, this);

        let o = this.we();
        if o.frozen {
            gp.rm.sprite(gp.rm.spr_iceblock).draw_src(
                g,
                o.ix as i32 - o.collisionOffsetX as i32 + o.iw as i32 - 32,
                o.iy as i32 - o.collisionOffsetY as i32 + o.ih as i32 - 32,
                &Rect { x: 0, y: 0, w: 32, h: 32 },
            );
        }
    }
}

/// Thaws, runs the spawn-in or map physics, burns up at terminal velocity, and turns around at ledges.
pub fn mo_walking_enemy_update<T: MO_WalkingEnemyTrait + ?Sized>(g: &mut Game, this: &mut T) {
    {
        let o = this.we_mut();
        if o.frozen {
            o.frozentimer -= 1;
            if o.frozentimer <= 0 {
                o.frozentimer = 0;
                o.frozen = false;

                o.velx = o.frozenvelocity;
                o.animationspeed = o.frozenanimationspeed;

                if o.fBouncing {
                    o.bounce = -VELENEMYBOUNCE;
                }

                g.eyecandy[2].emplace(EC_SingleAnimation::new(&g.rm, 
                    g.rm.spr_fireballexplosion,
                    (o.ix as i32 - o.collisionOffsetX as i32 + o.iw as i32 - 32) as i16,
                    (o.iy as i32 - o.collisionOffsetY as i32 + o.ih as i32 - 32) as i16,
                    3,
                    8,
                ));
            }
        }
    }

    if this.we().state == 0 {
        let o = this.we_mut();
        o.spawnradius -= vel(2.0);
        o.spawnangle += vel(0.05);

        if o.spawnradius < 10.0f32 {
            o.state = 1;
        }
    } else {
        io_moving_object_update(g, this);
    }

    // Deal with terminal burnup velocity
    if this.we().vely >= MAXVELY {
        this.we_mut().burnuptimer += 1;
        if this.we().burnuptimer as i32 > ticks(20) {
            if this.we().burnuptimer as i32 > ticks(80) {
                this.kill_object_map_hazard(g, -1);
            } else {
                let o = this.we();
                g.eyecandy[0].emplace(EC_SingleAnimation::new(&g.rm, 
                    g.rm.spr_burnup,
                    (o.ix as i32 + (o.collisionWidth as i32 >> 1) - 16) as i16,
                    (o.iy as i32 + (o.collisionHeight as i32 >> 1) - 16) as i16,
                    5,
                    4,
                ));
            }
        }
    } else {
        this.we_mut().burnuptimer = 0;
    }

    // If this enemy doesn't fall off of ledges, then take a look at the area in front of them
    // to determine if they need to turn around
    let o = this.we_mut();
    if !o.inair && !o.fFallOffLedges {
        let mut probeCenterX: i16 = (o.ix as i32 + (o.collisionWidth as i32 >> 1)) as i16;
        let mut probeFrontX: i16 = (o.ix as i32 + if o.velx > 0.0f32 { o.collisionWidth as i32 + 1 } else { -1 }) as i16;
        let mut probeY: i16 = (o.iy as i32 + o.collisionHeight as i32 + 5) as i16;

        if !o.platform.is_none() {
            let iFrontTileType: i32 = g.g_map.platform(o.platform).get_tile_type_from_coord(probeFrontX, probeY);
            let iCenterTileType: i32 = g.g_map.platform(o.platform).get_tile_type_from_coord(probeCenterX, probeY);

            let fFrontGap = iFrontTileType == tile_flag_nonsolid || iFrontTileType == tile_flag_super_death_top;
            let fCenterGap = iCenterTileType == tile_flag_nonsolid || iCenterTileType == tile_flag_super_death_top;

            // If there is a hole or the type will kill the enemy, then turn around
            if fFrontGap && fCenterGap {
                o.velx = -o.velx;
            }
        } else {
            if probeFrontX as i32 >= App::screenWidth {
                probeFrontX = (probeFrontX as i32 - App::screenWidth) as i16;
            } else if probeFrontX < 0 {
                probeFrontX = (probeFrontX as i32 + App::screenWidth) as i16;
            }

            if probeCenterX as i32 >= App::screenWidth {
                probeCenterX = (probeCenterX as i32 - App::screenWidth) as i16;
            } else if probeCenterX < 0 {
                probeCenterX = (probeCenterX as i32 + App::screenWidth) as i16;
            }

            if probeFrontX >= 0
                && (probeFrontX as i32) < App::screenWidth
                && probeCenterX >= 0
                && (probeCenterX as i32) < App::screenWidth
                && probeY >= 0
                && (probeY as i32) < App::screenHeight
            {
                probeFrontX = (probeFrontX as i32 / TILESIZE) as i16;
                probeCenterX = (probeCenterX as i32 / TILESIZE) as i16;
                probeY = (probeY as i32 / TILESIZE) as i16;

                let mut frontBlock = g.map_block(probeFrontX, probeY);
                let mut centerBlock = g.map_block(probeCenterX, probeY);

                let fFoundFrontBlock = !frontBlock.is_null() && !frontBlock.is_transparent() && !frontBlock.is_hidden();
                let fFoundCenterBlock = !centerBlock.is_null() && !centerBlock.is_transparent() && !centerBlock.is_hidden();

                if !fFoundFrontBlock && !fFoundCenterBlock {
                    let frontTile: i32 = g.g_map.map(probeFrontX as i32, probeY as i32);
                    let centerTile: i32 = g.g_map.map(probeCenterX as i32, probeY as i32);

                    let fFrontGap = (frontTile & tile_flag_super_death_top) != 0 || ((frontTile & tile_flag_solid) == 0 && (frontTile & tile_flag_solid_on_top) == 0);
                    let fCenterGap = (centerTile & tile_flag_super_death_top) != 0 || ((centerTile & tile_flag_solid) == 0 && (centerTile & tile_flag_solid_on_top) == 0);

                    if fFrontGap && fCenterGap {
                        o.velx = -o.velx;
                    }
                }
            }
        }
    }
}

/// Touching an invincible player, or being frozen, kills the enemy; otherwise a stomp calls `hittop_player`, any other touch `hitother`.
pub fn mo_walking_enemy_collide_player<T: MO_WalkingEnemyTrait + ?Sized>(g: &mut Game, this: &mut T, player: PlayerId) -> bool {
        let mut gp = g.ptr();
    let mut player = player;
    if this.we().state == 0 {
        return false;
    }

    if g.player(player).is_invincible() || this.we().frozen {
        let killStyle = this.we().killStyle;
        g.player(player).add_killer_award(g, PlayerId::NONE, killStyle);

        if g.mode().gamemode == game_mode_stomp && !g.mode().gameover {
            g.player(player).score(g).adjust_score(g, 1);
        }

        if this.we().frozen {
            this.shatter_die(g);
        } else {
            if_sound_on_play(g, &mut gp.rm.sfx_kicksound);
            this.die(g);
        }
    } else {
        let o = this.we();
        if g.player(player).fOldY + PH as f32 <= o.fOldY && g.player(player).iy as i32 + PH >= o.iy as i32 {
            return this.hittop_player(g, player);
        } else {
            return this.hitother(g, player);
        }
    }

    false
}

/// Default side contact: kills an unshielded player.
pub fn mo_walking_enemy_hitother<T: MO_WalkingEnemyTrait + ?Sized>(g: &mut Game, _this: &mut T, player: PlayerId) -> bool {
    let mut player = player;
    if g.player(player).is_shielded() {
        return false;
    }

    g.player(player).kill_player_map_hazard(g, false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill
}

/// Handles weapons and thrown objects: kills, credits the thrower in stomp mode, or freezes on an ice blast.
pub fn mo_walking_enemy_collide_object<T: MO_WalkingEnemyTrait + ?Sized>(g: &mut Game, this: &mut T, object: Ptr<dyn IO_MovingObjectTrait>) {
        let mut gp = g.ptr();
    let mut object = object;
    if this.we().state == 0 {
        return;
    }

    if !object.is_dead() {
        removeifprojectile(g, object, false, false);

        let r#type: MovingObjectType = object.get_moving_object_type();

        let fKillOnWeakWeapon = this.we().fKillOnWeakWeapon;
        let frozen = this.we().frozen;
        if ((r#type == movingobject_fireball || r#type == movingobject_hammer || r#type == movingobject_boomerang) && (fKillOnWeakWeapon || frozen))
            || r#type == movingobject_shell
            || r#type == movingobject_throwblock
            || r#type == movingobject_throwbox
            || r#type == movingobject_bulletbill
            || r#type == movingobject_podobo
            || r#type == movingobject_attackzone
            || r#type == movingobject_explosion
            || r#type == movingobject_sledgehammer
        {
            // Don't kill enemies with non-moving shells
            if r#type == movingobject_shell && object.get_state() == 2 {
                return;
            }

            // Don't kill enemies with slow or non-moving boxes
            if r#type == movingobject_throwbox && !object.as_any().downcast_mut::<CO_ThrowBox>().unwrap().has_kill_velocity() {
                return;
            }

            if g.mode().gamemode == game_mode_stomp && !g.mode().gameover {
                // Find the player that shot this fireball so we can attribute a kill
                let mut killer: PlayerId = get_player_from_global_id(g, object.iPlayerID);

                if !killer.is_none() {
                    let killStyle = this.we().killStyle;
                    g.player(killer).add_killer_award(g, PlayerId::NONE, killStyle);
                    g.player(killer).score(g).adjust_score(g, 1);

                    if r#type == movingobject_shell {
                        object.as_any().downcast_mut::<CO_Shell>().unwrap().add_moving_kill(g, killer);
                    }
                }
            }

            if this.we().frozen {
                this.shatter_die(g);
            } else {
                if_sound_on_play(g, &mut gp.rm.sfx_kicksound);

                if r#type == movingobject_attackzone {
                    this.die_and_drop_shell(g, true, false);
                } else {
                    this.die(g);
                }
            }

            if r#type == movingobject_shell || r#type == movingobject_throwblock {
                object.check_and_die(g);
            } else if r#type == movingobject_bulletbill || r#type == movingobject_attackzone || r#type == movingobject_throwbox {
                object.die(g);
            }
        } else if r#type == movingobject_iceblast {
            let o = this.we_mut();
            o.frozenvelocity = o.velx;
            o.velx = 0.0f32;
            o.animationspeed = 0;

            o.bounce = GRAVITATION;

            o.frozen = true;
            o.frozentimer = ticks(300) as i16;

            g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                g.rm.spr_fireballexplosion,
                (o.ix as i32 - o.collisionOffsetX as i32 + o.iw as i32 - 32) as i16,
                (o.iy as i32 - o.collisionOffsetY as i32 + o.ih as i32 - 32) as i16,
                3,
                8,
            ));
        }
    }
}

/// Moves the enemy to a random spawn point, trying up to 11 times.
pub fn mo_walking_enemy_place(g: &mut Game, this: &mut MO_WalkingEnemy) {
    let mut gp = g.ptr();
    let mut ix = this.ix;
    let mut iy = this.iy;
    let collisionWidth = this.collisionWidth;
    let collisionHeight = this.collisionHeight;
    let mut iAttempts: i16 = 10;
    while !gp.g_map.findspawnpoint(g, 5, &mut ix, &mut iy, collisionWidth, collisionHeight, false) && {
        let a = iAttempts;
        iAttempts -= 1;
        a > 0
    } {}
    this.ix = ix;
    this.iy = iy;
    this.fx = this.ix as f32;
    this.fy = this.iy as f32;
}

/// Breaks a frozen enemy into ice fragments.
pub fn mo_walking_enemy_shatter_die<T: MO_WalkingEnemyTrait + ?Sized>(g: &mut Game, this: &mut T) {
        let mut gp = g.ptr();
    if_sound_on_play(g, &mut gp.rm.sfx_breakblock);
    let o = this.we_mut();
    o.dead = true;

    let iBrokenIceX: i16 = (o.ix as i32 - o.collisionOffsetX as i32 + o.iw as i32 - 32) as i16;
    let iBrokenIceY: i16 = (o.iy as i32 - o.collisionOffsetY as i32 + o.ih as i32 - 32) as i16;
    let spr = g.rm.spr_brokeniceblock;
    g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, iBrokenIceX, iBrokenIceY, vel(-1.5), vel(-7.0), 4, 2, 0, 0, 16, 16));
    g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, (iBrokenIceX as i32 + 16) as i16, iBrokenIceY, vel(1.5), vel(-7.0), 4, 2, 0, 0, 16, 16));
    g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, iBrokenIceX, (iBrokenIceY as i32 + 16) as i16, vel(-1.5), vel(-4.0), 4, 2, 0, 0, 16, 16));
    g.eyecandy[2].emplace(EC_FallingObject::new(&gp.rm, spr, (iBrokenIceX as i32 + 16) as i16, (iBrokenIceY as i32 + 16) as i16, vel(1.5), vel(-4.0), 4, 2, 0, 0, 16, 16));

    g.game_values.unlocksecret2part2 += 1;
}

/// Virtual interface added by `MO_WalkingEnemy`; implementors must provide `hittop_player`.
/// Implementors forward `CObjectTrait::draw`, `update`, `collide_player`, `collide_object` to the
/// `mo_walking_enemy_*` functions unless overridden, and implement `IO_MovingObjectTrait::die`
/// (a no-op in `MO_WalkingEnemy`).
pub trait MO_WalkingEnemyTrait: IO_MovingObjectTrait {
    fn we(&self) -> &MO_WalkingEnemy;
    fn we_mut(&mut self) -> &mut MO_WalkingEnemy;

    fn place(&mut self, g: &mut Game) {
        mo_walking_enemy_place(g, self.we_mut())
    }

    /// Handles a player landing on top; returns true if the player died.
    fn hittop_player(&mut self, g: &mut Game, player: PlayerId) -> bool;
    /// Handles any non-stomp player contact; returns true if the player died.
    fn hitother(&mut self, g: &mut Game, player: PlayerId) -> bool {
        let mut gp = g.ptr();
        mo_walking_enemy_hitother(gp.get(), self, player)
    }

    fn shatter_die(&mut self, g: &mut Game) {
        mo_walking_enemy_shatter_die(g, self)
    }
    /// Kills the enemy and leaves a shell behind, or shatters it if frozen.
    fn die_and_drop_shell(&mut self, g: &mut Game, fBounce: bool, fFlip: bool) {
        if self.we().frozen {
            self.shatter_die(g);
            return;
        }
        self.we_mut().dead = true;
        self.drop_shell(g, fBounce, fFlip);
    }
    /// Spawns this enemy's shell, if it has one.
    fn drop_shell(&mut self, g: &mut Game, _fBounce: bool, _fFlip: bool) {}
}

impl Deref for dyn MO_WalkingEnemyTrait {
    type Target = MO_WalkingEnemy;
    #[inline(always)]
    fn deref(&self) -> &MO_WalkingEnemy {
        self.we()
    }
}

impl DerefMut for dyn MO_WalkingEnemyTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut MO_WalkingEnemy {
        self.we_mut()
    }
}

/// Implements `we`, `we_mut` for a type that derefs to `MO_WalkingEnemy`.
#[macro_export]
macro_rules! impl_walking_enemy_plumbing {
    () => {
        fn we(&self) -> &$crate::smw::objects::walkingenemy::walking_enemy::MO_WalkingEnemy {
            self
        }
        fn we_mut(&mut self) -> &mut $crate::smw::objects::walkingenemy::walking_enemy::MO_WalkingEnemy {
            self
        }
    };
}
