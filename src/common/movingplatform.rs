//! Port of src/common/movingplatform.cpp

use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::util::sdl_helpers::SdlSurfacePtr;
use crate::common::game::App;
use crate::common::gfx::gfx_drawpreview;
use crate::common::global::*;
use crate::common::global_constants::*;
use crate::common::map::{g_iCurrentDrawIndex, TilesetTile};
use crate::common::moving_platform_paths::MovingPlatformPathTrait;
use crate::common::object_base::cap_falling_velocity;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::tile_types::*;
use crate::globals::{Aliased, Ptr};
use crate::smw::main::{blitdest, screen, x_shake, y_shake};
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;
use sdl2::sys::*;
use std::ffi::CStr;
use std::ptr::{null, null_mut};

type CollisionStyle = i16;
const collision_none: CollisionStyle = 0;
const collision_normal: CollisionStyle = 1;
const collision_overlap_left: CollisionStyle = 2;
const collision_overlap_right: CollisionStyle = 3;

const SW: f32 = App::screenWidth as f32;

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

/// `(short)f / TILESIZE` as an index.
#[inline(always)]
fn tile_of(f: f32) -> i16 {
    ((f as i16) as i32 / TILESIZE) as i16
}

pub struct MovingPlatform {
    pub iTileData: Vec<TilesetTile>,
    pub iTileType: Vec<TileType>,
    pub ix: i16,
    pub iy: i16,
    pub iWidth: i16,
    pub iHeight: i16,
    pub iTileWidth: i16,
    pub iTileHeight: i16,
    pub iHalfWidth: i16,
    pub iHalfHeight: i16,

    pub fDead: bool,

    pub fx: f32,
    pub fy: f32,
    pub fOldX: f32,
    pub fOldY: f32,
    pub fOldVelX: f32,
    pub fOldVelY: f32,

    pub iSteps: i16,
    pub iOnStep: i16,

    pub sprites: [gfxSprite; 2],

    pub rSrcRect: SDL_Rect,
    pub rDstRect: SDL_Rect,

    pub fForwardDirection: bool,
    pub fStartDirection: bool,

    pub iDrawLayer: i16,

    pub pPath: Box<dyn MovingPlatformPathTrait>,

    pub fVelX: f32,
    pub fVelY: f32,

    pub iPlayerId: i16,

    pub _alias: Aliased,
}

impl MovingPlatform {
    /// C++ `new MovingPlatform(...)`.
    pub fn new(
        tiledata: Vec<TilesetTile>,
        tiletypes: Vec<TileType>,
        w: i16,
        h: i16,
        drawlayer: i16,
        path: Box<dyn MovingPlatformPathTrait>,
        fPreview: bool,
    ) -> Ptr<MovingPlatform> {
        let mut iTileSize: i16 = TILESIZE as i16;
        let mut iTileSizeIndex: i16 = 0;

        if fPreview {
            iTileSize = PREVIEWTILESIZE as i16;
            iTileSizeIndex = 1;
        }

        assert!(tiledata.len() == tiletypes.len());
        assert!(tiledata.len() == (w as i32 * h as i32) as usize);

        let iWidth = (w as i32 * iTileSize as i32) as i16;
        let iHeight = (h as i32 * iTileSize as i32) as i16;

        let mut this = Ptr::new_box(MovingPlatform {
            iTileData: tiledata,
            iTileType: tiletypes,
            ix: 0,
            iy: 0,
            iWidth,
            iHeight,
            iTileWidth: w,
            iTileHeight: h,
            iHalfWidth: iWidth >> 1,
            iHalfHeight: iHeight >> 1,
            fDead: false,
            fx: 0.0,
            fy: 0.0,
            fOldX: 0.0,
            fOldY: 0.0,
            fOldVelX: 0.0,
            fOldVelY: 0.0,
            iSteps: 0,
            iOnStep: 0,
            sprites: [gfxSprite::new(), gfxSprite::new()],
            rSrcRect: SDL_Rect { x: 0, y: 0, w: 0, h: 0 },
            rDstRect: SDL_Rect { x: 0, y: 0, w: 0, h: 0 },
            fForwardDirection: false,
            fStartDirection: false,
            iDrawLayer: drawlayer,
            pPath: path,
            fVelX: 0.0,
            fVelY: 0.0,
            iPlayerId: -1,
            _alias: Aliased::new(),
        });

        let self_ptr = this;
        this.pPath.path_mut().set_platform(self_ptr);

        this.reset_path();

        unsafe {
            for iSurface in 0..2 {
                let surf = SdlSurfacePtr::new(SDL_CreateRGBSurface(
                    0x0,
                    w as i32 * iTileSize as i32,
                    h as i32 * iTileSize as i32,
                    (*(*screen).format).BitsPerPixel as i32,
                    0,
                    0,
                    0,
                    0,
                ));

                if SDL_SetColorKey(surf.get(), SDL_bool::SDL_TRUE as i32, SDL_MapRGB(surf.format, 255, 0, 255)) < 0 {
                    print!("\n ERROR: Couldn't set ColorKey for moving platform: {}\n", sdl_error());
                }

                SDL_FillRect(surf.get(), null(), SDL_MapRGB(surf.format, 255, 0, 255));

                this.sprites[iSurface] = gfxSprite::from_surface(surf, Some(640));
                this.sprites[iSurface].set_wrap(640);
            }

            for iSurface in 0..2 {
                for iCol in 0..this.iTileWidth {
                    for iRow in 0..this.iTileHeight {
                        let tile = this.iTileData[(iCol as i32 * this.iTileHeight as i32 + iRow as i32) as usize];

                        if tile.iID as i32 == TILESETNONE {
                            continue;
                        }

                        if tile.iID >= 0 {
                            g_tilesetmanager.draw(this.sprites[iSurface].get_surface(), tile.iID, iTileSizeIndex, tile.iCol, tile.iRow, iCol, iRow);
                        } else if tile.iID as i32 == TILESETANIMATED {
                            rm.spr_tileanimation[iTileSizeIndex as usize].draw_src_to(
                                &*g_tilesetmanager.rect(iTileSizeIndex, tile.iCol * 4, tile.iRow),
                                this.sprites[iSurface].get_surface(),
                                &*g_tilesetmanager.rect(iTileSizeIndex, iCol, iRow),
                            );
                        } else if tile.iID as i32 == TILESETUNKNOWN {
                            rm.spr_unknowntile[iTileSizeIndex as usize].draw_src_to(
                                &*g_tilesetmanager.rect(iTileSizeIndex, 0, 0),
                                this.sprites[iSurface].get_surface(),
                                &*g_tilesetmanager.rect(iTileSizeIndex, iCol, iRow),
                            );
                        }
                    }
                }
            }
        }

        this.rSrcRect.x = 0;
        this.rSrcRect.y = 0;
        this.rSrcRect.w = w as i32 * iTileSize as i32;
        this.rSrcRect.h = h as i32 * iTileSize as i32;

        this.rDstRect.x = this.ix as i32 - this.iHalfWidth as i32;
        this.rDstRect.y = this.iy as i32 - this.iHalfHeight as i32;
        this.rDstRect.w = w as i32 * iTileSize as i32;
        this.rDstRect.h = h as i32 * iTileSize as i32;

        this.fVelX = this.pPath.velocity0().x;
        this.fVelY = this.pPath.velocity0().y;

        this.fOldVelX = this.fVelX;
        this.fOldVelY = this.fVelY;

        this
    }

