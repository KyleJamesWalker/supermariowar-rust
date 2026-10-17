//! The game's state as one value (docs/ARCHITECTURE_V2.md, Step 2: globals into a `Game` struct).

use crate::common::uicontrol::{ControlId, Ctl};
use crate::common::gfx::soft_surface::Surface;
use crate::smw::object_container::ObjectId;
use crate::smw::players::PlayerId;
use crate::common::eyecandy::{CEyecandyContainer, SpotlightManager};
use crate::common::game_mode::{CGameModeTrait, GAMEMODE_LAST};
use crate::common::file_list::{AnnouncerList, FiltersList, GraphicsList, MusicList, SkinList, SoundsList, TourList, WorldList, WorldMusicList};
use crate::common::map::CMap;
use crate::common::map_list::MapList;
use crate::common::random_number_generator::RngState;
use crate::common::sfx::{sfxMusic, SfxState};
use crate::common::score::CScore;
use crate::common::resource_manager::CResourceManager;
use crate::common::tileset_manager::CTilesetManager;
use crate::common::editor_harness::EditorHarness;
use crate::common::game_values::CGameValues;
use crate::common::gfx::gfx_sdl::GraphicsSDL;
use crate::common::gfx::gfx_palette::gfxPalette;
use crate::globals::{Aliased, Global, Owned, Ptr};
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
    /// A backend pad, as its events name it.
    Pad { pad: smw_platform::PadSlot, controller: bool },
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
    pub g_map: Owned<CMap>,
    pub g_tilesetmanager: Ptr<CTilesetManager>,
    pub noncolcontainer: CObjectContainer,
    pub objectcontainer: [CObjectContainer; 3],
    pub controls: crate::common::uicontrol::Controls,
    pub eyecandy: [CEyecandyContainer; 3],
    pub players: crate::smw::players::Players,
    pub score: [Global<CScore>; 4],
    pub score_cnt: i16,
    pub gamemodes: [Owned<dyn CGameModeTrait>; GAMEMODE_LAST as usize],
    pub bonushousemode: Owned<CGM_Bonus>,
    pub pipegamemode: Owned<CGM_Pipe_MiniGame>,
    pub bossgamemode: Owned<CGM_Boss_MiniGame>,
    pub boxesgamemode: Owned<CGM_Boxes_MiniGame>,
    pub currentgamemode: i16,
    pub rng: RngState,
    pub sfx: SfxState,
    pub backgroundmusic: [sfxMusic; 6],
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
    /// What a copy from `clone_match` owns where the original's fields point at shared values.
    match_copy: Option<MatchCopy>,
}

struct MatchCopy {
    _screen: Box<Surface>,
    _musiclist: Box<MusicList>,
}

impl Game {
    pub fn object(&self, id: ObjectId) -> Option<Ptr<dyn crate::common::object_base::CObjectTrait>> {
        match id.container() {
            3 => self.noncolcontainer.get(id),
            c => self.objectcontainer.get(c as usize)?.get(id),
        }
    }

    /// The item `id` names, or null: what the C++ `carriedItem` pointer held.
    pub fn carried(&self, id: ObjectId) -> Ptr<dyn crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait> {
        match self.object(id) {
            Some(mut obj) => obj.get().as_io_moving_object().and_then(|m| m.as_carried_object()).map(Ptr::from_mut).unwrap_or_else(Ptr::null),
            None => Ptr::null(),
        }
    }

    pub fn carried_item(&self, player: PlayerId) -> Ptr<dyn crate::smw::objects::moving::mo_carried_object::MO_CarriedObjectTrait> {
        self.carried(self.player(player).carriedItem)
    }

    pub fn ctl<T: 'static>(&self, ctl: crate::common::uicontrol::Ctl<T>) -> &'static mut T {
        self.controls.typed(ctl)
    }

