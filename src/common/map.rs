//! Port of src/common/map.cpp

pub mod map_reader;
pub mod map_reader15xx;
pub mod map_reader16xx;
pub mod map_reader17xx;
pub mod map_reader18xx;
pub mod map_reader_constants;

use crate::common::file_io::BinaryFile;
use crate::common::game::App;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::global::*;
use crate::common::global_constants::*;
use crate::common::io_block::IO_BlockTrait;
use crate::common::map::map_reader::get_loader_by_version;
use crate::common::moving_platform_paths::{EllipsePath, MovingPlatformPathTrait, PlatformPathType, StraightPath, StraightPathContinuous};
use crate::common::movingplatform::MovingPlatform;
use crate::common::path::{concat, convert_path, convert_path_pack, file_exists};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::tile_types::*;
use crate::common::version::Version;
use crate::globals::{Aliased, Ptr};
use crate::smw::main::{blitdest, screen};
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::CPlayer;
use sdl2::sys::image::{IMG_Load, IMG_SavePNG};
use sdl2::sys::*;
use std::ffi::{CStr, CString};
use std::io::Write;
use std::ptr::{null, null_mut};

pub const NUM_FRAMES_IN_TILE_ANIMATION: i16 = 4;
pub const NUM_FRAMES_BETWEEN_TILE_ANIMATION: i16 = 8;

pub type ReadType = u8;
pub const read_type_full: ReadType = 0;
pub const read_type_preview: ReadType = 1;
pub const read_type_summary: ReadType = 2;

pub type MapItemType = u8;
pub const GreenSpring: MapItemType = 0;
pub const Spike: MapItemType = 1;
pub const GreenKuriboShoe: MapItemType = 2;
pub const GoldenSpring: MapItemType = 3;
pub const StickyKuriboShoe: MapItemType = 4;
pub const ThrowBox: MapItemType = 5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

pub type WarpEnterDirection = i32;
pub const WARP_DOWN: WarpEnterDirection = 0;
pub const WARP_LEFT: WarpEnterDirection = 1;
pub const WARP_UP: WarpEnterDirection = 2;
pub const WARP_RIGHT: WarpEnterDirection = 3;
pub const WARP_UNDEFINED: WarpEnterDirection = -1;

pub type WarpExitDirection = i32;
pub const WARP_EXIT_UP: WarpExitDirection = 0;
pub const WARP_EXIT_RIGHT: WarpExitDirection = 1;
pub const WARP_EXIT_DOWN: WarpExitDirection = 2;
pub const WARP_EXIT_LEFT: WarpExitDirection = 3;
pub const WARP_EXIT_UNDEFINED: WarpExitDirection = -1;

