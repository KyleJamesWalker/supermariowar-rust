//! The game's state as one value (docs/ARCHITECTURE_V2.md, Step 2: globals into a `Game` struct).

use crate::common::eyecandy::CEyecandyContainer;
use crate::common::file_list::{AnnouncerList, FiltersList, GraphicsList, MusicList, SkinList, SoundsList, TourList, WorldList, WorldMusicList};
use crate::common::map::CMap;
use crate::common::map_list::MapList;
use crate::common::resource_manager::CResourceManager;
use crate::common::tileset_manager::CTilesetManager;
use crate::common::game_values::CGameValues;
use crate::globals::{Global, Ptr};
use crate::smw::object_container::CObjectContainer;

pub struct Game {
    pub game_values: Global<CGameValues>,
    /// Must be created before `maplist`, whose constructor reads it.
    pub filterslist: Ptr<FiltersList>,
    pub maplist: Ptr<MapList>,
    pub skinlist: Ptr<SkinList>,
    pub announcerlist: Ptr<AnnouncerList>,
    pub musiclist: Ptr<MusicList>,
    pub worldmusiclist: Ptr<WorldMusicList>,
    pub menugraphicspacklist: Ptr<GraphicsList>,
    pub worldgraphicspacklist: Ptr<GraphicsList>,
    pub gamegraphicspacklist: Ptr<GraphicsList>,
    pub soundpacklist: Ptr<SoundsList>,
    pub tourlist: Ptr<TourList>,
    pub worldlist: Ptr<WorldList>,
    pub rm: Ptr<CResourceManager>,
    pub g_map: Ptr<CMap>,
    pub g_tilesetmanager: Ptr<CTilesetManager>,
    pub noncolcontainer: CObjectContainer,
    pub objectcontainer: [CObjectContainer; 3],
    pub eyecandy: [CEyecandyContainer; 3],
}

impl Game {
    pub const fn new() -> Self {
        Game {
            game_values: Global::uninit(),
            filterslist: Ptr::null(),
            maplist: Ptr::null(),
            skinlist: Ptr::null(),
            announcerlist: Ptr::null(),
            musiclist: Ptr::null(),
            worldmusiclist: Ptr::null(),
            menugraphicspacklist: Ptr::null(),
            worldgraphicspacklist: Ptr::null(),
            gamegraphicspacklist: Ptr::null(),
            soundpacklist: Ptr::null(),
            tourlist: Ptr::null(),
            worldlist: Ptr::null(),
            rm: Ptr::null(),
            g_map: Ptr::null(),
            g_tilesetmanager: Ptr::null(),
            noncolcontainer: CObjectContainer::new(),
            objectcontainer: [CObjectContainer::new(), CObjectContainer::new(), CObjectContainer::new()],
            eyecandy: [CEyecandyContainer::new(), CEyecandyContainer::new(), CEyecandyContainer::new()],
        }
    }
}

static mut GAME: Game = Game::new();

#[inline(always)]
pub fn game() -> Ptr<Game> {
    Ptr::from_raw(unsafe { std::ptr::addr_of_mut!(GAME) })
}
