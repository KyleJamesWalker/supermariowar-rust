//! Port of src/smw/ui/MI_WorldPreviewDisplay.cpp

use crate::common::global_constants::PREVIEWTILESIZE;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use crate::smw::world::{g_worldmap, WorldMap};
use crate::common::gfx::gfx_sprite::gfxSprite;
use sdl2::sys::{SDL_Rect, SDL_Surface};

const PTS: i16 = PREVIEWTILESIZE as i16;

pub struct MI_WorldPreviewDisplay {
    pub ui_control: UI_Control,

    map_sprite: gfxSprite,

    iCols: i16,
    iRows: i16,

    iMapOffsetX: i16,
    iMapOffsetY: i16,
    iMapGlobalOffsetX: i16,
    iMapGlobalOffsetY: i16,
    iMapDrawOffsetCol: i16,
    iMapDrawOffsetRow: i16,

    iMoveDirection: i16,

    iAnimationTimer: i16,
    iAnimationFrame: i16,

    iScrollCols: i16,
    iScrollRows: i16,

    rectSrcSurface: SDL_Rect,
    rectDstSurface: SDL_Rect,

    iScrollSpeed: i16,
    iScrollSpeedTimer: i16,
}
crate::impl_base!(MI_WorldPreviewDisplay => ui_control: UI_Control);

const ZERO_RECT: SDL_Rect = SDL_Rect { x: 0, y: 0, w: 0, h: 0 };

impl MI_WorldPreviewDisplay {
    pub fn new(x: i16, y: i16, cols: i16, rows: i16) -> Self {
        let mut this = MI_WorldPreviewDisplay {
            ui_control: UI_Control::new(x, y),
            map_sprite: gfxSprite::blank(384, 304),
            iCols: cols,
            iRows: rows,
            iMapOffsetX: 0,
            iMapOffsetY: 0,
            iMapGlobalOffsetX: 0,
            iMapGlobalOffsetY: 0,
            iMapDrawOffsetCol: 0,
            iMapDrawOffsetRow: 0,
            iMoveDirection: 0,
            iAnimationTimer: 0,
            iAnimationFrame: 0,
            iScrollCols: 0,
            iScrollRows: 0,
            rectSrcSurface: ZERO_RECT,
            rectDstSurface: ZERO_RECT,
            iScrollSpeed: 0,
            iScrollSpeedTimer: 0,
        };
        this.init();
        this
    }

    fn init(&mut self) {
        self.iMapOffsetX = 0;
        self.iMapOffsetY = 0;

        self.iMapGlobalOffsetX = 0;
        self.iMapGlobalOffsetY = 0;

        self.iMapDrawOffsetCol = 0;
        self.iMapDrawOffsetRow = 0;

        self.iMoveDirection = if self.iScrollCols > 0 { 0 } else if self.iScrollRows > 0 { 1 } else { -1 };

        self.iAnimationTimer = 0;
        self.iAnimationFrame = 0;

        let (iWidth, iHeight): (i16, i16) = unsafe { (g_worldmap.iWidth, g_worldmap.iHeight) };

        if iWidth > 20 {
            self.rectSrcSurface.w = 320;
        } else {
            self.rectSrcSurface.w = iWidth as i32 * PREVIEWTILESIZE;
        }

        if iHeight > 15 {
            self.rectSrcSurface.h = 240;
        } else {
            self.rectSrcSurface.h = iHeight as i32 * PREVIEWTILESIZE;
        }

        self.rectDstSurface.w = 320;
        self.rectDstSurface.h = 240;
        self.rectDstSurface.x = self.m_pos.x as i32;
        self.rectDstSurface.y = self.m_pos.y as i32;

        if iWidth < 20 {
            self.rectDstSurface.x += (20 - iWidth as i32) * 8;
        }

        if iHeight < 15 {
            self.rectDstSurface.y += (15 - iHeight as i32) * 8;
        }

        let iNumScrollTiles: i16 = (if iWidth > 20 { iWidth - 20 } else { 0 }) + (if iHeight > 15 { iHeight - 15 } else { 0 });

        self.iScrollSpeed = 0;
        self.iScrollSpeedTimer = 0;
        if iNumScrollTiles < 9 {
            self.iScrollSpeed = (12 - iNumScrollTiles) >> 2;
        }
    }

    pub fn set_world(&mut self) {
        let (mut w, mut h): (i16, i16) = (0, 0);
        unsafe {
            g_worldmap.init(WorldMap::new_path(&worldlist.at(game_values.worldindex as usize).to_string_lossy(), PREVIEWTILESIZE as i16));
            g_worldmap.get_world_size(&mut w, &mut h);
        }

        self.iScrollCols = if w > 20 { w - 20 } else { 0 };
        self.iScrollRows = if h > 15 { h - 15 } else { 0 };

        self.init();

        self.update_map_surface(true);
    }

    fn update_map_surface(&mut self, fFullRefresh: bool) {
        unsafe {
            g_worldmap.draw_map_to_surface(-1, fFullRefresh, self.map_sprite.get_surface(), self.iMapDrawOffsetCol, self.iMapDrawOffsetRow, self.iAnimationFrame);
        }

        unsafe {
            let olddest = blitdest;
            blitdest = self.map_sprite.get_surface();
            g_worldmap.draw(self.iMapGlobalOffsetX, self.iMapGlobalOffsetY, false, false);
            blitdest = olddest;
        }
    }
}

