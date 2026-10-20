//! Base for every object that moves: sprite animation, screen wrap, and map, block and platform collision.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::map::PlatformId;
use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::movingplatform::MovingPlatform;
use crate::common::object_base::{object_moving, CObject, CObjectTrait};
use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game_mode::{game_mode_boxes_minigame, game_mode_stomp};
use crate::common::game_values::if_sound_on_play;
use crate::common::io_block::IO_BlockTrait;
use crate::common::object_base::cap_falling_velocity;
use crate::common::tile_types::*;
use crate::globals::*;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::carriable::{co_egg::CO_Egg, co_flag::CO_Flag, co_phanto_key::CO_PhantoKey, co_star::CO_Star};
use crate::smw::objects::moving::mo_coin::MO_Coin;
use crate::smw::player::get_player_from_global_id;
use crate::impl_base;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::objects::powerup::powerup::MO_PowerupTrait;
use crate::smw::objects::walkingenemy::walking_enemy::MO_WalkingEnemyTrait;
use crate::smw::player::CPlayer;
use std::ops::{Deref, DerefMut};

/// Base state for objects that move and animate; positions and sizes are in pixels.
#[derive(Clone)]

pub struct IO_MovingObject {
    pub cobject: CObject,

    /// Global ID of the player who owns or fired the object, or -1.
    pub iPlayerID: i16,
    /// Team of the owning player.
    pub iTeamID: i16,

    /// Position at the start of this tick (pixels).
    pub fOldX: f32,
    pub fOldY: f32,
    /// `fy` plus this tick's vertical velocity, computed before collision.
    pub fPrecalculatedY: f32,

    /// Number of animation frames in the sprite strip.
    pub iNumSprites: i16,
    /// X offset (pixels) of the current animation frame in the sprite.
    pub drawframe: i16,
    /// Tick counter toward the next animation frame.
    pub animationtimer: i16,

    /// Vertical velocity taken on landing; `GRAVITATION` means no bounce.
    pub bounce: f32,

    /// Ticks per animation frame; 0 stops the animation.
    pub animationspeed: i16,
    /// Width (pixels) of the frame strip; `drawframe` wraps back to `animationOffsetX` here.
    pub animationWidth: i16,
    /// Source offset (pixels) of the first frame in the sprite.
    pub animationOffsetX: i16,
    pub animationOffsetY: i16,

    pub inair: bool,
    pub onice: bool,

    /// Type tag other objects match on in collisions.
    pub movingObjectType: MovingObjectType,

    /// Moving platform the object is riding, or `PlatformId::NONE`.
    pub platform: PlatformId,
    /// Side a moving platform pushed the object from this tick, or -1; a push into a solid tile kills it.
    pub iHorizontalPlatformCollision: i16,
    pub iVerticalPlatformCollision: i16,

    /// Super-death tiles kill the object; when false it lands on them.
    pub fObjectDiesOnSuperDeathTiles: bool,
    /// Map collision runs for this object; read by `collides_with_map`.
    pub fObjectCollidesWithMap: bool,
}
impl_base!(IO_MovingObject => cobject: CObject);

impl IO_MovingObject {
    /// Collision and animation arguments of `-1` use the sprite frame size and zero offsets.
    #[allow(clippy::too_many_arguments)]
    pub fn new(g: &mut Game, nspr: SpriteId,
        pos: Vec2s,
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
    ) -> Self {
        let mut gp = g.ptr();
        let mut o = IO_MovingObject {
            cobject: CObject::new(gp.get(), nspr, pos),
            iPlayerID: 0,
            iTeamID: 0,
            fOldX: 0.0,
            fOldY: 0.0,
            fPrecalculatedY: 0.0,
            iNumSprites: 0,
            drawframe: 0,
            animationtimer: 0,
            bounce: 0.0,
            animationspeed: 0,
            animationWidth: 0,
            animationOffsetX: 0,
            animationOffsetY: 0,
            inair: false,
            onice: false,
            movingObjectType: movingobject_none,
            platform: PlatformId::NONE,
            iHorizontalPlatformCollision: 0,
            iVerticalPlatformCollision: 0,
            fObjectDiesOnSuperDeathTiles: false,
            fObjectCollidesWithMap: false,
        };

        o.iNumSprites = iNumSpr;

        if iAnimationWidth > -1 {
            o.iw = iAnimationWidth;
        } else if !o.spr.is_none() {
            o.iw = (g.rm.sprite(o.spr).get_width() as i16) / o.iNumSprites;
        }

        if iAnimationHeight > -1 {
            o.ih = iAnimationHeight;
        }

        o.animationtimer = 0;

        if !o.spr.is_none() {
            o.animationWidth = g.rm.sprite(o.spr).get_width() as i16;
        }

        o.fOldX = o.fx;
        o.fOldY = o.fy - o.ih as f32;

        o.animationspeed = aniSpeed;

        o.objectType = object_moving;
        o.movingObjectType = movingobject_none;

        if iCollisionWidth > -1 {
            o.collisionWidth = iCollisionWidth;
            o.collisionHeight = iCollisionHeight;
            o.collisionOffsetX = iCollisionOffsetX;
            o.collisionOffsetY = iCollisionOffsetY;
        } else {
            o.collisionWidth = o.iw;
            o.collisionHeight = o.ih;
            o.collisionOffsetX = 0;
            o.collisionOffsetY = 0;
        }

        if iAnimationOffsetX > -1 {
            o.animationOffsetX = iAnimationOffsetX;
            o.animationOffsetY = iAnimationOffsetY;
        } else {
            o.animationOffsetX = 0;
            o.animationOffsetY = 0;
        }

        o.drawframe = o.animationOffsetX;

        o.inair = false;
        o.onice = false;

        o.platform = PlatformId::NONE;
        o.iHorizontalPlatformCollision = -1;
        o.iVerticalPlatformCollision = -1;

        o.fObjectDiesOnSuperDeathTiles = true;
        o.fObjectCollidesWithMap = true;

        o.bounce = GRAVITATION;
        o
    }