    pub fn tile_at(&self, col: usize, row: usize) -> &TilesetTile {
        &self.iTileData[col * self.iTileHeight as usize + row]
    }

    pub fn tile_type_at(&self, col: usize, row: usize) -> TileType {
        self.iTileType[col * self.iTileHeight as usize + row]
    }

    //Draw a custom sprite on a tile of the platform, instead of a tileset tile
    pub fn paint_sprite_at(&mut self, spr: &gfxSprite, col: usize, row: usize) {
        let iTileSize: i16 = self.iWidth / self.iTileWidth;
        let dstPos = SDL_Rect { x: col as i32 * iTileSize as i32, y: row as i32 * iTileSize as i32, w: 0, h: 0 };

        for layer in &self.sprites {
            spr.draw_to(layer.get_surface(), &dstPos);
        }
    }

    fn flags_at(&self, col: i16, row: i16) -> i32 {
        tile_to_flags(self.tile_type_at(col as usize, row as usize)) as i32
    }

    pub fn set_xf(&mut self, xf: f32) {
        self.fx = xf;
        self.ix = self.fx as i16;
    }
    pub fn set_xi(&mut self, xi: i16) {
        self.ix = xi;
        self.fx = self.ix as f32;
    }
    pub fn set_yf(&mut self, yf: f32) {
        self.fy = yf;
        self.iy = self.fy as i16;
    }
    pub fn set_yi(&mut self, yi: i16) {
        self.iy = yi;
        self.fy = self.iy as f32;
    }

    pub fn set_player_id(&mut self, playerId: i16) {
        self.iPlayerId = playerId;
    }

    pub fn draw(&mut self) {
        unsafe {
            self.rDstRect.x = self.ix as i32 - self.iHalfWidth as i32 + x_shake as i32;
            self.rDstRect.y = self.iy as i32 - self.iHalfHeight as i32 + y_shake as i32;
            self.rDstRect.w = self.iWidth as i32;
            self.rDstRect.h = self.iHeight as i32;

            self.sprites[(1 - g_iCurrentDrawIndex) as usize].draw_src_to(&self.rSrcRect, blitdest, &self.rDstRect);
        }
    }

    /// `draw(short iOffsetX, short iOffsetY)`: path drawing for the map preview.
    pub fn draw_offset(&mut self, iOffsetX: i16, iOffsetY: i16) {
        gfx_drawpreview(
            self.sprites[0].get_surface(),
            (self.ix as i32 - self.iHalfWidth as i32 + iOffsetX as i32) as i16,
            (self.iy as i32 - self.iHalfHeight as i32 + iOffsetY as i32) as i16,
            0,
            0,
            self.iWidth,
            self.iHeight,
            iOffsetX,
            iOffsetY,
            (App::screenWidth / 2) as i16,
            (App::screenHeight / 2) as i16,
            true,
            Option::None,
        );
    }

    pub fn update(&mut self) {
        self.fOldX = self.pPath.current_pos0().x;
        self.fOldY = self.pPath.current_pos0().y;

        self.fOldVelX = self.pPath.velocity0().x;
        self.fOldVelY = self.pPath.velocity0().y;

        self.pPath.r#move(0);

        self.pPath.r#move(1);

        self.fVelX = self.pPath.velocity0().x;
        self.fVelY = self.pPath.velocity0().y;

        let x = self.pPath.current_pos0().x;
        let y = self.pPath.current_pos0().y;
        self.set_xf(x);
        self.set_yf(y);
    }

    pub fn reset_path(&mut self) {
        self.pPath.reset();

        let x = self.pPath.current_pos0().x;
        let y = self.pPath.current_pos0().y;
        self.set_xf(x);
        self.set_yf(y);

        self.fOldX = self.fx;
        self.fOldY = self.fy;
    }

