//! The game's state as one value (docs/ARCHITECTURE_V2.md, Step 2: globals into a `Game` struct).

use crate::common::gfx::soft_surface::Surface;
use crate::common::eyecandy::{CEyecandyContainer, SpotlightManager};
use crate::common::game_mode::{CGameModeTrait, GAMEMODE_LAST};
use crate::common::file_list::{AnnouncerList, FiltersList, GraphicsList, MusicList, SkinList, SoundsList, TourList, WorldList, WorldMusicList};
use crate::common::map::CMap;
use crate::common::map_list::MapList;
use crate::common::random_number_generator::RngState;
use crate::common::sfx::SfxState;
use crate::common::score::CScore;
use crate::common::resource_manager::CResourceManager;
use crate::common::tileset_manager::CTilesetManager;
use crate::common::editor_harness::EditorHarness;
use crate::common::game_values::CGameValues;
use crate::common::gfx::gfx_sdl::GraphicsSDL;
use crate::common::gfx::gfx_palette::gfxPalette;
use crate::globals::{Aliased, Global, Ptr};
use crate::leveleditor::leveleditor::LevelEditorState;
use crate::smw::gamemodes::{bonus_house::CGM_Bonus, mini_boss::CGM_Boss_MiniGame, mini_boxes::CGM_Boxes_MiniGame, mini_pipe::CGM_Pipe_MiniGame};
use crate::smw::fps_limiter::FPSLimiter;
use crate::smw::game_state::GameStateManager;
use crate::smw::gs_gameplay::GameplayState;
use crate::smw::gs_menu::MenuState;
use crate::smw::gs_splash_screen::SplashScreenState;
use crate::smw::harness::Harness;
use crate::smw::main::MainLoopState;
use crate::smw::net::{NetworkHandler, Networking};
use crate::smw::network::net_record::NetRecordState;
use crate::smw::net_outcomes::NetOutcomesState;
use crate::smw::net_random::NetRandomState;
use crate::smw::object_container::CObjectContainer;
#[cfg(not(target_os = "emscripten"))]
use crate::smw::pad::PadState;
use crate::smw::player::CPlayer;
#[cfg(not(target_os = "emscripten"))]
use crate::smw::touch::TouchState;
use crate::smw::world::WorldMap;
use crate::worldeditor::worldeditor::WorldEditorState;

pub const MAX_DEVICES: usize = 8;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Device {
    Pad(*mut sdl2::sys::SDL_Joystick),
    /// A replay's pad: nothing is open, and its events carry the device number.
    Replay,
}

pub struct Game {
    pub _alias: Aliased,
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
    pub players: crate::smw::players::Players,
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
    pub net_outcomes: NetOutcomesState,
    pub net_random: NetRandomState,
    pub x_shake: i16,
    pub y_shake: i16,
    pub shakeleft: bool,
    /// Locations for swirl spawn effects.
    pub g_iSwirlSpawnLocations: [[[i16; 25]; 2]; 4],
    pub spotlightManager: Global<SpotlightManager>,
    pub g_iWinningPlayer: i16,
    pub g_iCurrentDrawIndex: i16,
    /// Not in the C++ (whose `iNetworkID` is unused): while nonzero, new objects get `context << 16 | n`, so a net
    /// game's clients give the objects an event makes the same IDs (smw/net_random.rs).
    pub g_networkIDContext: u32,
    pub g_networkIDCount: u32,
    /// The file a match loaded, relative to the data directory so a checkpoint works with any data root.
    pub match_map: String,
    /// Whether the menu press that opened a text field came from a keyboard key; set by `UI_Menu`.
    pub text_field_opened_by_key: bool,
    pub g_worldmap: Global<WorldMap>,
    pub level_editor: LevelEditorState,
    pub world_editor: WorldEditorState,
    pub gsm: GameStateManager,
    pub gps: Option<GameplayState>,
    pub ms: Option<MenuState>,
    pub ss: Option<SplashScreenState>,
    /// The browser cannot block for input, so a control field being rebound reads this frame's events here.
    #[cfg(target_os = "emscripten")]
    pub frame_events: Vec<smw_platform::Input>,
    pub net_record: NetRecordState,
    pub screen: *mut Surface,
    /// The destination surface for all drawing; swapped from `screen` to another surface at times.
    pub blitdest: *mut Surface,
    pub fps_limiter: FPSLimiter,
    pub main_loop: MainLoopState,
    pub(crate) harness: Harness,
    pub(crate) editor_harness: EditorHarness,
    #[cfg(not(target_os = "emscripten"))]
    pub pad: PadState,
    #[cfg(not(target_os = "emscripten"))]
    pub touch: TouchState,
    pub input: crate::smw::input::InputQueue,
    pub services: Option<smw_platform::Services>,
    pub gfx: Global<GraphicsSDL>,
    pub gfx_palette: Global<gfxPalette>,
    pub RootDataDirectory: String,
    pub SMW_Root_Data_Dir: String,
    pub are_paths_initialized: bool,
    /// Indexed by the device number in bindings and replay lines; `None` is an empty slot.
    pub devices: Vec<Option<Device>>,
    #[cfg(target_os = "emscripten")]
    pub relay: crate::smw::platform::network::websocket::network_layer_websocket::Relay,
    #[cfg(target_os = "emscripten")]
    pub recv_buffer: [u8; smw_netplay::common_netplay::relay_frame::MAX_MESSAGE_LEN],
}

