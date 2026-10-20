//! Moving platforms: tile grids that follow a path and collide with players and objects like map tiles.

use crate::smw::players::PlayerId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::map::PlatformId;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::game::App;
use crate::common::gfx::gfx_drawpreview;
use crate::common::global::*;
use crate::common::global_constants::*;
use crate::common::map::TilesetTile;
use crate::common::moving_platform_paths::MovingPlatformPathTrait;
use crate::common::object_base::cap_falling_velocity;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::tile_types::*;
use crate::globals::{Aliased, Ptr};
use crate::smw::objectgame::removeifprojectile;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;
use std::ffi::CStr;
use std::ptr::{null, null_mut};

/// How an object overlaps a platform: not at all, directly, or across the screen's left or right wrap edge.
type CollisionStyle = i16;
const collision_none: CollisionStyle = 0;
const collision_normal: CollisionStyle = 1;
const collision_overlap_left: CollisionStyle = 2;
const collision_overlap_right: CollisionStyle = 3;

const SW: f32 = App::screenWidth as f32;

/// Tile index of pixel offset `f`, truncating `f` to `i16` before dividing.
#[inline(always)]
fn tile_of(f: f32) -> i16 {
    ((f as i16) as i32 / TILESIZE) as i16
}

#[derive(Clone)]

/// A moving platform. Positions are pixels at the platform's center; sizes are pixels unless named in tiles.
pub struct MovingPlatform {
    /// Tiles and their collision types, column-major (`col * iTileHeight + row`).
    pub iTileData: Vec<TilesetTile>,
    pub iTileType: Vec<TileType>,
    pub ix: i16,
    pub iy: i16,
    /// Size in pixels: tiles times 32, or 16 for a preview.
    pub iWidth: i16,
    pub iHeight: i16,
    pub iTileWidth: i16,
    pub iTileHeight: i16,
    pub iHalfWidth: i16,
    pub iHalfHeight: i16,

    /// A falling platform that left the screen; the map removes it.
    pub fDead: bool,

    pub fx: f32,
    pub fy: f32,
    /// Position and velocity at the start of this tick, used to resolve collisions.
    pub fOldX: f32,
    pub fOldY: f32,
    pub fOldVelX: f32,
    pub fOldVelY: f32,

    pub iSteps: i16,
    pub iOnStep: i16,

    /// Prerendered tiles, double-buffered so animated tiles can redraw one while the other shows.
    pub sprites: [gfxSprite; 2],

    pub rSrcRect: Rect,
    pub rDstRect: Rect,

    pub fForwardDirection: bool,
    pub fStartDirection: bool,

    /// Map draw layer (0 to 4) the platform draws on.
    pub iDrawLayer: i16,

    pub pPath: Box<dyn MovingPlatformPathTrait>,

    pub fVelX: f32,
    pub fVelY: f32,

    /// Player credited for kills the platform causes, or -1.
    pub iPlayerId: i16,

    /// This platform's handle; set when a map takes the platform.
    pub id: PlatformId,

    pub _alias: Aliased,
}

