//! Port of src/smw/objects/walkingenemy/WalkingEnemy.cpp

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
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::carriable::co_shell::CO_Shell;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::moving_object::{io_moving_object_draw, io_moving_object_update, IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, CPlayer};
use sdl2::sys::SDL_Rect;
use std::ops::{Deref, DerefMut};

//------------------------------------------------------------------------------
// class walking enemy (base class for goomba and koopa)
//------------------------------------------------------------------------------
pub struct MO_WalkingEnemy {
    pub io_moving_object: IO_MovingObject,

    pub spawnradius: f32,
    pub spawnangle: f32,

    pub iSpawnIconOffset: i16,
    pub killStyle: KillStyle,

    pub burnuptimer: i16,
    pub fKillOnWeakWeapon: bool,
    pub fBouncing: bool,
    pub fFallOffLedges: bool,

    pub frozen: bool,
    pub frozentimer: i16,
    pub frozenvelocity: f32,
    pub frozenanimationspeed: i16,
}
impl_base!(MO_WalkingEnemy => io_moving_object: IO_MovingObject);

impl MO_WalkingEnemy {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        nspr: Ptr<gfxSprite>,
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
        let mut o = MO_WalkingEnemy {
            io_moving_object: IO_MovingObject::new(
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
            o.velx = 1.0f32;
        } else {
            o.velx = -1.0f32;
        }

        o.movingObjectType = movingobject_none;

        o.fBouncing = bouncing;
        if o.fBouncing {
            o.bounce = -VELENEMYBOUNCE;
        } else {
            o.bounce = GRAVITATION;
        }

        o.spawnradius = 100.0f32;
        o.spawnangle = RANDOM_INT(1000) as f32 * 0.00628f32;
        o.inair = true;

        o.iSpawnIconOffset = 0;

        o.burnuptimer = 0;

        o.fKillOnWeakWeapon = killOnWeakWeapon;

        o.frozen = false;
        o.frozentimer = 0;
        o.frozenvelocity = o.velx;
        o.frozenanimationspeed = aniSpeed;

        o.fFallOffLedges = fallOffLedges;

        // Virtual call during construction: always MO_WalkingEnemy::place
        mo_walking_enemy_place(&mut o);
        o
    }

    pub fn get_kill_style(&self) -> KillStyle {
        self.killStyle
    }
}

pub fn mo_walking_enemy_draw<T: MO_WalkingEnemyTrait + ?Sized>(this: &mut T) {
    if this.we().state == 0 {
        let o = this.we();
        let numeyecandy: i16 = 8;
        let addangle: f32 = TWO_PI / numeyecandy as f32;
        let mut displayangle: f32 = o.spawnangle;

        for _k in 0..numeyecandy {
            let spawnX: i16 = (o.ix as i32 + (o.collisionWidth as i32 >> 1) - 8 + (o.spawnradius * cosf(displayangle)) as i16 as i32) as i16;
            let spawnY: i16 = (o.iy as i32 + (o.collisionHeight as i32 >> 1) - 8 + (o.spawnradius * sinf(displayangle)) as i16 as i32) as i16;

            displayangle += addangle;

            unsafe {
                rm.spr_awardsouls.draw_src(spawnX as i32, spawnY as i32, &SDL_Rect { x: o.iSpawnIconOffset as i32, y: 0, w: 16, h: 16 });
            }
        }
    } else {
        io_moving_object_draw(this);

        let o = this.we();
        if o.frozen {
            unsafe {
                rm.spr_iceblock.draw_src(
                    o.ix as i32 - o.collisionOffsetX as i32 + o.iw as i32 - 32,
                    o.iy as i32 - o.collisionOffsetY as i32 + o.ih as i32 - 32,
                    &SDL_Rect { x: 0, y: 0, w: 32, h: 32 },
                );
            }
        }
    }
}

