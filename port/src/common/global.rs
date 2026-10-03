//! Port of src/common/global.cpp

pub static mut RootDataDirectory: String = String::new();

pub static mut joystickcount: i16 = 0;

pub fn init_globals() {
    unsafe {
        RootDataDirectory = crate::common::path::get_root_directory() + "data";
    }
    crate::common::gfx::init_globals();
}

use crate::common::file_list::{AnnouncerList, FiltersList, GraphicsList, MusicList, SkinList, SoundsList, TourList, WorldList, WorldMusicList};
use crate::globals::Ptr;

pub static mut filterslist: Ptr<FiltersList> = Ptr::null();
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
