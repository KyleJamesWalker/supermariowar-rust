//! The game's state as one value (docs/ARCHITECTURE_V2.md, Step 2: globals into a `Game` struct).

use crate::common::eyecandy::CEyecandyContainer;
use crate::common::game_mode::{CGameModeTrait, GAMEMODE_LAST};
use crate::common::file_list::{AnnouncerList, FiltersList, GraphicsList, MusicList, SkinList, SoundsList, TourList, WorldList, WorldMusicList};
use crate::common::map::CMap;
use crate::common::map_list::MapList;
use crate::common::random_number_generator::RngState;
use crate::common::sfx::SfxState;
use crate::common::score::CScore;
use crate::common::resource_manager::CResourceManager;
use crate::common::tileset_manager::CTilesetManager;
use crate::common::game_values::CGameValues;
use crate::globals::{Global, Ptr};
use crate::smw::gamemodes::{bonus_house::CGM_Bonus, mini_boss::CGM_Boss_MiniGame, mini_boxes::CGM_Boxes_MiniGame, mini_pipe::CGM_Pipe_MiniGame};
use crate::smw::net::{NetworkHandler, Networking};
use crate::smw::object_container::CObjectContainer;
use crate::smw::player::CPlayer;

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
    pub players: Vec<Ptr<CPlayer>>,
    pub score: [Ptr<CScore>; 4],
    pub score_cnt: i16,
    pub gamemodes: [Ptr<dyn CGameModeTrait>; GAMEMODE_LAST as usize],
    pub bonushousemode: Ptr<CGM_Bonus>,
    pub pipegamemode: Ptr<CGM_Pipe_MiniGame>,
    pub bossgamemode: Ptr<CGM_Boss_MiniGame>,
    pub boxesgamemode: Ptr<CGM_Boxes_MiniGame>,
    pub currentgamemode: i16,
    pub rng: RngState,
    pub sfx: SfxState,
    pub networkHandler: Global<NetworkHandler>,
    pub netplay: Global<Networking>,
    pub backup_playercontrol: [i16; 4],
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
            players: Vec::new(),
            score: [Ptr::null(); 4],
            score_cnt: 0,
            gamemodes: [Ptr::null(); GAMEMODE_LAST as usize],
            bonushousemode: Ptr::null(),
            pipegamemode: Ptr::null(),
            bossgamemode: Ptr::null(),
            boxesgamemode: Ptr::null(),
            currentgamemode: 0,
            rng: RngState::new(),
            sfx: SfxState::new(),
            networkHandler: Global::uninit(),
            netplay: Global::uninit(),
            backup_playercontrol: [0; 4],
        }
    }
}

static mut GAME: Game = Game::new();

#[inline(always)]
pub fn game() -> Ptr<Game> {
    Ptr::from_raw(unsafe { std::ptr::addr_of_mut!(GAME) })
}