pub fn mo_walking_enemy_update<T: MO_WalkingEnemyTrait + ?Sized>(this: &mut T) {
    unsafe {
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

                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
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
            o.spawnradius -= 2.0f32;
            o.spawnangle += 0.05f32;

            if o.spawnradius < 10.0f32 {
                o.state = 1;
            }
        } else {
            io_moving_object_update(this);
        }

        // Deal with terminal burnup velocity
        if this.we().vely >= MAXVELY {
            this.we_mut().burnuptimer += 1;
            if this.we().burnuptimer > 20 {
                if this.we().burnuptimer > 80 {
                    this.kill_object_map_hazard(-1);
                } else {
                    let o = this.we();
                    eyecandy[0].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_burnup),
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

            if !o.platform.is_null() {
                let iFrontTileType: i32 = o.platform.get_tile_type_from_coord(probeFrontX, probeY);
                let iCenterTileType: i32 = o.platform.get_tile_type_from_coord(probeCenterX, probeY);

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

                    let mut frontBlock = g_map.block(probeFrontX, probeY);
                    let mut centerBlock = g_map.block(probeCenterX, probeY);

                    let fFoundFrontBlock = !frontBlock.is_null() && !frontBlock.is_transparent() && !frontBlock.is_hidden();
                    let fFoundCenterBlock = !centerBlock.is_null() && !centerBlock.is_transparent() && !centerBlock.is_hidden();

                    if !fFoundFrontBlock && !fFoundCenterBlock {
                        let frontTile: i32 = g_map.map(probeFrontX as i32, probeY as i32);
                        let centerTile: i32 = g_map.map(probeCenterX as i32, probeY as i32);

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
}

pub fn mo_walking_enemy_collide_player<T: MO_WalkingEnemyTrait + ?Sized>(this: &mut T, player: Ptr<CPlayer>) -> bool {
    let mut player = player;
    if this.we().state == 0 {
        return false;
    }

    unsafe {
        if player.is_invincible() || this.we().frozen {
            let killStyle = this.we().killStyle;
            player.add_killer_award(Ptr::null(), killStyle);

            if game_values.gamemode.gamemode == game_mode_stomp && !game_values.gamemode.gameover {
                player.score().adjust_score(1);
            }

            if this.we().frozen {
                this.shatter_die();
            } else {
                if_sound_on_play(&mut rm.sfx_kicksound);
                this.die();
            }
        } else {
            let o = this.we();
            if player.fOldY + PH as f32 <= o.fOldY && player.iy as i32 + PH >= o.iy as i32 {
                return this.hittop_player(player);
            } else {
                return this.hitother(player);
            }
        }
    }

    false
}

pub fn mo_walking_enemy_hitother<T: MO_WalkingEnemyTrait + ?Sized>(_this: &mut T, player: Ptr<CPlayer>) -> bool {
    let mut player = player;
    if player.is_shielded() {
        return false;
    }

    player.kill_player_map_hazard(false, KillStyle::Environment, false, -1) != PlayerKillType::NonKill
}

pub fn mo_walking_enemy_collide_object<T: MO_WalkingEnemyTrait + ?Sized>(this: &mut T, object: Ptr<dyn IO_MovingObjectTrait>) {
    let mut object = object;
    if this.we().state == 0 {
        return;
    }

    unsafe {
        if !object.is_dead() {
            removeifprojectile(object, false, false);

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

                if game_values.gamemode.gamemode == game_mode_stomp && !game_values.gamemode.gameover {
                    // Find the player that shot this fireball so we can attribute a kill
                    let mut killer: Ptr<CPlayer> = get_player_from_global_id(object.iPlayerID);

                    if !killer.is_null() {
                        let killStyle = this.we().killStyle;
                        killer.add_killer_award(Ptr::null(), killStyle);
                        killer.score().adjust_score(1);

                        if r#type == movingobject_shell {
                            object.as_any().downcast_mut::<CO_Shell>().unwrap().add_moving_kill(killer);
                        }
                    }
                }

                if this.we().frozen {
                    this.shatter_die();
                } else {
                    if_sound_on_play(&mut rm.sfx_kicksound);

                    if r#type == movingobject_attackzone {
                        this.die_and_drop_shell(true, false);
                    } else {
                        this.die();
                    }
                }

                if r#type == movingobject_shell || r#type == movingobject_throwblock {
                    object.check_and_die();
                } else if r#type == movingobject_bulletbill || r#type == movingobject_attackzone || r#type == movingobject_throwbox {
                    object.die();
                }
            } else if r#type == movingobject_iceblast {
                let o = this.we_mut();
                o.frozenvelocity = o.velx;
                o.velx = 0.0f32;
                o.animationspeed = 0;

                o.bounce = GRAVITATION;

                o.frozen = true;
                o.frozentimer = 300;

                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (o.ix as i32 - o.collisionOffsetX as i32 + o.iw as i32 - 32) as i16,
                    (o.iy as i32 - o.collisionOffsetY as i32 + o.ih as i32 - 32) as i16,
                    3,
                    8,
                ));
            }
        }
    }
}

