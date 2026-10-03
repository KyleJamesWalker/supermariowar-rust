//! Twin of tools/ref/map_render.cpp: renders maps as GameplayState draws them, to BMPs.
//!
//! SDL_VIDEODRIVER=dummy cargo run --release --example map_render -- <datadir> <outdir> <mapfile>...

use smw::common::file_list::{FiltersList, GraphicsList, SkinList};
use smw::common::game_values::CGameValues;
use smw::common::gfx::gfx_init;
use smw::common::global::load_current_map_background;
use smw::common::map::{g_iCurrentDrawIndex, read_type_full, CMap};
use smw::common::resource_manager::CResourceManager;
use smw::common::tileset_manager::CTilesetManager;
use smw::globals::*;
use sdl2::sys::{SDL_FillRect, SDL_FreeSurface, SDL_RWFromFile, SDL_SaveBMP_RW, SDL_Surface};
use std::ffi::CString;

fn save_bmp(s: *mut SDL_Surface, path: &str) {
    let c = CString::new(path).unwrap();
    unsafe {
        SDL_SaveBMP_RW(s, SDL_RWFromFile(c.as_ptr(), b"wb\0".as_ptr() as *const _), 1);
    }
}

unsafe fn draw_map() {
    blitdest = screen;
    rm.spr_backmap[g_iCurrentDrawIndex as usize].draw(0, 0);
    g_map.draw_platforms(0);
    g_map.draw_platforms(1);
    g_map.draw_platforms(2);
    if game_values.toplayer {
        g_map.drawfrontlayer();
    }
    g_map.draw_warp_locks();
    g_map.draw_platforms(3);
    g_map.draw_platforms(4);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    smw::globals::init_globals();
    unsafe {
        RootDataDirectory = args[1].clone();
        let outdir = &args[2];
        gfx_init(640, 480, false);
        blitdest = screen;
        rm = Ptr::new_box(CResourceManager::new());
        g_map = Ptr::from_box(CMap::new());
        g_tilesetmanager = Ptr::new_box(CTilesetManager::new());
        filterslist = Ptr::new_box(FiltersList::new());
        skinlist = Ptr::new_box(SkinList::new());
        menugraphicspacklist = Ptr::new_box(GraphicsList::new());
        worldgraphicspacklist = Ptr::new_box(GraphicsList::new());
        gamegraphicspacklist = Ptr::new_box(GraphicsList::new());
        if !game_values.is_initialized() {
            game_values.init(CGameValues::new());
        }
        CGameValues::init(&mut game_values);
        menugraphicspacklist.set_current_index(0);
        worldgraphicspacklist.set_current_index(0);
        gamegraphicspacklist.set_current_index(0);
        rm.load_all_graphics();

        for (n, file) in args[3..].iter().enumerate() {
            g_map.load_map(file, read_type_full);
            load_current_map_background();
            let rmp = rm;
            g_map.predrawbackground(&rmp.spr_background, &rmp.spr_backmap[0]);
            g_map.predrawforeground(&rmp.spr_frontmap[0]);
            g_map.predrawbackground(&rmp.spr_background, &rmp.spr_backmap[1]);
            g_map.predrawforeground(&rmp.spr_frontmap[1]);
            g_map.setup_animated_tiles();

            SDL_FillRect(screen, std::ptr::null(), 0);
            draw_map();
            save_bmp(screen, &format!("{}/{}_f0.bmp", outdir, n));

            for _ in 0..40 {
                g_map.update_platforms();
                g_map.update();
            }
            g_map.lockconnection(0);
            SDL_FillRect(screen, std::ptr::null(), 0);
            draw_map();
            save_bmp(screen, &format!("{}/{}_f40.bmp", outdir, n));

            let thumb = g_map.create_thumbnail_surface(false);
            if !thumb.is_null() {
                save_bmp(thumb, &format!("{}/{}_thumb.bmp", outdir, n));
                SDL_FreeSurface(thumb);
            }
        }
        println!("\n---\nrendered {}", args.len() - 3);
    }
}
