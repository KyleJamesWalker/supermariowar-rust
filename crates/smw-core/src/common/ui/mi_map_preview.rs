//! A half-size live preview of a map, with its platforms and hazards moving.

use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
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

/// `src` up to its first NUL, at most 255 bytes.
pub(crate) fn copy_map_name(src: &str) -> String {
    let bytes = src.as_bytes();
    let n = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len()).min(255);
    String::from_utf8_lossy(&bytes[..n]).into_owned()
}

/// A half-size map preview in a dialog frame; loading one replaces `Game::g_map` and the game's hazard objects.
pub struct MI_MapPreview {
    pub ui_control: UI_Control,

    pub spr: SpriteId,

    /// The prerendered layers: background with items, blocks, and foreground with warps.
    pub surfaceMapBackground: gfxSprite,
    pub surfaceMapBlockLayer: gfxSprite,
    pub surfaceMapForeground: gfxSprite,
    /// Where the preview draws on screen.
    pub rectDst: Rect,

    pub iWidth: i16,
    pub iIndent: i16,
    /// The map name shown; empty until the map changes after construction.
    pub szMapName: String,

    /// How far left of center the preview box is drawn, in pixels.
    pub iSlideListOut: i16,
}
crate::impl_base!(MI_MapPreview => ui_control: UI_Control);

impl MI_MapPreview {
    pub fn new(g: &mut Game, nspr: SpriteId, x: i16, y: i16, width: i16, indent: i16) -> Self {
        let surfaceMapBackground = gfxSprite::blank(g, (App::screenWidth / 2) as u32, (App::screenHeight / 2) as u32);
        let surfaceMapBlockLayer = gfxSprite::blank(g, (App::screenWidth / 2) as u32, (App::screenHeight / 2) as u32);
        let surfaceMapForeground = gfxSprite::blank(g, (App::screenWidth / 2) as u32, (App::screenHeight / 2) as u32);

        let mut this = MI_MapPreview {
            ui_control: UI_Control::new(x, y),
            spr: nspr,
            surfaceMapBackground,
            surfaceMapBlockLayer,
            surfaceMapForeground,
            rectDst: Rect { x: 0, y: 0, w: 0, h: 0 },
            iWidth: width,
            iIndent: indent,
            szMapName: String::new(),
            iSlideListOut: 0,
        };
        this.load_current_map(g);

        this.rectDst.x = x as i32 + 16;
        this.rectDst.y = y as i32 + 44;
        this.rectDst.w = App::screenWidth / 2;
        this.rectDst.h = App::screenHeight / 2;

        // Starts with no name shown, though the current map is loaded.
        this.szMapName.clear();
        this
    }

    /// Moves the previewed map's platforms and hazards one tick.
    pub fn update_impl(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Update hazards
        gp.noncolcontainer.update(g);

        gp.objectcontainer[1].update(g);
        g.objectcontainer[1].clean_dead_objects();

        gp.g_map.update_platforms(g);
    }

    pub fn draw_impl(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if !self.m_visible {
            return;
        }

        let iMapBoxX: i16 = (self.m_pos.x as i32 + (self.iWidth as i32 >> 1) - 176 - self.iSlideListOut as i32) as i16;
        let bx = iMapBoxX as i32;
        let y = self.m_pos.y as i32;

        //Draw the background for the map preview
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, bx, y + 30, &Rect { x: 0, y: 0, w: 336, h: 254 });
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, bx + 336, y + 30, &Rect { x: 496, y: 0, w: 16, h: 254 });
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, bx, y + 284, &Rect { x: 0, y: 464, w: 336, h: 16 });
        gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, bx + 336, y + 284, &Rect { x: 496, y: 464, w: 16, h: 16 });

        self.rectDst.x = bx + 16;

        self.surfaceMapBackground.draw_to(g.blitdest, &self.rectDst);

        gp.g_map.draw_platforms_offset(g, self.rectDst.x as i16, self.rectDst.y as i16, 0);

        self.surfaceMapBlockLayer.draw_to(g.blitdest, &self.rectDst);

        gp.g_map.draw_platforms_offset(g, self.rectDst.x as i16, self.rectDst.y as i16, 1);

        //Draw map hazards
        let (ox, oy) = (self.rectDst.x as i16, self.rectDst.y as i16);
        for i in 0..g.objectcontainer[1].list().len() {
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

        gp.g_map.draw_platforms_offset(g, self.rectDst.x as i16, self.rectDst.y as i16, 2);

        if g.game_values.toplayer {
            self.surfaceMapForeground.draw_to(g.blitdest, &self.rectDst);
        }

        gp.g_map.draw_platforms_offset(g, self.rectDst.x as i16, self.rectDst.y as i16, 3);
        gp.g_map.draw_platforms_offset(g, self.rectDst.x as i16, self.rectDst.y as i16, 4);
    }

    /// Loads the map list's selected map into the preview.
    pub fn load_current_map(&mut self, g: &mut Game) {
        let szMapPath = {
            self.szMapName = copy_map_name(g.maplist.current_shortmapname());
            g.maplist.current_filename().to_string()
        };

        self.load_map(g, &szMapPath);
    }

    /// Loads the map at `szMapPath` into `Game::g_map`, renders the preview layers, and spawns its hazards.
    pub fn load_map(&mut self, g: &mut Game, szMapPath: &str) {
        let mut gp = g.ptr();
        let start = std::time::Instant::now();
        gp.g_map.load_map(g, szMapPath, read_type_preview);
        let loaded = std::time::Instant::now();
        load_current_map_background(g);
        let background = std::time::Instant::now();

        gp.get().g_map.pre_draw_preview_background_spr(g, &gp.get().rm.sprite(gp.get().rm.spr_background), self.surfaceMapBackground.get_surface(), false);

        gp.g_map.pre_draw_preview_blocks(g, self.surfaceMapBlockLayer.get_surface(), false);

        gp.g_map.pre_draw_preview_map_items(g, self.surfaceMapBackground.get_surface(), false);

        gp.g_map.pre_draw_preview_foreground(g, self.surfaceMapForeground.get_surface(), false);

        gp.get().g_map.pre_draw_preview_warps(g, if gp.get().game_values.toplayer { self.surfaceMapForeground.get_surface() } else { self.surfaceMapBackground.get_surface() }, false);

        load_map_hazards(g, true);
        if crate::smw::perf_log::enabled() {
            let ms = |d: std::time::Duration| d.as_secs_f64() * 1000.0;
            println!(
                "[perf] map preview {}: load {:.1} ms, background {:.1} ms, total {:.1} ms",
                self.szMapName,
                ms(loaded - start),
                ms(background - loaded),
                ms(start.elapsed())
            );
        }
    }

    /// Selects the map named `paramSzMapName` (world-only maps first with `fWorld`) and previews it; false if not found.
    pub fn set_map(&mut self, g: &mut Game, paramSzMapName: &str, fWorld: bool) -> bool {
        let fFound = g.maplist.findexact(paramSzMapName, fWorld);
        self.load_current_map(g);

        fFound
    }

    /// Previews a map by path under the name `mapName`, without changing the map list's selection.
    pub fn set_special_map(&mut self, g: &mut Game, mapName: &str, szMapPath: &str) {
        self.szMapName = copy_map_name(mapName);

        self.load_map(g, szMapPath);
    }

    pub fn get_map_name(&self) -> &str {
        &self.szMapName
    }

    pub fn get_map_file_path(&self, g: &mut Game) -> String {
        g.maplist.current_filename().to_string()
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
