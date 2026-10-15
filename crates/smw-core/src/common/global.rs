//! Port of src/common/global.cpp

pub fn init_globals(g: &mut Game) {
    unsafe {
        // Upstream's global.cpp reads data from the settings directory on Android.
        let root = if cfg!(target_os = "android") { crate::common::path::get_home_directory() } else { crate::common::path::get_root_directory() };
        g.RootDataDirectory = root + "data";
        g.game_values.init(CGameValues::new());
    }
    crate::common::gfx::init_globals(g);
}

use crate::game::Game;
use crate::game::game;
use crate::common::game_values::CGameValues;
use crate::globals::{Global, Ptr};


pub static g_szMusicCategoryNames: [&str; crate::common::global_constants::MAXMUSICCATEGORY as usize] =
    ["Land", "Underground", "Underwater", "Castle", "Platforms", "Ghost", "Bonus", "Battle", "Desert", "Clouds", "Snow"];

pub fn load_current_map_background(g: &mut Game) {
    use crate::common::gfx::gfx_sprite::ImageLoader;
    use crate::common::path::{concat, convert_path_pack, file_exists};
    unsafe {
        let pack = g.gamegraphicspacklist.current_path().to_string_lossy().into_owned();
        let mut path = concat("gfx/packs/backgrounds/", &g.g_map.szBackgroundFile);
        path = convert_path_pack(g, &path, &pack);

        if !file_exists(&path) {
            path = convert_path_pack(g, "gfx/packs/backgrounds/Land_Classic.png", &pack);
        }

        g.rm.spr_background = ImageLoader::new(path).without_color_key().create(g);
    }
}
