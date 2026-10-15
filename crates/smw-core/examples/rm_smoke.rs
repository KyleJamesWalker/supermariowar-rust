//! Loads every sprite, font, skin and sound CResourceManager knows about from data/, headless.
//!
//! SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy cargo run --example rm_smoke -- [data dir]

#![allow(static_mut_refs)]

use smw::common::file_list::{GraphicsList, SkinList, SoundsList};
use smw::common::gfx::{gfx_init, gfx_loadpalette};
use smw::common::path::convert_path_pack;
use smw::common::resource_manager::CResourceManager;
use smw::common::sfx::sfx_init;
use smw::common::tileset_manager::CTilesetManager;
use smw::globals::*;
use std::path::Path;

fn main() {
    let data = std::env::args().nth(1).unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/../../data").to_string());

    smw::globals::init_globals(game().get());
    unsafe {
        game().RootDataDirectory = data;

        gfx_init(game().get(), 640, 480, false);
        game().blitdest = game().screen;
        sfx_init();

        game().rm = Ptr::new_box(CResourceManager::new());
        game().skinlist = Ptr::new_box(SkinList::new(game().get()));
        game().soundpacklist = Ptr::new_box(SoundsList::new());
        game().menugraphicspacklist = Ptr::new_box(GraphicsList::new());
        game().worldgraphicspacklist = Ptr::new_box(GraphicsList::new());
        game().gamegraphicspacklist = Ptr::new_box(GraphicsList::new());
        game().menugraphicspacklist.set_current_index(0);
        game().worldgraphicspacklist.set_current_index(0);
        game().gamegraphicspacklist.set_current_index(0);
        game().soundpacklist.set_current_index(0);

        let pack = game().gamegraphicspacklist.current_path().to_string_lossy().into_owned();
        assert!(gfx_loadpalette(game().get(), Path::new(&convert_path_pack(game().get(), "gfx/packs/palette.png", &pack))));

        game().rm.load_start_graphics(game().get());
        game().rm.load_all_graphics(game().get());
        let sound = game().rm.load_game_sounds(game().get());

        let mut skins = 0;
        for k in 0..game().skinlist.count() {
            for color in 0..4i16 {
                let _ = game().rm.load_full_skin(game().get(), k as i16, color);
                assert!(game().rm.load_menu_skin(game().get(), color, k as i16, color, true), "menu skin {}", k);
            }
            skins += 1;
        }

        println!("rm_smoke: graphics ok, {} skins x 4 colors ok, sounds loaded: {}", skins, sound);
    }
}