    pub fn get_moving_object_type(&self) -> MovingObjectType {
        self.movingObjectType
    }

    pub fn collides_with_map(&self) -> bool {
        self.fObjectCollidesWithMap
    }

    /// Slows `velx` toward zero with air, ice or ground friction.
    pub fn applyfriction(&mut self) {
        if self.velx > 0.0 {
            if self.inair {
                self.velx -= VELAIRFRICTION;
            } else if self.onice {
                self.velx -= VELICEFRICTION;
            } else {
                self.velx -= VELMOVINGFRICTION;
            }

            if self.velx < 0.0 {
                self.velx = 0.0;
            }
        } else if self.velx < 0.0 {
            if self.inair {
                self.velx += VELAIRFRICTION;
            } else if self.onice {
                self.velx += VELICEFRICTION;
            } else {
                self.velx += VELMOVINGFRICTION;
            }

            if self.velx > 0.0 {
                self.velx = 0.0;
            }
        }
    }

    /// Wraps the object to the opposite screen edge once it leaves the screen horizontally.
    pub fn flipsidesifneeded(&mut self) {
        if self.ix < 0 || self.fx < 0.0 {
            let fx = self.fx;
            self.set_xf(fx + App::screenWidth as f32);
            self.fOldX += App::screenWidth as f32;
        } else if self.ix as i32 >= App::screenWidth || self.fx >= App::screenWidth as f32 {
            let fx = self.fx;
            self.set_xf(fx - App::screenWidth as f32);
            self.fOldX -= App::screenWidth as f32;
        }
    }
}

/// Draws the current animation frame, offset from the collision box.
pub fn io_moving_object_draw<T: IO_MovingObjectTrait + ?Sized>(g: &mut Game, this: &mut T) {
    let o = this.mo();
    g.rm.sprite(o.spr).draw_src(
        g,
        (o.ix - o.collisionOffsetX) as i32,
        (o.iy - o.collisionOffsetY) as i32,
        &Rect { x: o.drawframe as i32, y: o.animationOffsetY as i32, w: o.iw as i32, h: o.ih as i32 },
    );
}

/// Default update: records the old position, runs map collision, then animates.
pub fn io_moving_object_update<T: IO_MovingObjectTrait + ?Sized>(g: &mut Game, this: &mut T) {
    {
        let o = this.mo_mut();
        o.fOldX = o.fx;
        o.fOldY = o.fy;
    }

    io_moving_object_collision_detection_map(g, this);

    this.animate();
}

/// Advances `drawframe` every `animationspeed` ticks.
pub fn io_moving_object_animate(o: &mut IO_MovingObject) {
    if o.animationspeed > 0 && {
        o.animationtimer += 1;
        o.animationtimer >= o.animationspeed
    } {
        o.animationtimer = 0;

        o.drawframe += o.iw;
        if o.drawframe >= o.animationWidth {
            o.drawframe = o.animationOffsetX;
        }
    }
}

/// Default player contact: removes the object.
pub fn io_moving_object_collide_player<T: IO_MovingObjectTrait + ?Sized>(this: &mut T, _player: PlayerId) -> bool {
    this.mo_mut().dead = true;
    false
}

