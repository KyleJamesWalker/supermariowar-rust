//! Loads every sprite, font, skin and sound CResourceManager knows about from data/, headless.
//!
//! SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy cargo run --example rm_smoke -- [data dir]

#![allow(static_mut_refs)]

use smw::common::file_list::{GraphicsList, SkinList, SoundsList};
use smw::common::gfx::{gfx_init, gfx_loadpalette};
use smw::common::path::convert_path_pack;
use smw::common::resource_manager::CResourceManager;
use smw::common::sfx::sfx_init;
use smw::globals::*;
use std::path::Path;

fn main() {
    let g = smw::game::Game::create();

    let mut gp = g.ptr();    let data = std::env::args().nth(1).unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../data").to_string());

    smw::globals::init_globals(gp.get());
    gp.RootDataDirectory = data;

    gfx_init(gp.get(), 640, 480, false);
    gp.blitdest = gp.screen;
    sfx_init(gp.get());

    gp.rm = Ptr::new_box(CResourceManager::new());
    gp.skinlist = Ptr::new_box(SkinList::new(gp.get()));
    gp.soundpacklist = Ptr::new_box(SoundsList::new(g));
    gp.menugraphicspacklist = Ptr::new_box(GraphicsList::new(g));
    gp.worldgraphicspacklist = Ptr::new_box(GraphicsList::new(g));
    gp.gamegraphicspacklist = Ptr::new_box(GraphicsList::new(g));
    gp.menugraphicspacklist.set_current_index(0);
    gp.worldgraphicspacklist.set_current_index(0);
    gp.gamegraphicspacklist.set_current_index(0);
    gp.soundpacklist.set_current_index(0);

    let pack = gp.gamegraphicspacklist.current_path().to_string_lossy().into_owned();
    assert!(gfx_loadpalette(gp.get(), Path::new(&convert_path_pack(gp.get(), "gfx/packs/palette.png", &pack))));

    gp.get().rm.load_start_graphics(gp.get());
    gp.get().rm.load_all_graphics(gp.get());
    let sound = gp.get().rm.load_game_sounds(gp.get());

    let mut skins = 0;
    for k in 0..gp.skinlist.count() {
        for color in 0..4i16 {
            let _ = gp.get().rm.load_full_skin(gp.get(), k as i16, color);
            assert!(gp.get().rm.load_menu_skin(gp.get(), color, k as i16, color, true), "menu skin {}", k);
        }
        skins += 1;
    }

    println!("rm_smoke: graphics ok, {} skins x 4 colors ok, sounds loaded: {}", skins, sound);
}