    /// `collide(CPlayer*)`
    pub fn collide_player(&mut self, mut player: Ptr<CPlayer>) {
        let this = Ptr::from_mut(self);
        let fx = self.fx;
        let fy = self.fy;
        let iHalfWidth = self.iHalfWidth as f32;
        let iHalfHeight = self.iHalfHeight as f32;
        let iWidth = self.iWidth as f32;
        let iHeight = self.iHeight as f32;
        let iPlayerId = self.iPlayerId;

        let mut coldec = self.coldec_player(player);
        if coldec == collision_none {
            if player.platform == this {
                player.platform = Ptr::null();
            }
            return;
        }

        if self.fDead {
            return;
        }

        let mut fColVelX = player.velx - self.fOldVelX;
        let mut fColVelY = player.vely - self.fOldVelY;

        if !player.platform.is_null() {
            fColVelY += player.platform.fOldVelY - GRAVITATION;

            if player.velx < -0.6 || player.velx > 0.6 {
                fColVelX += player.platform.fOldVelX;
            }
        } else if !player.inair {
            fColVelY -= GRAVITATION;
        }

        let fRelativeY1: f32;
        let fRelativeY2: f32;

        if player.platform != this {
            fRelativeY1 = player.fy - self.fOldY + iHalfHeight;
            fRelativeY2 = player.fy + PH as f32 - self.fOldY + iHalfHeight;
        } else {
            fRelativeY1 = player.fy - fy + iHalfHeight;
            fRelativeY2 = player.fy + PH as f32 - fy + iHalfHeight;
        }

        if fColVelX > 0.01 || player.iHorizontalPlatformCollision == 3 {
            let fRelativeX: f32 = if coldec == collision_normal {
                player.fx + PW as f32 - fx + iHalfWidth
            } else if coldec == collision_overlap_left {
                player.fx + PW as f32 - fx + iHalfWidth - SW
            } else {
                player.fx + PW as f32 - fx + iHalfWidth + SW
            };

            if fRelativeX >= 0.0 && fRelativeX < iWidth {
                let tx = tile_of(fRelativeX);

                let mut t1 = tile_flag_nonsolid;
                let mut t2 = tile_flag_nonsolid;

                if fRelativeY1 >= 0.0 && fRelativeY1 < iHeight {
                    t1 = self.flags_at(tx, tile_of(fRelativeY1));
                }

                if fRelativeY2 >= 0.0 && fRelativeY2 < iHeight {
                    t2 = self.flags_at(tx, tile_of(fRelativeY2));
                }

                if (t1 & tile_flag_solid) != 0 || (t2 & tile_flag_solid) != 0 {
                    let fDeathTileToLeft = ((t1 & tile_flag_death_on_left) != 0 && (t2 & tile_flag_death_on_left) != 0)
                        || ((t1 & tile_flag_death_on_left) != 0 && (t2 & tile_flag_solid) == 0)
                        || ((t1 & tile_flag_solid) == 0 && (t2 & tile_flag_death_on_left) != 0);

                    let fSuperDeathTileToLeft = ((t1 & tile_flag_super_or_player_death_left) != 0 && (t2 & tile_flag_super_or_player_death_left) != 0)
                        || ((t1 & tile_flag_super_or_player_death_left) != 0 && (t2 & tile_flag_solid) == 0)
                        || ((t1 & tile_flag_solid) == 0 && (t2 & tile_flag_super_or_player_death_left) != 0);

                    if player.iHorizontalPlatformCollision == 3 {
                        player.kill_player_map_hazard(true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else if fSuperDeathTileToLeft || (fDeathTileToLeft && !player.is_invincible() && !player.is_shielded() && !player.shyguy) {
                        if PlayerKillType::NonKill != player.kill_player_map_hazard(fSuperDeathTileToLeft, KillStyle::Environment, false, iPlayerId) {
                            return;
                        }
                    } else {
                        player.set_xf((((tx as i32) << 5) - PW) as f32 - 0.2 + fx - iHalfWidth);
                        player.flipsidesifneeded();

                        self.check_map_collision_right(player);

                        if self.fOldVelX < 0.0 {
                            player.fOldX = fx + 10.0;
                        } else {
                            player.fOldX = fx - 10.0;
                        }

                        player.iHorizontalPlatformCollision = 1;
                        player.iPlatformCollisionPlayerId = iPlayerId;

                        if player.velx > 0.0 {
                            player.velx = 0.0;
                        }
                    }
                }
            }
        } else if fColVelX < -0.01 || player.iHorizontalPlatformCollision == 1 {
            let fRelativeX: f32 = if coldec == collision_normal {
                player.fx - fx + iHalfWidth
            } else if coldec == collision_overlap_left {
                player.fx - fx + iHalfWidth - SW
            } else {
                player.fx - fx + iHalfWidth + SW
            };

            if fRelativeX >= 0.0 && fRelativeX < iWidth {
                let tx = tile_of(fRelativeX);

                let mut t1 = tile_flag_nonsolid;
                let mut t2 = tile_flag_nonsolid;

                if fRelativeY1 >= 0.0 && fRelativeY1 < iHeight {
                    t1 = self.flags_at(tx, tile_of(fRelativeY1));
                }

                if fRelativeY2 >= 0.0 && fRelativeY2 < iHeight {
                    t2 = self.flags_at(tx, tile_of(fRelativeY2));
                }

                if (t1 & tile_flag_solid) != 0 || (t2 & tile_flag_solid) != 0 {
                    let fDeathTileToRight = ((t1 & tile_flag_death_on_right) != 0 && (t2 & tile_flag_death_on_right) != 0)
                        || ((t1 & tile_flag_death_on_right) != 0 && (t2 & tile_flag_solid) == 0)
                        || ((t1 & tile_flag_solid) == 0 && (t2 & tile_flag_death_on_right) != 0);

                    let fSuperDeathTileToRight = ((t1 & tile_flag_super_or_player_death_right) != 0
                        && (t2 & tile_flag_super_or_player_death_right) != 0)
                        || ((t1 & tile_flag_super_or_player_death_right) != 0 && (t2 & tile_flag_solid) == 0)
                        || ((t1 & tile_flag_solid) == 0 && (t2 & tile_flag_super_or_player_death_right) != 0);

                    if player.iHorizontalPlatformCollision == 1 {
                        player.kill_player_map_hazard(true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else if fSuperDeathTileToRight || (fDeathTileToRight && !player.is_invincible() && !player.is_shielded() && !player.shyguy) {
                        if PlayerKillType::NonKill != player.kill_player_map_hazard(fSuperDeathTileToRight, KillStyle::Environment, false, iPlayerId) {
                            return;
                        }
                    } else {
                        player.set_xf((((tx as i32) << 5) + TILESIZE) as f32 + 0.2 + fx - iHalfWidth);
                        player.flipsidesifneeded();

                        self.check_map_collision_left(player);

                        if self.fOldVelX < 0.0 {
                            player.fOldX = fx + 10.0;
                        } else {
                            player.fOldX = fx - 10.0;
                        }

                        player.iHorizontalPlatformCollision = 3;
                        player.iPlatformCollisionPlayerId = iPlayerId;

                        if player.velx < 0.0 {
                            player.velx = 0.0;
                        }
                    }
                }
            }
        }

        let fRelativeX1: f32;
        let fRelativeX2: f32;

        coldec = self.coldec_player(player);
        if coldec == collision_none {
            if player.platform == this {
                player.platform = Ptr::null();
            }

            return;
        } else if coldec == collision_normal {
            fRelativeX1 = player.fx - fx + iHalfWidth;
            fRelativeX2 = player.fx + PW as f32 - fx + iHalfWidth;
        } else if coldec == collision_overlap_left {
            fRelativeX1 = player.fx - fx + iHalfWidth - SW;
            fRelativeX2 = player.fx + PW as f32 - fx + iHalfWidth - SW;
        } else {
            fRelativeX1 = player.fx - fx + iHalfWidth + SW;
            fRelativeX2 = player.fx + PW as f32 - fx + iHalfWidth + SW;
        }

        if fColVelY < 0.0 {
            let fRelativeY: f32 = if player.inair || player.platform == this {
                player.fPrecalculatedY - fy + iHalfHeight
            } else {
                player.fy - fy + iHalfHeight
            };

            if fRelativeY >= 0.0 && fRelativeY < iHeight {
                let ty = tile_of(fRelativeY);

                let mut t1 = tile_flag_nonsolid;
                let mut t2 = tile_flag_nonsolid;

                if fRelativeX1 >= 0.0 && fRelativeX1 < iWidth {
                    t1 = self.flags_at(tile_of(fRelativeX1), ty);
                }

                if fRelativeX2 >= 0.0 && fRelativeX2 < iWidth {
                    t2 = self.flags_at(tile_of(fRelativeX2), ty);
                }

                let fSolidTileOverPlayer = (t1 & tile_flag_solid) != 0 || (t2 & tile_flag_solid) != 0;

                let fDeathTileOverPlayer = ((t1 & tile_flag_death_on_bottom) != 0 && (t2 & tile_flag_death_on_bottom) != 0)
                    || ((t1 & tile_flag_death_on_bottom) != 0 && (t2 & tile_flag_solid) == 0)
                    || ((t1 & tile_flag_solid) == 0 && (t2 & tile_flag_death_on_bottom) != 0);

                let fSuperDeathTileOverPlayer = ((t1 & tile_flag_super_or_player_death_bottom) != 0
                    && (t2 & tile_flag_super_or_player_death_bottom) != 0)
                    || ((t1 & tile_flag_super_or_player_death_bottom) != 0 && (t2 & tile_flag_solid) == 0)
                    || ((t1 & tile_flag_solid) == 0 && (t2 & tile_flag_super_or_player_death_bottom) != 0);

                if fSolidTileOverPlayer
                    && !fSuperDeathTileOverPlayer
                    && (!fDeathTileOverPlayer || player.is_invincible() || player.is_shielded() || player.shyguy)
                {
                    if player.iVerticalPlatformCollision == 2 {
                        player.kill_player_map_hazard(true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else {
                        player.fPrecalculatedY = (((ty as i32) << 5) + TILESIZE) as f32 + 0.2 + fy - iHalfHeight;
                        player.fOldY = player.fPrecalculatedY - self.fVelY - GRAVITATION;

                        if player.vely < 0.0 && self.fVelY > 0.0 {
                            player.fPrecalculatedY += self.fVelY;
                        }

                        if player.vely < 0.0 {
                            player.vely = cap_falling_velocity(-player.vely * BOUNCESTRENGTH + self.fVelY);
                        }

                        player.iVerticalPlatformCollision = 0;
                        player.iPlatformCollisionPlayerId = iPlayerId;
                    }

                    return;
                } else if (t1 & tile_flag_player_or_death_on_bottom) != 0 || (t2 & tile_flag_player_or_death_on_bottom) != 0 {
                    if PlayerKillType::NonKill != player.kill_player_map_hazard(true, KillStyle::Environment, false, iPlayerId) {
                        return;
                    }
                }
            }
        } else {
            let fRelativeY = player.fPrecalculatedY + PH as f32 - fy + iHalfHeight;

            if fRelativeY >= 0.0 && fRelativeY < iHeight {
                let ty = tile_of(fRelativeY);

                let mut t1 = tile_flag_nonsolid;
                let mut t2 = tile_flag_nonsolid;

                if fRelativeX1 >= 0.0 && fRelativeX1 < iWidth {
                    t1 = self.flags_at(tile_of(fRelativeX1), ty);
                }

                if fRelativeX2 >= 0.0 && fRelativeX2 < iWidth {
                    t2 = self.flags_at(tile_of(fRelativeX2), ty);
                }

                let fSolidTileUnderPlayer = (t1 & tile_flag_solid) != 0 || (t2 & tile_flag_solid) != 0;

                if ((t1 & tile_flag_solid_on_top) != 0 || (t2 & tile_flag_solid_on_top) != 0)
                    && player.fOldY + PH as f32 <= ((ty as i32) << 5) as f32 + self.fOldY - iHalfHeight
                {
                    if player.fallthrough && !fSolidTileUnderPlayer {
                        player.fPrecalculatedY =
                            (((ty as i32) << 5) - PH) as f32 + (if self.fVelY > 0.0 { self.fVelY } else { 0.0 }) + 0.2 + fy - iHalfHeight;
                        player.inair = true;
                        player.platform = Ptr::null();
                    } else if player.iVerticalPlatformCollision == 0 {
                        player.kill_player_map_hazard(true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else {
                        player.platform = this;
                        player.iVerticalPlatformCollision = 2;
                        player.iPlatformCollisionPlayerId = iPlayerId;

                        player.fPrecalculatedY = (((ty as i32) << 5) - PH) as f32 - 0.2 + fy - iHalfHeight;
                        player.vely = GRAVITATION;
                        player.inair = false;
                        player.killsinrowinair = 0;
                        player.extrajumps = 0;
                    }

                    player.fOldY = player.fPrecalculatedY - self.fVelY;
                    player.fallthrough = false;
                    player.onice = false;

                    return;
                }

                let fDeathTileUnderPlayer = ((t1 & tile_flag_death_on_top) != 0 && (t2 & tile_flag_death_on_top) != 0)
                    || ((t1 & tile_flag_death_on_top) != 0 && (t2 & tile_flag_solid) == 0)
                    || ((t1 & tile_flag_solid) == 0 && (t2 & tile_flag_death_on_top) != 0);

                let fSuperDeathTileUnderPlayer = ((t1 & tile_flag_super_or_player_death_top) != 0
                    && (t2 & tile_flag_super_or_player_death_top) != 0)
                    || ((t1 & tile_flag_super_or_player_death_top) != 0 && (t2 & tile_flag_solid) == 0)
                    || ((t1 & tile_flag_solid) == 0 && (t2 & tile_flag_super_or_player_death_top) != 0);

                if fSolidTileUnderPlayer
                    && !fSuperDeathTileUnderPlayer
                    && (!fDeathTileUnderPlayer || player.is_invincible() || player.is_shielded() || player.kuriboshoe.is_on() || player.shyguy)
                {
                    if player.iVerticalPlatformCollision == 0 {
                        player.kill_player_map_hazard(true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else {
                        player.platform = this;
                        player.iVerticalPlatformCollision = 2;
                        player.iPlatformCollisionPlayerId = iPlayerId;

                        player.fPrecalculatedY = (((ty as i32) << 5) - PH) as f32 - 0.2 + fy - iHalfHeight;
                        player.fOldY = player.fPrecalculatedY - self.fVelY;

                        player.vely = GRAVITATION;
                        player.inair = false;
                        player.killsinrowinair = 0;
                        player.extrajumps = 0;

                        if ((t1 & tile_flag_ice) != 0 && ((t2 & tile_flag_ice) != 0 || t2 == tile_flag_nonsolid || t2 == tile_flag_gap))
                            || ((t2 & tile_flag_ice) != 0 && ((t1 & tile_flag_ice) != 0 || t1 == tile_flag_nonsolid || t1 == tile_flag_gap))
                        {
                            player.onice = true;
                        } else {
                            player.onice = false;
                        }

                        player.fallthrough = false;

                        return;
                    }
                } else if fDeathTileUnderPlayer || fSuperDeathTileUnderPlayer {
                    if PlayerKillType::NonKill != player.kill_player_map_hazard(fSuperDeathTileUnderPlayer, KillStyle::Environment, false, iPlayerId) {
                        return;
                    }
                } else if player.platform == this {
                    player.platform = Ptr::null();
                }
            }
        }
    }

    fn check_map_collision_right(&mut self, mut player: Ptr<CPlayer>) {
        unsafe {
            if player.fy + (PH as f32) < 0.0 {
                return;
            }

            let iTestBackgroundX: i16 = if player.fx + PW as f32 >= SW {
                (((player.fx as i16) as i32 + PW - App::screenWidth) / TILESIZE) as i16
            } else {
                (((player.fx as i16) as i32 + PW) / TILESIZE) as i16
            };

            let iTestBackgroundY: i16 = (((player.fy as i16) as i32 + PH) / TILESIZE) as i16;

            let mut topblock = g_map.block(iTestBackgroundX, iTestBackgroundY);

            if (!topblock.is_null() && !topblock.is_transparent() && !topblock.is_hidden())
                || (g_map.map(iTestBackgroundX as i32, iTestBackgroundY as i32) & tile_flag_solid) != 0
            {
                player.set_xf((((iTestBackgroundX as i32) << 5) - PW) as f32 - 0.2);
                player.flipsidesifneeded();
                return;
            }

            if player.fy < 0.0 {
                return;
            }

            let iTestBackgroundY2: i16 = tile_of(player.fy);

            let mut bottomblock = g_map.block(iTestBackgroundX, iTestBackgroundY2);

            if (!bottomblock.is_null() && !bottomblock.is_transparent() && !bottomblock.is_hidden())
                || (g_map.map(iTestBackgroundX as i32, iTestBackgroundY2 as i32) & tile_flag_solid) != 0
            {
                player.set_xf((((iTestBackgroundX as i32) << 5) - PW) as f32 - 0.2);
                player.flipsidesifneeded();
            }
        }
    }

    fn check_map_collision_left(&mut self, mut player: Ptr<CPlayer>) {
        unsafe {
            if player.fy + (PH as f32) < 0.0 {
                return;
            }

            let iTestBackgroundX: i16 = tile_of(player.fx);
            let iTestBackgroundY: i16 = (((player.fy as i16) as i32 + PH) / TILESIZE) as i16;

            let mut topblock = g_map.block(iTestBackgroundX, iTestBackgroundY);

            if (!topblock.is_null() && !topblock.is_transparent() && !topblock.is_hidden())
                || (g_map.map(iTestBackgroundX as i32, iTestBackgroundY as i32) & tile_flag_solid) != 0
            {
                player.set_xf((((iTestBackgroundX as i32) << 5) + TILESIZE) as f32 + 0.2);
                player.flipsidesifneeded();
                return;
            }

            if player.fy < 0.0 {
                return;
            }

            let iTestBackgroundY2: i16 = tile_of(player.fy);

            let mut bottomblock = g_map.block(iTestBackgroundX, iTestBackgroundY2);

            if (!bottomblock.is_null() && !bottomblock.is_transparent() && !bottomblock.is_hidden())
                || (g_map.map(iTestBackgroundX as i32, iTestBackgroundY2 as i32) & tile_flag_solid) != 0
            {
                player.set_xf((((iTestBackgroundX as i32) << 5) + TILESIZE) as f32 + 0.2);
                player.flipsidesifneeded();
            }
        }
    }

    pub fn collision_detection_check_sides(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        let coldec = self.coldec_object(object);
        if coldec == collision_none {
            return false;
        }

        if self.fDead {
            return false;
        }

        let fx = self.fx;
        let fy = self.fy;
        let iHalfWidth = self.iHalfWidth as f32;
        let iHalfHeight = self.iHalfHeight as f32;
        let iWidth = self.iWidth as f32;
        let iHeight = self.iHeight as f32;

        let mut iCase: u8 = 0;

        let mut fRelativeXLeft = object.fx - fx + iHalfWidth;
        if coldec == collision_overlap_left {
            fRelativeXLeft -= SW;
        } else if coldec == collision_overlap_right {
            fRelativeXLeft += SW;
        }

        let fRelativeXRight = fRelativeXLeft + object.collisionWidth as f32;

        let fRelativeYTop = object.fy - fy + iHalfHeight;
        let fRelativeYBottom = fRelativeYTop + object.collisionHeight as f32;

        let mut txLeft: i16 = -1;
        let mut txRight: i16 = -1;

        if fRelativeXLeft >= 0.0 && fRelativeXLeft < iWidth {
            txLeft = tile_of(fRelativeXLeft);
        }

        if fRelativeXRight >= 0.0 && fRelativeXRight < iWidth {
            txRight = tile_of(fRelativeXRight);
        }

        let mut tyTop: i16 = -1;
        let mut tyBottom: i16 = -1;

        if fRelativeYTop >= 0.0 && fRelativeYTop < iHeight {
            tyTop = tile_of(fRelativeYTop);
        }

        if fRelativeYBottom >= 0.0 && fRelativeYBottom < iHeight {
            tyBottom = tile_of(fRelativeYBottom);
        }

        if txLeft >= 0 {
            let mut t1 = tile_flag_nonsolid;
            let mut t2 = tile_flag_nonsolid;

            if tyTop >= 0 {
                t1 = self.flags_at(txLeft, tyTop);
            }

            if tyBottom >= 0 {
                t2 = self.flags_at(txLeft, tyBottom);
            }

            if (t1 & tile_flag_solid) != 0 {
                iCase |= 0x01;
            }

            if (t2 & tile_flag_solid) != 0 {
                iCase |= 0x04;
            }
        }

        if txRight >= 0 {
            let mut t1 = tile_flag_nonsolid;
            let mut t2 = tile_flag_nonsolid;

            if tyTop >= 0 {
                t1 = self.flags_at(txRight, tyTop);
            }

            if tyBottom >= 0 {
                t2 = self.flags_at(txRight, tyBottom);
            }

            if (t1 & tile_flag_solid) != 0 {
                iCase |= 0x02;
            }

            if (t2 & tile_flag_solid) != 0 {
                iCase |= 0x08;
            }
        }

        let txl = txLeft as i32;
        let txr = txRight as i32;
        let tyt = tyTop as i32;
        let tyb = tyBottom as i32;
        let cw = object.collisionWidth as i32;
        let ch = object.collisionHeight as i32;

        let x_right_of_left = || ((txl << 5) + TILESIZE) as f32 + 0.2 + fx - iHalfWidth;
        let x_left_of_right = || ((txr << 5) - cw) as f32 - 0.2 + fx - iHalfWidth;
        let y_below_top = || ((tyt << 5) + TILESIZE) as f32 + 0.2 + fy - iHalfHeight;
        let y_above_bottom = || ((tyb << 5) - ch) as f32 - 0.2 + fy - iHalfHeight;
        let center = (object.ix as i32) + (cw >> 1);

        match iCase {
            0 => {
                return false;
            }
            1 => {
                if center > (txl << 5) + TILESIZE {
                    object.set_xf(x_right_of_left());
                    object.flipsidesifneeded();
                } else {
                    object.set_yf(y_below_top());
                }
            }
            2 => {
                if center < (txr << 5) {
                    object.set_xf(x_left_of_right());
                    object.flipsidesifneeded();
                } else {
                    object.set_yf(y_below_top());
                }
            }
            3 => {
                object.set_yf(y_below_top());
            }
            4 => {
                if center > (txl << 5) + TILESIZE {
                    object.set_xf(x_right_of_left());
                    object.flipsidesifneeded();
                } else {
                    object.set_yf(y_above_bottom());
                }
            }
            5 => {
                object.set_xf(x_right_of_left());
                object.flipsidesifneeded();
            }
            6 => {
                if center > (txl << 5) + TILESIZE {
                    object.set_yf(y_below_top());
                    object.set_xf(x_right_of_left());
                    object.flipsidesifneeded();
                } else {
                    object.set_yf(y_above_bottom());
                    object.set_xf(x_left_of_right());
                    object.flipsidesifneeded();
                }
            }
            7 => {
                object.set_yf(y_below_top());
                object.set_xf(x_right_of_left());
                object.flipsidesifneeded();
            }
            8 => {
                if center < (txr << 5) {
                    object.set_xf(x_left_of_right());
                    object.flipsidesifneeded();
                } else {
                    object.set_yf(y_above_bottom());
                }
            }
            9 => {
                if center > (txl << 5) + TILESIZE {
                    object.set_yf(y_above_bottom());
                    object.set_xf(x_right_of_left());
                    object.flipsidesifneeded();
                } else {
                    object.set_yf(y_below_top());
                    object.set_xf(x_left_of_right());
                    object.flipsidesifneeded();
                }
            }
            10 => {
                object.set_xf(x_left_of_right());
                object.flipsidesifneeded();
            }
            11 => {
                object.set_yf(y_below_top());
                object.set_xf(x_left_of_right());
                object.flipsidesifneeded();
            }
            12 => {
                object.set_yf(y_above_bottom());
            }
            13 => {
                object.set_yf(y_above_bottom());
                object.set_xf(x_right_of_left());
                object.flipsidesifneeded();
            }
            14 => {
                object.set_yf(y_above_bottom());
                object.set_xf(x_left_of_right());
                object.flipsidesifneeded();
            }
            15 => {
                object.set_yf(((tyb << 5) + TILESIZE) as f32 + 0.2 + fy - iHalfHeight);
            }
            _ => {}
        }

        true
    }

    fn coldec_player(&self, player: Ptr<CPlayer>) -> CollisionStyle {
        let fx = self.fx;
        let fy = self.fy;
        let hw = self.iHalfWidth as f32;
        let hh = self.iHalfHeight as f32;
        let pw = PW as f32;
        let ph = PH as f32;

        if player.fx + pw < fx - hw {
            if player.fx + SW >= fx + hw || player.fx + pw + SW < fx - hw || player.fPrecalculatedY >= fy + hh || player.fPrecalculatedY + ph < fy - hh {
                collision_none
            } else {
                collision_overlap_right
            }
        } else if fx + hw < player.fx {
            if player.fx >= fx + hw + SW || player.fx + pw < fx - hw + SW || player.fPrecalculatedY >= fy + hh || player.fPrecalculatedY + ph < fy - hh {
                collision_none
            } else {
                collision_overlap_left
            }
        } else if player.fx >= fx + hw || fx - hw > player.fx + pw || player.fPrecalculatedY >= fy + hh || fy - hh > player.fPrecalculatedY + ph {
            collision_none
        } else {
            collision_normal
        }
    }

    pub fn get_tile_types_from_player(&self, player: Ptr<CPlayer>, lefttile: &mut i32, righttile: &mut i32) {
        *lefttile = tile_flag_nonsolid;
        *righttile = tile_flag_nonsolid;

        let fRelativeY = player.fPrecalculatedY + PH as f32 - self.fy + self.iHalfHeight as f32;

        if fRelativeY < 0.0 || fRelativeY >= self.iHeight as f32 {
            return;
        }

        let ty = tile_of(fRelativeY);

        let fRelativeX1 = player.fx - self.fx + self.iHalfWidth as f32;

        let fRelativeX2 = if player.fx + PW as f32 > SW {
            player.fx + PW as f32 - SW - self.fx + self.iHalfWidth as f32
        } else {
            player.fx + PW as f32 - self.fx + self.iHalfWidth as f32
        };

        if fRelativeX1 >= 0.0 && fRelativeX1 < self.iWidth as f32 {
            *lefttile = self.flags_at(tile_of(fRelativeX1), ty);
        }

        if fRelativeX2 >= 0.0 && fRelativeX2 < self.iWidth as f32 {
            *righttile = self.flags_at(tile_of(fRelativeX2), ty);
        }
    }

    pub fn get_tile_type_from_coord(&self, mut x: i16, y: i16) -> i32 {
        let fRelativeY = y as f32 - self.fy + self.iHalfHeight as f32;

        if fRelativeY < 0.0 || fRelativeY >= self.iHeight as f32 {
            return tile_flag_nonsolid;
        }

        if x as i32 >= App::screenWidth {
            x = (x as i32 - App::screenWidth) as i16;
        } else if x < 0 {
            x = (x as i32 + App::screenWidth) as i16;
        }

        let fRelativeX = x as f32 - self.fx + self.iHalfWidth as f32;

        if fRelativeX < 0.0 || fRelativeX >= self.iWidth as f32 {
            return tile_flag_nonsolid;
        }

        self.flags_at(tile_of(fRelativeX), tile_of(fRelativeY))
    }

    /// `collide(IO_MovingObject*)`
    pub fn collide_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        let this = Ptr::from_mut(self);
        let fx = self.fx;
        let fy = self.fy;
        let iHalfWidth = self.iHalfWidth as f32;
        let iHalfHeight = self.iHalfHeight as f32;
        let iWidth = self.iWidth as f32;
        let iHeight = self.iHeight as f32;

        let mut fColVelX = object.velx - self.fOldVelX;

        let mut coldec = self.coldec_object(object);
        if coldec == collision_none {
            if object.platform == this {
                object.platform = Ptr::null();
            }

            return;
        }

        if self.fDead {
            return;
        }

        let mut fColVelY = object.vely - self.fOldVelY;

        if !object.platform.is_null() {
            fColVelY += object.platform.fOldVelY - GRAVITATION;

            if object.velx < -0.6 || object.velx > 0.6 {
                fColVelX += object.platform.fOldVelX;
            }
        } else if !object.inair {
            fColVelY -= GRAVITATION;
        }

        let fRelativeY1: f32;
        let fRelativeY2: f32;

        if object.platform != this {
            fRelativeY1 = object.fy - self.fOldY + iHalfHeight;
            fRelativeY2 = fRelativeY1 + object.collisionHeight as f32;
        } else {
            fRelativeY1 = object.fy - fy + iHalfHeight;
            fRelativeY2 = fRelativeY1 + object.collisionHeight as f32;
        }

        if fColVelX > 0.01 || object.iHorizontalPlatformCollision == 3 {
            let cw = object.collisionWidth as f32;
            let fRelativeX: f32 = if coldec == collision_overlap_left {
                object.fx + cw - fx + iHalfWidth - SW
            } else if coldec == collision_overlap_right {
                object.fx + cw - fx + iHalfWidth + SW
            } else {
                object.fx + cw - fx + iHalfWidth
            };

            if fRelativeX >= 0.0 && fRelativeX < iWidth {
                let tx = tile_of(fRelativeX);

                let mut t1 = tile_flag_nonsolid;
                let mut t2 = tile_flag_nonsolid;

                if fRelativeY1 >= 0.0 && fRelativeY1 < iHeight {
                    t1 = self.flags_at(tx, tile_of(fRelativeY1));
                }

                if fRelativeY2 >= 0.0 && fRelativeY2 < iHeight {
                    t2 = self.flags_at(tx, tile_of(fRelativeY2));
                }

                if (t1 & tile_flag_solid) != 0 || (t2 & tile_flag_solid) != 0 {
                    if object.iHorizontalPlatformCollision == 3 {
                        object.kill_object_map_hazard(-1);
                        return;
                    } else {
                        let cwi = object.collisionWidth as i32;
                        object.set_xf((((tx as i32) << 5) - cwi) as f32 - 0.2 + fx - iHalfWidth);
                        object.flipsidesifneeded();

                        let iTestBackgroundY: i16 = tile_of(object.fy);
                        let iTestBackgroundY2: i16 = (((object.fy as i16) as i32 + object.collisionHeight as i32) / TILESIZE) as i16;

                        let iTestBackgroundX: i16 = if object.fx + object.collisionWidth as f32 >= SW {
                            (((object.fx as i16) as i32 + object.collisionWidth as i32 - App::screenWidth) / TILESIZE) as i16
                        } else {
                            (((object.fx as i16) as i32 + object.collisionWidth as i32) / TILESIZE) as i16
                        };

                        unsafe {
                            let mut topblock = g_map.block(iTestBackgroundX, iTestBackgroundY);
                            let mut bottomblock = g_map.block(iTestBackgroundX, iTestBackgroundY2);

                            if (!topblock.is_null() && !topblock.is_transparent() && !topblock.is_hidden())
                                || (!bottomblock.is_null() && !bottomblock.is_transparent() && !bottomblock.is_hidden())
                                || (g_map.map(iTestBackgroundX as i32, iTestBackgroundY as i32) & tile_flag_solid) != 0
                                || (g_map.map(iTestBackgroundX as i32, iTestBackgroundY2 as i32) & tile_flag_solid) != 0
                            {
                                let cwi = object.collisionWidth as i32;
                                object.set_xf((((iTestBackgroundX as i32) << 5) - cwi) as f32 - 0.2);
                                object.flipsidesifneeded();
                            }
                        }

                        if self.fOldVelX < 0.0 {
                            object.fOldX = fx + 10.0;
                        } else {
                            object.fOldX = fx - 10.0;
                        }

                        object.iHorizontalPlatformCollision = 1;

                        if object.velx > 0.0 {
                            object.velx = -object.velx;
                        }

                        removeifprojectile(object, true, true);

                        object.side_bounce(true);
                    }
                }
            }
        } else if fColVelX < -0.01 || object.iHorizontalPlatformCollision == 1 {
            let fRelativeX: f32 = if coldec == collision_overlap_left {
                object.fx - fx + iHalfWidth - SW
            } else if coldec == collision_overlap_right {
                object.fx - fx + iHalfWidth + SW
            } else {
                object.fx - fx + iHalfWidth
            };

            if fRelativeX >= 0.0 && fRelativeX < iWidth {
                let tx = tile_of(fRelativeX);

                let mut t1 = tile_flag_nonsolid;
                let mut t2 = tile_flag_nonsolid;

                if fRelativeY1 >= 0.0 && fRelativeY1 < iHeight {
                    t1 = self.flags_at(tx, tile_of(fRelativeY1));
                }

                if fRelativeY2 >= 0.0 && fRelativeY2 < iHeight {
                    t2 = self.flags_at(tx, tile_of(fRelativeY2));
                }

                if (t1 & tile_flag_solid) != 0 || (t2 & tile_flag_solid) != 0 {
                    if object.iHorizontalPlatformCollision == 1 {
                        object.kill_object_map_hazard(-1);
                        return;
                    } else {
                        object.set_xf((((tx as i32) << 5) + TILESIZE) as f32 + 0.2 + fx - iHalfWidth);
                        object.flipsidesifneeded();

                        let iTestBackgroundY: i16 = tile_of(object.fy);
                        let iTestBackgroundY2: i16 = (((object.fy as i16) as i32 + object.collisionHeight as i32) / TILESIZE) as i16;
                        let iTestBackgroundX: i16 = tile_of(object.fx);

                        unsafe {
                            let mut topblock = g_map.block(iTestBackgroundX, iTestBackgroundY);
                            let mut bottomblock = g_map.block(iTestBackgroundX, iTestBackgroundY2);

                            if (!topblock.is_null() && !topblock.is_transparent() && !topblock.is_hidden())
                                || (!bottomblock.is_null() && !bottomblock.is_transparent() && !bottomblock.is_hidden())
                                || (g_map.map(iTestBackgroundX as i32, iTestBackgroundY as i32) & tile_flag_solid) != 0
                                || (g_map.map(iTestBackgroundX as i32, iTestBackgroundY2 as i32) & tile_flag_solid) != 0
                            {
                                object.set_xf((((iTestBackgroundX as i32) << 5) + TILESIZE) as f32 + 0.2);
                                object.flipsidesifneeded();
                            }
                        }

                        if self.fOldVelX < 0.0 {
                            object.fOldX = fx + 10.0;
                        } else {
                            object.fOldX = fx - 10.0;
                        }

                        object.iHorizontalPlatformCollision = 3;

                        if object.velx < 0.0 {
                            object.velx = -object.velx;
                        }

                        removeifprojectile(object, true, true);

                        object.side_bounce(false);
                    }
                }
            }
        }

        let fRelativeX1: f32;
        let fRelativeX2: f32;

        coldec = self.coldec_object(object);
        let cw = object.collisionWidth as f32;
        if coldec == collision_none {
            if object.platform == this {
                object.platform = Ptr::null();
            }

            return;
        } else if coldec == collision_normal {
            fRelativeX1 = object.fx - fx + iHalfWidth;
            fRelativeX2 = object.fx + cw - fx + iHalfWidth;
        } else if coldec == collision_overlap_left {
            fRelativeX1 = object.fx - fx + iHalfWidth - SW;
            fRelativeX2 = object.fx + cw - fx + iHalfWidth - SW;
        } else {
            fRelativeX1 = object.fx - fx + iHalfWidth + SW;
            fRelativeX2 = object.fx + cw - fx + iHalfWidth + SW;
        }

        if fColVelY < 0.0 {
            let fRelativeY: f32 = if object.inair || object.platform == this {
                object.fPrecalculatedY - fy + iHalfHeight
            } else {
                object.fy - fy + iHalfHeight
            };

            if fRelativeY >= 0.0 && fRelativeY < iHeight {
                let ty = tile_of(fRelativeY);

                let mut t1 = tile_flag_nonsolid;
                let mut t2 = tile_flag_nonsolid;

                if fRelativeX1 >= 0.0 && fRelativeX1 < iWidth {
                    t1 = self.flags_at(tile_of(fRelativeX1), ty);
                }

                if fRelativeX2 >= 0.0 && fRelativeX2 < iWidth {
                    t2 = self.flags_at(tile_of(fRelativeX2), ty);
                }

                if (t1 & tile_flag_solid) != 0 || (t2 & tile_flag_solid) != 0 {
                    if object.iVerticalPlatformCollision == 2 {
                        object.kill_object_map_hazard(-1);
                        return;
                    } else {
                        object.fPrecalculatedY = (((ty as i32) << 5) + TILESIZE) as f32 + 0.2 + fy - iHalfHeight;
                        object.fOldY = object.fPrecalculatedY - self.fVelY - GRAVITATION;

                        if object.vely < 0.0 && self.fVelY > 0.0 {
                            object.fPrecalculatedY += self.fVelY;
                        }

                        if object.vely < 0.0 {
                            object.vely = cap_falling_velocity(-object.vely + self.fVelY);
                        }

                        object.iVerticalPlatformCollision = 0;
                    }

                    return;
                }
            }
        } else {
            let fRelativeY = object.fPrecalculatedY + object.collisionHeight as f32 - fy + iHalfHeight;

            if fRelativeY >= 0.0 && fRelativeY < iHeight {
                let ty = tile_of(fRelativeY);

                let mut t1 = tile_flag_nonsolid;
                let mut t2 = tile_flag_nonsolid;

                if fRelativeX1 >= 0.0 && fRelativeX1 < iWidth {
                    t1 = self.flags_at(tile_of(fRelativeX1), ty);
                }

                if fRelativeX2 >= 0.0 && fRelativeX2 < iWidth {
                    t2 = self.flags_at(tile_of(fRelativeX2), ty);
                }

                if ((t1 & tile_flag_solid_on_top) != 0 || (t2 & tile_flag_solid_on_top) != 0)
                    && object.fOldY + object.collisionHeight as f32 <= ((ty as i32) << 5) as f32 + self.fOldY - iHalfHeight
                {
                    if object.iVerticalPlatformCollision == 0 {
                        object.kill_object_map_hazard(-1);
                        return;
                    } else {
                        if object.bounce >= 0.0 {
                            object.platform = this;
                        }

                        object.iVerticalPlatformCollision = 2;

                        let chi = object.collisionHeight as i32;
                        object.fPrecalculatedY = (((ty as i32) << 5) - chi) as f32 - 0.2 + fy - iHalfHeight;
                        object.fOldY = object.fPrecalculatedY - self.fVelY;
                        object.vely = object.bottom_bounce();
                        object.inair = false;
                        object.onice = false;
                    }

                    return;
                }

                let fSuperDeathTileUnderObject = object.fObjectDiesOnSuperDeathTiles
                    && (((t1 & tile_flag_super_death_top) != 0 && (t2 & tile_flag_super_death_top) != 0)
                        || ((t1 & tile_flag_super_death_top) != 0 && (t2 & tile_flag_solid) == 0)
                        || ((t1 & tile_flag_solid) == 0 && (t2 & tile_flag_super_death_top) != 0));

                if ((t1 & tile_flag_solid) != 0 || (t2 & tile_flag_solid) != 0) && !fSuperDeathTileUnderObject {
                    if object.iVerticalPlatformCollision == 0 {
                        object.kill_object_map_hazard(-1);
                        return;
                    } else {
                        if object.bounce >= 0.0 {
                            object.platform = this;
                        }

                        object.iVerticalPlatformCollision = 2;

                        let chi = object.collisionHeight as i32;
                        object.fPrecalculatedY = (((ty as i32) << 5) - chi) as f32 - 0.2 + fy - iHalfHeight;
                        object.fOldY = object.fPrecalculatedY - self.fVelY;
                        object.vely = object.bottom_bounce();
                        object.inair = false;

                        if ((t1 & tile_flag_ice) != 0 && ((t2 & tile_flag_ice) != 0 || t2 == tile_flag_nonsolid || t2 == tile_flag_gap))
                            || ((t2 & tile_flag_ice) != 0 && ((t1 & tile_flag_ice) != 0 || t1 == tile_flag_nonsolid || t1 == tile_flag_gap))
                        {
                            object.onice = true;
                        } else {
                            object.onice = false;
                        }

                        return;
                    }
                } else if fSuperDeathTileUnderObject {
                    object.kill_object_map_hazard(-1);
                    return;
                } else if object.platform == this {
                    object.platform = Ptr::null();
                }
            }
        }
    }

    fn coldec_object(&self, object: Ptr<dyn IO_MovingObjectTrait>) -> CollisionStyle {
        let fx = self.fx;
        let fy = self.fy;
        let hw = self.iHalfWidth as f32;
        let hh = self.iHalfHeight as f32;
        let cw = object.collisionWidth as f32;
        let ch = object.collisionHeight as f32;

        if object.fx + cw < fx - hw {
            if object.fx + SW >= fx + hw || object.fx + cw + SW < fx - hw || object.fPrecalculatedY >= fy + hh || object.fPrecalculatedY + ch < fy - hh {
                collision_none
            } else {
                collision_overlap_right
            }
        } else if fx + hw < object.fx {
            if object.fx >= fx + hw + SW || object.fx + cw < fx - hw + SW || object.fPrecalculatedY >= fy + hh || object.fPrecalculatedY + ch < fy - hh {
                collision_none
            } else {
                collision_overlap_left
            }
        } else if object.fx >= fx + hw || fx - hw > object.fx + cw || object.fPrecalculatedY >= fy + hh || fy - hh > object.fPrecalculatedY + ch {
            collision_none
        } else {
            collision_normal
        }
    }

    pub fn is_in_no_spawn_zone(&self, mut iX: i16, iY: i16, w: i16, h: i16) -> bool {
        let p1 = *self.pPath.current_pos1();
        let iTop: i16 = ((p1.y as i16) as i32 - self.iHalfHeight as i32) as i16;

        if (iY as i32 + h as i32) < iTop as i32 {
            return false;
        }

        let iBottom: i16 = ((p1.y as i16) as i32 + self.iHalfHeight as i32) as i16;

        if iY >= iBottom {
            return false;
        }

        let iLeft: i16 = ((p1.x as i16) as i32 - self.iHalfWidth as i32) as i16;
        let iRight: i16 = ((p1.x as i16) as i32 + self.iHalfWidth as i32) as i16;

        if (iX as i32 + w as i32) < iLeft as i32 {
            iX = (iX as i32 + App::screenWidth) as i16;
        } else if iX >= iRight {
            iX = (iX as i32 - App::screenWidth) as i16;
        }

        if (iX as i32 + w as i32) < iLeft as i32 || iX >= iRight {
            return false;
        }

        let iRelativeX: [i16; 2] = [(iX as i32 - iLeft as i32) as i16, (iX as i32 + w as i32 - iLeft as i32) as i16];
        let iRelativeY: [i16; 2] = [(iY as i32 - iTop as i32) as i16, (iY as i32 + h as i32 - iTop as i32) as i16];

        for sX in 0..2 {
            for sY in 0..2 {
                if iRelativeX[sX] >= 0 && iRelativeX[sX] < self.iWidth && iRelativeY[sY] >= 0 && iRelativeY[sY] < self.iHeight {
                    let tx = (iRelativeX[sX] as i32 / TILESIZE) as i16;
                    let ty = (iRelativeY[sY] as i32 / TILESIZE) as i16;

                    let t = self.flags_at(tx, ty);

                    if (t & tile_flag_solid) != 0 {
                        return true;
                    }
                }
            }
        }

        false
    }
}