/// Applies velocity and resolves collisions with blocks, tiles and moving platforms, x axis first.
pub fn io_moving_object_collision_detection_map<T: IO_MovingObjectTrait + ?Sized>(g: &mut Game, this: &mut T) {
        let mut gp = g.ptr();
    let mut o = this.as_mo_ptr();
    let x = o.fx + o.velx;
    o.set_xf(x);
    o.flipsidesifneeded();

    o.fPrecalculatedY = o.fy + o.vely; // Before any int cast, or the bottom hit is missed and objects slide out of 1x1 gaps.

    let mut fPlatformVelX: f32 = 0.0;
    let mut fPlatformVelY: f32 = 0.0;

    let fTempY = o.fy;

    if !o.platform.is_none() {
        if !o.onice {
            fPlatformVelX = g.g_map.platform(o.platform).fVelX;
            let x = o.fx + fPlatformVelX;
            o.set_xf(x);
            o.flipsidesifneeded();
        }

        fPlatformVelY = g.g_map.platform(o.platform).fVelY;

        if g.g_map.platform(o.platform).fOldVelY < 0.0 {
            o.fy += g.g_map.platform(o.platform).fOldVelY;
        }

        o.fPrecalculatedY += g.g_map.platform(o.platform).fOldVelY;
    }

    o.iHorizontalPlatformCollision = -1;
    o.iVerticalPlatformCollision = -1;

    gp.g_map.moving_platform_collision_object(g, o);

    o.fy = fTempY;

    if o.fPrecalculatedY + (o.collisionHeight as f32) < 0.0 {
        // on top outside of the screen
        let y = o.fPrecalculatedY;
        o.set_yf(y);
        o.vely = cap_falling_velocity(GRAVITATION + o.vely);

        if o.platform.is_none() {
            o.inair = true;
            o.onice = false;
        }

        return;
    } else if o.fPrecalculatedY + o.collisionHeight as f32 >= App::screenHeight as f32 {
        // on ground outside of the screen?
        let y = -o.collisionHeight;
        o.set_yi(y);
        o.fOldY = o.fy - 1.0;
        o.onice = false;
        return;
    }

    let mut ty: i16 = ((o.fy as i16) as i32 / TILESIZE) as i16;
    let ty2: i16 = (((o.fy as i16) as i32 + o.collisionHeight as i32) / TILESIZE) as i16;
    let mut tx: i16 = -1;

    //-----------------------------------------------------------------
    //  x axis first (--)
    //-----------------------------------------------------------------
    if o.fy + o.collisionHeight as f32 >= 0.0 {
        if o.velx + fPlatformVelX > 0.01 || o.iHorizontalPlatformCollision == 3 {
            // moving right
            if o.fx + o.collisionWidth as f32 >= App::screenWidth as f32 {
                tx = ((o.fx + o.collisionWidth as f32 - App::screenWidth as f32) as i16 as i32 / TILESIZE) as i16;
                o.fOldX -= App::screenWidth as f32;
            } else {
                tx = (((o.fx as i16) as i32 + o.collisionWidth as i32) / TILESIZE) as i16;
            }

            let topblock = g.map_block(tx, ty);
            let bottomblock = g.map_block(tx, ty2);

            let fTopBlockSolid = !topblock.is_null() && !topblock.get().is_transparent() && !topblock.get().is_hidden();
            let fBottomBlockSolid = !bottomblock.is_null() && !bottomblock.get().is_transparent() && !bottomblock.get().is_hidden();
            if fTopBlockSolid || fBottomBlockSolid {
                let mut processOtherBlock = true;
                if fTopBlockSolid {
                    // collide with top block
                    if o.iHorizontalPlatformCollision == 3 {
                        o.kill_object_map_hazard(g, -1);
                        return;
                    }

                    topblock.get().collide_object_dir(g, o, 1);
                    o.flipsidesifneeded();
                    removeifprojectile(g, o, true, true);

                    processOtherBlock = false;

                    o.side_bounce(g, true);
                }

                if processOtherBlock && fBottomBlockSolid {
                    // then bottom
                    if o.iHorizontalPlatformCollision == 3 {
                        o.kill_object_map_hazard(g, -1);
                        return;
                    }

                    bottomblock.get().collide_object_dir(g, o, 1);
                    o.flipsidesifneeded();
                    removeifprojectile(g, o, true, true);

                    o.side_bounce(g, true);
                }
            } else if (g.g_map.map(tx as i32, ty as i32) & tile_flag_solid) != 0 || (g.g_map.map(tx as i32, ty2 as i32) & tile_flag_solid) != 0 {
                // collision on the right side.

                if o.iHorizontalPlatformCollision == 3 {
                    o.kill_object_map_hazard(g, -1);
                    return;
                }

                let x = (((tx as i32) << 5) - o.collisionWidth as i32) as f32 - 0.2; // move to the edge of the tile (tile on the right -> mind the object width)
                o.set_xf(x);
                o.fOldX = o.fx;

                if o.velx > 0.0 {
                    o.velx = -o.velx;
                }

                o.flipsidesifneeded();
                removeifprojectile(g, o, true, true);

                o.side_bounce(g, true);
            }
        } else if o.velx + fPlatformVelX < -0.01 || o.iHorizontalPlatformCollision == 1 {
            // moving left
            tx = ((o.fx as i16) as i32 / TILESIZE) as i16;

            // Just in case fx < 0 and wasn't caught by flipsidesifneeded()
            if tx < 0 {
                tx = 0;
            }

            let topblock = g.map_block(tx, ty);
            let bottomblock = g.map_block(tx, ty2);

            let fTopBlockSolid = !topblock.is_null() && !topblock.get().is_transparent() && !topblock.get().is_hidden();
            let fBottomBlockSolid = !bottomblock.is_null() && !bottomblock.get().is_transparent() && !bottomblock.get().is_hidden();
            if fTopBlockSolid || fBottomBlockSolid {
                let mut processOtherBlock = true;
                if fTopBlockSolid {
                    // collide with top block
                    if o.iHorizontalPlatformCollision == 1 {
                        o.kill_object_map_hazard(g, -1);
                        return;
                    }

                    topblock.get().collide_object_dir(g, o, 3);
                    o.flipsidesifneeded();
                    removeifprojectile(g, o, true, true);

                    processOtherBlock = false;

                    o.side_bounce(g, false);
                }

                if processOtherBlock && fBottomBlockSolid {
                    // then bottom
                    if o.iHorizontalPlatformCollision == 1 {
                        o.kill_object_map_hazard(g, -1);
                        return;
                    }

                    bottomblock.get().collide_object_dir(g, o, 3);
                    o.flipsidesifneeded();
                    removeifprojectile(g, o, true, true);

                    o.side_bounce(g, false);
                }
            } else if (g.g_map.map(tx as i32, ty as i32) & tile_flag_solid) != 0 || (g.g_map.map(tx as i32, ty2 as i32) & tile_flag_solid) != 0 {
                if o.iHorizontalPlatformCollision == 1 {
                    o.kill_object_map_hazard(g, -1);
                    return;
                }

                let x = (((tx as i32) << 5) + TILESIZE) as f32 + 0.2; // move to the edge of the tile
                o.set_xf(x);
                o.fOldX = o.fx;

                if o.velx < 0.0 {
                    o.velx = -o.velx;
                }

                o.flipsidesifneeded();
                removeifprojectile(g, o, true, true);

                o.side_bounce(g, false);
            }
        }
    }

    let txl: i16 = (o.ix as i32 / TILESIZE) as i16;
    let txr: i16 = if o.ix as i32 + o.collisionWidth as i32 >= App::screenWidth {
        ((o.ix as i32 + o.collisionWidth as i32 - App::screenWidth) / TILESIZE) as i16
    } else {
        ((o.ix as i32 + o.collisionWidth as i32) / TILESIZE) as i16
    };

    //-----------------------------------------------------------------
    //  then y axis (|)
    //-----------------------------------------------------------------

    let mut fMovingUp = o.vely;
    if !o.platform.is_none() {
        fMovingUp = o.vely + fPlatformVelY - o.bounce;
    }

    if fMovingUp < -0.01 {
        ty = ((o.fPrecalculatedY as i16) as i32 / TILESIZE) as i16;

        let leftblock = g.map_block(txl, ty);
        let rightblock = g.map_block(txr, ty);

        if !leftblock.is_null() && !leftblock.get().is_transparent() && !leftblock.get().is_hidden() {
            // then left
            if o.iVerticalPlatformCollision == 2 {
                o.kill_object_map_hazard(g, -1);
            }

            leftblock.get().collide_object_dir(g, o, 0);
            return;
        }

        if !rightblock.is_null() && !rightblock.get().is_transparent() && !rightblock.get().is_hidden() {
            // then right
            if o.iVerticalPlatformCollision == 2 {
                o.kill_object_map_hazard(g, -1);
            }

            rightblock.get().collide_object_dir(g, o, 0);
            return;
        }

        if (g.g_map.map(txl as i32, ty as i32) & tile_flag_solid) != 0 || (g.g_map.map(txr as i32, ty as i32) & tile_flag_solid) != 0 {
            if o.iVerticalPlatformCollision == 2 {
                o.kill_object_map_hazard(g, -1);
            }

            let y = (((ty as i32) << 5) + TILESIZE) as f32 + 0.2;
            o.set_yf(y);
            o.fOldY = o.fy - 1.0;

            if o.vely < 0.0 {
                o.vely = -o.vely;
            }
        } else {
            let y = o.fPrecalculatedY;
            o.set_yf(y);
            o.vely += GRAVITATION;
        }

        if o.platform.is_none() {
            o.inair = true;
            o.onice = false;
        }
    } else {
        // moving down / on ground
        ty = (((o.fPrecalculatedY as i16) as i32 + o.collisionHeight as i32) / TILESIZE) as i16;

        let leftblock = g.map_block(txl, ty);
        let rightblock = g.map_block(txr, ty);

        let fLeftBlockSolid = !leftblock.is_null() && !leftblock.get().is_transparent() && !leftblock.get().is_hidden();
        let fRightBlockSolid = !rightblock.is_null() && !rightblock.get().is_transparent() && !rightblock.get().is_hidden();

        if fLeftBlockSolid || fRightBlockSolid {
            let mut processOtherBlock = true;
            if fLeftBlockSolid {
                // collide with left block
                processOtherBlock = leftblock.get().collide_object_dir(g, o, 2);

                if o.platform.is_none() {
                    o.inair = false;
                    o.onice = false;
                }
            }

            if processOtherBlock && fRightBlockSolid {
                // then right
                rightblock.get().collide_object_dir(g, o, 2);

                if o.platform.is_none() {
                    o.inair = false;
                    o.onice = false;
                }
            }

            if o.iVerticalPlatformCollision == 0 {
                o.kill_object_map_hazard(g, -1);
            }

            return;
        }

        let leftTile: i32 = g.g_map.map(txl as i32, ty as i32);
        let rightTile: i32 = g.g_map.map(txr as i32, ty as i32);

        if (leftTile & tile_flag_solid_on_top) != 0 || (rightTile & tile_flag_solid_on_top) != 0 {
            if (o.fOldY + o.collisionHeight as f32) / (TILESIZE as f32) < ty as f32 {
                o.vely = o.bottom_bounce(g);
                let y = (((ty as i32) << 5) - o.collisionHeight as i32) as f32 - 0.2;
                o.set_yf(y);
                o.fOldY = o.fy - GRAVITATION;

                if o.platform.is_none() {
                    o.inair = false;
                    o.onice = false;
                }

                o.platform = PlatformId::NONE;

                if o.iVerticalPlatformCollision == 0 {
                    o.kill_object_map_hazard(g, -1);
                }

                return;
            }
        }

        let fSuperDeathTileUnderObject = o.fObjectDiesOnSuperDeathTiles
            && (((leftTile & tile_flag_super_death_top) != 0 && (rightTile & tile_flag_super_death_top) != 0)
                || ((leftTile & tile_flag_super_death_top) != 0 && (rightTile & tile_flag_solid) == 0)
                || ((leftTile & tile_flag_solid) == 0 && (rightTile & tile_flag_super_death_top) != 0));

        if ((leftTile & tile_flag_solid) != 0 || (rightTile & tile_flag_solid) != 0) && !fSuperDeathTileUnderObject {
            o.vely = o.bottom_bounce(g);
            let y = (((ty as i32) << 5) - o.collisionHeight as i32) as f32 - 0.2;
            o.set_yf(y);
            o.fOldY = o.fy;

            if o.platform.is_none() {
                o.inair = false;

                if ((leftTile & tile_flag_ice) != 0 && ((rightTile & tile_flag_ice) != 0 || rightTile == tile_flag_nonsolid || rightTile == tile_flag_gap))
                    || ((rightTile & tile_flag_ice) != 0 && ((leftTile & tile_flag_ice) != 0 || leftTile == tile_flag_nonsolid || leftTile == tile_flag_gap))
                {
                    o.onice = true;
                } else {
                    o.onice = false;
                }
            }

            o.platform = PlatformId::NONE;

            if o.iVerticalPlatformCollision == 0 {
                o.kill_object_map_hazard(g, -1);
            }
        } else if fSuperDeathTileUnderObject {
            o.kill_object_map_hazard(g, -1);
            return;
        } else {
            let y = o.fPrecalculatedY;
            o.set_yf(y);
            o.vely = cap_falling_velocity(GRAVITATION + o.vely);

            if o.platform.is_none() {
                o.inair = true;
            }
        }
    }

    if o.platform.is_none() && o.inair {
        o.onice = false;
    }
}

