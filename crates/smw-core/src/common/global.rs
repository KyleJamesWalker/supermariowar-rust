//! Port of src/common/global.cpp

pub static mut RootDataDirectory: String = String::new();


//Joystick-Init
pub static mut joysticks: *mut *mut SDL_Joystick = std::ptr::null_mut();
pub static mut joystickcount: i16 = 0;

pub fn init_globals() {
    unsafe {
        // Upstream's global.cpp reads data from the settings directory on Android.
        let root = if cfg!(target_os = "android") { crate::common::path::get_home_directory() } else { crate::common::path::get_root_directory() };
        RootDataDirectory = root + "data";
        game().game_values.init(CGameValues::new());
    }
    crate::common::gfx::init_globals();
}

use crate::game::game;
use crate::common::game_values::CGameValues;
use crate::globals::{Global, Ptr};
use sdl2::sys::SDL_Joystick;


pub static g_szMusicCategoryNames: [&str; crate::common::global_constants::MAXMUSICCATEGORY as usize] =
    ["Land", "Underground", "Underwater", "Castle", "Platforms", "Ghost", "Bonus", "Battle", "Desert", "Clouds", "Snow"];

pub fn load_current_map_background() {
    use crate::common::gfx::gfx_sprite::ImageLoader;
    use crate::common::path::{concat, convert_path_pack, file_exists};
    unsafe {
        let pack = game().gamegraphicspacklist.current_path().to_string_lossy().into_owned();
        let mut path = concat("gfx/packs/backgrounds/", &game().g_map.szBackgroundFile);
        path = convert_path_pack(&path, &pack);

        if !file_exists(&path) {
            path = convert_path_pack("gfx/packs/backgrounds/Land_Classic.png", &pack);
        }

        game().rm.spr_background = ImageLoader::new(path).without_color_key().create();
    }
}
