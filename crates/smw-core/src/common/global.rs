//! Game-wide setup and the free-standing global data and helpers every part of the game reads.

/// Sets the data directory and creates the game values and graphics state.
pub fn init_globals(g: &mut Game) {
    // Android keeps the data directory in the settings directory.
    let root = if cfg!(target_os = "android") { crate::common::path::get_home_directory(g) } else { crate::common::path::get_root_directory(g) };
    g.RootDataDirectory = root + "data";
    g.game_values.init(CGameValues::new());
    crate::common::gfx::init_globals(g);
}

use crate::game::Game;
use crate::common::game_values::CGameValues;
use crate::globals::{Global, Ptr};


/// Display names of the map music categories, indexed by category.
pub static g_szMusicCategoryNames: [&str; crate::common::global_constants::MAXMUSICCATEGORY as usize] =
    ["Land", "Underground", "Underwater", "Castle", "Platforms", "Ghost", "Bonus", "Battle", "Desert", "Clouds", "Snow"];

/// Loads the current map's background from the current graphics pack, or Land_Classic.png when that file is missing.
pub fn load_current_map_background(g: &mut Game) {
    use crate::common::gfx::gfx_sprite::ImageLoader;
    use crate::common::path::{concat, convert_path_pack, file_exists};
    let pack = g.gamegraphicspacklist.current_path().to_string_lossy().into_owned();
    let mut path = concat("gfx/packs/backgrounds/", &g.g_map.szBackgroundFile);
    path = convert_path_pack(g, &path, &pack);

    if !file_exists(&path) {
        path = convert_path_pack(g, "gfx/packs/backgrounds/Land_Classic.png", &pack);
    }

    *g.rm.sprite(g.rm.spr_background) = ImageLoader::new(path).without_color_key().create(g);
}