fn block_is_solid(block: Ptr<dyn IO_BlockTrait>) -> bool {
    !block.is_null() && !block.get().is_transparent() && !block.get().is_hidden()
}

/// Pushes the object out of any solid tile, block or platform it overlaps; returns true if it moved.
pub fn io_moving_object_collision_detection_checksides<T: IO_MovingObjectTrait + ?Sized>(g: &mut Game, this: &mut T) -> bool {
    let mut o = this.as_mo_ptr();
    // First figure out where the corners of this object are touching
    let mut iCase: u8 = 0;

    let nofliptxl: i16 = (o.ix as i32 >> 5) as i16;
    let txl: i16 = if o.ix < 0 { ((o.ix as i32 + App::screenWidth) >> 5) as i16 } else { nofliptxl };

    let nofliptxr: i16 = ((o.ix as i32 + o.collisionWidth as i32) >> 5) as i16;
    let txr: i16 = if o.ix as i32 + o.collisionWidth as i32 >= App::screenWidth {
        ((o.ix as i32 + o.collisionWidth as i32 - App::screenWidth) >> 5) as i16
    } else {
        nofliptxr
    };

    let ty: i16 = (o.iy as i32 >> 5) as i16;
    let ty2: i16 = ((o.iy as i32 + o.collisionHeight as i32) >> 5) as i16;

    if o.iy >= 0 {
        if (ty as i32) < MAPHEIGHT {
            if txl >= 0 && (txl as i32) < MAPWIDTH {
                if block_is_solid(g.map_block(txl, ty)) || (g.g_map.map(txl as i32, ty as i32) & tile_flag_solid) > 0 {
                    iCase |= 0x01;
                }
            }

            if txr >= 0 && (txr as i32) < MAPWIDTH {
                if block_is_solid(g.map_block(txr, ty)) || (g.g_map.map(txr as i32, ty as i32) & tile_flag_solid) > 0 {
                    iCase |= 0x02;
                }
            }
        }
    }

    if (o.iy as i32 + o.collisionHeight as i32) as f32 >= 0.0 {
        if (ty2 as i32) < MAPHEIGHT {
            if txl >= 0 && (txl as i32) < MAPWIDTH {
                if block_is_solid(g.map_block(txl, ty2)) || (g.g_map.map(txl as i32, ty2 as i32) & tile_flag_solid) > 0 {
                    iCase |= 0x04;
                }
            }

            if txr >= 0 && (txr as i32) < MAPWIDTH {
                if block_is_solid(g.map_block(txr, ty2)) || (g.g_map.map(txr as i32, ty2 as i32) & tile_flag_solid) > 0 {
                    iCase |= 0x08;
                }
            }
        }
    }

    let cw = o.collisionWidth as i32;
    let ch = o.collisionHeight as i32;
    let left_edge = (((nofliptxl as i32) << 5) + TILESIZE) as f32 + 0.2;
    let right_edge = (((nofliptxr as i32) << 5) - cw) as f32 - 0.2;
    let top_edge = (((ty as i32) << 5) + TILESIZE) as f32 + 0.2;
    let bottom_edge = (((ty2 as i32) << 5) - ch) as f32 - 0.2;
    let center = o.ix as i32 + (cw >> 1);

    let mut fRet = true;
    // Then determine which way is the best way to move this object out of the solid map areas
    match iCase {
        // Do nothing
        //[ ][ ]
        //[ ][ ]
        0 => {
            fRet = false;
        }

        //[X][ ]
        //[ ][ ]
        1 => {
            if center > ((nofliptxl as i32) << 5) + TILESIZE {
                o.set_xf(left_edge);
                o.flipsidesifneeded();
            } else {
                o.set_yf(top_edge);
            }
        }

        //[ ][X]
        //[ ][ ]
        2 => {
            if center < ((nofliptxr as i32) << 5) {
                o.set_xf(right_edge);
                o.flipsidesifneeded();
            } else {
                o.set_yf(top_edge);
            }
        }

        //[X][X]
        //[ ][ ]
        3 => {
            o.set_yf(top_edge);
        }

        //[ ][ ]
        //[X][ ]
        4 => {
            if center > ((nofliptxl as i32) << 5) + TILESIZE {
                o.set_xf(left_edge);
                o.flipsidesifneeded();
            } else {
                o.set_yf(bottom_edge);
            }
        }

        //[X][ ]
        //[X][ ]
        5 => {
            o.set_xf(left_edge);
            o.flipsidesifneeded();
        }

        //[ ][X]
        //[X][ ]
        6 => {
            if center > ((nofliptxl as i32) << 5) + TILESIZE {
                o.set_yf(top_edge);
                o.set_xf(left_edge);
                o.flipsidesifneeded();
            } else {
                o.set_yf(bottom_edge);
                o.set_xf(right_edge);
                o.flipsidesifneeded();
            }
        }

        //[X][X]
        //[X][ ]
        7 => {
            o.set_yf(top_edge);
            o.set_xf(left_edge);
            o.flipsidesifneeded();
        }

        //[ ][ ]
        //[ ][X]
        8 => {
            if center < ((nofliptxr as i32) << 5) {
                o.set_xf(right_edge);
                o.flipsidesifneeded();
            } else {
                o.set_yf(bottom_edge);
            }
        }

        //[X][ ]
        //[ ][X]
        9 => {
            if center > ((nofliptxl as i32) << 5) + TILESIZE {
                o.set_yf(bottom_edge);
                o.set_xf(left_edge);
                o.flipsidesifneeded();
            } else {
                o.set_yf(top_edge);
                o.set_xf(right_edge);
                o.flipsidesifneeded();
            }
        }

        //[ ][X]
        //[ ][X]
        10 => {
            o.set_xf(right_edge);
            o.flipsidesifneeded();
        }

        //[X][X]
        //[ ][X]
        11 => {
            o.set_yf(top_edge);
            o.set_xf(right_edge);
            o.flipsidesifneeded();
        }

        //[ ][ ]
        //[X][X]
        12 => {
            o.set_yf(bottom_edge);
        }

        //[X][ ]
        //[X][X]
        13 => {
            o.set_yf(bottom_edge);
            o.set_xf(left_edge);
            o.flipsidesifneeded();
        }

        //[ ][X]
        //[X][X]
        14 => {
            o.set_yf(bottom_edge);
            o.set_xf(right_edge);
            o.flipsidesifneeded();
        }

        // If object is completely inside a block, default to moving it down
        //[X][X]
        //[X][X]
        15 => {
            o.set_yf((((ty2 as i32) << 5) + TILESIZE) as f32 + 0.2);
        }

        _ => {
            fRet = false;
        }
    }

    // Held objects skip map collision, so `fPrecalculatedY` is stale here.
    o.fPrecalculatedY = o.fy;

    // Check moving platforms and make sure this object is not inside one
    fRet |= g.g_map.moving_platform_check_sides(o);

    fRet
}

