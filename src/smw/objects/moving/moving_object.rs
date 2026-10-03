//! Port of src/smw/objects/moving/MovingObject.cpp

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
use crate::smw::gs_gameplay::eyecandy;
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::carriable::{co_egg::CO_Egg, co_flag::CO_Flag, co_phanto_key::CO_PhantoKey, co_star::CO_Star};
use crate::smw::objects::moving::mo_coin::MO_Coin;
use crate::smw::player::get_player_from_global_id;
use crate::impl_base;
use crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait;
use crate::smw::objects::powerup::powerup::MO_PowerupTrait;
use crate::smw::objects::walkingenemy::walking_enemy::MO_WalkingEnemyTrait;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;
use std::ops::{Deref, DerefMut};

pub struct IO_MovingObject {
    pub cobject: CObject,

    pub iPlayerID: i16,
    pub iTeamID: i16,

    pub fOldX: f32,
    pub fOldY: f32,
    pub fPrecalculatedY: f32,

    pub iNumSprites: i16,
    pub drawframe: i16,
    pub animationtimer: i16,

    pub bounce: f32,

    pub animationspeed: i16,
    pub animationWidth: i16,
    pub animationOffsetX: i16,
    pub animationOffsetY: i16,

    pub inair: bool,
    pub onice: bool,

    pub movingObjectType: MovingObjectType,

    pub platform: Ptr<MovingPlatform>,
    pub iHorizontalPlatformCollision: i16,
    pub iVerticalPlatformCollision: i16,

    pub fObjectDiesOnSuperDeathTiles: bool,
    pub fObjectCollidesWithMap: bool,
}
impl_base!(IO_MovingObject => cobject: CObject);

impl IO_MovingObject {
    /// Pass `-1` for the C++ default arguments.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        nspr: Ptr<gfxSprite>,
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
        let mut o = IO_MovingObject {
            cobject: CObject::new(nspr, pos),
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
            platform: Ptr::null(),
            iHorizontalPlatformCollision: 0,
            iVerticalPlatformCollision: 0,
            fObjectDiesOnSuperDeathTiles: false,
            fObjectCollidesWithMap: false,
        };

        o.iNumSprites = iNumSpr;

        if iAnimationWidth > -1 {
            o.iw = iAnimationWidth;
        } else if !o.spr.is_null() {
            o.iw = (o.spr.get_width() as i16) / o.iNumSprites;
        }

        if iAnimationHeight > -1 {
            o.ih = iAnimationHeight;
        }

        o.animationtimer = 0;