pub fn mo_walking_enemy_place(this: &mut MO_WalkingEnemy) {
    let mut ix = this.ix;
    let mut iy = this.iy;
    let collisionWidth = this.collisionWidth;
    let collisionHeight = this.collisionHeight;
    let mut iAttempts: i16 = 10;
    unsafe {
        while !g_map.findspawnpoint(5, &mut ix, &mut iy, collisionWidth, collisionHeight, false) && {
            let a = iAttempts;
            iAttempts -= 1;
            a > 0
        } {}
    }
    this.ix = ix;
    this.iy = iy;
    this.fx = this.ix as f32;
    this.fy = this.iy as f32;
}

pub fn mo_walking_enemy_shatter_die<T: MO_WalkingEnemyTrait + ?Sized>(this: &mut T) {
    unsafe {
        if_sound_on_play(&mut rm.sfx_breakblock);
        let o = this.we_mut();
        o.dead = true;

        let iBrokenIceX: i16 = (o.ix as i32 - o.collisionOffsetX as i32 + o.iw as i32 - 32) as i16;
        let iBrokenIceY: i16 = (o.iy as i32 - o.collisionOffsetY as i32 + o.ih as i32 - 32) as i16;
        let spr = Ptr::from_mut(&mut rm.spr_brokeniceblock);
        eyecandy[2].emplace(EC_FallingObject::new(spr, iBrokenIceX, iBrokenIceY, -1.5f32, -7.0f32, 4, 2, 0, 0, 16, 16));
        eyecandy[2].emplace(EC_FallingObject::new(spr, (iBrokenIceX as i32 + 16) as i16, iBrokenIceY, 1.5f32, -7.0f32, 4, 2, 0, 0, 16, 16));
        eyecandy[2].emplace(EC_FallingObject::new(spr, iBrokenIceX, (iBrokenIceY as i32 + 16) as i16, -1.5f32, -4.0f32, 4, 2, 0, 0, 16, 16));
        eyecandy[2].emplace(EC_FallingObject::new(spr, (iBrokenIceX as i32 + 16) as i16, (iBrokenIceY as i32 + 16) as i16, 1.5f32, -4.0f32, 4, 2, 0, 0, 16, 16));

        game_values.unlocksecret2part2 += 1;
    }
}

/// Virtual interface added by `MO_WalkingEnemy`. `hittop(CPlayer*)` is `hittop_player` (pure).
/// Implementors forward `CObjectTrait::draw`, `update`, `collide_player`, `collide_object` to the
/// `mo_walking_enemy_*` functions unless overridden, and implement `IO_MovingObjectTrait::die`
/// (a no-op in `MO_WalkingEnemy`).
pub trait MO_WalkingEnemyTrait: IO_MovingObjectTrait {
    fn we(&self) -> &MO_WalkingEnemy;
    fn we_mut(&mut self) -> &mut MO_WalkingEnemy;

    fn place(&mut self) {
        mo_walking_enemy_place(self.we_mut())
    }

    fn hittop_player(&mut self, player: Ptr<CPlayer>) -> bool;
    fn hitother(&mut self, player: Ptr<CPlayer>) -> bool {
        mo_walking_enemy_hitother(self, player)
    }

    fn shatter_die(&mut self) {
        mo_walking_enemy_shatter_die(self)
    }
    fn die_and_drop_shell(&mut self, fBounce: bool, fFlip: bool) {
        if self.we().frozen {
            self.shatter_die();
            return;
        }
        self.we_mut().dead = true;
        self.drop_shell(fBounce, fFlip);
    }
    fn drop_shell(&mut self, _fBounce: bool, _fFlip: bool) {}
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
