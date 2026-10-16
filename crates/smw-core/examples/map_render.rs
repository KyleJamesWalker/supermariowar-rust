//! Twin of tools/ref/map_render.cpp: renders maps as GameplayState draws them, to BMPs.
//!
//! SDL_VIDEODRIVER=dummy cargo run --release --example map_render -- <datadir> <outdir> <mapfile>...

use smw::common::file_list::{FiltersList, GraphicsList, SkinList};
use smw::common::game_values::CGameValues;
use smw::common::gfx::gfx_init;
use smw::common::global::load_current_map_background;
use smw::common::map::{read_type_full, CMap};
use smw::common::resource_manager::CResourceManager;
use smw::common::tileset_manager::CTilesetManager;
use smw::globals::*;
use sdl2::sys::{SDL_FreeSurface, SDL_RWFromFile, SDL_SaveBMP_RW, SDL_Surface};
use std::ffi::CString;

fn save_bmp(s: *mut SDL_Surface, path: &str) {
    let c = CString::new(path).unwrap();
    unsafe {
        SDL_SaveBMP_RW(s, SDL_RWFromFile(c.as_ptr(), b"wb\0".as_ptr() as *const _), 1);
    }
}

unsafe fn draw_map(g: &mut smw::game::Game) {
    let mut gp = g.ptr();
    gp.blitdest = gp.screen;
    gp.rm.spr_backmap[gp.g_iCurrentDrawIndex as usize].draw(gp.get(), 0, 0);
    gp.get().g_map.draw_platforms(gp.get(), 0);
    gp.get().g_map.draw_platforms(gp.get(), 1);
    gp.get().g_map.draw_platforms(gp.get(), 2);
    if gp.game_values.toplayer {
        gp.get().g_map.drawfrontlayer(gp.get());
    }
    gp.get().g_map.draw_warp_locks(gp.get());
    gp.get().g_map.draw_platforms(gp.get(), 3);
    gp.get().g_map.draw_platforms(gp.get(), 4);
}

fn main() {
    let g = smw::game::Game::create();

    let mut gp = g.ptr();    let args: Vec<String> = std::env::args().collect();
    smw::globals::init_globals(gp.get());
    unsafe {
        gp.RootDataDirectory = args[1].clone();
        let outdir = &args[2];
        gfx_init(gp.get(), 640, 480, false);
        gp.blitdest = gp.screen;
        gp.rm = Ptr::new_box(CResourceManager::new());
        gp.g_map = Ptr::from_box(CMap::new());
        gp.filterslist = Ptr::new_box(FiltersList::new(g));
        gp.skinlist = Ptr::new_box(SkinList::new(gp.get()));
        gp.menugraphicspacklist = Ptr::new_box(GraphicsList::new(g));
        gp.worldgraphicspacklist = Ptr::new_box(GraphicsList::new(g));
        gp.gamegraphicspacklist = Ptr::new_box(GraphicsList::new(g));
        if !gp.game_values.is_initialized() {
            gp.game_values.init(CGameValues::new());
        }
        CGameValues::init(&mut gp.get().game_values, gp.get());
        gp.menugraphicspacklist.set_current_index(0);
        gp.worldgraphicspacklist.set_current_index(0);
        gp.gamegraphicspacklist.set_current_index(0);
        gp.get().rm.load_all_graphics(gp.get());

        for (n, file) in args[3..].iter().enumerate() {
            gp.get().g_map.load_map(gp.get(), file, read_type_full);
            load_current_map_background(gp.get());
            let rmp = gp.rm;
            gp.get().g_map.predrawbackground(gp.get(), &rmp.spr_background, &rmp.spr_backmap[0]);
            gp.get().g_map.predrawforeground(gp.get(), &rmp.spr_frontmap[0]);
            gp.get().g_map.predrawbackground(gp.get(), &rmp.spr_background, &rmp.spr_backmap[1]);
            gp.get().g_map.predrawforeground(gp.get(), &rmp.spr_frontmap[1]);
            gp.get().g_map.setup_animated_tiles(gp.get());

            smw::common::gfx::blit::fill_rect(gp.screen, std::ptr::null(), 0);
            draw_map(g);
            save_bmp(gp.screen, &format!("{}/{}_f0.bmp", outdir, n));

            for _ in 0..40 {
                gp.get().g_map.update_platforms(gp.get());
                gp.get().g_map.update(gp.get());
            }
            gp.g_map.lockconnection(0);
            smw::common::gfx::blit::fill_rect(gp.screen, std::ptr::null(), 0);
            draw_map(g);
            save_bmp(gp.screen, &format!("{}/{}_f40.bmp", outdir, n));

            let thumb = gp.get().g_map.create_thumbnail_surface(gp.get(), false);
            if !thumb.get_surface().is_null() {
                save_bmp(thumb.get_surface(), &format!("{}/{}_thumb.bmp", outdir, n));
            }
        }
        println!("\n---\nrendered {}", args.len() - 3);
    }
}
