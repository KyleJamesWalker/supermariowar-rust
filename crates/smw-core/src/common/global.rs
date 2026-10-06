//! Port of src/common/global.cpp

pub static mut RootDataDirectory: String = String::new();

pub static mut game_values: Global<CGameValues> = Global::uninit();

//Joystick-Init
pub static mut joysticks: *mut *mut SDL_Joystick = std::ptr::null_mut();
pub static mut joystickcount: i16 = 0;

pub fn init_globals() {
    unsafe {
        // Upstream's global.cpp reads data from the settings directory on Android.
        let root = if cfg!(target_os = "android") { crate::common::path::get_home_directory() } else { crate::common::path::get_root_directory() };
        RootDataDirectory = root + "data";
        game_values.init(CGameValues::new());
    }
    crate::common::gfx::init_globals();
}

use crate::common::file_list::{AnnouncerList, FiltersList, GraphicsList, MusicList, SkinList, SoundsList, TourList, WorldList, WorldMusicList};
use crate::common::game_values::CGameValues;
use crate::common::map_list::MapList;
use crate::globals::{Global, Ptr};
use sdl2::sys::SDL_Joystick;

pub static mut filterslist: Ptr<FiltersList> = Ptr::null(); //Filters list must be initiallized before maps list because it is used in maplist constructor
pub static mut maplist: Ptr<MapList> = Ptr::null();
pub static mut skinlist: Ptr<SkinList> = Ptr::null();
pub static mut announcerlist: Ptr<AnnouncerList> = Ptr::null();
pub static mut musiclist: Ptr<MusicList> = Ptr::null();
pub static mut worldmusiclist: Ptr<WorldMusicList> = Ptr::null();
pub static mut menugraphicspacklist: Ptr<GraphicsList> = Ptr::null();
pub static mut worldgraphicspacklist: Ptr<GraphicsList> = Ptr::null();
pub static mut gamegraphicspacklist: Ptr<GraphicsList> = Ptr::null();
pub static mut soundpacklist: Ptr<SoundsList> = Ptr::null();
pub static mut tourlist: Ptr<TourList> = Ptr::null();
pub static mut worldlist: Ptr<WorldList> = Ptr::null();

pub static mut rm: Ptr<crate::common::resource_manager::CResourceManager> = Ptr::null();
pub static mut g_map: Ptr<crate::common::map::CMap> = Ptr::null();
pub static mut g_tilesetmanager: Ptr<crate::common::tileset_manager::CTilesetManager> = Ptr::null();

pub static g_szMusicCategoryNames: [&str; crate::common::global_constants::MAXMUSICCATEGORY as usize] =
    ["Land", "Underground", "Underwater", "Castle", "Platforms", "Ghost", "Bonus", "Battle", "Desert", "Clouds", "Snow"];

pub fn load_current_map_background() {
    use crate::common::gfx::gfx_sprite::ImageLoader;
    use crate::common::path::{concat, convert_path_pack, file_exists};
    unsafe {
        let pack = gamegraphicspacklist.current_path().to_string_lossy().into_owned();
        let mut path = concat("gfx/packs/backgrounds/", &g_map.szBackgroundFile);
        path = convert_path_pack(&path, &pack);

        if !file_exists(&path) {
            path = convert_path_pack("gfx/packs/backgrounds/Land_Classic.png", &pack);
        }

        rm.spr_background = ImageLoader::new(path).without_color_key().create();
    }
}
