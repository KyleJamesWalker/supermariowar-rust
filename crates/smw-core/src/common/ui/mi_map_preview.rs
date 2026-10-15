//! Port of src/common/ui/MI_MapPreview.cpp

use crate::game::Game;
use crate::common::game::App;
use crate::common::map::read_type_preview;
use crate::common::gfx::gfx_sprite::gfxSprite;
use crate::common::uicontrol::{UI_Control, UI_ControlTrait};
use crate::globals::*;
use crate::common::global::load_current_map_background;

use crate::smw::objecthazard::load_map_hazards;
use crate::smw::objects::io_flame_cannon::IO_FlameCannon;
use crate::smw::objects::moving::mo_bullet_bill::MO_BulletBill;
use crate::smw::objects::moving::mo_pirhana_plant::MO_PirhanaPlant;
use crate::smw::objects::overmap::over_map_object::io_over_map_object_draw_offset;
use crate::smw::objects::overmap::wo_orbit_hazard::OMO_OrbitHazard;
use crate::smw::objects::overmap::wo_straight_path_hazard::OMO_StraightPathHazard;
use sdl2::sys::SDL_Rect;

#[cfg(not(target_os = "emscripten"))]
fn small_delay() {
    crate::services::delay(10);
}
#[cfg(target_os = "emscripten")]
fn small_delay() {}

/// `strncpy(dst, src, 255); dst[255] = 0;`
pub(crate) fn copy_map_name(src: &str) -> String {
    let bytes = src.as_bytes();
    let n = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len()).min(255);
    String::from_utf8_lossy(&bytes[..n]).into_owned()
}

pub struct MI_MapPreview {
    pub ui_control: UI_Control,

    pub spr: Ptr<gfxSprite>,

    pub surfaceMapBackground: gfxSprite,
    pub surfaceMapBlockLayer: gfxSprite,
    pub surfaceMapForeground: gfxSprite,
    pub rectDst: SDL_Rect,

    pub iWidth: i16,
    pub iIndent: i16,
    pub szMapName: String,

    pub iSlideListOut: i16,
}
crate::impl_base!(MI_MapPreview => ui_control: UI_Control);

impl MI_MapPreview {
    pub fn new(nspr: Ptr<gfxSprite>, x: i16, y: i16, width: i16, indent: i16) -> Self {
        let surfaceMapBackground = gfxSprite::blank(game().get(), (App::screenWidth / 2) as u32, (App::screenHeight / 2) as u32);
        let surfaceMapBlockLayer = gfxSprite::blank(game().get(), (App::screenWidth / 2) as u32, (App::screenHeight / 2) as u32);
        let surfaceMapForeground = gfxSprite::blank(game().get(), (App::screenWidth / 2) as u32, (App::screenHeight / 2) as u32);

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
        this.load_current_map(game().get());

        this.rectDst.x = x as i32 + 16;
        this.rectDst.y = y as i32 + 44;
        this.rectDst.w = App::screenWidth / 2;
        this.rectDst.h = App::screenHeight / 2;

        // C++ clears the name after LoadCurrentMap() filled it
        this.szMapName.clear();
        this
    }

    pub fn update_impl(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Update hazards
        unsafe {
            gp.noncolcontainer.update(g);

            gp.objectcontainer[1].update(g);
            g.objectcontainer[1].clean_dead_objects();

            g.g_map.update_platforms();
        }
    }

    pub fn draw_impl(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if !self.m_visible {
            return;
        }

        let iMapBoxX: i16 = (self.m_pos.x as i32 + (self.iWidth as i32 >> 1) - 176 - self.iSlideListOut as i32) as i16;
        let bx = iMapBoxX as i32;
        let y = self.m_pos.y as i32;

        unsafe {
            //Draw the background for the map preview
            gp.rm.menu_dialog.draw_src(g, bx, y + 30, &SDL_Rect { x: 0, y: 0, w: 336, h: 254 });
            gp.rm.menu_dialog.draw_src(g, bx + 336, y + 30, &SDL_Rect { x: 496, y: 0, w: 16, h: 254 });
            gp.rm.menu_dialog.draw_src(g, bx, y + 284, &SDL_Rect { x: 0, y: 464, w: 336, h: 16 });
            gp.rm.menu_dialog.draw_src(g, bx + 336, y + 284, &SDL_Rect { x: 496, y: 464, w: 16, h: 16 });

            self.rectDst.x = bx + 16;

            self.surfaceMapBackground.draw_to(g.blitdest, &self.rectDst);

            g.g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 0);

            self.surfaceMapBlockLayer.draw_to(g.blitdest, &self.rectDst);

            g.g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 1);