/// Kills the object on a map hazard. Mode objects (flags, stars, eggs, keys, boxes-mode coins) respawn elsewhere.
/// In stomp mode, enemies credit a point to `playerID` (-1 for no one).
pub fn io_moving_object_kill_object_map_hazard<T: IO_MovingObjectTrait + ?Sized>(g: &mut Game, this: &mut T, playerID: i16) {
        let mut gp = g.ptr();
    if this.mo().dead {
        return;
    }

    let movingObjectType = this.mo().movingObjectType;

    // If it is a throw box, trigger it's behavior and return
    if movingObjectType == movingobject_throwbox {
        this.die(g);
        return;
    }

    {
        let o = this.mo_mut();
        o.dead = true;
        g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
            g.rm.spr_fireballexplosion,
            (o.ix as i32 + (o.iw as i32 >> 1) - 16) as i16,
            (o.iy as i32 + (o.ih as i32 >> 1) - 16) as i16,
            3,
            4,
        ));
    }

    if movingObjectType == movingobject_fireball {
        let mut player = get_player_from_global_id(g, this.mo().iPlayerID);
        if !player.is_none() {
            g.player(player).decrease_projectiles_count();
        }

        if_sound_on_play(g, &mut gp.rm.sfx_hit);
    } else if movingObjectType == movingobject_egg {
        this.mo_mut().dead = false;
        this.as_any().downcast_mut::<CO_Egg>().unwrap().place_egg(g);
        if_sound_on_play(g, &mut gp.rm.sfx_transform);
    } else if movingObjectType == movingobject_star {
        this.mo_mut().dead = false;
        this.as_any().downcast_mut::<CO_Star>().unwrap().place_star(g);
        if_sound_on_play(g, &mut gp.rm.sfx_transform);
    } else if movingObjectType == movingobject_flag {
        this.mo_mut().dead = false;
        this.as_any().downcast_mut::<CO_Flag>().unwrap().place_flag(g);
        if_sound_on_play(g, &mut gp.rm.sfx_transform);
    } else if movingObjectType == movingobject_bomb {
        let mut player = get_player_from_global_id(g, this.mo().iPlayerID);
        if !player.is_none() {
            g.player(player).decrease_projectiles_count();
        }

        if_sound_on_play(g, &mut gp.rm.sfx_hit);
    } else if movingObjectType == movingobject_phantokey {
        this.mo_mut().dead = false;
        this.as_any().downcast_mut::<CO_PhantoKey>().unwrap().place_key(g);
        if_sound_on_play(g, &mut gp.rm.sfx_transform);
    } else if game_mode_boxes_minigame == g.mode().gamemode && movingObjectType == movingobject_coin {
        this.mo_mut().dead = false;
        this.as_any().downcast_mut::<MO_Coin>().unwrap().place_coin(g);
        if_sound_on_play(g, &mut gp.rm.sfx_transform);
    } else if g.mode().gamemode == game_mode_stomp
        && (movingObjectType == movingobject_goomba
            || movingObjectType == movingobject_koopa
            || movingObjectType == movingobject_spiny
            || movingObjectType == movingobject_buzzybeetle
            || movingObjectType == movingobject_cheepcheep)
    {
        if !g.mode().gameover {
            let mut player = get_player_from_global_id(g, playerID);

            if !player.is_none() {
                g.player(player).score(g).adjust_score(g, 1);
            }
        }

        if_sound_on_play(g, &mut gp.rm.sfx_hit);
    } else {
        if_sound_on_play(g, &mut gp.rm.sfx_hit);
    }
}