impl MovingPlatform {
    /// A `w` x `h`-tile platform; `tiledata` and `tiletypes` are column-major. Prerenders both draw buffers, at half
    /// size for a preview.
    pub fn new(g: &mut Game, 
        tiledata: Vec<TilesetTile>,
        tiletypes: Vec<TileType>,
        w: i16,
        h: i16,
        drawlayer: i16,
        path: Box<dyn MovingPlatformPathTrait>,
        fPreview: bool,
    ) -> Box<MovingPlatform> {
        let mut gp = g.ptr();
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

        let mut this = Box::new(MovingPlatform {
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
            rSrcRect: Rect { x: 0, y: 0, w: 0, h: 0 },
            rDstRect: Rect { x: 0, y: 0, w: 0, h: 0 },
            fForwardDirection: false,
            fStartDirection: false,
            iDrawLayer: drawlayer,
            pPath: path,
            fVelX: 0.0,
            fVelY: 0.0,
            iPlayerId: -1,
            id: PlatformId::NONE,
            _alias: Aliased::new(),
        });

        this.reset_path(g);

        unsafe {
            for iSurface in 0..2 {
                this.sprites[iSurface] = gfxSprite::blank(g, w as u32 * iTileSize as u32, h as u32 * iTileSize as u32);
                this.sprites[iSurface].set_wrap(640);

                let s = this.sprites[iSurface].get_surface();
                (*s).color_key = Some((*s).map_rgb(255, 0, 255));
                crate::common::gfx::blit::fill_rect(s, null(), (*s).map_rgb(255, 0, 255));
            }

            for iSurface in 0..2 {
                for iCol in 0..this.iTileWidth {
                    for iRow in 0..this.iTileHeight {
                        let tile = this.iTileData[(iCol as i32 * this.iTileHeight as i32 + iRow as i32) as usize];

                        if tile.iID as i32 == TILESETNONE {
                            continue;
                        }

                        if tile.iID >= 0 {
                            gp.g_tilesetmanager.draw(g, this.sprites[iSurface].get_surface(), tile.iID, iTileSizeIndex, tile.iCol, tile.iRow, iCol, iRow);
                        } else if tile.iID as i32 == TILESETANIMATED {
                            g.rm.sprite(g.rm.spr_tileanimation[iTileSizeIndex as usize]).draw_src_to(
                                &*g.g_tilesetmanager.rect(iTileSizeIndex, tile.iCol * 4, tile.iRow),
                                this.sprites[iSurface].get_surface(),
                                &*g.g_tilesetmanager.rect(iTileSizeIndex, iCol, iRow),
                            );
                        } else if tile.iID as i32 == TILESETUNKNOWN {
                            g.rm.sprite(g.rm.spr_unknowntile[iTileSizeIndex as usize]).draw_src_to(
                                &*g.g_tilesetmanager.rect(iTileSizeIndex, 0, 0),
                                this.sprites[iSurface].get_surface(),
                                &*g.g_tilesetmanager.rect(iTileSizeIndex, iCol, iRow),
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

    /// The tile at (`col`, `row`).
    pub fn tile_at(&self, col: usize, row: usize) -> &TilesetTile {
        &self.iTileData[col * self.iTileHeight as usize + row]
    }

    /// The collision type of tile (`col`, `row`).
    pub fn tile_type_at(&self, col: usize, row: usize) -> TileType {
        self.iTileType[col * self.iTileHeight as usize + row]
    }

    /// Draws `spr` over tile (`col`, `row`) in both draw buffers.
    pub fn paint_sprite_at(&mut self, spr: &gfxSprite, col: usize, row: usize) {
        let iTileSize: i16 = self.iWidth / self.iTileWidth;
        let dstPos = Rect { x: col as i32 * iTileSize as i32, y: row as i32 * iTileSize as i32, w: 0, h: 0 };

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

    /// Sets the player credited for kills the platform causes.
    pub fn set_player_id(&mut self, playerId: i16) {
        self.iPlayerId = playerId;
    }

    /// Draws the platform's shown buffer at its position plus camera shake.
    pub fn draw(&mut self, g: &mut Game) {
        self.rDstRect.x = self.ix as i32 - self.iHalfWidth as i32 + g.x_shake as i32;
        self.rDstRect.y = self.iy as i32 - self.iHalfHeight as i32 + g.y_shake as i32;
        self.rDstRect.w = self.iWidth as i32;
        self.rDstRect.h = self.iHeight as i32;

        self.sprites[(1 - g.g_iCurrentDrawIndex) as usize].draw_src_to(&self.rSrcRect, g.blitdest, &self.rDstRect);
    }

    /// Draws the platform at half scale, offset by (`iOffsetX`, `iOffsetY`), for the map preview.
    pub fn draw_offset(&mut self, g: &mut Game, iOffsetX: i16, iOffsetY: i16) {
        gfx_drawpreview(
            g,
            &self.sprites[0],
            (self.ix as i32 - self.iHalfWidth as i32 + iOffsetX as i32) as i16,
            (self.iy as i32 - self.iHalfHeight as i32 + iOffsetY as i32) as i16,
            0,
            0,
            self.iWidth,
            self.iHeight,
            &Rect { x: iOffsetX as i32, y: iOffsetY as i32, w: App::screenWidth / 2, h: App::screenHeight / 2 },
            true,
            Option::None,
        );
    }

    /// Moves both path tracks one tick and takes the new position from track 0.
    pub fn update(&mut self, g: &mut Game) {
        self.fOldX = self.pPath.current_pos0().x;
        self.fOldY = self.pPath.current_pos0().y;

        self.fOldVelX = self.pPath.velocity0().x;
        self.fOldVelY = self.pPath.velocity0().y;

        let this = Ptr::from_mut(self);
        self.pPath.r#move(g, 0, this);

        self.pPath.r#move(g, 1, this);

        self.fVelX = self.pPath.velocity0().x;
        self.fVelY = self.pPath.velocity0().y;

        let x = self.pPath.current_pos0().x;
        let y = self.pPath.current_pos0().y;
        self.set_xf(x);
        self.set_yf(y);
    }

    /// Returns to the path start, with the lookahead track advanced by the spawn animation.
    pub fn reset_path(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let this = Ptr::from_mut(self);
        self.pPath.reset(gp.get(), this);

        let x = self.pPath.current_pos0().x;
        let y = self.pPath.current_pos0().y;
        self.set_xf(x);
        self.set_yf(y);

        self.fOldX = self.fx;
        self.fOldY = self.fy;
    }

    /// Pushes `player` out of the platform's solid tiles, lands them on it, or kills them on death tiles or when crushed.
    pub fn collide_player(&mut self, g: &mut Game, mut player: PlayerId) {
        let this = Ptr::from_mut(self);
        let fx = self.fx;
        let fy = self.fy;
        let iHalfWidth = self.iHalfWidth as f32;
        let iHalfHeight = self.iHalfHeight as f32;
        let iWidth = self.iWidth as f32;
        let iHeight = self.iHeight as f32;
        let iPlayerId = self.iPlayerId;

        let mut coldec = self.coldec_player(g, player);
        if coldec == collision_none {
            if g.player(player).platform == self.id {
                g.player(player).platform = PlatformId::NONE;
            }
            return;
        }

        if self.fDead {
            return;
        }

        let mut fColVelX = g.player(player).velx - self.fOldVelX;
        let mut fColVelY = g.player(player).vely - self.fOldVelY;

        if !g.player(player).platform.is_none() {
            fColVelY += g.g_map.platform(g.player(player).platform).fOldVelY - GRAVITATION;

            if g.player(player).velx < -vel(0.6) || g.player(player).velx > vel(0.6) {
                fColVelX += g.g_map.platform(g.player(player).platform).fOldVelX;
            }
        } else if !g.player(player).inair {
            fColVelY -= GRAVITATION;
        }

        let fRelativeY1: f32;
        let fRelativeY2: f32;

        if g.player(player).platform != self.id {
            fRelativeY1 = g.player(player).fy - self.fOldY + iHalfHeight;
            fRelativeY2 = g.player(player).fy + PH as f32 - self.fOldY + iHalfHeight;
        } else {
            fRelativeY1 = g.player(player).fy - fy + iHalfHeight;
            fRelativeY2 = g.player(player).fy + PH as f32 - fy + iHalfHeight;
        }

        if fColVelX > 0.01 || g.player(player).iHorizontalPlatformCollision == 3 {
            let fRelativeX: f32 = if coldec == collision_normal {
                g.player(player).fx + PW as f32 - fx + iHalfWidth
            } else if coldec == collision_overlap_left {
                g.player(player).fx + PW as f32 - fx + iHalfWidth - SW
            } else {
                g.player(player).fx + PW as f32 - fx + iHalfWidth + SW
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

                    if g.player(player).iHorizontalPlatformCollision == 3 {
                        g.player(player).kill_player_map_hazard(g, true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else if fSuperDeathTileToLeft || (fDeathTileToLeft && !g.player(player).is_invincible() && !g.player(player).is_shielded() && !g.player(player).shyguy) {
                        if PlayerKillType::NonKill != g.player(player).kill_player_map_hazard(g, fSuperDeathTileToLeft, KillStyle::Environment, false, iPlayerId) {
                            return;
                        }
                    } else {
                        g.player(player).set_xf((((tx as i32) << 5) - PW) as f32 - 0.2 + fx - iHalfWidth);
                        g.player(player).flipsidesifneeded();

                        self.check_map_collision_right(g, player);

                        if self.fOldVelX < 0.0 {
                            g.player(player).fOldX = fx + 10.0;
                        } else {
                            g.player(player).fOldX = fx - 10.0;
                        }

                        g.player(player).iHorizontalPlatformCollision = 1;
                        g.player(player).iPlatformCollisionPlayerId = iPlayerId;

                        if g.player(player).velx > 0.0 {
                            g.player(player).velx = 0.0;
                        }
                    }
                }
            }
        } else if fColVelX < -0.01 || g.player(player).iHorizontalPlatformCollision == 1 {
            let fRelativeX: f32 = if coldec == collision_normal {
                g.player(player).fx - fx + iHalfWidth
            } else if coldec == collision_overlap_left {
                g.player(player).fx - fx + iHalfWidth - SW
            } else {
                g.player(player).fx - fx + iHalfWidth + SW
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

                    if g.player(player).iHorizontalPlatformCollision == 1 {
                        g.player(player).kill_player_map_hazard(g, true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else if fSuperDeathTileToRight || (fDeathTileToRight && !g.player(player).is_invincible() && !g.player(player).is_shielded() && !g.player(player).shyguy) {
                        if PlayerKillType::NonKill != g.player(player).kill_player_map_hazard(g, fSuperDeathTileToRight, KillStyle::Environment, false, iPlayerId) {
                            return;
                        }
                    } else {
                        g.player(player).set_xf((((tx as i32) << 5) + TILESIZE) as f32 + 0.2 + fx - iHalfWidth);
                        g.player(player).flipsidesifneeded();

                        self.check_map_collision_left(g, player);

                        if self.fOldVelX < 0.0 {
                            g.player(player).fOldX = fx + 10.0;
                        } else {
                            g.player(player).fOldX = fx - 10.0;
                        }

                        g.player(player).iHorizontalPlatformCollision = 3;
                        g.player(player).iPlatformCollisionPlayerId = iPlayerId;

                        if g.player(player).velx < 0.0 {
                            g.player(player).velx = 0.0;
                        }
                    }
                }
            }
        }

        let fRelativeX1: f32;
        let fRelativeX2: f32;

        coldec = self.coldec_player(g, player);
        if coldec == collision_none {
            if g.player(player).platform == self.id {
                g.player(player).platform = PlatformId::NONE;
            }

            return;
        } else if coldec == collision_normal {
            fRelativeX1 = g.player(player).fx - fx + iHalfWidth;
            fRelativeX2 = g.player(player).fx + PW as f32 - fx + iHalfWidth;
        } else if coldec == collision_overlap_left {
            fRelativeX1 = g.player(player).fx - fx + iHalfWidth - SW;
            fRelativeX2 = g.player(player).fx + PW as f32 - fx + iHalfWidth - SW;
        } else {
            fRelativeX1 = g.player(player).fx - fx + iHalfWidth + SW;
            fRelativeX2 = g.player(player).fx + PW as f32 - fx + iHalfWidth + SW;
        }

        if fColVelY < 0.0 {
            let fRelativeY: f32 = if g.player(player).inair || g.player(player).platform == self.id {
                g.player(player).fPrecalculatedY - fy + iHalfHeight
            } else {
                g.player(player).fy - fy + iHalfHeight
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
                    && (!fDeathTileOverPlayer || g.player(player).is_invincible() || g.player(player).is_shielded() || g.player(player).shyguy)
                {
                    if g.player(player).iVerticalPlatformCollision == 2 {
                        g.player(player).kill_player_map_hazard(g, true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else {
                        g.player(player).fPrecalculatedY = (((ty as i32) << 5) + TILESIZE) as f32 + 0.2 + fy - iHalfHeight;
                        g.player(player).fOldY = g.player(player).fPrecalculatedY - self.fVelY - GRAVITATION;

                        if g.player(player).vely < 0.0 && self.fVelY > 0.0 {
                            g.player(player).fPrecalculatedY += self.fVelY;
                        }

                        if g.player(player).vely < 0.0 {
                            g.player(player).vely = cap_falling_velocity(-g.player(player).vely * BOUNCESTRENGTH + self.fVelY);
                        }

                        g.player(player).iVerticalPlatformCollision = 0;
                        g.player(player).iPlatformCollisionPlayerId = iPlayerId;
                    }

                    return;
                } else if (t1 & tile_flag_player_or_death_on_bottom) != 0 || (t2 & tile_flag_player_or_death_on_bottom) != 0 {
                    if PlayerKillType::NonKill != g.player(player).kill_player_map_hazard(g, true, KillStyle::Environment, false, iPlayerId) {
                        return;
                    }
                }
            }
        } else {
            let fRelativeY = g.player(player).fPrecalculatedY + PH as f32 - fy + iHalfHeight;

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
                    && g.player(player).fOldY + PH as f32 <= ((ty as i32) << 5) as f32 + self.fOldY - iHalfHeight
                {
                    if g.player(player).fallthrough && !fSolidTileUnderPlayer {
                        g.player(player).fPrecalculatedY =
                            (((ty as i32) << 5) - PH) as f32 + (if self.fVelY > 0.0 { self.fVelY } else { 0.0 }) + 0.2 + fy - iHalfHeight;
                        g.player(player).inair = true;
                        g.player(player).platform = PlatformId::NONE;
                    } else if g.player(player).iVerticalPlatformCollision == 0 {
                        g.player(player).kill_player_map_hazard(g, true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else {
                        g.player(player).platform = self.id;
                        g.player(player).iVerticalPlatformCollision = 2;
                        g.player(player).iPlatformCollisionPlayerId = iPlayerId;

                        g.player(player).fPrecalculatedY = (((ty as i32) << 5) - PH) as f32 - 0.2 + fy - iHalfHeight;
                        g.player(player).vely = GRAVITATION;
                        g.player(player).inair = false;
                        g.player(player).killsinrowinair = 0;
                        let gid = g.player(player).globalID;
                        crate::smw::stats::touched(g, gid);
                        g.player(player).extrajumps = 0;
                    }

                    g.player(player).fOldY = g.player(player).fPrecalculatedY - self.fVelY;
                    g.player(player).fallthrough = false;
                    g.player(player).onice = false;

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
                    && (!fDeathTileUnderPlayer || g.player(player).is_invincible() || g.player(player).is_shielded() || g.player(player).kuriboshoe.is_on() || g.player(player).shyguy)
                {
                    if g.player(player).iVerticalPlatformCollision == 0 {
                        g.player(player).kill_player_map_hazard(g, true, KillStyle::Environment, true, iPlayerId);
                        return;
                    } else {
                        g.player(player).platform = self.id;
                        g.player(player).iVerticalPlatformCollision = 2;
                        g.player(player).iPlatformCollisionPlayerId = iPlayerId;

                        g.player(player).fPrecalculatedY = (((ty as i32) << 5) - PH) as f32 - 0.2 + fy - iHalfHeight;
                        g.player(player).fOldY = g.player(player).fPrecalculatedY - self.fVelY;

                        g.player(player).vely = GRAVITATION;
                        g.player(player).inair = false;
                        g.player(player).killsinrowinair = 0;
                        let gid = g.player(player).globalID;
                        crate::smw::stats::touched(g, gid);
                        g.player(player).extrajumps = 0;

                        if ((t1 & tile_flag_ice) != 0 && ((t2 & tile_flag_ice) != 0 || t2 == tile_flag_nonsolid || t2 == tile_flag_gap))
                            || ((t2 & tile_flag_ice) != 0 && ((t1 & tile_flag_ice) != 0 || t1 == tile_flag_nonsolid || t1 == tile_flag_gap))
                        {
                            g.player(player).onice = true;
                        } else {
                            g.player(player).onice = false;
                        }

                        g.player(player).fallthrough = false;

                        return;
                    }
                } else if fDeathTileUnderPlayer || fSuperDeathTileUnderPlayer {
                    if PlayerKillType::NonKill != g.player(player).kill_player_map_hazard(g, fSuperDeathTileUnderPlayer, KillStyle::Environment, false, iPlayerId) {
                        return;
                    }
                } else if g.player(player).platform == self.id {
                    g.player(player).platform = PlatformId::NONE;
                }
            }
        }
    }

    /// After a platform pushes `player` right into the map, pushes them back out of solid map tiles.
    fn check_map_collision_right(&mut self, g: &mut Game, mut player: PlayerId) {
        if g.player(player).fy + (PH as f32) < 0.0 {
            return;
        }

        let iTestBackgroundX: i16 = if g.player(player).fx + PW as f32 >= SW {
            (((g.player(player).fx as i16) as i32 + PW - App::screenWidth) / TILESIZE) as i16
        } else {
            (((g.player(player).fx as i16) as i32 + PW) / TILESIZE) as i16
        };

        let iTestBackgroundY: i16 = (((g.player(player).fy as i16) as i32 + PH) / TILESIZE) as i16;

        let mut topblock = g.map_block(iTestBackgroundX, iTestBackgroundY);

        if (!topblock.is_null() && !topblock.is_transparent() && !topblock.is_hidden())
            || (g.g_map.map(iTestBackgroundX as i32, iTestBackgroundY as i32) & tile_flag_solid) != 0
        {
            g.player(player).set_xf((((iTestBackgroundX as i32) << 5) - PW) as f32 - 0.2);
            g.player(player).flipsidesifneeded();
            return;
        }

        if g.player(player).fy < 0.0 {
            return;
        }

        let iTestBackgroundY2: i16 = tile_of(g.player(player).fy);

        let mut bottomblock = g.map_block(iTestBackgroundX, iTestBackgroundY2);

        if (!bottomblock.is_null() && !bottomblock.is_transparent() && !bottomblock.is_hidden())
            || (g.g_map.map(iTestBackgroundX as i32, iTestBackgroundY2 as i32) & tile_flag_solid) != 0
        {
            g.player(player).set_xf((((iTestBackgroundX as i32) << 5) - PW) as f32 - 0.2);
            g.player(player).flipsidesifneeded();
        }
    }

    /// After a platform pushes `player` left into the map, pushes them back out of solid map tiles.
    fn check_map_collision_left(&mut self, g: &mut Game, mut player: PlayerId) {
        if g.player(player).fy + (PH as f32) < 0.0 {
            return;
        }

        let iTestBackgroundX: i16 = tile_of(g.player(player).fx);
        let iTestBackgroundY: i16 = (((g.player(player).fy as i16) as i32 + PH) / TILESIZE) as i16;

        let mut topblock = g.map_block(iTestBackgroundX, iTestBackgroundY);

        if (!topblock.is_null() && !topblock.is_transparent() && !topblock.is_hidden())
            || (g.g_map.map(iTestBackgroundX as i32, iTestBackgroundY as i32) & tile_flag_solid) != 0
        {
            g.player(player).set_xf((((iTestBackgroundX as i32) << 5) + TILESIZE) as f32 + 0.2);
            g.player(player).flipsidesifneeded();
            return;
        }

        if g.player(player).fy < 0.0 {
            return;
        }

        let iTestBackgroundY2: i16 = tile_of(g.player(player).fy);

        let mut bottomblock = g.map_block(iTestBackgroundX, iTestBackgroundY2);

        if (!bottomblock.is_null() && !bottomblock.is_transparent() && !bottomblock.is_hidden())
            || (g.g_map.map(iTestBackgroundX as i32, iTestBackgroundY2 as i32) & tile_flag_solid) != 0
        {
            g.player(player).set_xf((((iTestBackgroundX as i32) << 5) + TILESIZE) as f32 + 0.2);
            g.player(player).flipsidesifneeded();
        }
    }

    /// Pushes `object` out of any solid platform tiles it overlaps; returns whether it overlapped one.
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

    /// How `player`'s next position overlaps the platform, accounting for horizontal screen wrap.
    fn coldec_player(&self, g: &Game, player: PlayerId) -> CollisionStyle {
        let fx = self.fx;
        let fy = self.fy;
        let hw = self.iHalfWidth as f32;
        let hh = self.iHalfHeight as f32;
        let pw = PW as f32;
        let ph = PH as f32;

        if g.player(player).fx + pw < fx - hw {
            if g.player(player).fx + SW >= fx + hw || g.player(player).fx + pw + SW < fx - hw || g.player(player).fPrecalculatedY >= fy + hh || g.player(player).fPrecalculatedY + ph < fy - hh {
                collision_none
            } else {
                collision_overlap_right
            }
        } else if fx + hw < g.player(player).fx {
            if g.player(player).fx >= fx + hw + SW || g.player(player).fx + pw < fx - hw + SW || g.player(player).fPrecalculatedY >= fy + hh || g.player(player).fPrecalculatedY + ph < fy - hh {
                collision_none
            } else {
                collision_overlap_left
            }
        } else if g.player(player).fx >= fx + hw || fx - hw > g.player(player).fx + pw || g.player(player).fPrecalculatedY >= fy + hh || fy - hh > g.player(player).fPrecalculatedY + ph {
            collision_none
        } else {
            collision_normal
        }
    }

    /// Collision flags of the platform tiles under `player`'s left and right edges, non-solid when off the platform.
    pub fn get_tile_types_from_player(&self, g: &Game, player: PlayerId, lefttile: &mut i32, righttile: &mut i32) {
        *lefttile = tile_flag_nonsolid;
        *righttile = tile_flag_nonsolid;

        let fRelativeY = g.player(player).fPrecalculatedY + PH as f32 - self.fy + self.iHalfHeight as f32;

        if fRelativeY < 0.0 || fRelativeY >= self.iHeight as f32 {
            return;
        }

        let ty = tile_of(fRelativeY);

        let fRelativeX1 = g.player(player).fx - self.fx + self.iHalfWidth as f32;

        let fRelativeX2 = if g.player(player).fx + PW as f32 > SW {
            g.player(player).fx + PW as f32 - SW - self.fx + self.iHalfWidth as f32
        } else {
            g.player(player).fx + PW as f32 - self.fx + self.iHalfWidth as f32
        };

        if fRelativeX1 >= 0.0 && fRelativeX1 < self.iWidth as f32 {
            *lefttile = self.flags_at(tile_of(fRelativeX1), ty);
        }

        if fRelativeX2 >= 0.0 && fRelativeX2 < self.iWidth as f32 {
            *righttile = self.flags_at(tile_of(fRelativeX2), ty);
        }
    }

    /// Collision flags of the platform tile at screen pixel (`x`, `y`), wrapping `x`; non-solid off the platform.
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

    /// Pushes `object` out of the platform's solid tiles or lands it on top.
    pub fn collide_object(&mut self, g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>) {
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
            if object.platform == self.id {
                object.platform = PlatformId::NONE;
            }

            return;
        }

        if self.fDead {
            return;
        }

        let mut fColVelY = object.vely - self.fOldVelY;

        if !object.platform.is_none() {
            fColVelY += g.g_map.platform(object.platform).fOldVelY - GRAVITATION;

            if object.velx < -vel(0.6) || object.velx > vel(0.6) {
                fColVelX += g.g_map.platform(object.platform).fOldVelX;
            }
        } else if !object.inair {
            fColVelY -= GRAVITATION;
        }

        let fRelativeY1: f32;
        let fRelativeY2: f32;

        if object.platform != self.id {
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
                        object.kill_object_map_hazard(g, -1);
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

                        let mut topblock = g.map_block(iTestBackgroundX, iTestBackgroundY);
                        let mut bottomblock = g.map_block(iTestBackgroundX, iTestBackgroundY2);

                        if (!topblock.is_null() && !topblock.is_transparent() && !topblock.is_hidden())
                            || (!bottomblock.is_null() && !bottomblock.is_transparent() && !bottomblock.is_hidden())
                            || (g.g_map.map(iTestBackgroundX as i32, iTestBackgroundY as i32) & tile_flag_solid) != 0
                            || (g.g_map.map(iTestBackgroundX as i32, iTestBackgroundY2 as i32) & tile_flag_solid) != 0
                        {
                            let cwi = object.collisionWidth as i32;
                            object.set_xf((((iTestBackgroundX as i32) << 5) - cwi) as f32 - 0.2);
                            object.flipsidesifneeded();
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

                        removeifprojectile(g, object, true, true);

                        object.side_bounce(g, true);
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
                        object.kill_object_map_hazard(g, -1);
                        return;
                    } else {
                        object.set_xf((((tx as i32) << 5) + TILESIZE) as f32 + 0.2 + fx - iHalfWidth);
                        object.flipsidesifneeded();

                        let iTestBackgroundY: i16 = tile_of(object.fy);
                        let iTestBackgroundY2: i16 = (((object.fy as i16) as i32 + object.collisionHeight as i32) / TILESIZE) as i16;
                        let iTestBackgroundX: i16 = tile_of(object.fx);

                        let mut topblock = g.map_block(iTestBackgroundX, iTestBackgroundY);
                        let mut bottomblock = g.map_block(iTestBackgroundX, iTestBackgroundY2);

                        if (!topblock.is_null() && !topblock.is_transparent() && !topblock.is_hidden())
                            || (!bottomblock.is_null() && !bottomblock.is_transparent() && !bottomblock.is_hidden())
                            || (g.g_map.map(iTestBackgroundX as i32, iTestBackgroundY as i32) & tile_flag_solid) != 0
                            || (g.g_map.map(iTestBackgroundX as i32, iTestBackgroundY2 as i32) & tile_flag_solid) != 0
                        {
                            object.set_xf((((iTestBackgroundX as i32) << 5) + TILESIZE) as f32 + 0.2);
                            object.flipsidesifneeded();
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

                        removeifprojectile(g, object, true, true);

                        object.side_bounce(g, false);
                    }
                }
            }
        }

        let fRelativeX1: f32;
        let fRelativeX2: f32;

        coldec = self.coldec_object(object);
        let cw = object.collisionWidth as f32;
        if coldec == collision_none {
            if object.platform == self.id {
                object.platform = PlatformId::NONE;
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
            let fRelativeY: f32 = if object.inair || object.platform == self.id {
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
                        object.kill_object_map_hazard(g, -1);
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
                        object.kill_object_map_hazard(g, -1);
                        return;
                    } else {
                        if object.bounce >= 0.0 {
                            object.platform = self.id;
                        }

                        object.iVerticalPlatformCollision = 2;

                        let chi = object.collisionHeight as i32;
                        object.fPrecalculatedY = (((ty as i32) << 5) - chi) as f32 - 0.2 + fy - iHalfHeight;
                        object.fOldY = object.fPrecalculatedY - self.fVelY;
                        object.vely = object.bottom_bounce(g);
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
                        object.kill_object_map_hazard(g, -1);
                        return;
                    } else {
                        if object.bounce >= 0.0 {
                            object.platform = self.id;
                        }

                        object.iVerticalPlatformCollision = 2;

                        let chi = object.collisionHeight as i32;
                        object.fPrecalculatedY = (((ty as i32) << 5) - chi) as f32 - 0.2 + fy - iHalfHeight;
                        object.fOldY = object.fPrecalculatedY - self.fVelY;
                        object.vely = object.bottom_bounce(g);
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
                    object.kill_object_map_hazard(g, -1);
                    return;
                } else if object.platform == self.id {
                    object.platform = PlatformId::NONE;
                }
            }
        }
    }

    /// How `object`'s next position overlaps the platform, accounting for horizontal screen wrap.
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

    /// Whether a `w` x `h` box at (`iX`, `iY`) touches a solid tile at the platform's lookahead position.
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