            //Draw map hazards
            let (ox, oy) = (self.rectDst.x as i16, self.rectDst.y as i16);
            for i in 0..game().objectcontainer[1].list().len() {
                let mut obj = g.objectcontainer[1].list()[i];
                let any = obj.as_any();
                if let Some(hazard) = any.downcast_mut::<OMO_OrbitHazard>() {
                    io_over_map_object_draw_offset(g, hazard, ox, oy);
                } else if let Some(hazard) = any.downcast_mut::<OMO_StraightPathHazard>() {
                    io_over_map_object_draw_offset(g, hazard, ox, oy);
                } else if let Some(hazard) = any.downcast_mut::<IO_FlameCannon>() {
                    hazard.draw_offset(g, ox, oy);
                } else if let Some(hazard) = any.downcast_mut::<MO_BulletBill>() {
                    hazard.draw_offset(g, ox, oy);
                } else if let Some(hazard) = any.downcast_mut::<MO_PirhanaPlant>() {
                    hazard.draw_offset(g, ox, oy);
                }
            }

            g.g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 2);

            if g.game_values.toplayer {
                self.surfaceMapForeground.draw_to(g.blitdest, &self.rectDst);
            }

            g.g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 3);
            g.g_map.draw_platforms_offset(self.rectDst.x as i16, self.rectDst.y as i16, 4);
        }
    }

    pub fn load_current_map(&mut self, g: &mut Game) {
        let szMapPath = unsafe {
            self.szMapName = copy_map_name(g.maplist.current_shortmapname());
            g.maplist.current_filename().to_string()
        };

        self.load_map(g, &szMapPath);
    }

    pub fn load_map(&mut self, g: &mut Game, szMapPath: &str) {
        let mut gp = g.ptr();
        unsafe {
            g.g_map.load_map(szMapPath, read_type_preview);
            small_delay(); //Sleeps to help the music from skipping

            load_current_map_background(g);
            small_delay();

            gp.get().g_map.pre_draw_preview_background_spr(g, &gp.get().rm.spr_background, self.surfaceMapBackground.get_surface(), false);
            small_delay();

            gp.g_map.pre_draw_preview_blocks(g, self.surfaceMapBlockLayer.get_surface(), false);
            small_delay();

            gp.g_map.pre_draw_preview_map_items(g, self.surfaceMapBackground.get_surface(), false);
            small_delay();

            gp.g_map.pre_draw_preview_foreground(g, self.surfaceMapForeground.get_surface(), false);
            small_delay();

            gp.get().g_map.pre_draw_preview_warps(g, if gp.get().game_values.toplayer { self.surfaceMapForeground.get_surface() } else { self.surfaceMapBackground.get_surface() }, false);
            small_delay();

            load_map_hazards(g, true);
        }
    }

    pub fn set_map(&mut self, g: &mut Game, paramSzMapName: &str, fWorld: bool) -> bool {
        let fFound = unsafe { g.maplist.findexact(paramSzMapName, fWorld) };
        self.load_current_map(g);

        fFound
    }

    pub fn set_special_map(&mut self, g: &mut Game, mapName: &str, szMapPath: &str) {
        self.szMapName = copy_map_name(mapName);

        self.load_map(g, szMapPath);
    }

    pub fn get_map_name(&self) -> &str {
        &self.szMapName
    }

    pub fn get_map_file_path(&self, g: &mut Game) -> String {
        unsafe { g.maplist.current_filename().to_string() }
    }

    pub fn set_dimensions(&mut self, width: i16, indent: i16) {
        self.iWidth = width;
        self.iIndent = indent;
    }
}

impl UI_ControlTrait for MI_MapPreview {
    crate::impl_ctl!();

    fn update(&mut self, g: &mut Game) {
        self.update_impl(g);
    }

    fn draw(&mut self, g: &mut Game) {
        self.draw_impl(g);
    }
}