/// Virtual interface added by `IO_MovingObject` (`draw`, `update`, `collide` are on `CObjectTrait`).
/// The `IO_MovingObject` fields of `IO_MovingObjectTrait::harness_hash`, and the carried-object, walking-enemy and
/// powerup fields for those kinds.
pub fn mo_harness_hash<T: IO_MovingObjectTrait + ?Sized>(this: &mut T, h: &mut crate::smw::rollback::Fnv) {
    let m = this.mo();
    h.i16s(&[m.iPlayerID, m.iTeamID, m.drawframe, m.animationtimer, m.iHorizontalPlatformCollision, m.iVerticalPlatformCollision]);
    h.f32s(&[m.fOldX, m.fOldY, m.fPrecalculatedY, m.bounce]);
    h.bools(&[m.inair, m.onice, m.platform.is_none(), m.fObjectDiesOnSuperDeathTiles, m.fObjectCollidesWithMap]);
    h.u32(m.movingObjectType as u32);
    if let Some(c) = this.as_carried_object() {
        let c = c.carried();
        h.player(c.owner);
        h.bools(&[c.fCarriedByKuriboShoe, c.fSmoking]);
        h.f32s(&[c.dKickX, c.dKickY]);
        h.i16s(&[c.iOwnerLeftOffset, c.iOwnerRightOffset, c.iOwnerUpOffset]);
    }
    if let Some(w) = this.as_walking_enemy() {
        let w = w.we();
        h.f32s(&[w.spawnradius, w.spawnangle, w.frozenvelocity]);
        h.i16s(&[w.killStyle as i16, w.burnuptimer, w.frozentimer, w.frozenanimationspeed]);
        h.bools(&[w.fKillOnWeakWeapon, w.fBouncing, w.fFallOffLedges, w.frozen]);
    }
    if let Some(p) = this.as_powerup() {
        h.f32s(&[p.pu().desty]);
    }
}

