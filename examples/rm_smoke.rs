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
    let data = std::env::args().nth(1).unwrap_or_else(|| concat!(env!("CARGO_MANIFEST_DIR"), "/data").to_string());

    smw::globals::init_globals();
    unsafe {
        RootDataDirectory = data;

        gfx_init(640, 480, false);
        blitdest = screen;
        sfx_init();

        rm = Ptr::new_box(CResourceManager::new());
        skinlist = Ptr::new_box(SkinList::new());
        soundpacklist = Ptr::new_box(SoundsList::new());
        menugraphicspacklist = Ptr::new_box(GraphicsList::new());
        worldgraphicspacklist = Ptr::new_box(GraphicsList::new());
        gamegraphicspacklist = Ptr::new_box(GraphicsList::new());
        menugraphicspacklist.set_current_index(0);
        worldgraphicspacklist.set_current_index(0);
        gamegraphicspacklist.set_current_index(0);
        soundpacklist.set_current_index(0);

        let pack = gamegraphicspacklist.current_path().to_string_lossy().into_owned();
        assert!(gfx_loadpalette(Path::new(&convert_path_pack("gfx/packs/palette.png", &pack))));

        rm.load_start_graphics();
        rm.load_all_graphics();
        let sound = rm.load_game_sounds();

        let mut skins = 0;
        for k in 0..skinlist.count() {
            for color in 0..4i16 {
                let _ = rm.load_full_skin(k as i16, color);
                assert!(rm.load_menu_skin(color, k as i16, color, true), "menu skin {}", k);
            }
            skins += 1;
        }

        println!("rm_smoke: graphics ok, {} skins x 4 colors ok, sounds loaded: {}", skins, sound);
    }
}