impl UI_ControlTrait for MI_WorldPreviewDisplay {
    crate::impl_ctl!();

    fn update(&mut self) {
        if !self.m_visible {
            return;
        }

        let mut fNeedMapSurfaceUpdate = false;
        let mut fNeedFullRefresh = false;

        self.iAnimationTimer += 1;
        if self.iAnimationTimer > 15 {
            self.iAnimationTimer = 0;
            self.iAnimationFrame += PTS;

            if self.iAnimationFrame >= 64 {
                self.iAnimationFrame = 0;
            }

            fNeedMapSurfaceUpdate = true;
        }

        self.iScrollSpeedTimer += 1;
        if self.iScrollSpeedTimer > self.iScrollSpeed {
            self.iScrollSpeedTimer = 0;

            if self.iMoveDirection == 0 {
                self.iMapGlobalOffsetX -= 1;
                self.iMapOffsetX += 1;
                if self.iMapOffsetX >= PTS {
                    self.iMapOffsetX = 0;

                    fNeedMapSurfaceUpdate = true;
                    fNeedFullRefresh = true;

                    self.iMapDrawOffsetCol += 1;
                    if self.iMapDrawOffsetCol >= self.iScrollCols {
                        if self.iScrollRows > 0 {
                            self.iMoveDirection = 1;
                            self.iMapOffsetY = 0;
                        } else {
                            self.iMoveDirection = 2;
                            self.iMapOffsetX = PTS;
                            self.iMapDrawOffsetCol -= 1;
                            self.iMapGlobalOffsetX += PTS;
                        }
                    }
                }
            } else if self.iMoveDirection == 1 {
                self.iMapGlobalOffsetY -= 1;
                self.iMapOffsetY += 1;
                if self.iMapOffsetY >= PTS {
                    self.iMapOffsetY = 0;

                    fNeedMapSurfaceUpdate = true;
                    fNeedFullRefresh = true;

                    self.iMapDrawOffsetRow += 1;
                    if self.iMapDrawOffsetRow >= self.iScrollRows {
                        if self.iScrollCols > 0 {
                            self.iMoveDirection = 2;
                            self.iMapOffsetX = PTS;
                            self.iMapDrawOffsetCol -= 1;
                            self.iMapGlobalOffsetX += PTS;
                        } else {
                            self.iMoveDirection = 3;
                            self.iMapOffsetY = PTS;
                            self.iMapDrawOffsetRow -= 1;
                            self.iMapGlobalOffsetY += PTS;
                        }
                    }
                }
            } else if self.iMoveDirection == 2 {
                //scroll left
                self.iMapGlobalOffsetX += 1;
                self.iMapOffsetX -= 1;
                if self.iMapOffsetX <= 0 {
                    fNeedMapSurfaceUpdate = true;
                    fNeedFullRefresh = true;

                    self.iMapDrawOffsetCol -= 1;
                    if self.iMapDrawOffsetCol < 0 {
                        self.iMapDrawOffsetCol = 0;

                        if self.iScrollRows > 0 {
                            self.iMoveDirection = 3;
                            self.iMapOffsetY = PTS;
                            self.iMapDrawOffsetRow -= 1;
                            self.iMapGlobalOffsetY += PTS;
                            self.iMapGlobalOffsetX -= PTS;
                        } else {
                            self.iMoveDirection = 0;
                            self.iMapOffsetX = 0;
                            self.iMapGlobalOffsetX -= PTS;
                        }
                    } else {
                        self.iMapOffsetX = PTS;
                    }
                }
            } else if self.iMoveDirection == 3 {
                //scroll up
                self.iMapGlobalOffsetY += 1;
                self.iMapOffsetY -= 1;
                if self.iMapOffsetY <= 0 {
                    fNeedMapSurfaceUpdate = true;
                    fNeedFullRefresh = true;

                    self.iMapDrawOffsetRow -= 1;
                    if self.iMapDrawOffsetRow < 0 {
                        self.iMapDrawOffsetRow = 0;
                        self.iMoveDirection = if self.iScrollCols > 0 { 0 } else { 1 };

                        if self.iScrollCols > 0 {
                            self.iMoveDirection = 0;
                            self.iMapOffsetX = 0;
                            self.iMapGlobalOffsetY -= PTS;
                        } else {
                            self.iMoveDirection = 1;
                            self.iMapOffsetY = 0;
                            self.iMapGlobalOffsetY -= PTS;
                        }
                    } else {
                        self.iMapOffsetY = PTS;
                    }
                }
            }
        }

        if fNeedMapSurfaceUpdate {
            self.update_map_surface(fNeedFullRefresh);
        }
    }

    fn draw(&mut self) {
        if !self.m_visible {
            return;
        }

        self.rectSrcSurface.x = self.iMapOffsetX as i32;
        self.rectSrcSurface.y = self.iMapOffsetY as i32;

        self.map_sprite.draw_src_to(&self.rectSrcSurface, unsafe { blitdest }, &self.rectDstSurface);
    }
}