        if !o.spr.is_null() {
            o.animationWidth = o.spr.get_width() as i16;
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

        o.platform = Ptr::null();
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

pub fn io_moving_object_draw<T: IO_MovingObjectTrait + ?Sized>(this: &mut T) {
    let o = this.mo();
    o.spr.draw_src(
        (o.ix - o.collisionOffsetX) as i32,
        (o.iy - o.collisionOffsetY) as i32,
        &SDL_Rect { x: o.drawframe as i32, y: o.animationOffsetY as i32, w: o.iw as i32, h: o.ih as i32 },
    );
}

pub fn io_moving_object_update<T: IO_MovingObjectTrait + ?Sized>(this: &mut T) {
    {
        let o = this.mo_mut();
        o.fOldX = o.fx;
        o.fOldY = o.fy;
    }

    io_moving_object_collision_detection_map(this);

    this.animate();
}

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

pub fn io_moving_object_collide_player<T: IO_MovingObjectTrait + ?Sized>(this: &mut T, _player: Ptr<CPlayer>) -> bool {
    this.mo_mut().dead = true;
    false
}

// TODO / FIXME: The collision code is almost the same as in CPlayer
pub fn io_moving_object_collision_detection_map<T: IO_MovingObjectTrait + ?Sized>(this: &mut T) {
    let mut o = this.as_mo_ptr();
    unsafe {
        let x = o.fx + o.velx;
        o.set_xf(x);
        o.flipsidesifneeded();

        o.fPrecalculatedY = o.fy + o.vely; // Fixes weird float rounding error.  Must be computed here before casting to int.  Otherwise, this will miss the bottom collision, but then hit the side collision and the player can slide out of 1x1 spaces.

        let mut fPlatformVelX: f32 = 0.0;
        let mut fPlatformVelY: f32 = 0.0;

        let fTempY = o.fy;

        if !o.platform.is_null() {
            if !o.onice {
                fPlatformVelX = o.platform.fVelX;
                let x = o.fx + fPlatformVelX;
                o.set_xf(x);
                o.flipsidesifneeded();
            }

            fPlatformVelY = o.platform.fVelY;

            if o.platform.fOldVelY < 0.0 {
                o.fy += o.platform.fOldVelY;
            }

            o.fPrecalculatedY += o.platform.fOldVelY;
        }

        o.iHorizontalPlatformCollision = -1;
        o.iVerticalPlatformCollision = -1;

        g_map.moving_platform_collision_object(o);

        o.fy = fTempY;

        if o.fPrecalculatedY + (o.collisionHeight as f32) < 0.0 {
            // on top outside of the screen
            let y = o.fPrecalculatedY;
            o.set_yf(y);
            o.vely = cap_falling_velocity(GRAVITATION + o.vely);

            if o.platform.is_null() {
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

        // Could be optimized with bit shift >> 5
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

                let topblock = g_map.block(tx, ty);
                let bottomblock = g_map.block(tx, ty2);

                let fTopBlockSolid = !topblock.is_null() && !topblock.get().is_transparent() && !topblock.get().is_hidden();
                let fBottomBlockSolid = !bottomblock.is_null() && !bottomblock.get().is_transparent() && !bottomblock.get().is_hidden();
                if fTopBlockSolid || fBottomBlockSolid {
                    let mut processOtherBlock = true;
                    if fTopBlockSolid {
                        // collide with top block
                        if o.iHorizontalPlatformCollision == 3 {
                            o.kill_object_map_hazard(-1);
                            return;
                        }

                        topblock.get().collide_object_dir(o, 1);
                        o.flipsidesifneeded();
                        removeifprojectile(o, true, true);

                        processOtherBlock = false;

                        o.side_bounce(true);
                    }

                    if processOtherBlock && fBottomBlockSolid {
                        // then bottom
                        if o.iHorizontalPlatformCollision == 3 {
                            o.kill_object_map_hazard(-1);
                            return;
                        }

                        bottomblock.get().collide_object_dir(o, 1);
                        o.flipsidesifneeded();
                        removeifprojectile(o, true, true);

                        o.side_bounce(true);
                    }
                } else if (g_map.map(tx as i32, ty as i32) & tile_flag_solid) != 0 || (g_map.map(tx as i32, ty2 as i32) & tile_flag_solid) != 0 {
                    // collision on the right side.

                    if o.iHorizontalPlatformCollision == 3 {
                        o.kill_object_map_hazard(-1);
                        return;
                    }

                    let x = (((tx as i32) << 5) - o.collisionWidth as i32) as f32 - 0.2; // move to the edge of the tile (tile on the right -> mind the object width)
                    o.set_xf(x);
                    o.fOldX = o.fx;

                    if o.velx > 0.0 {
                        o.velx = -o.velx;
                    }

                    o.flipsidesifneeded();
                    removeifprojectile(o, true, true);

                    o.side_bounce(true);
                }
            } else if o.velx + fPlatformVelX < -0.01 || o.iHorizontalPlatformCollision == 1 {
                // moving left
                tx = ((o.fx as i16) as i32 / TILESIZE) as i16;

                // Just in case fx < 0 and wasn't caught by flipsidesifneeded()
                if tx < 0 {
                    tx = 0;
                }

                let topblock = g_map.block(tx, ty);
                let bottomblock = g_map.block(tx, ty2);

                let fTopBlockSolid = !topblock.is_null() && !topblock.get().is_transparent() && !topblock.get().is_hidden();
                let fBottomBlockSolid = !bottomblock.is_null() && !bottomblock.get().is_transparent() && !bottomblock.get().is_hidden();
                if fTopBlockSolid || fBottomBlockSolid {
                    let mut processOtherBlock = true;
                    if fTopBlockSolid {
                        // collide with top block
                        if o.iHorizontalPlatformCollision == 1 {
                            o.kill_object_map_hazard(-1);
                            return;
                        }

                        topblock.get().collide_object_dir(o, 3);
                        o.flipsidesifneeded();
                        removeifprojectile(o, true, true);

                        processOtherBlock = false;

                        o.side_bounce(false);
                    }

                    if processOtherBlock && fBottomBlockSolid {
                        // then bottom
                        if o.iHorizontalPlatformCollision == 1 {
                            o.kill_object_map_hazard(-1);
                            return;
                        }

                        bottomblock.get().collide_object_dir(o, 3);
                        o.flipsidesifneeded();
                        removeifprojectile(o, true, true);

                        o.side_bounce(false);
                    }
                } else if (g_map.map(tx as i32, ty as i32) & tile_flag_solid) != 0 || (g_map.map(tx as i32, ty2 as i32) & tile_flag_solid) != 0 {
                    if o.iHorizontalPlatformCollision == 1 {
                        o.kill_object_map_hazard(-1);
                        return;
                    }

                    let x = (((tx as i32) << 5) + TILESIZE) as f32 + 0.2; // move to the edge of the tile
                    o.set_xf(x);
                    o.fOldX = o.fx;

                    if o.velx < 0.0 {
                        o.velx = -o.velx;
                    }

                    o.flipsidesifneeded();
                    removeifprojectile(o, true, true);

                    o.side_bounce(false);
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
        if !o.platform.is_null() {
            fMovingUp = o.vely + fPlatformVelY - o.bounce;
        }

        if fMovingUp < -0.01 {
            ty = ((o.fPrecalculatedY as i16) as i32 / TILESIZE) as i16;

            let leftblock = g_map.block(txl, ty);
            let rightblock = g_map.block(txr, ty);

            if !leftblock.is_null() && !leftblock.get().is_transparent() && !leftblock.get().is_hidden() {
                // then left
                if o.iVerticalPlatformCollision == 2 {
                    o.kill_object_map_hazard(-1);
                }

                leftblock.get().collide_object_dir(o, 0);
                return;
            }

            if !rightblock.is_null() && !rightblock.get().is_transparent() && !rightblock.get().is_hidden() {
                // then right
                if o.iVerticalPlatformCollision == 2 {
                    o.kill_object_map_hazard(-1);
                }

                rightblock.get().collide_object_dir(o, 0);
                return;
            }

            if (g_map.map(txl as i32, ty as i32) & tile_flag_solid) != 0 || (g_map.map(txr as i32, ty as i32) & tile_flag_solid) != 0 {
                if o.iVerticalPlatformCollision == 2 {
                    o.kill_object_map_hazard(-1);
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

            if o.platform.is_null() {
                o.inair = true;
                o.onice = false;
            }
        } else {
            // moving down / on ground
            ty = (((o.fPrecalculatedY as i16) as i32 + o.collisionHeight as i32) / TILESIZE) as i16;

            let leftblock = g_map.block(txl, ty);
            let rightblock = g_map.block(txr, ty);

            let fLeftBlockSolid = !leftblock.is_null() && !leftblock.get().is_transparent() && !leftblock.get().is_hidden();
            let fRightBlockSolid = !rightblock.is_null() && !rightblock.get().is_transparent() && !rightblock.get().is_hidden();

            if fLeftBlockSolid || fRightBlockSolid {
                let mut processOtherBlock = true;
                if fLeftBlockSolid {
                    // collide with left block
                    processOtherBlock = leftblock.get().collide_object_dir(o, 2);

                    if o.platform.is_null() {
                        o.inair = false;
                        o.onice = false;
                    }
                }

                if processOtherBlock && fRightBlockSolid {
                    // then right
                    rightblock.get().collide_object_dir(o, 2);

                    if o.platform.is_null() {
                        o.inair = false;
                        o.onice = false;
                    }
                }

                if o.iVerticalPlatformCollision == 0 {
                    o.kill_object_map_hazard(-1);
                }

                return;
            }

            let leftTile: i32 = g_map.map(txl as i32, ty as i32);
            let rightTile: i32 = g_map.map(txr as i32, ty as i32);

            if (leftTile & tile_flag_solid_on_top) != 0 || (rightTile & tile_flag_solid_on_top) != 0 {
                if (o.fOldY + o.collisionHeight as f32) / (TILESIZE as f32) < ty as f32 {
                    o.vely = o.bottom_bounce();
                    let y = (((ty as i32) << 5) - o.collisionHeight as i32) as f32 - 0.2;
                    o.set_yf(y);
                    o.fOldY = o.fy - GRAVITATION;

                    if o.platform.is_null() {
                        o.inair = false;
                        o.onice = false;
                    }

                    o.platform = Ptr::null();

                    if o.iVerticalPlatformCollision == 0 {
                        o.kill_object_map_hazard(-1);
                    }

                    return;
                }
            }

            let fSuperDeathTileUnderObject = o.fObjectDiesOnSuperDeathTiles
                && (((leftTile & tile_flag_super_death_top) != 0 && (rightTile & tile_flag_super_death_top) != 0)
                    || ((leftTile & tile_flag_super_death_top) != 0 && (rightTile & tile_flag_solid) == 0)
                    || ((leftTile & tile_flag_solid) == 0 && (rightTile & tile_flag_super_death_top) != 0));

            if ((leftTile & tile_flag_solid) != 0 || (rightTile & tile_flag_solid) != 0) && !fSuperDeathTileUnderObject {
                o.vely = o.bottom_bounce();
                let y = (((ty as i32) << 5) - o.collisionHeight as i32) as f32 - 0.2;
                o.set_yf(y);
                o.fOldY = o.fy;

                if o.platform.is_null() {
                    o.inair = false;

                    if ((leftTile & tile_flag_ice) != 0 && ((rightTile & tile_flag_ice) != 0 || rightTile == tile_flag_nonsolid || rightTile == tile_flag_gap))
                        || ((rightTile & tile_flag_ice) != 0 && ((leftTile & tile_flag_ice) != 0 || leftTile == tile_flag_nonsolid || leftTile == tile_flag_gap))
                    {
                        o.onice = true;
                    } else {
                        o.onice = false;
                    }
                }

                o.platform = Ptr::null();

                if o.iVerticalPlatformCollision == 0 {
                    o.kill_object_map_hazard(-1);
                }
            } else if fSuperDeathTileUnderObject {
                o.kill_object_map_hazard(-1);
                return;
            } else {
                let y = o.fPrecalculatedY;
                o.set_yf(y);
                o.vely = cap_falling_velocity(GRAVITATION + o.vely);

                if o.platform.is_null() {
                    o.inair = true;
                }
            }
        }

        if o.platform.is_null() && o.inair {
            o.onice = false;
        }
    }
}

fn block_is_solid(block: Ptr<dyn IO_BlockTrait>) -> bool {
    !block.is_null() && !block.get().is_transparent() && !block.get().is_hidden()
}

// This method checks the object against the map and moves it outside of any tiles or blocks it might be inside of
pub fn io_moving_object_collision_detection_checksides<T: IO_MovingObjectTrait + ?Sized>(this: &mut T) -> bool {
    let mut o = this.as_mo_ptr();
    unsafe {
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
                    if block_is_solid(g_map.block(txl, ty)) || (g_map.map(txl as i32, ty as i32) & tile_flag_solid) > 0 {
                        iCase |= 0x01;
                    }
                }

                if txr >= 0 && (txr as i32) < MAPWIDTH {
                    if block_is_solid(g_map.block(txr, ty)) || (g_map.map(txr as i32, ty as i32) & tile_flag_solid) > 0 {
                        iCase |= 0x02;
                    }
                }
            }
        }

        if (o.iy as i32 + o.collisionHeight as i32) as f32 >= 0.0 {
            if (ty2 as i32) < MAPHEIGHT {
                if txl >= 0 && (txl as i32) < MAPWIDTH {
                    if block_is_solid(g_map.block(txl, ty2)) || (g_map.map(txl as i32, ty2 as i32) & tile_flag_solid) > 0 {
                        iCase |= 0x04;
                    }
                }

                if txr >= 0 && (txr as i32) < MAPWIDTH {
                    if block_is_solid(g_map.block(txr, ty2)) || (g_map.map(txr as i32, ty2 as i32) & tile_flag_solid) > 0 {
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

        // Updated object to have correct precalculatedY since it wasn't getting collision detection updates
        // while it was being held by the player
        o.fPrecalculatedY = o.fy;

        // Check moving platforms and make sure this object is not inside one
        fRet |= g_map.moving_platform_check_sides(o);

        fRet
    }
}

// This method probably needs to be rewritten so that it just calls into the object being killed
// And let it handle not dying and placing itself somewhere else if necessary
/// `KillObjectMapHazard(short playerID = -1)`
pub fn io_moving_object_kill_object_map_hazard<T: IO_MovingObjectTrait + ?Sized>(this: &mut T, playerID: i16) {
    unsafe {
        if this.mo().dead {
            return;
        }

        let movingObjectType = this.mo().movingObjectType;

        // If it is a throw box, trigger it's behavior and return
        if movingObjectType == movingobject_throwbox {
            this.die();
            return;
        }

        {
            let o = this.mo_mut();
            o.dead = true;
            eyecandy[2].emplace(EC_SingleAnimation::new(
                Ptr::from_mut(&mut rm.spr_fireballexplosion),
                (o.ix as i32 + (o.iw as i32 >> 1) - 16) as i16,
                (o.iy as i32 + (o.ih as i32 >> 1) - 16) as i16,
                3,
                4,
            ));
        }

        if movingObjectType == movingobject_fireball {
            let mut player = get_player_from_global_id(this.mo().iPlayerID);
            if !player.is_null() {
                player.decrease_projectiles_count();
            }

            if_sound_on_play(&mut rm.sfx_hit);
        } else if movingObjectType == movingobject_egg {
            this.mo_mut().dead = false;
            this.as_any().downcast_mut::<CO_Egg>().unwrap().place_egg();
            if_sound_on_play(&mut rm.sfx_transform);
        } else if movingObjectType == movingobject_star {
            this.mo_mut().dead = false;
            this.as_any().downcast_mut::<CO_Star>().unwrap().place_star();
            if_sound_on_play(&mut rm.sfx_transform);
        } else if movingObjectType == movingobject_flag {
            this.mo_mut().dead = false;
            this.as_any().downcast_mut::<CO_Flag>().unwrap().place_flag();
            if_sound_on_play(&mut rm.sfx_transform);
        } else if movingObjectType == movingobject_bomb {
            let mut player = get_player_from_global_id(this.mo().iPlayerID);
            if !player.is_null() {
                player.decrease_projectiles_count();
            }

            if_sound_on_play(&mut rm.sfx_hit);
        } else if movingObjectType == movingobject_phantokey {
            this.mo_mut().dead = false;
            this.as_any().downcast_mut::<CO_PhantoKey>().unwrap().place_key();
            if_sound_on_play(&mut rm.sfx_transform);
        } else if game_mode_boxes_minigame == game_values.gamemode.gamemode && movingObjectType == movingobject_coin {
            this.mo_mut().dead = false;
            this.as_any().downcast_mut::<MO_Coin>().unwrap().place_coin();
            if_sound_on_play(&mut rm.sfx_transform);
        } else if game_values.gamemode.gamemode == game_mode_stomp
            && (movingObjectType == movingobject_goomba
                || movingObjectType == movingobject_koopa
                || movingObjectType == movingobject_spiny
                || movingObjectType == movingobject_buzzybeetle
                || movingObjectType == movingobject_cheepcheep)
        {
            if !game_values.gamemode.gameover {
                let mut player = get_player_from_global_id(playerID);

                if !player.is_null() {
                    player.score.adjust_score(1);
                }
            }

            if_sound_on_play(&mut rm.sfx_hit);
        } else {
            if_sound_on_play(&mut rm.sfx_hit);
        }
    }
}

/// Virtual interface added by `IO_MovingObject` (`draw`, `update`, `collide` are on `CObjectTrait`).
pub trait IO_MovingObjectTrait: CObjectTrait {
    fn mo(&self) -> &IO_MovingObject;
    fn mo_mut(&mut self) -> &mut IO_MovingObject;
    fn as_mo_ptr(&mut self) -> Ptr<dyn IO_MovingObjectTrait>;

    fn animate(&mut self) {
        io_moving_object_animate(self.mo_mut())
    }

    fn side_bounce(&mut self, _fRightSide: bool) {}
    fn bottom_bounce(&mut self) -> f32 {
        self.mo().bounce
    }

    fn check_and_die(&mut self) {
        self.mo_mut().dead = true;
    }
    fn die(&mut self) {
        self.mo_mut().dead = true;
    }

    /// Non-virtual in C++; on the trait so it can dispatch to `die()` / downcasts.
    fn kill_object_map_hazard(&mut self, playerID: i16) {
        io_moving_object_kill_object_map_hazard(self, playerID)
    }

    /// `static_cast` / `dynamic_cast` hooks for the intermediate bases.
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