pub trait IO_MovingObjectTrait: CObjectTrait {
    fn mo(&self) -> &IO_MovingObject;
    fn mo_mut(&mut self) -> &mut IO_MovingObject;
    fn as_mo_ptr(&mut self) -> Ptr<dyn IO_MovingObjectTrait>;

    /// Feeds the object's simulated state beyond `CObject` into a desync checksum; overrides add their own fields.
    fn harness_hash(&mut self, h: &mut crate::smw::rollback::Fnv) {
        mo_harness_hash(self, h);
    }

    /// Advances the animation; called once per update.
    fn animate(&mut self) {
        io_moving_object_animate(self.mo_mut())
    }

    /// Called after hitting a wall on the right (`_fRightSide`) or left.
    fn side_bounce(&mut self, g: &mut Game, _fRightSide: bool) {}
    /// Returns the vertical velocity to take on landing.
    fn bottom_bounce(&mut self, g: &mut Game) -> f32 {
        self.mo().bounce
    }

    /// Called when the object hits a player or object; kills it unless it survives that hit.
    fn check_and_die(&mut self, g: &mut Game) {
        self.mo_mut().dead = true;
    }
    /// Removes the object; overrides add death effects or respawn.
    fn die(&mut self, g: &mut Game) {
        self.mo_mut().dead = true;
    }