#[derive(Clone, Copy, Debug, Default)]
pub struct Warp {
    pub direction: WarpEnterDirection,
    pub connection: i16,
    pub id: i16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WarpExit {
    pub direction: WarpExitDirection,
    pub connection: i16,
    pub id: i16,

    pub x: i16,
    pub y: i16,

    pub lockx: i16,
    pub locky: i16,

    pub warpx: i16,
    pub warpy: i16,
    pub numblocks: i16,

    pub locktimer: i16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SpawnArea {
    pub left: i16,
    pub top: i16,
    pub width: i16,
    pub height: i16,
    pub size: i16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MapItem {
    pub itype: MapItemType,
    pub ix: i16,
    pub iy: i16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MapHazard {
    pub itype: i16,
    pub ix: i16,
    pub iy: i16,

    pub iparam: [i16; NUMMAPHAZARDPARAMS as usize],
    pub dparam: [f32; NUMMAPHAZARDPARAMS as usize],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TilesetTile {
    pub iID: i16,
    pub iCol: i16,
    pub iRow: i16,
}

const ZERO_RECT: SDL_Rect = SDL_Rect { x: 0, y: 0, w: 0, h: 0 };

pub struct AnimatedTile {
    pub id: i16,
    pub layers: [TilesetTile; 4],
    pub rSrc: [[SDL_Rect; 4]; 4],
    pub rAnimationSrc: [[SDL_Rect; 4]; 2],
    pub rDest: SDL_Rect,
    pub fBackgroundAnimated: bool,
    pub fForegroundAnimated: bool,
    pub pPlatform: Ptr<MovingPlatform>,
}

impl AnimatedTile {
    /// `new AnimatedTile()` (value-initialized).
    fn new() -> Self {
        AnimatedTile {
            id: 0,
            layers: [TilesetTile::default(); 4],
            rSrc: [[ZERO_RECT; 4]; 4],
            rAnimationSrc: [[ZERO_RECT; 4]; 2],
            rDest: ZERO_RECT,
            fBackgroundAnimated: false,
            fForegroundAnimated: false,
            pPlatform: Ptr::null(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MapBlock {
    pub iType: i16,
    pub iSettings: [i16; NUM_BLOCK_SETTINGS as usize],
    pub fHidden: bool,
}

impl Default for MapBlock {
    fn default() -> Self {
        MapBlock { iType: 0, iSettings: [0; NUM_BLOCK_SETTINGS as usize], fHidden: false }
    }
}

pub static mut g_iCurrentDrawIndex: i16 = 0;

const fn r(x: i32, y: i32, w: i32, h: i32) -> SDL_Rect {
    SDL_Rect { x, y, w, h }
}

//[Direction][Frame]
pub static g_rFlameRects: [[SDL_Rect; 4]; 4] = [
    [r(0, 0, 96, 32), r(0, 32, 96, 32), r(0, 64, 96, 32), r(0, 96, 96, 32)],
    [r(96, 0, 96, 32), r(96, 32, 96, 32), r(96, 64, 96, 32), r(96, 96, 96, 32)],
    [r(0, 128, 32, 96), r(32, 128, 32, 96), r(64, 128, 32, 96), r(96, 128, 32, 96)],
    [r(128, 128, 32, 96), r(160, 128, 32, 96), r(192, 128, 32, 96), r(224, 128, 32, 96)],
];

//[Type][Direction][Frame]
pub static g_rPirhanaRects: [[[SDL_Rect; 4]; 4]; 4] = [
    [
        [r(0, 0, 32, 48), r(32, 0, 32, 48), r(64, 0, 32, 48), r(96, 0, 32, 48)],
        [r(128, 0, 32, 48), r(160, 0, 32, 48), r(192, 0, 32, 48), r(224, 0, 32, 48)],
        [r(304, 0, 48, 32), r(304, 32, 48, 32), r(304, 64, 48, 32), r(304, 96, 48, 32)],
        [r(304, 128, 48, 32), r(304, 160, 48, 32), r(304, 192, 48, 32), r(304, 224, 48, 32)],
    ],
    [
        [r(0, 48, 32, 48), r(32, 48, 32, 48), r(64, 48, 32, 48), r(96, 48, 32, 48)],
        [r(128, 48, 32, 48), r(160, 48, 32, 48), r(192, 48, 32, 48), r(224, 48, 32, 48)],
        [r(256, 0, 48, 32), r(256, 32, 48, 32), r(256, 64, 48, 32), r(256, 96, 48, 32)],
        [r(256, 128, 48, 32), r(256, 160, 48, 32), r(256, 192, 48, 32), r(256, 224, 48, 32)],
    ],
    [
        [r(0, 96, 32, 64), r(32, 96, 32, 64), r(0, 0, 0, 0), r(0, 0, 0, 0)],
        [r(64, 96, 32, 64), r(96, 96, 32, 64), r(0, 0, 0, 0), r(0, 0, 0, 0)],
        [r(192, 128, 64, 32), r(192, 160, 64, 32), r(0, 0, 0, 0), r(0, 0, 0, 0)],
        [r(192, 192, 64, 32), r(192, 224, 64, 32), r(0, 0, 0, 0), r(0, 0, 0, 0)],
    ],
    [
        [r(0, 160, 32, 48), r(32, 160, 32, 48), r(0, 0, 0, 0), r(0, 0, 0, 0)],
        [r(64, 160, 32, 48), r(96, 160, 32, 48), r(0, 0, 0, 0), r(0, 0, 0, 0)],
        [r(144, 128, 48, 32), r(144, 160, 48, 32), r(0, 0, 0, 0), r(0, 0, 0, 0)],
        [r(144, 192, 48, 32), r(144, 224, 48, 32), r(0, 0, 0, 0), r(0, 0, 0, 0)],
    ],
];

pub static iPlatformPathDotOffset: [i16; 3] = [0, 12, 18];
pub static iPlatformPathDotSize: [i16; 3] = [12, 6, 4];

pub static iFireballHazardSize: [i16; 3] = [18, 9, 5];

pub static iStandardOffset: [i16; 3] = [0, 32, 48];
pub static dBulletBillFrequency: [f32; 3] = [10.0, 5.0, 2.5];

fn small_delay() {
    unsafe { SDL_Delay(10) };
}

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

fn get_screen_width(iSize: i32) -> i32 {
    match iSize {
        0 => App::screenWidth,
        1 => App::screenWidth / 2,
        2 => App::screenWidth / 4,
        _ => std::panic::panic_any("invalid iSize for width"),
    }
}

fn get_screen_height(iSize: i32) -> i32 {
    match iSize {
        0 => App::screenHeight,
        1 => App::screenHeight / 2,
        2 => App::screenHeight / 4,
        _ => std::panic::panic_any("invalid iSize for height"),
    }
}

#[inline]
unsafe fn blit(src: *mut SDL_Surface, srcrect: *const SDL_Rect, dst: *mut SDL_Surface, dstrect: *mut SDL_Rect) -> i32 {
    SDL_UpperBlit(src, srcrect, dst, dstrect)
}

pub fn draw_map_hazard(hazard: &MapHazard, iSize: i16, fDrawCenter: bool) {
    unsafe {
        let s = iSize as usize;
        let iSizeShift: i16 = 5 - iSize;
        let iTileSize: i16 = 1 << iSizeShift;
        let dot = iPlatformPathDotSize[s] as i32;
        let ts = iTileSize as i32;

        let rDotSrc = SDL_Rect { x: iPlatformPathDotOffset[s] as i32 + 22, y: 0, w: dot, h: dot };
        let mut rDotDst = ZERO_RECT;
        let rPathSrc = SDL_Rect { x: iStandardOffset[s] as i32, y: 12, w: ts, h: ts };

        let mut rPathDst = SDL_Rect {
            x: (hazard.ix as i32) << (iSizeShift - 1),
            y: (hazard.iy as i32) << (iSizeShift - 1),
            w: ts,
            h: ts,
        };

        if fDrawCenter && hazard.itype <= 1 {
            rm.spr_platformpath.draw_src_to(&rPathSrc, blitdest, &rPathDst);
        }

        if hazard.itype == 0 {
            let iNumDots: i16 = 16;
            let dRadius: f32 = ((hazard.iparam[0] as i32 - 1) * 24) as f32 / (1i32 << iSize) as f32 + (dot >> 1) as f32;
            let mut dAngle: f32 = hazard.dparam[1];
            for _iDot in 0..iNumDots {
                rDotDst.x = (dRadius * dAngle.cos()) as i16 as i32 + rPathDst.x + (ts >> 1) - (dot >> 1);
                rDotDst.y = (dRadius * dAngle.sin()) as i16 as i32 + rPathDst.y + (ts >> 1) - (dot >> 1);
                rDotDst.w = dot;
                rDotDst.h = dot;

                rm.spr_platformpath.draw_src(rDotDst.x, rDotDst.y, &SDL_Rect { x: rDotSrc.x, y: rDotSrc.y, w: rDotDst.w, h: rDotDst.h });
                dAngle += TWO_PI / iNumDots as f32;
            }

            let fb = iFireballHazardSize[s] as i32;
            for iFireball in 0..hazard.iparam[0] {
                let x: i16 = (((hazard.ix as i32) << (iSizeShift - 1))
                    + ((iFireball as i32 * (24 >> iSize)) as f32 * hazard.dparam[1].cos()) as i16 as i32
                    + (ts >> 1)
                    - (fb >> 1)) as i16;
                let y: i16 = (((hazard.iy as i32) << (iSizeShift - 1))
                    + ((iFireball as i32 * (24 >> iSize)) as f32 * hazard.dparam[1].sin()) as i16 as i32
                    + (ts >> 1)
                    - (fb >> 1)) as i16;

                rm.spr_hazard_fireball[s].draw_src(x as i32, y as i32, &SDL_Rect { x: 0, y: 0, w: fb, h: fb });
            }
        } else if hazard.itype == 1 {
            let iNumDots: i16 = 16;
            let mut dRadius: f32 = (hazard.dparam[2] + (ts >> 1) as f32 - (dot >> 1) as f32) / (1i32 << iSize) as f32;
            let mut dAngle: f32 = hazard.dparam[1];
            for _iDot in 0..iNumDots {
                rDotDst.x = (dRadius * dAngle.cos()) as i16 as i32 + rPathDst.x + (ts >> 1) - (dot >> 1);
                rDotDst.y = (dRadius * dAngle.sin()) as i16 as i32 + rPathDst.y + (ts >> 1) - (dot >> 1);
                rDotDst.w = dot;
                rDotDst.h = dot;

                rm.spr_platformpath.draw_src(rDotDst.x, rDotDst.y, &SDL_Rect { x: rDotSrc.x, y: rDotSrc.y, w: rDotDst.w, h: rDotDst.h });
                dAngle += TWO_PI / iNumDots as f32;
            }

            let dSector: f32 = TWO_PI / hazard.iparam[0] as f32;
            dAngle = hazard.dparam[1];
            dRadius = hazard.dparam[2] / (1i32 << iSize) as f32;
            for _iRotodisc in 0..hazard.iparam[0] {
                let x: i16 = (rPathDst.x + (dRadius * dAngle.cos()) as i16 as i32) as i16;
                let y: i16 = (rPathDst.y + (dRadius * dAngle.sin()) as i16 as i32) as i16;

                rm.spr_hazard_rotodisc[s].draw_src(x as i32, y as i32, &SDL_Rect { x: 0, y: 0, w: ts, h: ts });

                dAngle += dSector;
            }
        } else if hazard.itype == 2 {
            rm.spr_hazard_bulletbill[s].draw_src(
                rPathDst.x,
                rPathDst.y,
                &SDL_Rect { x: 0, y: if hazard.dparam[0] < 0.0 { 0 } else { ts }, w: ts, h: ts },
            );

            let mut iBulletPathX: i16 = (rPathDst.x - dot) as i16;
            if hazard.dparam[0] > 0.0 {
                iBulletPathX = (rPathDst.x + ts) as i16;
            }

            let iBulletPathSpacing: i16 = (hazard.dparam[0] * dBulletBillFrequency[s]) as i16;
            while iBulletPathX >= 0 && (iBulletPathX as i32) < get_screen_width(iSize as i32) {
                rDotDst = SDL_Rect { x: iBulletPathX as i32, y: rPathDst.y + ((ts - dot) >> 1), w: dot, h: dot };
                rm.spr_platformpath.draw_src_to(&rDotSrc, blitdest, &rDotDst);

                iBulletPathX = (iBulletPathX as i32
                    + if (hazard.iparam[0] as f32) < 0.0 { -(iBulletPathSpacing as i32) } else { iBulletPathSpacing as i32 })
                    as i16;
            }
        } else if hazard.itype == 3 {
            let rect = &g_rFlameRects[hazard.iparam[1] as usize][2];

            let mut iOffsetX: i16 = 0;
            let mut iOffsetY: i16 = 0;

            if hazard.iparam[1] == 1 {
                iOffsetX = -(iTileSize << 1);
            } else if hazard.iparam[1] == 2 {
                iOffsetY = -(iTileSize << 1);
            }

            rm.spr_hazard_flame[s].draw_src(
                rPathDst.x + iOffsetX as i32,
                rPathDst.y + iOffsetY as i32,
                &SDL_Rect { x: rect.x >> iSize, y: rect.y >> iSize, w: rect.w >> iSize, h: rect.h >> iSize },
            );
        } else if hazard.itype >= 4 && hazard.itype <= 7 {
            let rect = &g_rPirhanaRects[(hazard.itype - 4) as usize][hazard.iparam[1] as usize][0];
            let mut iOffsetX: i16 = 0;
            let mut iOffsetY: i16 = 0;

            if hazard.iparam[1] == 0 {
                if hazard.itype == 6 {
                    iOffsetY = -iTileSize;
                } else {
                    iOffsetY = -(iTileSize >> 1);
                }
            } else if hazard.iparam[1] == 2 {
                if hazard.itype == 6 {
                    iOffsetX = -iTileSize;
                } else {
                    iOffsetX = -(iTileSize >> 1);
                }
            }

            rm.spr_hazard_pirhanaplant[s].draw_src(
                rPathDst.x + iOffsetX as i32,
                rPathDst.y + iOffsetY as i32,
                &SDL_Rect { x: rect.x >> iSize, y: rect.y >> iSize, w: rect.w >> iSize, h: rect.h >> iSize },
            );
        }
    }
}

unsafe fn blit_platform_tile(tile: &TilesetTile, iSize: i16, bltrect: &mut SDL_Rect) {
    if tile.iID >= 0 {
        blit(
            g_tilesetmanager.tileset(tile.iID as usize).surface(iSize as usize),
            g_tilesetmanager.rect(iSize, tile.iCol, tile.iRow),
            blitdest,
            bltrect,
        );
    } else if tile.iID as i32 == TILESETANIMATED {
        blit(rm.spr_tileanimation[iSize as usize].get_surface(), g_tilesetmanager.rect(iSize, tile.iCol * 4, tile.iRow), blitdest, bltrect);
    } else if tile.iID as i32 == TILESETUNKNOWN {
        blit(rm.spr_unknowntile[iSize as usize].get_surface(), g_tilesetmanager.rect(iSize, 0, 0), blitdest, bltrect);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_platform(
    pathtype: PlatformPathType,
    tiles: &[TilesetTile],
    startX: i16,
    startY: i16,
    endX: i16,
    endY: i16,
    angle: f32,
    radiusX: f32,
    radiusY: f32,
    iSize: i16,
    iPlatformWidth: i16,
    iPlatformHeight: i16,
    fDrawPlatform: bool,
    fDrawShadow: bool,
) {
    unsafe {
        let s = iSize as usize;
        let iStartX: i16 = startX >> iSize;
        let iStartY: i16 = startY >> iSize;
        let iEndX: i16 = endX >> iSize;
        let iEndY: i16 = endY >> iSize;

        let fRadiusX: f32 = radiusX / (1i32 << iSize) as f32;
        let fRadiusY: f32 = radiusY / (1i32 << iSize) as f32;

        let iSizeShift: i16 = 5 - iSize;
        let iTileSize: i32 = 1 << iSizeShift;
        let pw = iPlatformWidth as i32;
        let ph = iPlatformHeight as i32;
        let dot = iPlatformPathDotSize[s] as i32;

        if fDrawPlatform {
            for iPlatformX in 0..iPlatformWidth {
                for iPlatformY in 0..iPlatformHeight {
                    let tile = &tiles[(iPlatformX as i32 * ph + iPlatformY as i32) as usize];

                    let mut iDstX: i32;
                    let iDstY: i32;

                    if pathtype == PlatformPathType::Ellipse {
                        iDstX = iStartX as i32 + ((iPlatformX as i32) << iSizeShift) + (fRadiusX * angle.cos()) as i16 as i32
                            - (pw << (iSizeShift - 1));
                        iDstY = iStartY as i32 + ((iPlatformY as i32) << iSizeShift) + (fRadiusY * angle.sin()) as i16 as i32
                            - (ph << (iSizeShift - 1));
                    } else {
                        iDstX = iStartX as i32 + ((iPlatformX as i32) << iSizeShift) - (pw << (iSizeShift - 1));
                        iDstY = iStartY as i32 + ((iPlatformY as i32) << iSizeShift) - (ph << (iSizeShift - 1));
                    }

                    let mut bltrect = SDL_Rect { x: iDstX, y: iDstY, w: iTileSize, h: iTileSize };
                    blit_platform_tile(tile, iSize, &mut bltrect);

                    let mut fNeedWrap = false;
                    if iDstX + iTileSize >= get_screen_width(iSize as i32) {
                        iDstX -= get_screen_width(iSize as i32);
                        fNeedWrap = true;
                    } else if iDstX < 0 {
                        iDstX += get_screen_width(iSize as i32);
                        fNeedWrap = true;
                    }

                    if fNeedWrap {
                        bltrect.x = iDstX;
                        bltrect.y = iDstY;
                        bltrect.w = iTileSize;
                        bltrect.h = iTileSize;

                        blit_platform_tile(tile, iSize, &mut bltrect);
                    }
                }
            }
        }

        let rPathSrc = SDL_Rect { x: iPlatformPathDotOffset[s] as i32, y: 0, w: dot, h: dot };
        let mut rPathDst: SDL_Rect;

        let draw_tile_marks = |spr: &gfxSprite, x0: i32, y0: i32| {
            for iCol in 0..iPlatformWidth {
                for iRow in 0..iPlatformHeight {
                    if tiles[(iCol as i32 * ph + iRow as i32) as usize].iID != -2 {
                        spr.draw_src(
                            x0 + ((iCol as i32) << iSizeShift),
                            y0 + ((iRow as i32) << iSizeShift),
                            &SDL_Rect { x: 0, y: 0, w: iTileSize, h: iTileSize },
                        );
                    }
                }
            }
        };

        if pathtype == PlatformPathType::Straight {
            if fDrawShadow {
                draw_tile_marks(&rm.spr_platformstarttile, iStartX as i32 - (pw << (iSizeShift - 1)), iStartY as i32 - (ph << (iSizeShift - 1)));
                draw_tile_marks(&rm.spr_platformendtile, iEndX as i32 - (pw << (iSizeShift - 1)), iEndY as i32 - (ph << (iSizeShift - 1)));
            }

            let dDiffX: f32 = (iEndX as i32 - iStartX as i32) as f32;
            let dDiffY: f32 = (iEndY as i32 - iStartY as i32) as f32;

            let iDistance: i16 = (dDiffX * dDiffX + dDiffY * dDiffY).sqrt() as i16;

            let iNumSpots: i16 = iDistance >> iSizeShift;
            let dIncrementX: f32 = dDiffX / iNumSpots as f32;
            let dIncrementY: f32 = dDiffY / iNumSpots as f32;

            let mut dX: f32 = iStartX as f32 - (dot >> 1) as f32;
            let mut dY: f32 = iStartY as f32 - (dot >> 1) as f32;

            for _iSpot in 0..(iNumSpots as i32 + 1) {
                rPathDst = SDL_Rect { x: dX as i16 as i32, y: dY as i16 as i32, w: dot, h: dot };
                rm.spr_platformpath.draw_src_to(&rPathSrc, blitdest, &rPathDst);

                dX += dIncrementX;
                dY += dIncrementY;
            }
        } else if pathtype == PlatformPathType::StraightContinuous {
            if fDrawShadow {
                draw_tile_marks(&rm.spr_platformstarttile, iStartX as i32 - (pw << (iSizeShift - 1)), iStartY as i32 - (ph << (iSizeShift - 1)));
            }

            let dIncrementX: f32 = iTileSize as f32 * angle.cos();
            let dIncrementY: f32 = iTileSize as f32 * angle.sin();

            let mut dX: f32 = iStartX as f32 - (dot >> 1) as f32;
            let mut dY: f32 = iStartY as f32 - (dot >> 1) as f32;

            for _iSpot in 0..50 {
                rPathDst = SDL_Rect { x: dX as i16 as i32, y: dY as i16 as i32, w: dot, h: dot };
                rm.spr_platformpath.draw_src_to(&rPathSrc, blitdest, &rPathDst);

                let mut iWrapX: i16 = dX as i16;
                let mut iWrapY: i16 = dY as i16;
                let mut fNeedWrap = false;
                if dX + dot as f32 >= get_screen_width(iSize as i32) as f32 {
                    iWrapX = (dX - get_screen_width(iSize as i32) as f32) as i16;
                    fNeedWrap = true;
                } else if dX < 0.0 {
                    iWrapX = (dX + get_screen_width(iSize as i32) as f32) as i16;
                    fNeedWrap = true;
                }

                if dY + dot as f32 >= get_screen_height(iSize as i32) as f32 {
                    iWrapY = (dY - get_screen_height(iSize as i32) as f32) as i16;
                    fNeedWrap = true;
                } else if dY < 0.0 {
                    iWrapY = (dY + get_screen_height(iSize as i32) as f32) as i16;
                    fNeedWrap = true;
                }

                if fNeedWrap {
                    rPathDst = SDL_Rect { x: iWrapX as i32, y: iWrapY as i32, w: dot, h: dot };
                    rm.spr_platformpath.draw_src_to(&rPathSrc, blitdest, &rPathDst);
                }

                dX += dIncrementX;
                dY += dIncrementY;
            }
        } else if pathtype == PlatformPathType::Ellipse {
            if fDrawShadow {
                let iEllipseStartX: i16 = ((fRadiusX * angle.cos()) as i16 as i32 - (pw << (iSizeShift - 1)) + iStartX as i32) as i16;
                let iEllipseStartY: i16 = ((fRadiusY * angle.sin()) as i16 as i32 - (ph << (iSizeShift - 1)) + iStartY as i32) as i16;

                draw_tile_marks(&rm.spr_platformstarttile, iEllipseStartX as i32, iEllipseStartY as i32);
            }

            let mut fAngle: f32 = angle;
            for _iSpot in 0..32 {
                let iX: i16 = ((fRadiusX * fAngle.cos()) as i16 as i32 - (dot >> 1) + iStartX as i32) as i16;
                let iY: i16 = ((fRadiusY * fAngle.sin()) as i16 as i32 - (dot >> 1) + iStartY as i32) as i16;

                rPathDst = SDL_Rect { x: iX as i32, y: iY as i32, w: dot, h: dot };
                rm.spr_platformpath.draw_src_to(&rPathSrc, blitdest, &rPathDst);

                if iX as i32 + dot >= get_screen_width(iSize as i32) {
                    rPathDst = SDL_Rect { x: iX as i32 - get_screen_width(iSize as i32), y: iY as i32, w: dot, h: dot };
                    rm.spr_platformpath.draw_src_to(&rPathSrc, blitdest, &rPathDst);
                } else if iX < 0 {
                    rPathDst = SDL_Rect { x: iX as i32 + get_screen_width(iSize as i32), y: iY as i32, w: dot, h: dot };
                    rm.spr_platformpath.draw_src_to(&rPathSrc, blitdest, &rPathDst);
                }

                fAngle += TWO_PI / 32.0;
            }
        }
    }
}

const MW: usize = MAPWIDTH as usize;

// clang arm64 offsets of CMap members in the C++ reference (tools/ref/cmap_layout.cpp).
pub const CPP_OFF_MAPDATA: i32 = 38;
pub const CPP_OFF_MAPDATATOP: i32 = 7238;
pub const CPP_OFF_OBJECTDATA: i32 = 7538;
pub const CPP_OFF_BLOCKDATA: i32 = 24344;
pub const CPP_OFF_NOSPAWN: i32 = 26744;
pub const CPP_OFF_WARPDATA: i32 = 36368;
pub const CPP_SIZEOF_MAPBLOCK: i32 = 56;
const MH: usize = MAPHEIGHT as usize;
const ML: usize = MAPLAYERS as usize;
const NSPAWN: usize = NUMSPAWNAREATYPES as usize;

pub struct CMap {
    pub szBackgroundFile: String,
    pub backgroundID: i16,
    pub eyecandy: [i16; 3],
    pub musicCategoryID: i16,

    pub iNumRaceGoals: i16,
    pub iNumFlagBases: i16,

    pub mapdata: [[[TilesetTile; ML]; MH]; MW],
    pub mapdatatop: [[TileType; MH]; MW],
    pub objectdata: [[MapBlock; MH]; MW],
    pub blockdata: [[Ptr<dyn IO_BlockTrait>; MH]; MW],
    pub nospawn: [[[bool; MH]; MW]; NSPAWN],

    pub animatedtiles: Vec<Box<AnimatedTile>>,

    pub platforms: Vec<Ptr<MovingPlatform>>,
    pub tempPlatforms: Vec<Ptr<MovingPlatform>>,

    pub mapitems: Vec<MapItem>,
    pub maphazards: Vec<MapHazard>,

    pub spawnareas: [[SpawnArea; MAXSPAWNAREAS as usize]; NSPAWN],
    pub numspawnareas: [i16; NSPAWN],
    pub totalspawnsize: [i16; NSPAWN],

    pub warpdata: [[Warp; MH]; MW],
    pub numwarpexits: i16,
    pub warpexits: [WarpExit; MAXWARPS as usize],
    pub warplocktimer: [i16; 10],
    pub warplocked: [bool; 10],
    pub maxConnection: i16,

    pub tilebltrect: SDL_Rect,
    pub bltrect: SDL_Rect,

    pub drawareas: [SDL_Rect; MAXDRAWAREAS as usize],
    pub numdrawareas: i16,

    pub iSwitches: [i16; 4],
    pub switchBlocks: [Vec<Ptr<dyn IO_BlockTrait>>; 8],

    pub fAutoFilter: [bool; NUM_AUTO_FILTERS as usize],

    pub racegoallocations: [Point; MAXRACEGOALS as usize],

    pub flagbaselocations: [Point; 4],

    pub iTileAnimationTimer: i16,
    pub iTileAnimationFrame: i16,

    pub iAnimatedBackgroundLayers: i16,
    pub animatedFrontmapSurface: *mut SDL_Surface,
    pub animatedTilesSurface: *mut SDL_Surface,

    pub iAnimatedTileCount: i16,
    pub iAnimatedVectorIndices: [i16; NUM_FRAMES_BETWEEN_TILE_ANIMATION as usize + 1],
    pub animatedBackmapSurface: *mut SDL_Surface,

    pub platformdrawlayer: [Vec<Ptr<MovingPlatform>>; 5],

    pub _alias: Aliased,
}

impl CMap {
    /// C++ `new CMap`. Members the C++ leaves default-initialized start at zero.
    pub fn new() -> Box<CMap> {
        Box::new(CMap {
            szBackgroundFile: String::new(),
            backgroundID: 0,
            eyecandy: [0; 3],
            musicCategoryID: 0,
            iNumRaceGoals: 0,
            iNumFlagBases: 0,
            mapdata: [[[TilesetTile::default(); ML]; MH]; MW],
            mapdatatop: [[TileType::NonSolid; MH]; MW],
            objectdata: [[MapBlock::default(); MH]; MW],
            blockdata: [[Ptr::null(); MH]; MW],
            nospawn: [[[false; MH]; MW]; NSPAWN],
            animatedtiles: Vec::new(),
            platforms: Vec::new(),
            tempPlatforms: Vec::new(),
            mapitems: Vec::with_capacity(MAXMAPITEMS as usize),
            maphazards: Vec::with_capacity(MAXMAPHAZARDS as usize),
            spawnareas: [[SpawnArea::default(); MAXSPAWNAREAS as usize]; NSPAWN],
            numspawnareas: [0; NSPAWN],
            totalspawnsize: [0; NSPAWN],
            warpdata: [[Warp::default(); MH]; MW],
            numwarpexits: 0,
            warpexits: [WarpExit::default(); MAXWARPS as usize],
            warplocktimer: [0; 10],
            warplocked: [false; 10],
            maxConnection: 0,
            tilebltrect: ZERO_RECT,
            bltrect: ZERO_RECT,
            drawareas: [ZERO_RECT; MAXDRAWAREAS as usize],
            numdrawareas: 0,
            iSwitches: [0; 4],
            switchBlocks: Default::default(),
            fAutoFilter: [false; NUM_AUTO_FILTERS as usize],
            racegoallocations: [Point::default(); MAXRACEGOALS as usize],
            flagbaselocations: [Point::default(); 4],
            iTileAnimationTimer: 0,
            iTileAnimationFrame: 0,
            iAnimatedBackgroundLayers: 0,
            animatedFrontmapSurface: null_mut(),
            animatedTilesSurface: null_mut(),
            iAnimatedTileCount: 0,
            iAnimatedVectorIndices: [0; NUM_FRAMES_BETWEEN_TILE_ANIMATION as usize + 1],
            animatedBackmapSurface: null_mut(),
            platformdrawlayer: Default::default(),
            _alias: Aliased::new(),
        })
    }

    /// Returns the tile flags of the front-most visible tile.
    pub fn map(&self, x: i32, y: i32) -> i32 {
        let flat = x * MAPHEIGHT + y;
        if (0..MAPWIDTH * MAPHEIGHT).contains(&flat) {
            return tile_to_flags(self.mapdatatop[(flat / MAPHEIGHT) as usize][(flat % MAPHEIGHT) as usize]) as i32;
        }
        match self.cpp_byte(CPP_OFF_MAPDATATOP + flat) {
            Some(b) => tile_to_flags(TileType(b)) as i32,
            Option::None => panic!("CMap::map({}, {}): the C++ reads block pointer bytes as a tile", x, y),
        }
    }

    pub fn block(&self, x: i16, y: i16) -> Ptr<dyn IO_BlockTrait> {
        let flat = x as i32 * MAPHEIGHT + y as i32;
        if (0..MAPWIDTH * MAPHEIGHT).contains(&flat) {
            return self.blockdata[(flat / MAPHEIGHT) as usize][(flat % MAPHEIGHT) as usize];
        }
        let off = CPP_OFF_BLOCKDATA + flat * 8;
        if (0..8).all(|i| self.cpp_byte(off + i) == Some(0)) {
            return Ptr::null();
        }
        panic!("CMap::block({}, {}): C++ would deref garbage at CMap offset {}", x, y, off);
    }

    /// The C++ `blockdata[x][y] == NULL` test, which compares the pointer without dereferencing it.
    pub fn block_is_null(&self, x: i16, y: i16) -> bool {
        let flat = x as i32 * MAPHEIGHT + y as i32;
        if (0..MAPWIDTH * MAPHEIGHT).contains(&flat) {
            return self.blockdata[(flat / MAPHEIGHT) as usize][(flat % MAPHEIGHT) as usize].is_null();
        }
        let off = CPP_OFF_BLOCKDATA + flat * 8;
        (0..8).all(|i| self.cpp_byte(off + i) == Some(0))
    }

    /// One byte of the C++ CMap object at `off`, so out-of-range C++ indexing (which stays inside the
    /// object) reads what it would. None is a byte of a non-null block pointer. Covers mapdata through
    /// nospawn; anything else panics.
    pub fn cpp_byte(&self, off: i32) -> Option<u8> {
        let cells = MAPWIDTH * MAPHEIGHT;
        let cell = |e: i32| ((e / MAPHEIGHT) as usize, (e % MAPHEIGHT) as usize);
        if (CPP_OFF_MAPDATA..CPP_OFF_MAPDATATOP).contains(&off) {
            let rel = off - CPP_OFF_MAPDATA;
            let e = rel / 6;
            let (x, y) = cell(e / MAPLAYERS);
            let t = &self.mapdata[x][y][(e % MAPLAYERS) as usize];
            let v = [t.iID, t.iCol, t.iRow][((rel % 6) / 2) as usize];
            return Some(v.to_le_bytes()[(rel % 2) as usize]);
        }
        if (CPP_OFF_MAPDATATOP..CPP_OFF_MAPDATATOP + cells).contains(&off) {
            let (x, y) = cell(off - CPP_OFF_MAPDATATOP);
            return Some(self.mapdatatop[x][y].0);
        }
        if (CPP_OFF_OBJECTDATA..CPP_OFF_OBJECTDATA + cells * CPP_SIZEOF_MAPBLOCK).contains(&off) {
            let rel = off - CPP_OFF_OBJECTDATA;
            let (x, y) = cell(rel / CPP_SIZEOF_MAPBLOCK);
            let b = &self.objectdata[x][y];
            let r = rel % CPP_SIZEOF_MAPBLOCK;
            return Some(match r {
                0..=1 => b.iType.to_le_bytes()[r as usize],
                2..=53 => b.iSettings[((r - 2) / 2) as usize].to_le_bytes()[(r % 2) as usize],
                54 => b.fHidden as u8,
                _ => 0,
            });
        }
        if (CPP_OFF_OBJECTDATA + cells * CPP_SIZEOF_MAPBLOCK..CPP_OFF_BLOCKDATA).contains(&off) {
            return Some(0);
        }
        if (CPP_OFF_BLOCKDATA..CPP_OFF_NOSPAWN).contains(&off) {
            let (x, y) = cell((off - CPP_OFF_BLOCKDATA) / 8);
            return if self.blockdata[x][y].is_null() { Some(0) } else { Option::None };
        }
        if (CPP_OFF_NOSPAWN..CPP_OFF_NOSPAWN + NUMSPAWNAREATYPES * cells).contains(&off) {
            let rel = off - CPP_OFF_NOSPAWN;
            let (x, y) = cell(rel % cells);
            return Some(self.nospawn[(rel / cells) as usize][x][y] as u8);
        }
        panic!("C++ out-of-bounds CMap read at offset {} is not modelled", off);
    }

    pub fn warp(&mut self, x: i16, y: i16) -> &mut Warp {
        &mut self.warpdata[x as usize][y as usize]
    }

    pub fn blockat(&mut self, x: i16, y: i16) -> &mut MapBlock {
        &mut self.objectdata[x as usize][y as usize]
    }

    pub fn spawn(&self, iType: i16, x: i16, y: i16) -> bool {
        !self.nospawn[iType as usize][x as usize][y as usize]
    }

    pub fn isconnectionlocked(&self, connection: i32) -> bool {
        self.warplocked[connection as usize]
    }

    pub fn clear_map(&mut self) {
        for j in 0..MH {
            for i in 0..MW {
                for k in 0..ML {
                    self.mapdata[i][j][k].iID = TILESETNONE as i16;
                }

                self.mapdatatop[i][j] = TileType::NonSolid;

                self.objectdata[i][j].iType = -1;
                self.warpdata[i][j].direction = WARP_UNDEFINED;
                self.warpdata[i][j].connection = -1;

                for iSpawn in 0..NSPAWN {
                    self.nospawn[iSpawn][i][j] = false;
                }
            }
        }

        self.eyecandy = [0; 3];

        self.mapitems.clear();
        self.maphazards.clear();

        for iSwitch in 0..4 {
            self.iSwitches[iSwitch] = 0;
        }

        self.bltrect.w = TILESIZE;
        self.bltrect.h = TILESIZE;
    }

    pub fn clear_platforms(&mut self) {
        for iLayer in 0..5 {
            self.platformdrawlayer[iLayer].clear();
        }

        for platform in self.platforms.drain(..) {
            platform.delete();
        }

        for platform in self.tempPlatforms.drain(..) {
            platform.delete();
        }
    }

    fn clear_animated_tiles(&mut self) {
        self.animatedtiles.clear();
    }

    pub fn load_map(&mut self, file: &str, iReadType: ReadType) {
        self.iTileAnimationTimer = 0;
        self.iTileAnimationFrame = 0;

        self.clear_animated_tiles();

        self.eyecandy = [0; 3];
        self.mapitems.clear();
        self.maphazards.clear();

        let mut mapfile = BinaryFile::new(file, "rb");
        if !mapfile.is_open() {
            println!();
            println!(" ERROR: Couldn't open map");
            return;
        }

        let version = Version {
            major: mapfile.read_i32() as u8,
            minor: mapfile.read_i32() as u8,
            patch: mapfile.read_i32() as u8,
            build: mapfile.read_i32() as u8,
        };

        if iReadType != read_type_summary {
            print!("loading map {}", file);

            if iReadType == read_type_preview {
                print!(" (preview)");
            }

            if version >= (Version { major: 1, minor: 6, patch: 0, build: 0 }) {
                print!(" [v{}.{}.{}.{}]", version.major, version.minor, version.patch, version.build);
            } else {
                print!(" [v1.5]");
            }

            print!(" ...");
        }

        let mut reader = get_loader_by_version(&version);
        reader.load(self, &mut mapfile, iReadType);
        drop(reader);

        if iReadType == read_type_summary {
            return;
        }

        self.clear_warp_locks();
        println!(" done");
        let _ = std::io::stdout().flush();
    }

    pub fn update_all_tile_gaps(&mut self) {
        for j in 0..MAPHEIGHT as i16 {
            for i in 0..MAPWIDTH as i16 {
                self.set_tile_gap(i, j);
            }
        }
    }

    pub fn update_tile_gap(&mut self, i: i16, j: i16) {
        for iRow in j..=j + 1 {
            for iCol in i - 1..=i + 1 {
                if iRow as i32 >= MAPHEIGHT {
                    break;
                }

                let mut ix = iCol;
                if ix < 0 {
                    ix = (MAPWIDTH - 1) as i16;
                } else if ix as i32 >= MAPWIDTH {
                    ix = 0;
                }

                self.set_tile_gap(ix, iRow);
            }
        }
    }

    fn set_tile_gap(&mut self, i: i16, j: i16) {
        let mut iLeftTile = i - 1;
        if iLeftTile < 0 {
            iLeftTile = (MAPWIDTH - 1) as i16;
        }

        let mut iRightTile = i + 1;
        if iRightTile as i32 >= MAPWIDTH {
            iRightTile = 0;
        }

        let (l, c, rt, j) = (iLeftTile as usize, i as usize, iRightTile as usize, j as usize);

        let mut topLeftTile: i32 = 0;
        let mut topCenterTile: i32 = 0;
        let mut topRightTile: i32 = 0;

        let mut topLeftBlock: Ptr<dyn IO_BlockTrait> = Ptr::null();
        let mut topCenterBlock: Ptr<dyn IO_BlockTrait> = Ptr::null();
        let mut topRightBlock: Ptr<dyn IO_BlockTrait> = Ptr::null();

        if j > 0 {
            topLeftTile = tile_to_flags(self.mapdatatop[l][j - 1]) as i32;
            topCenterTile = tile_to_flags(self.mapdatatop[c][j - 1]) as i32;
            topRightTile = tile_to_flags(self.mapdatatop[rt][j - 1]) as i32;

            topLeftBlock = self.blockdata[l][j - 1];
            topCenterBlock = self.blockdata[c][j - 1];
            topRightBlock = self.blockdata[rt][j - 1];
        }

        let leftTile = tile_to_flags(self.mapdatatop[l][j]) as i32;
        let centerTile = tile_to_flags(self.mapdatatop[c][j]) as i32;
        let rightTile = tile_to_flags(self.mapdatatop[rt][j]) as i32;

        let leftBlock = self.blockdata[l][j];
        let centerBlock = self.blockdata[c][j];
        let rightBlock = self.blockdata[rt][j];

        let opaque = |mut b: Ptr<dyn IO_BlockTrait>| !b.is_null() && !b.is_transparent() && !b.is_hidden();

        let fLeftSolid = (leftTile != tile_flag_nonsolid && leftTile != tile_flag_gap) || opaque(leftBlock);
        let fCenterSolid = (centerTile != tile_flag_nonsolid && centerTile != tile_flag_gap) || opaque(centerBlock);
        let fRightSolid = (rightTile != tile_flag_nonsolid && rightTile != tile_flag_gap) || opaque(rightBlock);

        let fTopLeftSolid = (topLeftTile & tile_flag_solid) != 0 || opaque(topLeftBlock);
        let fTopCenterSolid = (topCenterTile & tile_flag_solid) != 0 || opaque(topCenterBlock);
        let fTopRightSolid = (topRightTile & tile_flag_solid) != 0 || opaque(topRightBlock);

        if fLeftSolid && !fCenterSolid && fRightSolid && !fTopLeftSolid && !fTopCenterSolid && !fTopRightSolid {
            self.mapdatatop[c][j] = TileType::Gap;
        } else if self.mapdatatop[c][j] == TileType::Gap {
            self.mapdatatop[c][j] = TileType::NonSolid;
        }
    }

    pub fn save_map(&mut self, file: &str) {
        print!("saving map {} ... ", file);

        let mut mapfile = BinaryFile::new(file, "wb");
        if !mapfile.is_open() {
            println!();
            println!(" ERROR: couldn't save map");
            return;
        }

        mapfile.write_i32(1);
        mapfile.write_i32(8);
        mapfile.write_i32(0);
        mapfile.write_i32(4);

        let mut usedtile = [[false; MH]; MW];

        let mut iHazardCount: i32 = 0;
        let mut iWarpCount: i32 = 0;
        let mut iIceCount: i32 = 0;
        let mut iPowerupBlockCount: i32 = 0;
        let mut iPlatformCount: i32 = 0;
        let mut iDensity: i32 = 0;
        let mut iOnOffBlockCount: i32 = 0;
        let mut iThrowBlockCount: i32 = 0;
        let mut iBreakableBlockCount: i32 = 0;
        let mut iItemDestroyableBlockCount: i32 = 0;
        let mut iHiddenBlockCount: i32 = 0;

        for platform in &self.platforms {
            for iCol in 0..platform.iTileWidth {
                for iRow in 0..platform.iTileHeight {
                    let iType = platform.tile_type_at(iCol as usize, iRow as usize);
                    let iFlags = tile_to_flags(iType) as i32;

                    let tile = platform.tile_at(iCol as usize, iRow as usize);

                    if tile.iID as i32 != TILESETNONE {
                        iPlatformCount += 1;
                    }

                    if (iFlags & tile_flag_has_death) != 0 {
                        iHazardCount += 1;
                    }

                    if (iFlags & tile_flag_ice) != 0 {
                        iIceCount += 1;
                    }
                }
            }
        }

        let mut numWarpExits: i16 = 0;
        for j in 0..MH {
            for i in 0..MW {
                if self.warpdata[i][j].connection != -1 && !usedtile[i][j] {
                    let mut movex = 0usize;
                    let mut movey = 0usize;
                    let mut currentx = i;
                    let mut currenty = j;

                    if self.warpdata[i][j].direction == 0 || self.warpdata[i][j].direction == 2 {
                        movex = 1;
                    } else {
                        movey = 1;
                    }

                    while currentx < MW && currenty < MH {
                        if self.warpdata[currentx][currenty].direction != self.warpdata[i][j].direction
                            || self.warpdata[currentx][currenty].connection != self.warpdata[i][j].connection
                        {
                            break;
                        }

                        usedtile[currentx][currenty] = true;

                        if (numWarpExits as i32) < MAXWARPS {
                            self.warpdata[currentx][currenty].id = numWarpExits;
                        } else {
                            self.warpdata[currentx][currenty].connection = -1;
                            self.warpdata[currentx][currenty].direction = WARP_UNDEFINED;
                        }

                        currentx += movex;
                        currenty += movey;
                    }

                    numWarpExits += 1;
                }

                let iBlockType = self.objectdata[i][j].iType;
                let iFlags = tile_to_flags(self.mapdatatop[i][j]) as i32;

                if (iFlags & tile_flag_has_death) != 0 {
                    iHazardCount += 1;
                }

                if self.warpdata[i][j].connection != -1 {
                    iWarpCount += 1;
                }

                if (iFlags & tile_flag_ice) != 0 {
                    iIceCount += 1;
                }

                if iBlockType == 1 || iBlockType == 15 {
                    iPowerupBlockCount += 1;
                }

                if iBlockType == 0 {
                    iBreakableBlockCount += 1;
                }

                if iBlockType == 6 || iBlockType == 16 {
                    iThrowBlockCount += 1;
                }

                if (11..=14).contains(&iBlockType) {
                    iOnOffBlockCount += 1;
                }

                if (iFlags & tile_flag_solid) != 0 {
                    iDensity += 1;
                }

                if matches!(iBlockType, 1 | 3 | 4 | 5 | 15 | 17 | 18) && self.objectdata[i][j].fHidden {
                    iHiddenBlockCount += 1;
                }

                if (20..=29).contains(&iBlockType) {
                    iItemDestroyableBlockCount += 1;
                }
            }
        }

        mapfile.write_i32(iHazardCount);
        mapfile.write_i32(iWarpCount);
        mapfile.write_i32(iIceCount);
        mapfile.write_i32(iPowerupBlockCount);
        mapfile.write_i32(iBreakableBlockCount);
        mapfile.write_i32(iThrowBlockCount);
        mapfile.write_i32(iOnOffBlockCount);
        mapfile.write_i32(iPlatformCount);
        mapfile.write_i32(self.maphazards.len() as i32);
        mapfile.write_i32(iItemDestroyableBlockCount);
        mapfile.write_i32(iHiddenBlockCount);
        mapfile.write_i32(self.mapitems.len() as i32);
        mapfile.write_i32(iDensity);

        unsafe {
            let iTilesetCount: i16 = g_tilesetmanager.count() as i16;
            let mut fTilesetUsed = vec![false; iTilesetCount.max(0) as usize];

            for j in 0..MH {
                for i in 0..MW {
                    for k in 0..ML {
                        if self.mapdata[i][j][k].iID >= 0 {
                            fTilesetUsed[self.mapdata[i][j][k].iID as usize] = true;
                        }
                    }
                }
            }

            for platform in &self.platforms {
                for iCol in 0..platform.iTileWidth {
                    for iRow in 0..platform.iTileHeight {
                        let tile = platform.tile_at(iCol as usize, iRow as usize);
                        if tile.iID >= 0 {
                            fTilesetUsed[tile.iID as usize] = true;
                        }
                    }
                }
            }

            let iUsedTilesets: i16 = fTilesetUsed.iter().filter(|&&u| u).count() as i16;

            mapfile.write_i32(iUsedTilesets as i32);

            for iTileset in 0..iTilesetCount {
                if fTilesetUsed[iTileset as usize] {
                    mapfile.write_i32(iTileset as i32);
                    let name = g_tilesetmanager.tileset(iTileset as usize).name().to_string();
                    mapfile.write_string_long(&name);
                }
            }

            for j in 0..MH {
                for i in 0..MW {
                    for k in 0..ML {
                        let tile = &mut self.mapdata[i][j][k];

                        if tile.iID >= 0 {
                            let ts = g_tilesetmanager.tileset(tile.iID as usize);
                            if tile.iCol < 0 || tile.iCol >= ts.width() {
                                tile.iCol = 0;
                            }

                            if tile.iRow < 0 || tile.iRow >= ts.height() {
                                tile.iRow = 0;
                            }
                        }

                        mapfile.write_i8(tile.iID as i8);
                        mapfile.write_i8(tile.iCol as i8);
                        mapfile.write_i8(tile.iRow as i8);
                    }

                    mapfile.write_i8(self.objectdata[i][j].iType as i8);
                    mapfile.write_bool(self.objectdata[i][j].fHidden);
                }
            }

            mapfile.write_string_long(&self.szBackgroundFile);

            for iSwitch in 0..4 {
                mapfile.write_i32(self.iSwitches[iSwitch] as i32);
            }

            mapfile.write_i32(self.platforms.len() as i32);

            for platform in &self.platforms {
                mapfile.write_i32(platform.iTileWidth as i32);
                mapfile.write_i32(platform.iTileHeight as i32);

                for iCol in 0..platform.iTileWidth {
                    for iRow in 0..platform.iTileHeight {
                        let mut tile = *platform.tile_at(iCol as usize, iRow as usize);

                        if tile.iID >= 0 {
                            let ts = g_tilesetmanager.tileset(tile.iID as usize);
                            if tile.iCol < 0 || tile.iCol >= ts.width() {
                                tile.iCol = 0;
                            }

                            if tile.iRow < 0 || tile.iRow >= ts.height() {
                                tile.iRow = 0;
                            }
                        }

                        mapfile.write_i8(tile.iID as i8);
                        mapfile.write_i8(tile.iCol as i8);
                        mapfile.write_i8(tile.iRow as i8);

                        mapfile.write_i32(platform.tile_type_at(iCol as usize, iRow as usize).0 as i32);
                    }
                }

                mapfile.write_i32(platform.iDrawLayer as i32);

                let iPathType: i16 = platform.pPath.path_type_id() as i16;
                mapfile.write_i32(iPathType as i32);

                let mut p = *platform;
                let any = p.pPath.as_any();
                if let Some(path) = any.downcast_ref::<StraightPath>() {
                    mapfile.write_float(path.start_pos().x);
                    mapfile.write_float(path.start_pos().y);
                    mapfile.write_float(path.end_pos().x);
                    mapfile.write_float(path.end_pos().y);
                    mapfile.write_float(path.base.speed());
                } else if let Some(path) = any.downcast_ref::<StraightPathContinuous>() {
                    mapfile.write_float(path.start_pos().x);
                    mapfile.write_float(path.start_pos().y);
                    mapfile.write_float(path.angle());
                    mapfile.write_float(path.base.speed());
                } else if let Some(path) = any.downcast_ref::<EllipsePath>() {
                    mapfile.write_float(path.radius().x);
                    mapfile.write_float(path.radius().y);
                    mapfile.write_float(path.center_pos().x);
                    mapfile.write_float(path.center_pos().y);
                    mapfile.write_float(path.start_angle());
                    mapfile.write_float(path.base.speed());
                }
            }
        }

        mapfile.write_i32(self.mapitems.len() as i32);

        for item in &self.mapitems {
            mapfile.write_i32(item.itype as i32);
            mapfile.write_i32(item.ix as i32);
            mapfile.write_i32(item.iy as i32);
        }

        mapfile.write_i32(self.maphazards.len() as i32);

        for hazard in &self.maphazards {
            mapfile.write_i32(hazard.itype as i32);
            mapfile.write_i32(hazard.ix as i32);
            mapfile.write_i32(hazard.iy as i32);

            for iParam in 0..NUMMAPHAZARDPARAMS as usize {
                mapfile.write_i32(hazard.iparam[iParam] as i32);
            }

            for iParam in 0..NUMMAPHAZARDPARAMS as usize {
                mapfile.write_float(hazard.dparam[iParam]);
            }
        }

        mapfile.write_i32(self.eyecandy[0] as i32);
        mapfile.write_i32(self.eyecandy[1] as i32);
        mapfile.write_i32(self.eyecandy[2] as i32);

        mapfile.write_i32(self.musicCategoryID as i32);

        for j in 0..MH {
            for i in 0..MW {
                mapfile.write_i32(self.mapdatatop[i][j].0 as i32);

                mapfile.write_i32(self.warpdata[i][j].direction);
                mapfile.write_i32(self.warpdata[i][j].connection as i32);
                mapfile.write_i32(self.warpdata[i][j].id as i32);

                for iType in 0..NSPAWN {
                    mapfile.write_bool(self.nospawn[iType][i][j]);
                }
            }
        }

        let mut iBlockCount: i16 = 0;
        let mut iSwitchBlockCount: i16 = 0;
        for j in 0..MH {
            for i in 0..MW {
                if self.objectdata[i][j].iType == 1 || self.objectdata[i][j].iType == 15 {
                    iBlockCount += 1;
                }

                if (11..=14).contains(&self.objectdata[i][j].iType) {
                    iSwitchBlockCount += 1;
                }
            }
        }

        mapfile.write_i32(iSwitchBlockCount as i32);
        for j in 0..MH {
            for i in 0..MW {
                if (11..=14).contains(&self.objectdata[i][j].iType) {
                    mapfile.write_i8(i as i8);
                    mapfile.write_i8(j as i8);
                    mapfile.write_i8(self.objectdata[i][j].iSettings[0] as i8);
                }
            }
        }

        mapfile.write_i32(numWarpExits as i32);

        usedtile = [[false; MH]; MW];

        for j in 0..MH {
            for i in 0..MW {
                if self.warpdata[i][j].connection != -1 && !usedtile[i][j] {
                    let mut movex: i32 = 0;
                    let mut movey: i32 = 0;
                    let mut currentx: i32 = i as i32;
                    let mut currenty: i32 = j as i32;

                    if self.warpdata[i][j].direction == 0 || self.warpdata[i][j].direction == 2 {
                        movex = 1;
                    } else {
                        movey = 1;
                    }

                    let mut numblocks: i32 = 0;
                    while currentx < MAPWIDTH && currenty < MAPHEIGHT {
                        let (cx, cy) = (currentx as usize, currenty as usize);
                        if self.warpdata[cx][cy].direction != self.warpdata[i][j].direction
                            || self.warpdata[cx][cy].connection != self.warpdata[i][j].connection
                        {
                            break;
                        }

                        usedtile[cx][cy] = true;

                        currentx += movex;
                        currenty += movey;
                        numblocks += 1;
                    }

                    currentx -= movex;
                    currenty -= movey;

                    mapfile.write_i32(self.warpdata[i][j].direction);
                    mapfile.write_i32(self.warpdata[i][j].connection as i32);
                    mapfile.write_i32(self.warpdata[i][j].id as i32);

                    let (ii, jj) = (i as i32, j as i32);
                    let dir = self.warpdata[i][j].direction;
                    if dir == 0 {
                        mapfile.write_i32((((currentx << 5) + TILESIZE - (ii << 5)) >> 1) + (ii << 5) - HALFPW);
                        mapfile.write_i32((jj << 5) - 1 + PHOFFSET);

                        mapfile.write_i32((((currentx << 5) + TILESIZE - (ii << 5)) >> 1) + (ii << 5) - 16);
                        mapfile.write_i32(jj << 5);
                    } else if dir == 2 {
                        mapfile.write_i32((((currentx << 5) + TILESIZE - (ii << 5)) >> 1) + (ii << 5) - HALFPW);
                        mapfile.write_i32((jj << 5) + 1 + PHOFFSET);

                        mapfile.write_i32((((currentx << 5) + TILESIZE - (ii << 5)) >> 1) + (ii << 5) - 16);
                        mapfile.write_i32(jj << 5);
                    } else if dir == 1 {
                        mapfile.write_i32((ii << 5) + TILESIZE - PW - PWOFFSET);
                        mapfile.write_i32((currenty << 5) + TILESIZE - PH - 1);

                        mapfile.write_i32(ii << 5);
                        mapfile.write_i32((((currenty << 5) + TILESIZE - (jj << 5)) >> 1) + (jj << 5) - 16);
                    } else if dir == 3 {
                        mapfile.write_i32((ii << 5) - 1 + PWOFFSET);
                        mapfile.write_i32((currenty << 5) + TILESIZE - PH - 1);

                        mapfile.write_i32(ii << 5);
                        mapfile.write_i32((((currenty << 5) + TILESIZE - (jj << 5)) >> 1) + (jj << 5) - 16);
                    }

                    mapfile.write_i32(ii);
                    mapfile.write_i32(jj);
                    mapfile.write_i32(numblocks);
                }
            }
        }

        for iType in 0..=5i16 {
            self.calculatespawnareas(iType, false, false);

            if self.numspawnareas[iType as usize] == 0 {
                self.calculatespawnareas(iType, true, false);
            }

            if self.numspawnareas[iType as usize] == 0 {
                self.calculatespawnareas(iType, true, true);
            }
        }

        for i in 0..NSPAWN {
            mapfile.write_i32(self.numspawnareas[i] as i32);

            for m in 0..self.numspawnareas[i] as usize {
                let a = self.spawnareas[i][m];
                mapfile.write_i32(a.left as i32);
                mapfile.write_i32(a.top as i32);
                mapfile.write_i32(a.width as i32);
                mapfile.write_i32(a.height as i32);
                mapfile.write_i32(a.size as i32);
            }
        }

        usedtile = [[false; MH]; MW];

        for j in 0..MH {
            for i in 0..MW {
                if (self.mapdata[i][j][2].iID as i32) <= TILESETNONE && (self.mapdata[i][j][3].iID as i32) <= TILESETNONE {
                    usedtile[i][j] = true;
                }
            }
        }

        self.numdrawareas = 0;
        for j in 0..MAPHEIGHT {
            for i in 0..MAPWIDTH {
                if !usedtile[i as usize][j as usize] {
                    let mut fDownDone = false;
                    let mut fRightDone = false;

                    let mut downsize: i32 = j + 1;
                    let mut rightsize: i32 = i + 1;
                    let mut attempt: i32 = 1;

                    loop {
                        if !fRightDone {
                            for right in i..rightsize {
                                if right >= MAPWIDTH || j + attempt >= MAPHEIGHT || usedtile[right as usize][(j + attempt) as usize] {
                                    fRightDone = true;
                                }
                            }

                            if !fRightDone {
                                downsize += 1;
                            }
                        }

                        if !fDownDone {
                            for down in j..downsize {
                                if i + attempt >= MAPWIDTH || down >= MAPHEIGHT || usedtile[(i + attempt) as usize][down as usize] {
                                    fDownDone = true;
                                }
                            }

                            if !fDownDone {
                                rightsize += 1;
                            }
                        }

                        if fDownDone && fRightDone {
                            if (self.numdrawareas as i32) < MAXDRAWAREAS {
                                let d = &mut self.drawareas[self.numdrawareas as usize];
                                d.x = (i << 5) as i16 as i32;
                                d.y = (j << 5) as i16 as i32;
                                d.w = ((rightsize - i) << 5) as u16 as i32;
                                d.h = ((downsize - j) << 5) as u16 as i32;

                                self.numdrawareas += 1;
                            }

                            for down in j..downsize {
                                for right in i..rightsize {
                                    usedtile[right as usize][down as usize] = true;
                                }
                            }

                            break;
                        }

                        attempt += 1;
                    }
                }
            }
        }

        mapfile.write_i32(self.numdrawareas as i32);

        for m in 0..self.numdrawareas as usize {
            mapfile.write_i32(self.drawareas[m].x);
            mapfile.write_i32(self.drawareas[m].y);
            mapfile.write_i32(self.drawareas[m].w);
            mapfile.write_i32(self.drawareas[m].h);
        }

        mapfile.write_i32(iBlockCount as i32);

        for j in 0..MH {
            for i in 0..MW {
                if self.objectdata[i][j].iType == 1 || self.objectdata[i][j].iType == 15 {
                    mapfile.write_i8(i as i8);
                    mapfile.write_i8(j as i8);

                    mapfile.write_i8(NUM_BLOCK_SETTINGS as i8);
                    for iSetting in 0..NUM_BLOCK_SETTINGS as usize {
                        mapfile.write_i8(self.objectdata[i][j].iSettings[iSetting] as i8);
                    }
                }
            }
        }

        mapfile.write_i32(self.iNumRaceGoals as i32);
        for j in 0..self.iNumRaceGoals as usize {
            mapfile.write_i32(self.racegoallocations[j].x as i32);
            mapfile.write_i32(self.racegoallocations[j].y as i32);
        }

        mapfile.write_i32(self.iNumFlagBases as i32);
        for j in 0..self.iNumFlagBases as usize {
            mapfile.write_i32(self.flagbaselocations[j].x as i32);
            mapfile.write_i32(self.flagbaselocations[j].y as i32);
        }

        drop(mapfile);

        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o774));

        println!("done");
    }

    pub fn create_thumbnail_surface(&mut self, fUseClassicPack: bool) -> *mut SDL_Surface {
        unsafe {
            let sThumbnail = SDL_CreateRGBSurface((*screen).flags, 160, 120, 16, 0, 0, 0, 0);

            let mut path;

            if fUseClassicPack {
                let localSzBackgroundFile = concat("gfx/packs/Classic/backgrounds/", &g_map.szBackgroundFile);
                path = convert_path(&localSzBackgroundFile);

                if !file_exists(&path) {
                    path = convert_path("gfx/packs/Classic/backgrounds/Land_Classic.png");
                }
            } else {
                let localSzBackgroundFile = concat("gfx/packs/backgrounds/", &g_map.szBackgroundFile);
                let pack = gamegraphicspacklist.current_path().to_string_lossy().into_owned();
                path = convert_path_pack(&localSzBackgroundFile, &pack);

                if !file_exists(&path) {
                    path = convert_path_pack("gfx/packs/backgrounds/Land_Classic.png", &pack);
                }
            }

            let cpath = CString::new(path).unwrap();
            let sBackground = IMG_Load(cpath.as_ptr());
            if sBackground.is_null() {
                println!("ERROR: Couldn't load thumbnail background: {}", sdl_error());
                return null_mut();
            }

            let srcRectBackground = SDL_Rect { x: 0, y: 0, w: App::screenWidth, h: App::screenHeight };
            let mut dstRectBackground = SDL_Rect { x: 0, y: 0, w: 160, h: 120 };

            if SDL_UpperBlitScaled(sBackground, &srcRectBackground, sThumbnail, &mut dstRectBackground) < 0 {
                eprint!("SDL_SoftStretch error: {}\n", sdl_error());
                return null_mut();
            }

            SDL_FreeSurface(sBackground);

            self.pre_draw_preview_background(sThumbnail, true);
            self.pre_draw_preview_blocks(sThumbnail, true);
            self.pre_draw_preview_map_items(sThumbnail, true);
            self.draw_thumbnail_hazards(sThumbnail);
            self.draw_thumbnail_platforms(sThumbnail);
            self.pre_draw_preview_foreground(sThumbnail, true);
            self.pre_draw_preview_warps(sThumbnail, true);

            sThumbnail
        }
    }

    pub fn save_thumbnail(&mut self, sFile: &str, fUseClassicPack: bool) {
        let sThumbnail = self.create_thumbnail_surface(fUseClassicPack);

        if sThumbnail.is_null() {
            return;
        }

        unsafe {
            let c = CString::new(sFile).unwrap();
            IMG_SavePNG(sThumbnail, c.as_ptr());

            SDL_FreeSurface(sThumbnail);
        }
    }

    fn calculatespawnareas(&mut self, iType: i16, fUseTempBlocks: bool, fIgnoreDeath: bool) {
        let t = iType as usize;
        let mut usedtile = [[false; MH]; MW];

        let blocks_landing = |objBlock: i16| {
            objBlock != -1 && objBlock != 0 && objBlock != 2 && objBlock != 6 && !(11..=14).contains(&objBlock) && objBlock != 16 && objBlock < 19
        };
        let is_deadly = |ty: TileType| {
            ty == TileType::DeathOnTop || ty == TileType::Death || ty == TileType::SuperDeathTop || ty == TileType::SuperDeath || ty == TileType::PlayerDeath
        };

        for j in 0..MH {
            for i in 0..MW {
                let mut fUsed = false;

                if j >= 13 || j == 0 {
                    fUsed = true;
                }

                if !fUsed && self.nospawn[t][i][j] {
                    fUsed = true;
                }

                if !fUsed && (tile_to_flags(self.mapdatatop[i][j]) as i32 & tile_flag_solid) != 0 {
                    fUsed = true;
                }

                if !fUsed && self.objectdata[i][j].iType != -1 {
                    fUsed = true;
                }

                if (0..=4).contains(&iType) {
                    if !fUsed && j > 0 && (tile_to_flags(self.mapdatatop[i][j - 1]) as i32 & tile_flag_death_on_bottom) != 0 {
                        fUsed = true;
                    }

                    if !fUsed && !fIgnoreDeath {
                        let mut m = j;
                        while m < MH {
                            let ty = self.mapdatatop[i][m];
                            let flags = tile_to_flags(ty) as i32;
                            let objBlock = self.objectdata[i][m].iType;

                            if m == j && (flags & tile_flag_solid_on_top) != 0 {
                                m += 1;
                                continue;
                            }

                            if is_deadly(ty) {
                                fUsed = true;
                                break;
                            }

                            if fUseTempBlocks {
                                if (ty != TileType::NonSolid && ty != TileType::Gap) || objBlock != -1 {
                                    break;
                                }
                            } else if (ty != TileType::NonSolid && ty != TileType::Gap) || blocks_landing(objBlock) {
                                break;
                            }
                            m += 1;
                        }

                        if m == MH {
                            for m in 0..j {
                                let ty = self.mapdatatop[i][m];
                                let objBlock = self.objectdata[i][m].iType;

                                if is_deadly(ty) {
                                    fUsed = true;
                                    break;
                                }

                                if fUseTempBlocks {
                                    if (ty != TileType::NonSolid && ty != TileType::Gap) || objBlock != -1 {
                                        break;
                                    }
                                } else if (ty != TileType::NonSolid && ty != TileType::Gap) || blocks_landing(objBlock) {
                                    break;
                                }
                            }
                        }
                    }
                }

                usedtile[i][j] = fUsed;
            }
        }

        self.numspawnareas[t] = 0;
        for j in 0..MAPHEIGHT as i16 {
            for i in 0..MAPWIDTH as i16 {
                if !usedtile[i as usize][j as usize] {
                    let mut fDownDone = false;
                    let mut fRightDone = false;

                    let mut downsize: i16 = j + 1;
                    let mut rightsize: i16 = i + 1;
                    let mut attempt: i16 = 1;

                    loop {
                        if !fRightDone {
                            for right in i as i32..rightsize as i32 {
                                let ja = j as i32 + attempt as i32;
                                if right >= MAPWIDTH || ja >= MAPHEIGHT || usedtile[right as usize][ja as usize] {
                                    fRightDone = true;
                                }
                            }

                            if !fRightDone {
                                downsize += 1;
                            }
                        }

                        if !fDownDone {
                            for down in j as i32..downsize as i32 {
                                let ia = i as i32 + attempt as i32;
                                if ia >= MAPWIDTH || down >= MAPHEIGHT || usedtile[ia as usize][down as usize] {
                                    fDownDone = true;
                                }
                            }

                            if !fDownDone {
                                rightsize += 1;
                            }
                        }

                        if fDownDone && fRightDone {
                            if (self.numspawnareas[t] as i32) < MAXSPAWNAREAS {
                                let a = &mut self.spawnareas[t][self.numspawnareas[t] as usize];
                                a.left = i;
                                a.top = j;
                                a.width = rightsize - 1 - i;
                                a.height = downsize - 1 - j;
                                a.size = ((rightsize as i32 - i as i32) * (downsize as i32 - j as i32)) as i16;

                                self.numspawnareas[t] += 1;
                            }

                            for down in j..downsize {
                                for right in i..rightsize {
                                    usedtile[right as usize][down as usize] = true;
                                }
                            }

                            break;
                        }

                        attempt += 1;
                    }
                }
            }
        }
    }

    fn animate_tiles(&mut self, iFrame: i16) {
        let f = iFrame as usize;
        if self.iAnimatedVectorIndices[f] == self.iAnimatedVectorIndices[f + 1] {
            return;
        }

        unsafe {
            for iTile in self.iAnimatedVectorIndices[f]..self.iAnimatedVectorIndices[f + 1] {
                let tile = &mut self.animatedtiles[iTile as usize];
                let rDst: *mut SDL_Rect = &mut tile.rDest;
                let frame = self.iTileAnimationFrame as usize;

                if tile.fBackgroundAnimated {
                    blit(self.animatedTilesSurface, &tile.rAnimationSrc[0][frame], self.animatedBackmapSurface, rDst);
                }

                if tile.fForegroundAnimated {
                    blit(self.animatedTilesSurface, &tile.rAnimationSrc[1][frame], self.animatedFrontmapSurface, rDst);
                }

                if !tile.pPlatform.is_null() {
                    blit(
                        self.animatedTilesSurface,
                        &tile.rAnimationSrc[0][frame],
                        tile.pPlatform.sprites[g_iCurrentDrawIndex as usize].get_surface(),
                        rDst,
                    );
                }
            }
        }
    }

    fn draw(&mut self, targetSurface: *mut SDL_Surface, layer: i32) {
        unsafe {
            self.bltrect.x = 0;
            for i in 0..MW {
                self.bltrect.y = -TILESIZE;

                for j in 0..MH {
                    self.bltrect.y += TILESIZE;

                    let tile = self.mapdata[i][j][layer as usize];

                    if tile.iID as i32 == TILESETNONE {
                        continue;
                    }

                    if tile.iID >= 0 {
                        g_tilesetmanager.draw(targetSurface, tile.iID, 0, tile.iCol, tile.iRow, i as i16, j as i16);
                    } else if tile.iID as i32 == TILESETANIMATED {
                        let iNewTileId: i16 = (j * MW + i) as i16;
                        let fNeedNewAnimatedTile = !self.animatedtiles.iter().any(|t| t.id == iNewTileId);

                        if fNeedNewAnimatedTile {
                            let mut animatedtile = Box::new(AnimatedTile::new());
                            animatedtile.id = iNewTileId;

                            animatedtile.fBackgroundAnimated = false;
                            animatedtile.fForegroundAnimated = false;
                            animatedtile.pPlatform = Ptr::null();

                            for iLayer in 0..4usize {
                                let layerTile = self.mapdata[i][j][iLayer];
                                animatedtile.layers[iLayer] = layerTile;

                                if layerTile.iID >= 0 {
                                    animatedtile.rSrc[iLayer][0] =
                                        SDL_Rect { x: (layerTile.iCol as i32) << 5, y: (layerTile.iRow as i32) << 5, w: TILESIZE, h: TILESIZE };
                                } else if layerTile.iID as i32 == TILESETANIMATED {
                                    for iRect in 0..4i32 {
                                        animatedtile.rSrc[iLayer][iRect as usize] = SDL_Rect {
                                            x: (iRect + ((layerTile.iCol as i32) << 2)) << 5,
                                            y: (layerTile.iRow as i32) << 5,
                                            w: TILESIZE,
                                            h: TILESIZE,
                                        };
                                    }

                                    if iLayer < 2 || !game_values.toplayer {
                                        animatedtile.fBackgroundAnimated = true;
                                    }

                                    if iLayer >= 2 && game_values.toplayer {
                                        animatedtile.fForegroundAnimated = true;
                                    }
                                }
                            }

                            animatedtile.rDest = SDL_Rect { x: self.bltrect.x, y: self.bltrect.y, w: TILESIZE, h: TILESIZE };
                            self.animatedtiles.push(animatedtile);
                        }
                    } else if tile.iID as i32 == TILESETUNKNOWN {
                        blit(rm.spr_unknowntile[0].get_surface(), g_tilesetmanager.rect(0, 0, 0), targetSurface, &mut self.bltrect);
                    }
                }

                self.bltrect.x += TILESIZE;
            }

            self.bltrect.x = 0;
            self.bltrect.y = 0;
            self.bltrect.w = App::screenWidth;
            self.bltrect.h = App::screenHeight;
        }
    }

    fn add_platform_animated_tiles(&mut self) {
        for &platform in &self.platforms {
            let iHeight = platform.iTileHeight;
            let iWidth = platform.iTileWidth;

            let mut iDestX: i16 = 0;
            let mut iDestY: i16 = 0;

            for iRow in 0..iHeight {
                for iCol in 0..iWidth {
                    if platform.tile_at(iCol as usize, iRow as usize).iID as i32 == TILESETANIMATED {
                        let mut animatedtile = Box::new(AnimatedTile::new());
                        animatedtile.id = -1;

                        animatedtile.fBackgroundAnimated = false;
                        animatedtile.fForegroundAnimated = false;
                        animatedtile.pPlatform = platform;

                        let tile = *platform.tile_at(iCol as usize, iRow as usize);
                        animatedtile.layers[0] = tile;

                        for iRect in 0..4i32 {
                            animatedtile.rSrc[0][iRect as usize] =
                                SDL_Rect { x: (iRect + ((tile.iCol as i32) << 2)) << 5, y: (tile.iRow as i32) << 5, w: TILESIZE, h: TILESIZE };
                        }

                        animatedtile.rDest = SDL_Rect { x: iDestX as i32, y: iDestY as i32, w: TILESIZE, h: TILESIZE };
                        self.animatedtiles.push(animatedtile);
                    }

                    iDestX = (iDestX as i32 + TILESIZE) as i16;
                }

                iDestY = (iDestY as i32 + TILESIZE) as i16;
                iDestX = 0;
            }
        }
    }

    fn draw_thumbnail_hazards(&mut self, targetSurface: *mut SDL_Surface) {
        unsafe {
            blitdest = targetSurface;

            for hazard in &self.maphazards {
                draw_map_hazard(hazard, 2, false);
            }

            blitdest = screen;
        }
    }

    fn draw_thumbnail_platforms(&mut self, targetSurface: *mut SDL_Surface) {
        unsafe {
            blitdest = targetSurface;

            for &platform in &self.platforms {
                let mut p = platform;
                let w = p.iTileWidth;
                let h = p.iTileHeight;
                let tiles = p.iTileData.clone();
                let basepath = p.pPath.as_any();

                if let Some(path) = basepath.downcast_ref::<StraightPath>() {
                    draw_platform(
                        path.path_type_id(),
                        &tiles,
                        (path.start_pos().x * 2.0) as i16,
                        (path.start_pos().y * 2.0) as i16,
                        (path.end_pos().x * 2.0) as i16,
                        (path.end_pos().y * 2.0) as i16,
                        0.0,
                        0.0,
                        0.0,
                        2,
                        w,
                        h,
                        true,
                        true,
                    );
                } else if let Some(path) = basepath.downcast_ref::<StraightPathContinuous>() {
                    draw_platform(
                        path.path_type_id(),
                        &tiles,
                        (path.start_pos().x * 2.0) as i16,
                        (path.start_pos().y * 2.0) as i16,
                        0,
                        0,
                        path.angle(),
                        0.0,
                        0.0,
                        2,
                        w,
                        h,
                        true,
                        true,
                    );
                } else if let Some(path) = basepath.downcast_ref::<EllipsePath>() {
                    draw_platform(
                        path.path_type_id(),
                        &tiles,
                        (path.center_pos().x * 2.0) as i16,
                        (path.center_pos().y * 2.0) as i16,
                        0,
                        0,
                        path.start_angle(),
                        path.radius().x * 2.0,
                        path.radius().y * 2.0,
                        2,
                        w,
                        h,
                        true,
                        true,
                    );
                }
            }

            blitdest = screen;
        }
    }

    pub fn pre_draw_preview_warps(&mut self, targetSurface: *mut SDL_Surface, fThumbnail: bool) {
        let mut iTileSize: i16 = 16;
        let mut iScreenshotSize: usize = 0;

        if fThumbnail {
            iTileSize = 8;
            iScreenshotSize = 1;
        }

        unsafe {
            for j in 0..MH {
                for i in 0..MW {
                    let wWarp = g_map.warpdata[i][j];

                    if wWarp.connection != -1 {
                        let ts = iTileSize as i32;
                        let rSrc = SDL_Rect { x: wWarp.connection as i32 * ts, y: wWarp.direction * ts, w: ts, h: ts };
                        let mut rDst = SDL_Rect { x: i as i32 * ts, y: j as i32 * ts, w: ts, h: ts };

                        rm.spr_thumbnail_warps[iScreenshotSize].draw_src_to(&rSrc, targetSurface, &rDst);
                    }
                }
            }
        }
    }

    pub fn pre_draw_preview_map_items(&mut self, targetSurface: *mut SDL_Surface, fThumbnail: bool) {
        let mut iTileSize: i32 = 16;
        let mut iScreenshotSize: usize = 0;

        if fThumbnail {
            iTileSize = 8;
            iScreenshotSize = 1;
        }

        unsafe {
            for item in &self.mapitems {
                let rSrc = SDL_Rect { x: item.itype as i32 * iTileSize, y: 0, w: iTileSize, h: iTileSize };
                let mut rDst = SDL_Rect { x: item.ix as i32 * iTileSize, y: item.iy as i32 * iTileSize, w: iTileSize, h: iTileSize };

                rm.spr_thumbnail_mapitems[iScreenshotSize].draw_src_to(&rSrc, targetSurface, &rDst);
            }
        }
    }

    pub fn pre_draw_preview_background(&mut self, targetSurface: *mut SDL_Surface, fThumbnail: bool) {
        self.draw_preview(targetSurface, 0, fThumbnail);
        small_delay();

        self.draw_preview(targetSurface, 1, fThumbnail);
        small_delay();

        if unsafe { !game_values.toplayer } {
            self.draw_preview(targetSurface, 2, fThumbnail);
            small_delay();

            self.draw_preview(targetSurface, 3, fThumbnail);
            small_delay();
        }
    }

    /// `preDrawPreviewBackground(gfxSprite*, SDL_Surface*, bool)`
    pub fn pre_draw_preview_background_spr(&mut self, background: &gfxSprite, targetSurface: *mut SDL_Surface, fThumbnail: bool) {
        let srcrect = SDL_Rect { x: 0, y: 0, w: App::screenWidth, h: App::screenHeight };

        let mut dstrect = SDL_Rect { x: 0, y: 0, w: 0, h: 0 };

        if fThumbnail {
            dstrect.w = 160;
            dstrect.h = 120;
        } else {
            dstrect.w = App::screenWidth / 2;
            dstrect.h = App::screenHeight / 2;
        }

        background.draw_stretch(&srcrect, targetSurface, &dstrect);

        small_delay();
        self.pre_draw_preview_background(targetSurface, fThumbnail);
    }

    pub fn pre_draw_preview_blocks(&mut self, targetSurface: *mut SDL_Surface, fThumbnail: bool) {
        if !fThumbnail {
            unsafe {
                SDL_FillRect(targetSurface, null(), SDL_MapRGB((*targetSurface).format, 255, 0, 255));
                SDL_SetColorKey(targetSurface, SDL_bool::SDL_TRUE as i32, SDL_MapRGB((*targetSurface).format, 255, 0, 255));
            }
            small_delay();
        }

        self.draw_preview_blocks(targetSurface, fThumbnail);
    }

    pub fn pre_draw_preview_foreground(&mut self, targetSurface: *mut SDL_Surface, fThumbnail: bool) {
        if !fThumbnail {
            unsafe {
                SDL_FillRect(targetSurface, null(), SDL_MapRGB((*targetSurface).format, 255, 0, 255));
                SDL_SetColorKey(targetSurface, SDL_bool::SDL_TRUE as i32, SDL_MapRGB((*targetSurface).format, 255, 0, 255));
            }
            small_delay();
        }

        if unsafe { !game_values.toplayer } {
            return;
        }

        self.draw_preview(targetSurface, 2, fThumbnail);
        small_delay();
        self.draw_preview(targetSurface, 3, fThumbnail);
    }

    fn draw_preview(&mut self, targetSurface: *mut SDL_Surface, layer: i32, fThumbnail: bool) {
        let mut iTilesetSize: i16 = 1;

        if fThumbnail {
            iTilesetSize = 2;
        }

        unsafe {
            for i in 0..MW {
                for j in 0..MH {
                    let tile = self.mapdata[i][j][layer as usize];
                    if tile.iID as i32 == TILESETNONE {
                        continue;
                    }

                    if tile.iID >= 0 {
                        g_tilesetmanager.draw(targetSurface, tile.iID, iTilesetSize, tile.iCol, tile.iRow, i as i16, j as i16);
                    } else if tile.iID as i32 == TILESETANIMATED {
                        blit(
                            rm.spr_tileanimation[iTilesetSize as usize].get_surface(),
                            g_tilesetmanager.rect(iTilesetSize, tile.iCol * 4, tile.iRow),
                            targetSurface,
                            g_tilesetmanager.rect(iTilesetSize, i as i16, j as i16),
                        );
                    } else if tile.iID as i32 == TILESETUNKNOWN {
                        blit(
                            rm.spr_unknowntile[iTilesetSize as usize].get_surface(),
                            g_tilesetmanager.rect(iTilesetSize, 0, 0),
                            targetSurface,
                            g_tilesetmanager.rect(iTilesetSize, i as i16, j as i16),
                        );
                    }
                }
            }
        }
    }

    fn draw_preview_blocks(&mut self, targetSurface: *mut SDL_Surface, fThumbnail: bool) {
        let mut iBlockSize: i32 = PREVIEWTILESIZE;

        if fThumbnail {
            iBlockSize = THUMBTILESIZE;
        }

        let mut rectDst = SDL_Rect { x: 0, y: 0, w: iBlockSize, h: iBlockSize };
        let mut rectSrc = SDL_Rect { x: 0, y: 0, w: iBlockSize, h: iBlockSize };

        unsafe {
            rectDst.x = 0;
            for i in 0..MW {
                rectDst.y = -iBlockSize;

                for j in 0..MH {
                    rectDst.y += iBlockSize;

                    let ts = self.objectdata[i][j].iType as i32;
                    if ts == -1 {
                        continue;
                    }

                    if self.objectdata[i][j].fHidden {
                        continue;
                    }

                    rectSrc.x = (ts * iBlockSize) as i16 as i32;
                    rectSrc.y = 0;

                    if (7..=10).contains(&ts) {
                        if self.iSwitches[((ts - 7) % 4) as usize] == 0 {
                            rectSrc.y = iBlockSize;
                        }
                    } else if (11..=14).contains(&ts) {
                        if self.objectdata[i][j].iSettings[0] == 0 {
                            rectSrc.y = iBlockSize;
                        }
                    } else if (15..=19).contains(&ts) {
                        rectSrc.x = iBlockSize * (ts - 15);
                        rectSrc.y = iBlockSize;
                    } else if (20..=29).contains(&ts) {
                        rectSrc.x = iBlockSize * (ts - 20);
                        rectSrc.y = iBlockSize << 1;
                    }

                    if fThumbnail {
                        rm.spr_blocks[2].draw_src_to(&rectSrc, targetSurface, &rectDst);
                    } else {
                        rm.spr_blocks[1].draw_src_to(&rectSrc, targetSurface, &rectDst);
                    }
                }

                rectDst.x += iBlockSize;
            }
        }
    }

    pub fn predrawbackground(&mut self, background: &gfxSprite, mapspr: &gfxSprite) {
        let mut r = SDL_Rect { x: 0, y: 0, w: App::screenWidth, h: App::screenHeight };

        unsafe {
            background.draw_to(mapspr.get_surface(), &r);
        }

        self.draw(mapspr.get_surface(), 0);
        self.draw(mapspr.get_surface(), 1);

        if unsafe { !game_values.toplayer } {
            self.draw(mapspr.get_surface(), 2);
            self.draw(mapspr.get_surface(), 3);
        }

        self.add_platform_animated_tiles();
    }

    pub fn predrawforeground(&mut self, foregroundspr: &gfxSprite) {
        unsafe {
            let s = foregroundspr.get_surface();
            if (*s).flags & SDL_RLEACCEL != 0 {
                SDL_LockSurface(s);
            }

            SDL_FillRect(s, null(), SDL_MapRGB((*s).format, 255, 0, 255));
            SDL_SetColorKey(s, SDL_bool::SDL_TRUE as i32, SDL_MapRGB((*s).format, 255, 0, 255));

            if (*s).flags & SDL_RLEACCEL != 0 {
                SDL_UnlockSurface(s);
            }

            self.draw(s, 2);
            self.draw(s, 3);
        }
    }

    pub fn setup_animated_tiles(&mut self) {
        unsafe {
            self.iAnimatedBackgroundLayers = 2;
            if !game_values.toplayer {
                self.iAnimatedBackgroundLayers = 4;
            }

            g_iCurrentDrawIndex = 0;

            self.iAnimatedTileCount = self.animatedtiles.len() as i16;

            if !self.animatedTilesSurface.is_null() {
                SDL_FreeSurface(self.animatedTilesSurface);
                self.animatedTilesSurface = null_mut();
            }

            if self.iAnimatedTileCount > 0 {
                self.animatedFrontmapSurface = rm.spr_frontmap[g_iCurrentDrawIndex as usize].get_surface();
                self.animatedBackmapSurface = rm.spr_backmap[g_iCurrentDrawIndex as usize].get_surface();
                self.animatedTilesSurface =
                    SDL_CreateRGBSurface((*screen).flags, 1024, 1024, (*(*screen).format).BitsPerPixel as i32, 0, 0, 0, 0);
                let ats = self.animatedTilesSurface;

                let iTransparentColor = SDL_MapRGB((*ats).format, 255, 0, 255);

                let mut iter = 0usize;
                let lim = self.animatedtiles.len();

                let mut fSrcSurfaceFull = false;

                let mut rDst = SDL_Rect { x: 0, y: 0, w: 32, h: 32 };
                while iter != lim && !fSrcSurfaceFull {
                    let tile = &mut *self.animatedtiles[iter];
                    let rSrc: *const SDL_Rect = &tile.rDest;

                    if tile.fBackgroundAnimated {
                        for sTileAnimationFrame in 0..4usize {
                            tile.rAnimationSrc[0][sTileAnimationFrame] = rDst;

                            rm.spr_background.draw_src_to(&*rSrc, ats, &rDst);

                            for iLayer in 0..self.iAnimatedBackgroundLayers as usize {
                                let tilesetTile = tile.layers[iLayer];
                                if tilesetTile.iID >= 0 {
                                    blit(
                                        g_tilesetmanager.tileset(tilesetTile.iID as usize).surface(0),
                                        &tile.rSrc[iLayer][0],
                                        ats,
                                        &mut rDst,
                                    );
                                } else if tilesetTile.iID as i32 == TILESETANIMATED {
                                    rm.spr_tileanimation[0].draw_src_to(&tile.rSrc[iLayer][sTileAnimationFrame], ats, &rDst);
                                } else if tilesetTile.iID as i32 == TILESETUNKNOWN {
                                    blit(rm.spr_unknowntile[0].get_surface(), g_tilesetmanager.rect(0, 0, 0), ats, &mut rDst);
                                }
                            }

                            rDst.x += 32;
                            if rDst.x >= 1024 {
                                rDst.x = 0;
                                rDst.y += 32;
                                if rDst.y >= 1024 {
                                    tile.fForegroundAnimated = false;
                                    fSrcSurfaceFull = true;
                                    break;
                                }
                            }
                        }
                    }

                    if fSrcSurfaceFull {
                        iter += 1;
                        break;
                    }

                    if tile.fForegroundAnimated {
                        for sTileAnimationFrame in 0..4usize {
                            tile.rAnimationSrc[1][sTileAnimationFrame] = rDst;

                            SDL_FillRect(ats, &rDst, iTransparentColor);

                            for iLayer in 2..4usize {
                                let tilesetTile = tile.layers[iLayer];
                                if tilesetTile.iID >= 0 {
                                    blit(
                                        g_tilesetmanager.tileset(tilesetTile.iID as usize).surface(0),
                                        &tile.rSrc[iLayer][0],
                                        ats,
                                        &mut rDst,
                                    );
                                } else if tilesetTile.iID as i32 == TILESETANIMATED {
                                    rm.spr_tileanimation[0].draw_src_to(&tile.rSrc[iLayer][sTileAnimationFrame], ats, &rDst);
                                } else if tilesetTile.iID as i32 == TILESETUNKNOWN {
                                    blit(rm.spr_unknowntile[0].get_surface(), g_tilesetmanager.rect(0, 0, 0), ats, &mut rDst);
                                }
                            }

                            rDst.x += 32;
                            if rDst.x >= 1024 {
                                rDst.x = 0;
                                rDst.y += 32;
                                if rDst.y >= 1024 {
                                    fSrcSurfaceFull = true;
                                    break;
                                }
                            }
                        }
                    }

                    if fSrcSurfaceFull {
                        iter += 1;
                        break;
                    }

                    if !tile.pPlatform.is_null() {
                        for sTileAnimationFrame in 0..4usize {
                            tile.rAnimationSrc[0][sTileAnimationFrame] = rDst;

                            SDL_FillRect(ats, &rDst, iTransparentColor);

                            let tilesetTile = tile.layers[0];
                            if tilesetTile.iID as i32 == TILESETANIMATED {
                                rm.spr_tileanimation[0].draw_src_to(&tile.rSrc[0][sTileAnimationFrame], ats, &rDst);
                            } else {
                                println!();
                                println!(" ERROR: A nonanimated platform tile was added to the animated tile list");
                            }

                            rDst.x += 32;
                            if rDst.x >= 1024 {
                                rDst.x = 0;
                                rDst.y += 32;
                                if rDst.y >= 1024 {
                                    fSrcSurfaceFull = true;
                                    break;
                                }
                            }
                        }
                    }

                    iter += 1;
                }

                while iter != lim {
                    let tile = &mut self.animatedtiles[iter];

                    tile.fBackgroundAnimated = false;
                    tile.fForegroundAnimated = false;
                    tile.pPlatform = Ptr::null();

                    iter += 1;
                }

                for iAnimatedFrame in 0..=NUM_FRAMES_BETWEEN_TILE_ANIMATION {
                    self.iAnimatedVectorIndices[iAnimatedFrame as usize] =
                        ((iAnimatedFrame as i32 * self.iAnimatedTileCount as i32) / NUM_FRAMES_BETWEEN_TILE_ANIMATION as i32) as i16;
                }

                for iFrame in 0..NUM_FRAMES_BETWEEN_TILE_ANIMATION {
                    self.animate_tiles(iFrame);
                }

                self.animatedFrontmapSurface = rm.spr_frontmap[(1 - g_iCurrentDrawIndex) as usize].get_surface();
                self.animatedBackmapSurface = rm.spr_backmap[(1 - g_iCurrentDrawIndex) as usize].get_surface();

                self.animate_tiles(0);
            }
        }
    }

    pub fn update_platforms(&mut self) {
        for &platform in &self.platforms.clone() {
            let mut p = platform;
            p.update();
        }

        let mut iter = 0usize;
        while iter < self.tempPlatforms.len() {
            let mut p = self.tempPlatforms[iter];
            if p.fDead {
                p.delete();
                self.tempPlatforms.remove(iter);
            } else {
                p.update();
                iter += 1;
            }
        }
    }

    pub fn draw_platforms(&mut self, iLayer: i16) {
        for &platform in &self.platformdrawlayer[iLayer as usize].clone() {
            let mut p = platform;
            p.draw();
        }

        if iLayer == 2 {
            for &platform in &self.tempPlatforms.clone() {
                let mut p = platform;
                p.draw();
            }
        }
    }

    /// `drawPlatforms(short iOffsetX, short iOffsetY, short iLayer)`
    pub fn draw_platforms_offset(&mut self, iOffsetX: i16, iOffsetY: i16, iLayer: i16) {
        for &platform in &self.platformdrawlayer[iLayer as usize].clone() {
            let mut p = platform;
            p.draw_offset(iOffsetX, iOffsetY);
        }
    }

    /// `movingPlatformCollision(CPlayer*)`; defined in smw/player.cpp.
    pub fn moving_platform_collision_player(&mut self, player: Ptr<CPlayer>) {
        for i in 0..self.platforms.len() {
            let mut p = self.platforms[i];
            p.collide_player(player);
            if !player.isready() {
                return;
            }
        }

        for i in 0..self.tempPlatforms.len() {
            let mut p = self.tempPlatforms[i];
            p.collide_player(player);
            if !player.isready() {
                return;
            }
        }
    }

    /// `movingPlatformCollision(IO_MovingObject*)`
    pub fn moving_platform_collision_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) {
        for i in 0..self.platforms.len() {
            let mut p = self.platforms[i];
            p.collide_object(object);
        }

        for i in 0..self.tempPlatforms.len() {
            let mut p = self.tempPlatforms[i];
            p.collide_object(object);
        }
    }

    pub fn moving_platform_check_sides(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) -> bool {
        let mut fRet = false;
        for i in 0..self.platforms.len() {
            let mut p = self.platforms[i];
            fRet |= p.collision_detection_check_sides(object);
        }

        for i in 0..self.tempPlatforms.len() {
            let mut p = self.tempPlatforms[i];
            fRet |= p.collision_detection_check_sides(object);
        }

        fRet
    }

    pub fn reset_platforms(&mut self) {
        for &platform in &self.platforms.clone() {
            let mut p = platform;
            p.reset_path();
        }

        for platform in self.tempPlatforms.drain(..) {
            platform.delete();
        }
    }

    pub fn lockconnection(&mut self, connection: i32) {
        if connection == -1 {
            for iConnection in 0..=self.maxConnection {
                self.warplocked[iConnection as usize] = true;
            }
        } else {
            self.warplocked[connection as usize] = true;
        }
    }

    pub fn get_random_warp_exit(&mut self, connection: i32, currentID: i32) -> Ptr<WarpExit> {
        let mut indices = [0i32; MAXWARPS as usize];
        let mut numIndices: i32 = 0;

        let mut currentWarp: Ptr<WarpExit> = Ptr::null();

        for k in 0..self.numwarpexits as usize {
            if self.warpexits[k].connection as i32 == connection {
                if self.warpexits[k].id as i32 == currentID {
                    currentWarp = Ptr::from_mut(&mut self.warpexits[k]);
                } else {
                    indices[numIndices as usize] = k as i32;
                    numIndices += 1;
                }
            }
        }

        if numIndices == 0 {
            return currentWarp;
        }

        Ptr::from_mut(&mut self.warpexits[indices[RANDOM_INT(numIndices) as usize] as usize])
    }

    pub fn clear_warp_locks(&mut self) {
        for iConnection in 0..10 {
            self.warplocktimer[iConnection] = 0;
            self.warplocked[iConnection] = false;
        }

        for iWarpExit in 0..self.numwarpexits as usize {
            self.warpexits[iWarpExit].locktimer = 0;
        }
    }

    pub fn draw_warp_locks(&mut self) {
        unsafe {
            for iWarpExit in 0..self.numwarpexits as usize {
                let we = self.warpexits[iWarpExit];
                if self.warplocked[we.connection as usize] || we.locktimer > 0 {
                    rm.spr_warplock.draw(we.lockx as i32, we.locky as i32);
                }
            }
        }
    }

    pub fn update(&mut self) {
        unsafe {
            for iConnection in 0..=self.maxConnection {
                let c = iConnection as usize;
                if self.warplocked[c] {
                    self.warplocktimer[c] += 1;
                    if self.warplocktimer[c] > game_values.warplocktime {
                        self.warplocked[c] = false;
                        self.warplocktimer[c] = 0;
                    }
                }
            }

            for iWarpExit in 0..self.numwarpexits as usize {
                if self.warpexits[iWarpExit].locktimer > 0 {
                    self.warpexits[iWarpExit].locktimer -= 1;
                }
            }

            self.iTileAnimationTimer += 1;
            if self.iTileAnimationTimer >= NUM_FRAMES_BETWEEN_TILE_ANIMATION {
                self.iTileAnimationTimer = 0;

                self.animatedFrontmapSurface = rm.spr_frontmap[g_iCurrentDrawIndex as usize].get_surface();
                self.animatedBackmapSurface = rm.spr_backmap[g_iCurrentDrawIndex as usize].get_surface();

                g_iCurrentDrawIndex = 1 - g_iCurrentDrawIndex;

                self.iTileAnimationFrame += 1;
                if self.iTileAnimationFrame >= NUM_FRAMES_IN_TILE_ANIMATION {
                    self.iTileAnimationFrame = 0;
                }
            }

            if self.iAnimatedTileCount > 0 {
                self.animate_tiles(self.iTileAnimationTimer);
            }
        }
    }

    pub fn findspawnpoint(&mut self, iType: i16, x: &mut i16, y: &mut i16, width: i16, height: i16, tilealigned: bool) -> bool {
        let t = iType as usize;
        if self.totalspawnsize[t] <= 0 {
            *x = RANDOM_INT(App::screenWidth) as i16;
            *y = RANDOM_INT((App::screenHeight as f32 * 0.87) as i32) as i16;
            return true;
        }

        let spawnarea: i32 = RANDOM_INT(self.totalspawnsize[t] as i32);

        let mut currentsize: i32 = 0;
        for m in 0..self.numspawnareas[t] as usize {
            let a = self.spawnareas[t][m];
            currentsize += a.size as i32;

            if spawnarea >= currentsize {
                continue;
            }

            let areawidth: i16 = (((a.width as i32) << 5) + TILESIZE) as i16;
            let areaheight: i16 = (((a.height as i32) << 5) + TILESIZE) as i16;

            if width > areawidth || height > areaheight {
                continue;
            }

            if tilealigned {
                let mut xoffset: i16 = a.width;
                let mut yoffset: i16 = a.height;

                if xoffset > 0 {
                    xoffset = RANDOM_INT(xoffset as i32) as i16;
                }

                if yoffset > 0 {
                    yoffset = RANDOM_INT(yoffset as i32) as i16;
                }

                *x = (((xoffset as i32) << 5) + ((a.left as i32) << 5) + (TILESIZE >> 1) - ((width as i32) >> 1)) as i16;
                *y = (((yoffset as i32) << 5) + ((a.top as i32) << 5) + (TILESIZE >> 1) - ((height as i32) >> 1)) as i16;
            } else {
                let mut xoffset: i16 = (areawidth as i32 - width as i32 - 2) as i16;
                let mut yoffset: i16 = (areaheight as i32 - height as i32 - 2) as i16;

                if xoffset > 0 {
                    xoffset = (RANDOM_INT(xoffset as i32) as i16 as i32 + 1) as i16;
                }

                if yoffset > 0 {
                    yoffset = (RANDOM_INT(yoffset as i32) as i16 as i32 + 1) as i16;
                }

                *x = (xoffset as i32 + ((a.left as i32) << 5)) as i16;
                *y = (yoffset as i32 + ((a.top as i32) << 5)) as i16;
            }

            break;
        }

        for &platform in &self.platforms {
            if platform.is_in_no_spawn_zone(*x, *y, width, height) {
                return false;
            }
        }

        for &platform in &self.tempPlatforms {
            if platform.is_in_no_spawn_zone(*x, *y, width, height) {
                return false;
            }
        }

        true
    }

    pub fn add_permanent_platform(&mut self, platform: Ptr<MovingPlatform>) {
        self.platforms.push(platform);
        self.platformdrawlayer[platform.iDrawLayer as usize].push(platform);
    }

    pub fn add_temporary_platform(&mut self, platform: Ptr<MovingPlatform>) {
        self.tempPlatforms.push(platform);
    }

    pub fn is_in_platform_no_spawn_zone(&self, x: i16, y: i16, width: i16, height: i16) -> bool {
        for &platform in &self.platforms {
            if platform.is_in_no_spawn_zone(x, y, width, height) {
                return true;
            }
        }

        for &platform in &self.tempPlatforms {
            if platform.is_in_no_spawn_zone(x, y, width, height) {
                return true;
            }
        }

        false
    }

    pub fn drawfrontlayer(&mut self) {
        unsafe {
            for k in 0..self.numdrawareas as usize {
                let d = self.drawareas[k];
                rm.spr_frontmap[g_iCurrentDrawIndex as usize].draw_src(d.x, d.y, &d);
            }
        }
    }

    pub fn checkforwarp(&self, iData1: i16, iData2: i16, iData3: i16, iDirection: i16) -> bool {
        let (warp1, warp2) = if iDirection == 0 || iDirection == 2 {
            (&self.warpdata[iData1 as usize][iData3 as usize], &self.warpdata[iData2 as usize][iData3 as usize])
        } else {
            (&self.warpdata[iData1 as usize][iData2 as usize], &self.warpdata[iData1 as usize][iData3 as usize])
        };

        warp1.direction == warp2.direction
            && warp1.id == warp2.id
            && warp1.direction == iDirection as i32
            && !self.warplocked[warp1.connection as usize]
            && self.warpexits[warp1.id as usize].locktimer <= 0
    }

    pub fn optimize(&mut self) {
        unsafe {
            for j in 0..MH {
                for i in 0..MW {
                    for m in 1..ML {
                        let tile = self.mapdata[i][j][m];
                        let r#type = g_tilesetmanager.tileset(tile.iID as isize as usize).tile_type(tile.iCol as usize, tile.iRow as usize);
                        if r#type != TileType::NonSolid && r#type != TileType::Gap && r#type != TileType::SolidOnTop {
                            let mut k = m as i32 - 1;
                            while k >= 0 {
                                let ku = k as usize;
                                if self.mapdata[i][j][ku].iID as i32 == TILESETNONE {
                                    let fromTile = self.mapdata[i][j][ku + 1];
                                    self.mapdata[i][j][ku] = fromTile;

                                    self.mapdata[i][j][ku + 1].iID = TILESETNONE as i16;
                                    self.mapdata[i][j][ku + 1].iCol = 0;
                                    self.mapdata[i][j][ku + 1].iRow = 0;
                                } else {
                                    break;
                                }
                                k -= 1;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::common::file_list::FiltersList;
    use crate::common::game_values::CGameValues;
    use crate::common::resource_manager::CResourceManager;
    use crate::common::tileset_manager::CTilesetManager;
    use std::fmt::Write as _;
    use std::sync::Once;

    static INIT: Once = Once::new();
    static GLOBALS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Tests that touch C++ globals hold this so they don't run concurrently.
    pub(crate) fn lock_globals() -> std::sync::MutexGuard<'static, ()> {
        GLOBALS.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Brings up the globals `CMap::load_map` touches, against the repository's data/ directory.
    pub(crate) fn init_map_globals() {
        INIT.call_once(|| unsafe {
            RootDataDirectory = concat!(env!("CARGO_MANIFEST_DIR"), "/data").to_string();
            std::env::set_var("SDL_VIDEODRIVER", "dummy");
            SDL_Init(0);
            screen = SDL_CreateRGBSurface(0, 640, 480, 32, 0, 0, 0, 0);
            blitdest = screen;
            filterslist = Ptr::new_box(FiltersList::new());
            if !game_values.is_initialized() {
                game_values.init(CGameValues::new());
            }
            CGameValues::init(&mut game_values);
            rm = Ptr::new_box(CResourceManager::new());
            g_tilesetmanager = Ptr::new_box(CTilesetManager::new(&convert_path("gfx/packs/Classic")));
            g_map = Ptr::from_box(CMap::new());
        });
    }

    /// Same text as tools/ref/map_dump.cpp prints after `---`.
    pub(crate) fn dump(m: &mut CMap, readtype: ReadType) -> String {
        let mut o = String::new();
        let _ = writeln!(
            o,
            "bg={} bgid={} ec={},{},{} music={} races={} flags={}",
            m.szBackgroundFile, m.backgroundID, m.eyecandy[0], m.eyecandy[1], m.eyecandy[2], m.musicCategoryID, m.iNumRaceGoals, m.iNumFlagBases
        );
        o += "filters";
        for f in m.fAutoFilter {
            let _ = write!(o, " {}", f as i32);
        }
        o += "\n";
        if readtype == read_type_summary {
            return o;
        }
        for j in 0..MH {
            for i in 0..MW {
                let _ = write!(o, "t {} {}", i, j);
                for k in 0..ML {
                    let t = m.mapdata[i][j][k];
                    let _ = write!(o, " {}/{}/{}", t.iID, t.iCol, t.iRow);
                }
                let ob = &m.objectdata[i][j];
                let _ = write!(o, " top={} obj={} hid={}", m.mapdatatop[i][j].0, ob.iType, ob.fHidden as i32);
                if ob.iType == 1 || ob.iType == 15 || (11..=14).contains(&ob.iType) {
                    o += " set";
                    for s in 0..NUM_BLOCK_SETTINGS as usize {
                        let _ = write!(o, " {}", ob.iSettings[s]);
                    }
                }
                let w = m.warpdata[i][j];
                let _ = write!(o, " warp={}/{}/{} ns", w.direction, w.connection, w.id);
                for t in 0..NSPAWN {
                    let _ = write!(o, "{}", m.nospawn[t][i][j] as i32);
                }
                o += "\n";
            }
        }
        let _ = writeln!(o, "switches {} {} {} {}", m.iSwitches[0], m.iSwitches[1], m.iSwitches[2], m.iSwitches[3]);
        for it in &m.mapitems {
            let _ = writeln!(o, "item {} {} {}", it.itype, it.ix, it.iy);
        }
        for h in &m.maphazards {
            let _ = write!(o, "hazard {} {} {}", h.itype, h.ix, h.iy);
            for p in h.iparam {
                let _ = write!(o, " {}", p);
            }
            for p in h.dparam {
                let _ = write!(o, " {:.6}", p);
            }
            o += "\n";
        }
        for &platform in &m.platforms {
            let mut p = platform;
            let _ = writeln!(
                o,
                "platform {} {} layer={} path={} x={:.6} y={:.6} vx={:.6} vy={:.6}",
                p.iTileWidth,
                p.iTileHeight,
                p.iDrawLayer,
                p.pPath.path_type_id() as i32,
                p.fx,
                p.fy,
                p.fVelX,
                p.fVelY
            );
            for i in 0..p.iTileData.len() {
                let t = p.iTileData[i];
                let _ = write!(o, " {}/{}/{}:{}", t.iID, t.iCol, t.iRow, p.iTileType[i].0);
            }
            o += "\n";
            for _ in 0..120 {
                p.update();
            }
            let s1 = *p.pPath.current_pos1();
            let _ = writeln!(o, " after120 x={:.6} y={:.6} vx={:.6} vy={:.6} s1={:.6},{:.6}", p.fx, p.fy, p.fVelX, p.fVelY, s1.x, s1.y);
        }
        if readtype == read_type_preview {
            return o;
        }
        let _ = writeln!(o, "warpexits {} maxconn {}", m.numwarpexits, m.maxConnection);
        for w in 0..m.numwarpexits.max(0) as usize {
            let e = m.warpexits[w];
            let _ =
                writeln!(o, "we {} {} {} {} {} {} {} {} {} {}", e.direction, e.connection, e.id, e.x, e.y, e.lockx, e.locky, e.warpx, e.warpy, e.numblocks);
        }
        for t in 0..NSPAWN {
            let _ = write!(o, "spawn {} n={} total={}", t, m.numspawnareas[t], m.totalspawnsize[t]);
            for a in 0..m.numspawnareas[t].max(0) as usize {
                let s = m.spawnareas[t][a];
                let _ = write!(o, " [{} {} {} {} {}]", s.left, s.top, s.width, s.height, s.size);
            }
            o += "\n";
        }
        let _ = write!(o, "drawareas {}", m.numdrawareas);
        for d in 0..m.numdrawareas.max(0) as usize {
            let r = m.drawareas[d];
            let _ = write!(o, " [{} {} {} {}]", r.x, r.y, r.w, r.h);
        }
        o += "\n";
        for r in 0..m.iNumRaceGoals.max(0) as usize {
            let _ = writeln!(o, "race {} {}", m.racegoallocations[r].x, m.racegoallocations[r].y);
        }
        for f in 0..m.iNumFlagBases.max(0) as usize {
            let _ = writeln!(o, "flagbase {} {}", m.flagbaselocations[f].x, m.flagbaselocations[f].y);
        }
        m.update_all_tile_gaps();
        o += "gaps";
        for j in 0..MH {
            for i in 0..MW {
                if m.mapdatatop[i][j] == TileType::Gap {
                    let _ = write!(o, " {},{}", i, j);
                }
            }
        }
        o += "\n";
        o
    }

    fn all_map_files() -> Vec<String> {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
        let mut out = Vec::new();
        let mut stack = vec![std::path::PathBuf::from(root)];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "map") {
                    out.push(p.to_string_lossy().into_owned());
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn loads_every_map_without_panicking() {
        let _g = lock_globals();
        init_map_globals();
        let maps = all_map_files();
        assert!(maps.len() > 400, "found {} maps", maps.len());
        unsafe {
            for f in &maps {
                for rt in [read_type_summary, read_type_preview, read_type_full] {
                    g_map.load_map(f, rt);
                    let _ = dump(&mut g_map, rt);
                }
            }
        }
    }

    /// Same text as tools/ref/map_list_dump.cpp prints after `---`.
    fn map_list_dump() -> String {
        use crate::common::global::maplist;
        use crate::common::map_list::MapList;
        use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType};
        unsafe {
            let root = RootDataDirectory.clone();
            let rel = |p: &str| if p.len() > root.len() { p[root.len()..].to_string() } else { p.to_string() };
            maplist = Ptr::new_box(MapList::new(false));
            maplist.read_filters();

            let mut o = String::new();
            let cur = |o: &mut String, what: &str| {
                let _ = writeln!(o, "{} cur={} file={}", what, maplist.current_shortmapname(), rel(maplist.current_filename()));
            };
            let _ = writeln!(o, "count={} filtered={}", maplist.count(), maplist.filtered_count());
            for i in 0..maplist.maps.len() {
                let n = maplist.maps.node(i);
                let _ = write!(o, "map {} {} idx={} f=", maplist.maps.key(i), rel(&n.filename), n.iIndex);
                for b in &n.pfFilters {
                    let _ = write!(o, "{}", *b as i32);
                }
                o += "\n";
            }
            for i in 0..maplist.worldmaps.len() {
                let _ = writeln!(o, "world {} {}", maplist.worldmaps.key(i), rel(&maplist.worldmaps.node(i).filename));
            }
            for (i, v) in game_values.piFilterIcons.iter().enumerate() {
                let _ = writeln!(o, "icon {} {}", i, v);
            }

            RandomNumberGenerator::generator().reseed(42);
            for _ in 0..3 {
                maplist.next(false);
                cur(&mut o, "next");
            }
            maplist.prev(false);
            cur(&mut o, "prev");
            for _ in 0..5 {
                maplist.random(false);
                cur(&mut o, "random");
            }
            let _ = writeln!(o, "findexact {}", maplist.findexact("death valley", false) as i32);
            cur(&mut o, "after findexact");
            let _ = writeln!(o, "findexact {}", maplist.findexact("no such map", false) as i32);
            cur(&mut o, "after miss");
            let _ = writeln!(o, "findworld {}", maplist.findexact("special_bonushouse", true) as i32);
            cur(&mut o, "after findworld");
            let _ = writeln!(o, "find {}", maplist.find("matsy") as i32);
            cur(&mut o, "after find");
            let _ = writeln!(o, "startswith {}", maplist.startswith_char(b'm') as i32);
            cur(&mut o, "after startswith m");
            let _ = writeln!(o, "startswithstr {}", maplist.startswith("bo") as i32);
            cur(&mut o, "after startswith bo");
            let nfilters = NUM_AUTO_FILTERS as usize + filterslist.count();
            for f in 0..NUM_AUTO_FILTERS as usize {
                let mut filters = vec![false; nfilters];
                filters[f] = true;
                maplist.apply_filters(&filters);
                let _ = write!(o, "filter {} filtered={} on={}", f, maplist.filtered_count(), game_values.fFiltersOn as i32);
                cur(&mut o, "");
                for _ in 0..4 {
                    maplist.next(true);
                    cur(&mut o, " next");
                }
                maplist.prev(true);
                cur(&mut o, " prev");
                maplist.random(true);
                cur(&mut o, " random");
                let it = maplist.get_iterator_at(1, true);
                let _ = writeln!(o, " at1 {}", if it == maplist.end() { "end".to_string() } else { maplist.key_at(it).to_string() });
            }
            maplist.apply_filters(&vec![false; nfilters]);
            for _ in 0..5 {
                let _ = writeln!(o, "randomFilename {}", rel(&maplist.random_filename()));
            }
            o
        }
    }

    /// Compares MapList against the original C++ (`tools/ref/map_list_dump.sh`); set `SMW_MAP_LIST_DUMP`.
    #[test]
    fn matches_cpp_map_list_dump() {
        let _g = lock_globals();
        let Ok(bin) = std::env::var("SMW_MAP_LIST_DUMP") else {
            eprintln!("SMW_MAP_LIST_DUMP not set; skipping C++ comparison");
            return;
        };
        init_map_globals();
        let data = unsafe { RootDataDirectory.clone() };
        let out = std::process::Command::new(&bin).env("SDL_VIDEODRIVER", "dummy").arg(&data).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let expected = text.split("\n---\n").nth(1).unwrap_or("").to_string();
        assert!(expected.lines().count() > 300, "C++ dump produced no output");
        let actual = map_list_dump();
        if let Some(l) = actual.lines().zip(expected.lines()).position(|(a, b)| a != b) {
            panic!("line {}:\n  rust: {}\n  c++:  {}", l, actual.lines().nth(l).unwrap(), expected.lines().nth(l).unwrap());
        }
        assert_eq!(actual.lines().count(), expected.lines().count());
    }

    /// Compares against the original C++ (`tools/ref/map_dump.sh` builds it). Skipped when the
    /// binary is absent; set `SMW_MAP_DUMP` to its path.
    #[test]
    fn matches_cpp_map_dump() {
        let _g = lock_globals();
        let Ok(bin) = std::env::var("SMW_MAP_DUMP") else {
            eprintln!("SMW_MAP_DUMP not set; skipping C++ comparison");
            return;
        };
        init_map_globals();
        let data = concat!(env!("CARGO_MANIFEST_DIR"), "/data");
        let mut mismatches = Vec::new();
        unsafe {
            for f in all_map_files() {
                for rt in [read_type_summary, read_type_preview, read_type_full] {
                    let out = std::process::Command::new(&bin).env("SDL_VIDEODRIVER", "dummy").args([data, &f, &rt.to_string()]).output().unwrap();
                    let text = String::from_utf8_lossy(&out.stdout);
                    let expected = text.split("\n---\n").nth(1).unwrap_or("").to_string();

                    g_map = Ptr::from_box(CMap::new());
                    g_map.load_map(&f, rt);
                    let actual = dump(&mut g_map, rt);
                    if actual != expected {
                        let line = actual.lines().zip(expected.lines()).position(|(a, b)| a != b);
                        let show = line.map(|l| format!("\n  rust: {}\n  c++:  {}", actual.lines().nth(l).unwrap_or(""), expected.lines().nth(l).unwrap_or("")));
                        mismatches.push(format!("{} rt={} first diff line {:?}{}", f, rt, line, show.unwrap_or_default()));
                    }
                }
            }
        }
        assert!(mismatches.is_empty(), "{} mismatches:\n{}", mismatches.len(), mismatches.join("\n"));
    }
}