    /// Panics on `ControlId::NONE` or a removed control.
    pub fn control(&self, id: crate::common::uicontrol::ControlId) -> &'static mut dyn crate::common::uicontrol::UI_ControlTrait {
        self.controls.get(id).unwrap_or_else(|| panic!("no control {:?}", id))
    }

    pub fn player_keys(&self, keys: crate::smw::player::PlayerKeys) -> &'static mut crate::common::input::COutputControl {
        let controls: *const crate::common::input::COutputControl = match keys {
            crate::smw::player::PlayerKeys::Local(i) => &self.game_values.playerInput.outputControls[i as usize],
            crate::smw::player::PlayerKeys::Net(i) => &self.netplay.netPlayerInput.outputControls[i as usize],
        };
        unsafe { &mut *(controls as *mut crate::common::input::COutputControl) }
    }

    /// The current game mode, or null before one is chosen.
    pub fn mode(&self) -> Ptr<dyn CGameModeTrait> {
        use crate::common::game_mode::ModeId;
        let dyn_mode = |p: *mut dyn CGameModeTrait| Ptr::from_raw(p);
        match self.game_values.gamemode {
            None => Ptr::null(),
            Some(ModeId::Standard(i)) => self.gamemodes[i as usize].ptr(),
            Some(ModeId::BonusHouse) => dyn_mode(self.bonushousemode.ptr().as_ptr()),
            Some(ModeId::Pipe) => dyn_mode(self.pipegamemode.ptr().as_ptr()),
            Some(ModeId::Boss) => dyn_mode(self.bossgamemode.ptr().as_ptr()),
            Some(ModeId::Boxes) => dyn_mode(self.boxesgamemode.ptr().as_ptr()),
        }
    }

    /// The input configuration `slot` names, or null.
    pub fn input_control(&self, slot: Option<(u8, u8)>) -> Ptr<crate::common::input::CInputPlayerControl> {
        match slot {
            Some((set, kind)) => Ptr::from_raw(&self.game_values.inputConfiguration[set as usize][kind as usize] as *const _ as *mut _),
            None => Ptr::null(),
        }
    }

    pub fn map_block(&self, x: i16, y: i16) -> Ptr<dyn crate::common::io_block::IO_BlockTrait> {
        self.g_map.block(&self.noncolcontainer, x, y)
    }

    pub fn update_tile_gap(&mut self, i: i16, j: i16) {
        let mut map = self.g_map.ptr();
        map.update_tile_gap(&self.noncolcontainer, i, j);
    }

    /// The object `id` names, if it is still alive and a `T`.
    pub fn object_as<T: 'static>(&self, id: ObjectId) -> Option<&'static mut T> {
        let mut obj = self.object(id)?;
        let typed: *mut T = obj.get().as_any().downcast_mut::<T>()?;
        Some(unsafe { &mut *typed })
    }

    /// The backend pad open as device `index`.
    pub fn pad(&self, index: usize) -> Option<smw_platform::PadSlot> {
        match self.devices.get(index) {
            Some(Some(Device::Pad { pad, .. })) => Some(*pad),
            _ => None,
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
            g_map: Owned::empty(),
            g_tilesetmanager: Ptr::null(),
            noncolcontainer: CObjectContainer::new(3),
            objectcontainer: [CObjectContainer::new(0), CObjectContainer::new(1), CObjectContainer::new(2)],
            controls: crate::common::uicontrol::Controls::new(),
            eyecandy: [CEyecandyContainer::new(), CEyecandyContainer::new(), CEyecandyContainer::new()],
            players: crate::smw::players::Players::new(),
            score: [Global::uninit(), Global::uninit(), Global::uninit(), Global::uninit()],
            score_cnt: 0,
            gamemodes: [const { Owned::empty() }; GAMEMODE_LAST as usize],
            bonushousemode: Owned::empty(),
            pipegamemode: Owned::empty(),
            bossgamemode: Owned::empty(),
            boxesgamemode: Owned::empty(),
            currentgamemode: 0,
            rng: RngState::new(),
            sfx: SfxState::new(),
            backgroundmusic: [const { sfxMusic::new() }; 6],
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
            match_copy: None,
        }
    }

    /// A copy of the running match (docs/HANDLES.md, "What a copy of `Game` shares"). It shares the loaded assets,
    /// runs on `services`, draws to its own screen and has no menus or editors. The harness gives it the replay's
    /// input but no outputs.
    pub fn clone_match(&self, services: smw_platform::Services) -> Box<Game> {
        let mut screen = Box::new(unsafe { (*self.screen).clone() });
        let screen_ptr: *mut Surface = &mut *screen;
        let mut musiclist = Box::new((*self.musiclist).clone());
        let musiclist_ptr = Ptr::from_mut(&mut *musiclist);
        let mut sfx = self.sfx.clone();
        sfx.sfx_audible = false;
        let mut networkHandler: Global<NetworkHandler> = Global::uninit();
        networkHandler.init(NetworkHandler::default());
        Box::new(Game {
            _alias: Aliased::new(),
            game_values: self.game_values.clone(),
            filterslist: self.filterslist,
            maplist: self.maplist,
            skinlist: self.skinlist,
            announcerlist: self.announcerlist,
            musiclist: musiclist_ptr,
            worldmusiclist: self.worldmusiclist,
            menugraphicspacklist: self.menugraphicspacklist,
            worldgraphicspacklist: self.worldgraphicspacklist,
            gamegraphicspacklist: self.gamegraphicspacklist,
            soundpacklist: self.soundpacklist,
            tourlist: self.tourlist,
            worldlist: self.worldlist,
            rm: self.rm,
            g_map: self.g_map.clone(),
            g_tilesetmanager: self.g_tilesetmanager,
            noncolcontainer: self.noncolcontainer.clone(),
            objectcontainer: self.objectcontainer.clone(),
            controls: crate::common::uicontrol::Controls::new(),
            eyecandy: self.eyecandy.clone(),
            players: self.players.clone(),
            score: self.score.clone(),
            score_cnt: self.score_cnt,
            gamemodes: std::array::from_fn(|i| {
                let mut mode: Owned<dyn CGameModeTrait> = Owned::empty();
                if self.gamemodes[i].is_set() {
                    mode.set(self.gamemodes[i].clone_box());
                }
                mode
            }),
            bonushousemode: self.bonushousemode.clone(),
            pipegamemode: self.pipegamemode.clone(),
            bossgamemode: self.bossgamemode.clone(),
            boxesgamemode: self.boxesgamemode.clone(),
            currentgamemode: self.currentgamemode,
            rng: self.rng.clone(),
            sfx,
            backgroundmusic: self.backgroundmusic.clone(),
            networkHandler,
            netplay: self.netplay.clone(),
            backup_playercontrol: self.backup_playercontrol,
            net_outcomes: self.net_outcomes.clone(),
            net_random: self.net_random.clone(),
            x_shake: self.x_shake,
            y_shake: self.y_shake,
            shakeleft: self.shakeleft,
            g_iSwirlSpawnLocations: self.g_iSwirlSpawnLocations,
            spotlightManager: self.spotlightManager.clone(),
            g_iWinningPlayer: self.g_iWinningPlayer,
            g_iCurrentDrawIndex: self.g_iCurrentDrawIndex,
            g_networkIDContext: self.g_networkIDContext,
            g_networkIDCount: self.g_networkIDCount,
            match_map: self.match_map.clone(),
            text_field_opened_by_key: false,
            g_worldmap: self.g_worldmap.clone(),
            level_editor: LevelEditorState::new(),
            world_editor: WorldEditorState::new(),
            gsm: self.gsm.clone(),
            gps: self.gps.clone(),
            ms: None,
            ss: None,
            #[cfg(target_os = "emscripten")]
            frame_events: Vec::new(),
            net_record: self.net_record.clone(),
            screen: screen_ptr,
            blitdest: if self.blitdest == self.screen { screen_ptr } else { self.blitdest },
            fps_limiter: FPSLimiter::new(),
            main_loop: self.main_loop.clone(),
            harness: self.harness.clone_inputs(),
            editor_harness: EditorHarness::new(),
            #[cfg(not(target_os = "emscripten"))]
            pad: PadState::new(),
            #[cfg(not(target_os = "emscripten"))]
            touch: TouchState::new(),
            input: self.input.clone(),
            services: Some(services),
            gfx: Global::uninit(),
            gfx_palette: self.gfx_palette.clone(),
            RootDataDirectory: self.RootDataDirectory.clone(),
            SMW_Root_Data_Dir: self.SMW_Root_Data_Dir.clone(),
            are_paths_initialized: self.are_paths_initialized,
            devices: self.devices.clone(),
            #[cfg(target_os = "emscripten")]
            relay: crate::smw::platform::network::websocket::network_layer_websocket::Relay::new(),
            #[cfg(target_os = "emscripten")]
            recv_buffer: [0; smw_netplay::common_netplay::relay_frame::MAX_MESSAGE_LEN],
            match_copy: Some(MatchCopy { _screen: screen, _musiclist: musiclist }),
        })
    }
}

impl Game {
    /// The game an entry point runs. It lives on the heap for the rest of the process.
    pub fn create() -> &'static mut Game {
        Box::leak(Box::new(Game::new()))
    }
}