    /// Kills the object on a map hazard; see [`io_moving_object_kill_object_map_hazard`].
    fn kill_object_map_hazard(&mut self, g: &mut Game, playerID: i16) {
        io_moving_object_kill_object_map_hazard(g, self, playerID)
    }

    /// Downcasts to the intermediate base traits; `None` unless the object implements one.
    fn as_carried_object(&mut self) -> Option<&mut dyn MO_CarriedObjectTrait> {
        None
    }
    fn as_powerup(&mut self) -> Option<&mut dyn MO_PowerupTrait> {
        None
    }
    fn as_walking_enemy(&mut self) -> Option<&mut dyn MO_WalkingEnemyTrait> {
        None
    }
}

impl Deref for dyn IO_MovingObjectTrait {
    type Target = IO_MovingObject;
    #[inline(always)]
    fn deref(&self) -> &IO_MovingObject {
        self.mo()
    }
}

impl DerefMut for dyn IO_MovingObjectTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut IO_MovingObject {
        self.mo_mut()
    }
}

/// Implements `mo`, `mo_mut`, `as_mo_ptr` for a type that derefs to `IO_MovingObject`.
#[macro_export]
macro_rules! impl_io_moving_object_plumbing {
    () => {
        fn mo(&self) -> &$crate::smw::objects::moving::moving_object::IO_MovingObject {
            self
        }
        fn mo_mut(&mut self) -> &mut $crate::smw::objects::moving::moving_object::IO_MovingObject {
            self
        }
        fn as_mo_ptr(&mut self) -> $crate::globals::Ptr<dyn $crate::smw::objects::moving::moving_object::IO_MovingObjectTrait> {
            $crate::globals::Ptr::from_mut(self as &mut dyn $crate::smw::objects::moving::moving_object::IO_MovingObjectTrait)
        }
    };
}
