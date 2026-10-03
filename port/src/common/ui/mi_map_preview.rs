//! Port of src/common/ui/MI_MapPreview.cpp

use crate::common::game::App;
use crate::common::map::read_type_preview;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use sdl2::sys::{SDL_CreateRGBSurface, SDL_Delay, SDL_Rect, SDL_Surface, SDL_UpperBlit};

fn small_delay() {
    unsafe { SDL_Delay(10) };
}

/// `strncpy(dst, src, 255); dst[255] = 0;`
pub(crate) fn copy_map_name(src: &str) -> String {
    let bytes = src.as_bytes();
    let n = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len()).min(255);
    String::from_utf8_lossy(&bytes[..n]).into_owned()
}

pub struct MI_MapPreview {
    pub ui_control: UI_Control,

    pub spr: Ptr<gfxSprite>,

    pub surfaceMapBackground: *mut SDL_Surface,
    pub surfaceMapBlockLayer: *mut SDL_Surface,
    pub surfaceMapForeground: *mut SDL_Surface,
    pub rectDst: SDL_Rect,

    pub iWidth: i16,
    pub iIndent: i16,
    pub szMapName: String,

    pub iSlideListOut: i16,
}
crate::impl_base!(MI_MapPreview => ui_control: UI_Control);

impl MI_MapPreview {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, width: i16, indent: i16) -> Self {
        let (surfaceMapBackground, surfaceMapBlockLayer, surfaceMapForeground) = unsafe {
            (
                SDL_CreateRGBSurface(0, App::screenWidth / 2, App::screenHeight / 2, 16, 0, 0, 0, 0),
                SDL_CreateRGBSurface(0, App::screenWidth / 2, App::screenHeight / 2, 16, 0, 0, 0, 0),
                SDL_CreateRGBSurface(0, App::screenWidth / 2, App::screenHeight / 2, 16, 0, 0, 0, 0),
            )
        };

        let mut this = MI_MapPreview {
            ui_control: UI_Control::new(x, y),
            spr: nspr,
            surfaceMapBackground,
            surfaceMapBlockLayer,
            surfaceMapForeground,
            rectDst: SDL_Rect { x: 0, y: 0, w: 0, h: 0 },
            iWidth: width,
            iIndent: indent,
            szMapName: String::new(),
            iSlideListOut: 0,
        };
        this.load_current_map();

        this.rectDst.x = x as i32 + 16;
        this.rectDst.y = y as i32 + 44;
        this.rectDst.w = App::screenWidth / 2;
        this.rectDst.h = App::screenHeight / 2;

        // C++ clears the name after LoadCurrentMap() filled it
        this.szMapName.clear();
        this
    }

    pub fn update_impl(&mut self) {
        //Update hazards
        // TODO(gameplay): noncolcontainer.update();

        // TODO(gameplay): objectcontainer[1].update();
        // TODO(gameplay): objectcontainer[1].cleanDeadObjects();

        unsafe { g_map.update_platforms() };
    }

    pub fn draw_impl(&mut self) {
        if !self.m_visible {
            return;
        }

        let iMapBoxX: i16 = (self.m_pos.x as i32 + (self.iWidth as i32 >> 1) - 176 - self.iSlideListOut as i32) as i16;
        let bx = iMapBoxX as i32;
        let y = self.m_pos.y as i32;

        unsafe {
            //Draw the background for the map preview
            rm.menu_dialog.draw_src(bx, y + 30, &SDL_Rect { x: 0, y: 0, w: 336, h: 254 });
            rm.menu_dialog.draw_src(bx + 336, y + 30, &SDL_Rect { x: 496, y: 0, w: 16, h: 254 });
            rm.menu_dialog.draw_src(bx, y + 284, &SDL_Rect { x: 0, y: 464, w: 336, h: 16 });
            rm.menu_dialog.draw_src(bx + 336, y + 284, &SDL_Rect { x: 496, y: 464, w: 16, h: 16 });

            self.rectDst.x = bx + 16;

            SDL_UpperBlit(self.surfaceMapBackground, std::ptr::null(), blitdest, &mut self.rectDst);

            g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 0);

            SDL_UpperBlit(self.surfaceMapBlockLayer, std::ptr::null(), blitdest, &mut self.rectDst);

            g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 1);

            //Draw map hazards
            // TODO(objects): draw OMO_OrbitHazard/OMO_StraightPathHazard/IO_FlameCannon/MO_BulletBill/MO_PirhanaPlant in objectcontainer[1] at rectDst

            g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 2);

            if game_values.toplayer {
                SDL_UpperBlit(self.surfaceMapForeground, std::ptr::null(), blitdest, &mut self.rectDst);
            }

            g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 3);
            g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 4);
        }
    }

    pub fn load_current_map(&mut self) {
        let szMapPath = unsafe {
            self.szMapName = copy_map_name(maplist.current_shortmapname());
            maplist.current_filename().to_string()
        };

        self.load_map(&szMapPath);
    }

    pub fn load_map(&mut self, szMapPath: &str) {
        unsafe {
            g_map.load_map(szMapPath, read_type_preview);
            small_delay(); //Sleeps to help the music from skipping

            // TODO(global): LoadCurrentMapBackground();
            small_delay();

            g_map.pre_draw_preview_background_spr(&rm.spr_background, self.surfaceMapBackground, false);
            small_delay();

            g_map.pre_draw_preview_blocks(self.surfaceMapBlockLayer, false);
            small_delay();

            g_map.pre_draw_preview_map_items(self.surfaceMapBackground, false);
            small_delay();

            g_map.pre_draw_preview_foreground(self.surfaceMapForeground, false);
            small_delay();

            g_map.pre_draw_preview_warps(if game_values.toplayer { self.surfaceMapForeground } else { self.surfaceMapBackground }, false);
            small_delay();

            // TODO(objecthazard): LoadMapHazards(true);
        }
    }

    pub fn set_map(&mut self, paramSzMapName: &str, fWorld: bool) -> bool {
        let fFound = unsafe { maplist.findexact(paramSzMapName, fWorld) };
        self.load_current_map();

        fFound
    }

    pub fn set_special_map(&mut self, mapName: &str, szMapPath: &str) {
        self.szMapName = copy_map_name(mapName);

        self.load_map(szMapPath);
    }

    pub fn get_map_name(&self) -> &str {
        &self.szMapName
    }

    pub fn get_map_file_path(&self) -> String {
        unsafe { maplist.current_filename().to_string() }
    }

    pub fn set_dimensions(&mut self, width: i16, indent: i16) {
        self.iWidth = width;
        self.iIndent = indent;
    }
}

impl UI_ControlTrait for MI_MapPreview {
    crate::impl_ctl!();

    fn update(&mut self) {
        self.update_impl();
    }

    fn draw(&mut self) {
        self.draw_impl();
    }
}