impl Game {
    /// The backend joystick open as device `index`, or null.
    pub fn joystick(&self, index: usize) -> *mut sdl2::sys::SDL_Joystick {
        match self.devices.get(index) {
            Some(Some(Device::Pad(js))) => *js,
            _ => std::ptr::null_mut(),
        }
    }

    /// A copyable handle to this game, for reaching a field while `self` is also passed on.
    #[inline(always)]
    pub fn ptr(&mut self) -> Ptr<Game> {
        Ptr::from_mut(self)
    }

    pub const fn new() -> Self {
        Game {
            _alias: Aliased::new(),
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
            players: crate::smw::players::Players::new(),
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
            net_outcomes: NetOutcomesState::new(),
            net_random: NetRandomState::new(),
            x_shake: 0,
            y_shake: 0,
            shakeleft: false,
            g_iSwirlSpawnLocations: [[[0; 25]; 2]; 4],
            spotlightManager: Global::uninit(),
            g_iWinningPlayer: 0,
            g_iCurrentDrawIndex: 0,
            g_networkIDContext: 0,
            g_networkIDCount: 0,
            match_map: String::new(),
            text_field_opened_by_key: false,
            g_worldmap: Global::uninit(),
            level_editor: LevelEditorState::new(),
            world_editor: WorldEditorState::new(),
            gsm: GameStateManager::new(),
            gps: None,
            ms: None,
            ss: None,
            #[cfg(target_os = "emscripten")]
            frame_events: Vec::new(),
            net_record: NetRecordState::new(),
            screen: std::ptr::null_mut(),
            blitdest: std::ptr::null_mut(),
            fps_limiter: FPSLimiter::new(),
            main_loop: MainLoopState::new(),
            harness: Harness::new(),
            editor_harness: EditorHarness::new(),
            #[cfg(not(target_os = "emscripten"))]
            pad: PadState::new(),
            #[cfg(not(target_os = "emscripten"))]
            touch: TouchState::new(),
            input: crate::smw::input::InputQueue::new(),
            services: None,
            gfx: Global::uninit(),
            gfx_palette: Global::uninit(),
            RootDataDirectory: String::new(),
            SMW_Root_Data_Dir: String::new(),
            are_paths_initialized: false,
            devices: Vec::new(),
            #[cfg(target_os = "emscripten")]
            relay: crate::smw::platform::network::websocket::network_layer_websocket::Relay::new(),
            #[cfg(target_os = "emscripten")]
            recv_buffer: [0; smw_netplay::common_netplay::relay_frame::MAX_MESSAGE_LEN],
        }
    }
}

impl Game {
    /// The game an entry point runs. It lives on the heap for the rest of the process.
    pub fn create() -> &'static mut Game {
        Box::leak(Box::new(Game::new()))
    }
}
