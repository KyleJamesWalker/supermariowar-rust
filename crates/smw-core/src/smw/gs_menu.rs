//! The menu game state: owns every menu screen, routes their menu codes and starts matches.

use crate::common::keys::*;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use smw_platform::{Input, InputEvent};
use crate::common::file_list::{MusicCategory, WorldMusicCategory};
use crate::common::game::App;
use crate::common::game_mode::*;
use crate::common::game_values::{if_sound_on_play, AppState};
use crate::common::gameplay_styles::TournamentControlStyle;
use crate::common::gfx::{gfx_changefullscreen, gfx_loadpalette, gfx_take_screenshot};
use crate::common::global_constants::{ticks, MAX_PLAYERS, PANNOUNCER_SOUND_LAST, TICKS_PER_SECOND, TILESIZE};
use crate::common::map::{read_type_full, read_type_preview};
use crate::common::map_list::fgets_lines;
use crate::common::match_types::{Boss, MatchType, Minigame};
use crate::common::path::{convert_path, convert_path_pack, get_name_from_file_name, strip_path_and_extension};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::sfx::{sfx_setmusicvolume, sfx_setsoundvolume, sfxMusic, sfxSound};
use crate::common::ui::menu_code::*;
use crate::common::uimenu::UI_Menu;
use crate::common::version::Version;
use crate::common::world_tour_stop::{parse_tour_stop_line, reset_tour_stops, Strtok, TourStop};
use crate::globals::*;
use crate::common::eyecandy::EC_Announcement;
use crate::smw::world::WorldMap;

use crate::smw::game_state::{GameState, GameStateManager};
use crate::smw::gs_gameplay::{lookup_team_id, GameplayState};
use crate::smw::harness;

use crate::smw::menu::bonus_wheel_menu::UI_BonusWheelMenu;
use crate::smw::menu::game_settings_menu::UI_GameSettingsMenu;
use crate::smw::menu::main_menu::UI_MainMenu;
use crate::smw::menu::map_filter_edit_menu::UI_MapFilterEditMenu;
use crate::smw::menu::match_selection_menu::UI_MatchSelectionMenu;
use crate::smw::menu::mode_options_menu::UI_ModeOptionsMenu;
use crate::smw::menu::options::eye_candy_options_menu::UI_EyeCandyOptionsMenu;
use crate::smw::menu::options::gameplay_options_menu::UI_GameplayOptionsMenu;
use crate::smw::menu::options::graphics_options_menu::UI_GraphicsOptionsMenu;
use crate::smw::menu::options::powerup_drop_rates_menu::UI_PowerupDropRatesMenu;
use crate::smw::menu::options::powerup_settings_menu::UI_PowerupSettingsMenu;
use crate::smw::menu::options::projectile_limits_menu::UI_ProjectileLimitsMenu;
use crate::smw::menu::options::projectile_options_menu::UI_ProjectileOptionsMenu;
use crate::smw::menu::options::sound_options_menu::UI_SoundOptionsMenu;
use crate::smw::menu::options::team_options_menu::UI_TeamOptionsMenu;
use crate::smw::menu::options_menu::UI_OptionsMenu;
use crate::smw::menu::player_controls_menu::UI_PlayerControlsMenu;
use crate::smw::menu::player_controls_select_menu::UI_PlayerControlsSelectMenu;
use crate::smw::menu::team_select_menu::UI_TeamSelectMenu;
use crate::smw::menu::tour_stop_menu::UI_TourStopMenu;
use crate::smw::menu::tournament_scoreboard_menu::UI_TournamentScoreboardMenu;
use crate::smw::menu::world_menu::UI_WorldMenu;
use crate::smw::menu::network::net_edit_servers_menu::UI_NetEditServersMenu;
use crate::smw::menu::network::net_lobby_menu::UI_NetLobbyMenu;
use crate::smw::menu::network::net_new_room_menu::UI_NetNewRoomMenu;
use crate::smw::menu::network::net_new_room_settings_menu::UI_NetNewRoomSettingsMenu;
use crate::smw::menu::network::net_room_menu::UI_NetRoomMenu;
use crate::smw::menu::network::net_servers_menu::UI_NetServersMenu;
use crate::common::path::get_home_directory;
use crate::common_netplay::protocol_definitions::*;
use crate::smw::net::{net_end_session, net_start_session, LobbyError};
use std::ops::{Deref, DerefMut};
use std::path::Path;

/// Error banner the menus show for `iDisplayErrorTimer` frames.
pub type DisplayError = i32;
pub const DISPLAY_ERROR_NONE: DisplayError = 0;
pub const DISPLAY_ERROR_READ_TOUR_FILE: DisplayError = 1;
pub const DISPLAY_ERROR_READ_WORLD_FILE: DisplayError = 2;
pub const DISPLAY_ERROR_MAP_FILTER: DisplayError = 3;
/// Shows `error_text`.
pub const DISPLAY_ERROR_TEXT: DisplayError = 4;

/// Which menu `MenuState` shows; the mode options menu's sub-menus are numbered by game mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuId {
    None,
    Main,
    Options,
    GameplayOptions,
    TeamOptions,
    PowerupDropRates,
    PowerupSettings,
    ProjectileLimits,
    ProjectileOptions,
    GraphicsOptions,
    EyeCandyOptions,
    SoundOptions,
    Stats,
    PlayerControlsSelect,
    PlayerControls,
    ModeOptions,
    MatchSelection,
    GameSettings,
    MapFilterEdit,
    TourStop,
    World,
    TeamSelect,
    TournamentScoreboard,
    BonusWheel,
    NetServers,
    NetEditServers,
    NetLobby,
    NetNewRoom,
    NetNewRoomSettings,
    NetRoom,
    ModeSettings(u8),
    BossSettings,
}

/// A menu that `init` builds.
struct Slot<T>(Option<Box<T>>);

impl<T> Slot<T> {
    const fn empty() -> Self {
        Slot(None)
    }
}

impl<T> Deref for Slot<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_deref().expect("menu not built")
    }
}

impl<T> DerefMut for Slot<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_deref_mut().expect("menu not built")
    }
}

/// Stores `value` in `slot`; the new menu is built before the old one is dropped.
fn assign<T>(slot: &mut Slot<T>, value: Box<T>) {
    slot.0 = Some(value);
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
}

/// The menu's error bar, near the bottom of the screen, with `text` in it.
pub fn draw_message_bar(g: &mut Game, text: &str) {
    let mut gp = g.ptr();
    gp.rm.sprite(gp.rm.spr_selectfield).draw_src(g, 70, 400, &rect(0, 0, 484, 32));
    gp.rm.sprite(gp.rm.spr_selectfield).draw_src(g, 554, 400, &rect(496, 0, 16, 32));
    gp.rm.menu_font_large.draw_chop_centered(g, 320, 405, 470, text);
}

/// The menu game state; one instance, built by `init` once game data is loaded.
pub struct MenuState {
    mCurrentMenu: MenuId,
    mMainMenu: Slot<UI_MainMenu>,

    // Options menu
    mOptionsMenu: Slot<UI_OptionsMenu>,
    mGameplayOptionsMenu: Slot<UI_GameplayOptionsMenu>,
    mTeamOptionsMenu: Slot<UI_TeamOptionsMenu>,
    mPowerupDropRatesMenu: Slot<UI_PowerupDropRatesMenu>,
    mPowerupSettingsMenu: Slot<UI_PowerupSettingsMenu>,
    mProjectileLimitsMenu: Slot<UI_ProjectileLimitsMenu>,
    mProjectileOptionsMenu: Slot<UI_ProjectileOptionsMenu>,
    mGraphicsOptionsMenu: Slot<UI_GraphicsOptionsMenu>,
    mEyeCandyOptionsMenu: Slot<UI_EyeCandyOptionsMenu>,
    mSoundOptionsMenu: Slot<UI_SoundOptionsMenu>,
    mStatsMenu: Slot<crate::smw::menu::options::stats_menu::UI_StatsMenu>,

    // Controls menu
    mPlayerControlsSelectMenu: Slot<UI_PlayerControlsSelectMenu>,
    mPlayerControlsMenu: Slot<UI_PlayerControlsMenu>,

    // Gameplay menus
    mModeOptionsMenu: Slot<UI_ModeOptionsMenu>,
    mMatchSelectionMenu: Slot<UI_MatchSelectionMenu>,
    mGameSettingsMenu: Slot<UI_GameSettingsMenu>,
    mMapFilterEditMenu: Slot<UI_MapFilterEditMenu>,
    mTourStopMenu: Slot<UI_TourStopMenu>,
    mWorldMenu: Slot<UI_WorldMenu>,
    mTeamSelectMenu: Slot<UI_TeamSelectMenu>,
    mTournamentScoreboardMenu: Slot<UI_TournamentScoreboardMenu>,
    mBonusWheelMenu: Slot<UI_BonusWheelMenu>,

    // Multiplayer menus
    mNetServersMenu: Slot<UI_NetServersMenu>,
    mNetEditServersMenu: Slot<UI_NetEditServersMenu>,
    mNetLobbyMenu: Slot<UI_NetLobbyMenu>,
    mNetNewRoomMenu: Slot<UI_NetNewRoomMenu>,
    mNetNewRoomSettingsMenu: Slot<UI_NetNewRoomSettingsMenu>,
    mNetRoomMenu: Slot<UI_NetRoomMenu>,

    /// Error banner shown, for `iDisplayErrorTimer` more frames.
    iDisplayError: DisplayError,
    iDisplayErrorTimer: i16,
    /// Message shown with `DISPLAY_ERROR_TEXT`.
    error_text: String,
    /// Frames on the servers menu; LAN servers merge into the list once a second.
    nearby_timer: u32,
    /// The new-room settings menu is open from a room, to pick the next match's map and mode.
    room_settings: bool,
    /// Set in a tournament, tour or world, so leaving it to the main menu restarts the menu music.
    fNeedMenuMusicReset: bool,


    /// Progress through the minigame unlock code; nothing reads it.
    iUnlockMinigameOptionIndex: i16,

    /// Step of the AI setting up a tournament game: 1 map, 2 mode, 3 goal, 4 start; `iTournamentAITimer` frames apart.
    iTournamentAIStep: i16,
    iTournamentAITimer: i16,

    _alias: Aliased,
}

impl MenuId {
    fn mode_settings(iMode: i16) -> MenuId {
        MenuId::ModeSettings(iMode as u8)
    }
}

impl MenuState {
    fn new() -> Self {
        MenuState {
            mCurrentMenu: MenuId::None,
            mMainMenu: Slot::empty(),
            mOptionsMenu: Slot::empty(),
            mGameplayOptionsMenu: Slot::empty(),
            mTeamOptionsMenu: Slot::empty(),
            mPowerupDropRatesMenu: Slot::empty(),
            mPowerupSettingsMenu: Slot::empty(),
            mProjectileLimitsMenu: Slot::empty(),
            mProjectileOptionsMenu: Slot::empty(),
            mGraphicsOptionsMenu: Slot::empty(),
            mEyeCandyOptionsMenu: Slot::empty(),
            mSoundOptionsMenu: Slot::empty(),
            mStatsMenu: Slot::empty(),
            mPlayerControlsSelectMenu: Slot::empty(),
            mPlayerControlsMenu: Slot::empty(),
            mModeOptionsMenu: Slot::empty(),
            mMatchSelectionMenu: Slot::empty(),
            mGameSettingsMenu: Slot::empty(),
            mMapFilterEditMenu: Slot::empty(),
            mTourStopMenu: Slot::empty(),
            mWorldMenu: Slot::empty(),
            mTeamSelectMenu: Slot::empty(),
            mTournamentScoreboardMenu: Slot::empty(),
            mBonusWheelMenu: Slot::empty(),
            mNetServersMenu: Slot::empty(),
            mNetEditServersMenu: Slot::empty(),
            mNetLobbyMenu: Slot::empty(),
            mNetNewRoomMenu: Slot::empty(),
            mNetNewRoomSettingsMenu: Slot::empty(),
            mNetRoomMenu: Slot::empty(),
            iDisplayError: DISPLAY_ERROR_NONE,
            iDisplayErrorTimer: 0,
            error_text: String::new(),
            nearby_timer: 0,
            room_settings: false,
            fNeedMenuMusicReset: false,
            iUnlockMinigameOptionIndex: 0,
            iTournamentAIStep: 0,
            iTournamentAITimer: 0,
            _alias: Aliased::new(),
        }
    }

    /// The menu `mCurrentMenu` names.
    fn current(&mut self) -> &'static mut UI_Menu {
        let this: *mut MenuState = self;
        let this = unsafe { &mut *this };
        match this.mCurrentMenu {
            MenuId::None => panic!("no current menu"),
            MenuId::Main => &mut **this.mMainMenu,
            MenuId::Options => &mut **this.mOptionsMenu,
            MenuId::GameplayOptions => &mut **this.mGameplayOptionsMenu,
            MenuId::TeamOptions => &mut **this.mTeamOptionsMenu,
            MenuId::PowerupDropRates => &mut **this.mPowerupDropRatesMenu,
            MenuId::PowerupSettings => &mut **this.mPowerupSettingsMenu,
            MenuId::ProjectileLimits => &mut **this.mProjectileLimitsMenu,
            MenuId::ProjectileOptions => &mut **this.mProjectileOptionsMenu,
            MenuId::GraphicsOptions => &mut **this.mGraphicsOptionsMenu,
            MenuId::EyeCandyOptions => &mut **this.mEyeCandyOptionsMenu,
            MenuId::SoundOptions => &mut **this.mSoundOptionsMenu,
            MenuId::Stats => &mut **this.mStatsMenu,
            MenuId::PlayerControlsSelect => &mut **this.mPlayerControlsSelectMenu,
            MenuId::PlayerControls => &mut **this.mPlayerControlsMenu,
            MenuId::ModeOptions => &mut **this.mModeOptionsMenu,
            MenuId::MatchSelection => &mut **this.mMatchSelectionMenu,
            MenuId::GameSettings => &mut **this.mGameSettingsMenu,
            MenuId::MapFilterEdit => &mut **this.mMapFilterEditMenu,
            MenuId::TourStop => &mut **this.mTourStopMenu,
            MenuId::World => &mut **this.mWorldMenu,
            MenuId::TeamSelect => &mut **this.mTeamSelectMenu,
            MenuId::TournamentScoreboard => &mut **this.mTournamentScoreboardMenu,
            MenuId::BonusWheel => &mut **this.mBonusWheelMenu,
            MenuId::NetServers => &mut **this.mNetServersMenu,
            MenuId::NetEditServers => &mut **this.mNetEditServersMenu,
            MenuId::NetLobby => &mut **this.mNetLobbyMenu,
            MenuId::NetNewRoom => &mut **this.mNetNewRoomMenu,
            MenuId::NetNewRoomSettings => &mut **this.mNetNewRoomSettingsMenu,
            MenuId::NetRoom => &mut **this.mNetRoomMenu,
            MenuId::ModeSettings(mode) => unsafe { &mut *this.mModeOptionsMenu.get_options_menu(mode as i16).as_ptr() },
            MenuId::BossSettings => unsafe { &mut *this.mModeOptionsMenu.get_boss_options_menu().as_ptr() },
        }
    }

    /// The world map menu, for controls that add eye candy to it.
    pub fn world_menu(&mut self) -> Ptr<UI_WorldMenu> {
        Ptr::from_mut(&mut *self.mWorldMenu)
    }

    /// The singleton, created on first use.
    pub fn instance(g: &mut Game) -> &'static mut MenuState {
        g.ptr().get().ms.get_or_insert_with(MenuState::new)
    }

    /// A pad plugged in after launch changed who plays (main.rs `assign_hotplugged`). Team select takes the change,
    /// and the teams the game settings menu already has are rebuilt from it.
    pub fn players_changed(&mut self, g: &mut Game) {
        if self.mCurrentMenu == MenuId::None {
            return;
        }
        self.mTeamSelectMenu.players_changed(g);
        if self.mCurrentMenu == MenuId::GameSettings {
            g.score_cnt = self.mTeamSelectMenu.get_team_count(g);
        }
    }

    /// The current menu's name in harness dumps, or `unknown`.
    pub fn harness_menu_name(&self) -> &'static str {
        let names: [(MenuId, &'static str); 29] = [
            (MenuId::Main, "main"),
            (MenuId::Options, "options"),
            (MenuId::GameplayOptions, "gameplay_options"),
            (MenuId::TeamOptions, "team_options"),
            (MenuId::PowerupDropRates, "powerup_drop_rates"),
            (MenuId::PowerupSettings, "powerup_settings"),
            (MenuId::ProjectileLimits, "projectile_limits"),
            (MenuId::ProjectileOptions, "projectile_options"),
            (MenuId::GraphicsOptions, "graphics_options"),
            (MenuId::EyeCandyOptions, "eyecandy_options"),
            (MenuId::SoundOptions, "sound_options"),
            (MenuId::Stats, "stats"),
            (MenuId::PlayerControlsSelect, "player_controls_select"),
            (MenuId::PlayerControls, "player_controls"),
            (MenuId::ModeOptions, "mode_options"),
            (MenuId::MatchSelection, "match_selection"),
            (MenuId::GameSettings, "game_settings"),
            (MenuId::MapFilterEdit, "map_filter_edit"),
            (MenuId::TourStop, "tour_stop"),
            (MenuId::World, "world"),
            (MenuId::TeamSelect, "team_select"),
            (MenuId::TournamentScoreboard, "tournament_scoreboard"),
            (MenuId::BonusWheel, "bonus_wheel"),
            (MenuId::NetServers, "net_servers"),
            (MenuId::NetEditServers, "net_edit_servers"),
            (MenuId::NetLobby, "net_lobby"),
            (MenuId::NetNewRoom, "net_new_room"),
            (MenuId::NetNewRoomSettings, "net_new_room_settings"),
            (MenuId::NetRoom, "net_room"),
        ];
        for (m, name) in names.iter() {
            if *m == self.mCurrentMenu {
                return name;
            }
        }
        "unknown"
    }

    /// The names an `await` line can give the current menu: its dump name, and for `mode_settings` and
    /// `boss_settings`, which the dump calls `unknown`, that name too.
    pub fn harness_names(&self) -> Vec<&'static str> {
        match self.mCurrentMenu {
            MenuId::ModeSettings(_) => vec!["mode_settings", "unknown"],
            MenuId::BossSettings => vec!["boss_settings", "unknown"],
            _ => vec![self.harness_menu_name()],
        }
    }

    /// Index of the focused control in the current menu, or -1 with no menu.
    pub fn harness_focus_index(&mut self) -> i32 {
        if self.mCurrentMenu != MenuId::None {
            self.current().current_focus_index()
        } else {
            -1
        }
    }

    /// Whether the focused control is being modified.
    pub fn harness_modifying(&mut self) -> bool {
        self.mCurrentMenu != MenuId::None && self.current().is_modifying()
    }

    /// Loads the selected tour's stops, at most 10, and shows the current one; false if it has none.
    fn read_tour_file(&mut self, g: &mut Game) -> bool {
        reset_tour_stops(g);

        let fp = std::fs::File::open(g.tourlist.at(g.game_values.tourindex as usize)).expect("fopen tour file");
        let ignorable_leads: &[u8] = b" #\n\r\t";

        let mut fReadVersion = false;
        let mut version = Version::default();
        for buffer in fgets_lines(fp) {
            if g.game_values.tourstops.len() >= 10 {
                break;
            }

            // An empty line is skipped like a comment.
            let lead = buffer.as_bytes().first().copied().unwrap_or(0);
            if lead == 0 || ignorable_leads.contains(&lead) {
                continue;
            }

            if !fReadVersion {
                fReadVersion = true;

                let mut st = Strtok::new(buffer.as_bytes());
                if let Some(psz) = st.tok(b".\n") {
                    version.major = st.atoi(psz) as u8;
                }

                if let Some(psz) = st.tok(b".\n") {
                    version.minor = st.atoi(psz) as u8;
                }

                if let Some(psz) = st.tok(b".\n") {
                    version.patch = st.atoi(psz) as u8;
                }

                if let Some(psz) = st.tok(b".\n") {
                    version.build = st.atoi(psz) as u8;
                }

                continue;
            }

            let ts: TourStop = parse_tour_stop_line(g, buffer.as_bytes(), &version, false);
            g.game_values.tourstops.push(Box::new(ts));
        }

        if !g.game_values.tourstops.is_empty() {
            g.ctl(self.mTourStopMenu.miTourStop).refresh(g, g.game_values.tourstopcurrent as i16);

            //For old tours, turn on the bonus wheel at the end
            if version.major == 1 && version.minor == 7 && version.patch == 0 && version.build <= 1 {
                let last = g.game_values.tourstops.len() - 1;
                g.game_values.tourstops[last].iBonusType = 1;
            }
        }

        !g.game_values.tourstops.is_empty()
    }

    /// Leaves the menu for a match: saves the config, loads skins, announcer and music, and starts the fade.
    fn start_game(&mut self, g: &mut Game) {
        let mut gp = g.ptr();

        g.game_values.appstate = AppState::StartGame;

        gp.game_values.write_config(g);

        //Load skins for players
        for k in 0..4usize {
            if g.game_values.playercontrol[k] > 0 {
                if g.netplay.active {
                    if g.netplay.is_local_slot(k) {
                        // local player uses local skin
                        let skin = gp.rm.load_full_skin(g, gp.game_values.skinids[k], gp.game_values.colorids[k]);
                        g.rm.set_strip(g.rm.spr_player[k], skin);
                    } else {
                        let path = format!("{}net_skin{}.bmp", get_home_directory(g), k);

                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gp.rm.load_full_skin_path(g, Path::new(&path), k as i16)));
                        match result {
                            Ok(skin) => g.rm.set_strip(g.rm.spr_player[k], skin),
                            Err(_) => {
                                println!("[warning] Could not load netplay skin of player {}, using default", k);
                                let skin = gp.rm.load_full_skin(g, gp.game_values.skinids[k], gp.game_values.colorids[k]);
                                g.rm.set_strip(g.rm.spr_player[k], skin);
                            }
                        }
                    }
                } else if g.game_values.randomskin[k] {
                    g.game_values.skinids[k] = RANDOM_INT(gp.get(), g.skinlist.count() as i32) as i16;
                    let skin = gp.rm.load_full_skin(g, gp.game_values.skinids[k], gp.game_values.colorids[k]);
                    g.rm.set_strip(g.rm.spr_player[k], skin);
                } else {
                    let skin = gp.rm.load_full_skin(g, gp.game_values.skinids[k], gp.game_values.colorids[k]);
                    g.rm.set_strip(g.rm.spr_player[k], skin);
                }
            }
        }

        //Load announcer sounds if changed
        if g.game_values.loadedannouncer != g.announcerlist.current_index() {
            g.game_values.loadedannouncer = g.announcerlist.current_index();

            //Delete the old sounds
            for k in 0..PANNOUNCER_SOUND_LAST as usize {
                g.rm.sfx_announcer[k] = sfxSound::new();
            }

            let announcerfile = std::fs::File::open(g.announcerlist.current_path()).expect("fopen announcer file");

            let mut announcerIndex: i32 = 0;

            for mut szBuffer in fgets_lines(announcerfile) {
                if announcerIndex >= PANNOUNCER_SOUND_LAST {
                    break;
                }

                //Ignore comment lines
                let lead = szBuffer.as_bytes().first().copied().unwrap_or(0);
                if lead == b'#' || lead == b' ' || lead == b'\t' || lead == b'\n' || lead == b'\r' {
                    continue;
                }

                //Clean off carriage returns
                if let Some(k) = szBuffer.find(['\r', '\n']) {
                    szBuffer.truncate(k);
                }

                //If it is not "[none]", then add this announcer sound
                if szBuffer != "[none]" {
                    let sound = sfxSound::from_file_or_silence(gp.get(), Path::new(&convert_path(g, &szBuffer)));
                    g.rm.sfx_announcer[announcerIndex as usize] = sound;
                }

                announcerIndex += 1;
            }
        }

        //Load soundtrack music if changed
        if g.game_values.loadedmusic as i64 != g.musiclist.current_index() as i64 {
            g.game_values.loadedmusic = g.musiclist.current_index() as i16;
            g.backgroundmusic[1] = sfxMusic::from_file_or_silence(g, gp.musiclist.music(0)); //Stage Clear
            g.backgroundmusic[3] = sfxMusic::from_file_or_silence(g, gp.musiclist.music(2)); //Tournament Menu
            g.backgroundmusic[4] = sfxMusic::from_file_or_silence(g, gp.musiclist.music(3)); //Tournament Over
        }

        gp.backgroundmusic[2].stop(g);
        gp.rm.sfx_announcer[11].play(g);

        g.game_values.screenfade = 8;
        g.game_values.screenfadespeed = 8;
    }

    /// Gives game setup to team `iControlTeam` (-1 for all) and arms the AI timer if that team has no humans.
    fn set_controlling_team_for_settings_menu(&mut self, g: &mut Game, iControlTeam: i16, fDisplayMessage: bool) {
        self.mGameSettingsMenu.set_controlling_team(iControlTeam);

        self.mModeOptionsMenu.set_controlling_team(iControlTeam);

        g.ctl(self.mGameSettingsMenu.miMapFilterScroll).set_controlling_team(iControlTeam);
        g.ctl(self.mMapFilterEditMenu.miMapBrowser).set_controlling_team(iControlTeam);

        if fDisplayMessage {
            self.display_controlling_team_message(g, iControlTeam);
        }

        //Scan controlling team members and if they are only bots, then let the cpu control the selection
        self.iTournamentAITimer = 0;
        self.iTournamentAIStep = 0;
        self.mGameSettingsMenu.set_allow_exit(false);

        if iControlTeam >= 0 {
            let mut fNeedAI = true;
            for iPlayer in 0..g.game_values.teamcounts[iControlTeam as usize] {
                if g.game_values.playercontrol[g.game_values.teamids[iControlTeam as usize][iPlayer as usize] as usize] == 1 {
                    fNeedAI = false;
                    break;
                }
            }

            if fNeedAI {
                self.iTournamentAITimer = ticks(60) as i16;
                self.mGameSettingsMenu.set_allow_exit(true);
            }
        }
    }

    /// Announces who controls game setup.
    fn display_controlling_team_message(&mut self, g: &mut Game, iControlTeam: i16) {
        //Display the team that is in control of selecting the next game
        let szMessage = if iControlTeam < 0 {
            "All Teams Are In Control".to_string()
        } else if g.game_values.teamcounts[iControlTeam as usize] <= 1 {
            format!("Player {} Is In Control", g.game_values.teamids[iControlTeam as usize][0] + 1)
        } else {
            format!("Team {} Is In Control", iControlTeam + 1)
        };

        let iIcon: i16 = if iControlTeam < 0 { 4 } else { g.game_values.colorids[g.game_values.teamids[iControlTeam as usize][0] as usize] };
        self.current().eyeCandy.emplace(EC_Announcement::new(
            Ptr::from_mut(&mut g.rm.menu_font_large),
            g.rm.spr_announcementicons,
            szMessage,
            iIcon,
            ticks(120) as i16,
            100,
        ));
    }

    /// Quits, except on the web, where a page cannot close its tab.
    fn exit(&mut self, g: &mut Game) {
        if !cfg!(target_os = "emscripten") {
            self.quit(g);
        }
    }

    /// `exit` without its web early return. The main menu's Exit button uses it, so the web build
    /// ends the session and the page shows its game-over overlay (web_quit_if_done in main.rs).
    fn quit(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        g.game_values.appstate = AppState::Quit;
        gp.game_values.write_config(g);
    }

    /// Returns to the main menu after a match series, restarting the menu music if needed.
    fn reset_tournament_back_to_main_menu(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.mCurrentMenu = MenuId::Main;
        self.current().reset_menu(g);

        if g.game_values.matchtype != MatchType::SingleGame && g.game_values.matchtype != MatchType::QuickGame && g.game_values.matchtype != MatchType::MiniGame && g.game_values.matchtype != MatchType::NetGame {
            if self.fNeedMenuMusicReset {
                gp.backgroundmusic[3].stop(g);
                gp.backgroundmusic[2].play(g, false, false);
                self.fNeedMenuMusicReset = false;
            }
        }
    }

    /// The `MENU_CODE_TO_GAME_SETUP_MENU` branch of `MenuState::update`.
    fn to_game_setup_menu(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Moves teams to the first arrays in the list and counts the number of teams
        g.score_cnt = self.mTeamSelectMenu.get_team_count(g);
        self.iDisplayError = DISPLAY_ERROR_NONE;
        self.iDisplayErrorTimer = 0;
        let mut fErrorReadingTourFile = false;

        if MatchType::MiniGame == g.game_values.matchtype {
            let minigame: Minigame = self.mMatchSelectionMenu.get_minigame(g);
            match minigame {
                Minigame::PipeCoin => {
                    g.pipegamemode.goal = 50;
                    g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Pipe);
                }
                Minigame::HammerBoss | Minigame::BombBoss | Minigame::FireBoss => {
                    g.game_values.gamemodemenusettings.boss.difficulty = 2;
                    g.game_values.gamemodemenusettings.boss.hitpoints = 5;

                    g.bossgamemode.goal = 5;
                    g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Boss);
                }
                Minigame::Boxes => {
                    g.boxesgamemode.goal = 10;
                    g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Boxes);
                }
            }
            match minigame {
                Minigame::HammerBoss => {
                    g.game_values.gamemodemenusettings.boss.bosstype = Boss::Hammer;
                }
                Minigame::BombBoss => {
                    g.game_values.gamemodemenusettings.boss.bosstype = Boss::Bomb;
                }
                Minigame::FireBoss => {
                    g.game_values.gamemodemenusettings.boss.bosstype = Boss::Fire;
                }
                _ => {}
            }
            self.start_game(g);
        } else if MatchType::QuickGame == g.game_values.matchtype {
            let iRandomMode = RANDOM_INT(gp.get(), GAMEMODE_LAST) as i16;
            g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((iRandomMode as usize) as u8));

            //Choose a goal from the lower values for a quicker game
            let iRandOption = (RANDOM_INT(gp.get(), 6) + 1) as i16;
            let goal = g.mode().get_options()[iRandOption as usize].iValue;
            g.mode().goal = goal;

            g.game_values.tournamentwinner = -1;

            self.start_game(g);
        } else {
            //Load the tour here if one was selected
            if g.game_values.matchtype == MatchType::Tour {
                if !self.read_tour_file(g) {
                    self.iDisplayError = DISPLAY_ERROR_READ_TOUR_FILE;
                    self.iDisplayErrorTimer = ticks(120) as i16;
                    fErrorReadingTourFile = true;
                } else {
                    let numGames = g.game_values.tourstops.len() as i16;
                    g.ctl(self.mTournamentScoreboardMenu.miTournamentScoreboard).create_scoreboard(g, gp.score_cnt, numGames, gp.rm.spr_tour_markers);
                }
            } else if g.game_values.matchtype == MatchType::Tournament {
                //Set who is controlling the tournament menu
                if g.game_values.tournamentcontrolstyle == TournamentControlStyle::Random || g.game_values.tournamentcontrolstyle == TournamentControlStyle::RandomLoser {
                    //Random
                    g.game_values.tournamentcontrolteam = RANDOM_INT(gp.get(), g.score_cnt as i32) as i16;
                } else if g.game_values.tournamentcontrolstyle == TournamentControlStyle::RoundRobin {
                    //Round robin
                    g.game_values.tournamentcontrolteam = 0;
                } else {
                    //The first game of the tournament is controlled by all players
                    g.game_values.tournamentcontrolteam = -1;
                }

                g.game_values.tournamentnextcontrol = 1; //if round robin is selected, choose the next team next

                let numGames = g.game_values.tournamentgames;
                g.ctl(self.mTournamentScoreboardMenu.miTournamentScoreboard).create_scoreboard(g, gp.score_cnt, numGames, gp.rm.menu_mode_large);
            } else if g.game_values.matchtype == MatchType::World {
                let this = Ptr::from_mut(self);
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut this = this;
                    gp.get().g_worldmap.init(WorldMap::new_path(g, &gp.get().worldlist.at(gp.get().game_values.worldindex as usize).to_string_lossy(), TILESIZE as i16));

                    g.ctl(this.mTournamentScoreboardMenu.miTournamentScoreboard).create_scoreboard(g, gp.score_cnt, 0, gp.rm.spr_tour_markers);

                    gp.g_worldmap.set_initial_powerups(g);

                    //If a player had a stored powerup from another game, add it to their inventory
                    for iPlayer in 0..4i16 {
                        if g.game_values.storedpowerups[iPlayer as usize] != -1 {
                            let iTeamId = lookup_team_id(g, iPlayer) as usize;
                            if g.game_values.worldpowerupcount[iTeamId] < 32 {
                                let count = g.game_values.worldpowerupcount[iTeamId] as usize;
                                g.game_values.worldpowerupcount[iTeamId] += 1;
                                g.game_values.worldpowerups[iTeamId][count] = g.game_values.storedpowerups[iPlayer as usize];
                            }

                            g.game_values.storedpowerups[iPlayer as usize] = -1;
                        }
                    }

                    g.ctl(this.mWorldMenu.miWorld).init(g);
                    let team = RANDOM_INT(gp.get(), g.score_cnt as i32) as i16;
                    g.ctl(this.mWorldMenu.miWorld).set_controlling_team(g, team);
                }));
                if result.is_err() {
                    self.iDisplayError = DISPLAY_ERROR_READ_WORLD_FILE;
                    self.iDisplayErrorTimer = ticks(120) as i16;
                    fErrorReadingTourFile = true;
                }
            }

            if !fErrorReadingTourFile {
                self.mTournamentScoreboardMenu.eyeCandy.clean();

                //Initialize tournament values
                g.game_values.tournamentwinner = -1;

                //Setup wins counters for tournament/tour
                for k in 0..4usize {
                    g.game_values.tournament_scores[k].wins = 0;
                    g.game_values.tournament_scores[k].total = 0;
                }

                if MatchType::SingleGame == g.game_values.matchtype || MatchType::Tournament == g.game_values.matchtype || MatchType::NetGame == g.game_values.matchtype {
                    let szCurrentMapName = g.ctl(self.mGameSettingsMenu.miMapField).szMapName.clone();
                    g.maplist.findexact(&szCurrentMapName, false);
                    g.ctl(self.mGameSettingsMenu.miMapField).load_current_map(g);

                    g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((self.mGameSettingsMenu.get_current_game_mode_id(g) as usize) as u8));

                    for iMode in 0..GAMEMODE_LAST as usize {
                        g.gamemodes[iMode].goal = g.ctl(self.mGameSettingsMenu.miGoalField[iMode]).current_value();
                    }

                    if self.mGameSettingsMenu.get_current_game_mode_id(g) as GameModeType == game_mode_owned {
                        self.mGameSettingsMenu.hide_gm_settings_btn(g);
                    }

                    self.mCurrentMenu = MenuId::GameSettings;
                    self.current().reset_menu(g);

                    //If it is a tournament, then set the controlling team
                    if MatchType::Tournament == g.game_values.matchtype {
                        self.set_controlling_team_for_settings_menu(g, g.game_values.tournamentcontrolteam, true);
                    }
                } else if MatchType::Tour == g.game_values.matchtype {
                    self.mCurrentMenu = MenuId::TourStop;
                    self.current().reset_menu(g);
                } else if MatchType::World == g.game_values.matchtype {
                    g.game_values.screenfadespeed = 8;
                    g.game_values.screenfade = 8;
                    g.game_values.appstate = AppState::StartWorld;

                    //Play enter world sound
                    gp.rm.sfx_enterstage.play(g);
                }

                //Setup items on next menu
                for iGameMode in 0..GAMEMODE_LAST as usize {
                    g.ctl(self.mGameSettingsMenu.miGoalField[iGameMode]).hide_item(-1, g.game_values.matchtype == MatchType::Tournament);
                }

                if g.game_values.matchtype == MatchType::World {
                    self.mGameSettingsMenu.set_header_text(g, "World Game Menu");
                } else if g.game_values.matchtype == MatchType::Tour {
                    self.mGameSettingsMenu.set_header_text(g, "Tour Game Menu");
                } else if g.game_values.matchtype == MatchType::Tournament {
                    self.mGameSettingsMenu.set_header_text(g, "Tournament Game Menu");
                } else {
                    self.mGameSettingsMenu.set_header_text(g, "Single Game Menu");
                }
            }
        }
    }

    /// Picks the team that sets up the next tournament game, per `tournamentcontrolstyle`.
    fn choose_next_tournament_control_team(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Set the next controlling team
        match g.game_values.tournamentcontrolstyle {
            TournamentControlStyle::GameWinner => {
                //Winning Team
                g.game_values.tournamentcontrolteam = g.mode().winningteam;
            }

            TournamentControlStyle::GameLoser => {
                //Losing Team
                let mut iNumInPlace: i16 = 0;
                let mut iInPlace: [i16; 4] = [0; 4];
                let mut iLowestPlace: i16 = 0;
                for iScore in 0..g.score_cnt {
                    if g.score[iScore as usize].place == iLowestPlace {
                        iInPlace[iNumInPlace as usize] = iScore;
                        iNumInPlace += 1;
                    } else if g.score[iScore as usize].place > iLowestPlace {
                        iNumInPlace = 1;
                        iInPlace[0] = iScore;
                        iLowestPlace = g.score[iScore as usize].place;
                    }
                }

                g.game_values.tournamentcontrolteam = iInPlace[RANDOM_INT(gp.get(), iNumInPlace as i32) as usize];
            }

            TournamentControlStyle::LeadingTeams => {
                //Tournament Ahead Teams
                let mut iNumInPlace: i16 = 0;
                let mut iInPlace: [i16; 4] = [0; 4];
                let mut iMostWins: i16 = 0;
                for iTeam in 0..g.score_cnt {
                    if g.game_values.tournament_scores[iTeam as usize].wins == iMostWins {
                        iInPlace[iNumInPlace as usize] = iTeam;
                        iNumInPlace += 1;
                    } else if g.game_values.tournament_scores[iTeam as usize].wins > iMostWins {
                        iNumInPlace = 1;
                        iInPlace[0] = iTeam;
                        iMostWins = g.game_values.tournament_scores[iTeam as usize].wins;
                    }
                }

                g.game_values.tournamentcontrolteam = iInPlace[RANDOM_INT(gp.get(), iNumInPlace as i32) as usize];
            }

            TournamentControlStyle::TrailingTeams => {
                //Tournament Behind Teams
                let mut iNumInPlace: i16 = 0;
                let mut iInPlace: [i16; 4] = [0, 0, 0, 0];
                let mut iLeastWins: i16 = 20; //Most possible wins are 10

                for iTeam in 0..g.score_cnt {
                    if g.game_values.tournament_scores[iTeam as usize].wins == iLeastWins {
                        iInPlace[iNumInPlace as usize] = iTeam;
                        iNumInPlace += 1;
                    } else if g.game_values.tournament_scores[iTeam as usize].wins < iLeastWins {
                        iNumInPlace = 1;
                        iInPlace[0] = iTeam;
                        iLeastWins = g.game_values.tournament_scores[iTeam as usize].wins;
                    }
                }

                g.game_values.tournamentcontrolteam = iInPlace[RANDOM_INT(gp.get(), iNumInPlace as i32) as usize];
            }

            TournamentControlStyle::Random => {
                //Random
                g.game_values.tournamentcontrolteam = RANDOM_INT(gp.get(), g.score_cnt as i32) as i16;
            }

            TournamentControlStyle::RandomLoser => {
                //Random Losing Team
                let mut iNumInPlace: i16 = 0;
                let mut iInPlace: [i16; 4] = [0, 0, 0, 0];
                let mut iWinner: i16 = 0;

                for iTeam in 0..g.score_cnt {
                    if g.score[iTeam as usize].place == 0 {
                        iWinner = iTeam;
                        break;
                    }
                }

                for iTeam in 0..g.score_cnt {
                    if iTeam == iWinner {
                        continue;
                    }

                    iInPlace[iNumInPlace as usize] = iTeam;
                    iNumInPlace += 1;
                }

                g.game_values.tournamentcontrolteam = iInPlace[RANDOM_INT(gp.get(), iNumInPlace as i32) as usize];
            }

            TournamentControlStyle::RoundRobin => {
                //Round Robin
                g.game_values.tournamentcontrolteam = g.game_values.tournamentnextcontrol;

                g.game_values.tournamentnextcontrol += 1;
                if g.game_values.tournamentnextcontrol >= g.score_cnt {
                    g.game_values.tournamentnextcontrol = 0;
                }
            }

            _ => {
                g.game_values.tournamentcontrolteam = -1;
            }
        }
    }

    /// Starts the match once the fade-out ends: applies the mode settings, loads the map and its music.
    fn enter_gameplay(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.game_values.matchtype == MatchType::QuickGame {
            self.mModeOptionsMenu.set_random_game_mode_settings(g, g.mode().gamemode as i16);
        } else if g.game_values.matchtype == MatchType::NetGame {
            // Net games use the mode settings the host sent at the sync, so every client plays the same match.
            crate::smw::net::reseed_for_match(g);
            // Map & Mode sets only `currentgamemode`; the match plays `gamemode`, which joiners take from the host.
            if g.netplay.theHostIsMe {
                g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard(g.currentgamemode as u8));
            }
        } else {
            crate::smw::gs_gameplay::set_game_mode_settings_from_menu(g);
        }

        if g.game_values.matchtype == MatchType::World && g.game_values.tourstops[g.game_values.tourstopcurrent].iStageType == 1 {
            load_match_map(g, &convert_path(gp.get(), "maps/special/two52_special_bonushouse.map"));
            crate::common::global::load_current_map_background(g);

            if g.game_values.music {
                g.backgroundmusic[0] = sfxMusic::from_file_or_silence(g, gp.worldmusiclist.current_music(WorldMusicCategory::Bonus, ""));
                gp.backgroundmusic[0].play(g, false, false);
            }
        } else {
            let mut sShortMapName = String::new();

            let mut fMiniGameMapFound = false;

            if g.game_values.matchtype == MatchType::World {
                let gm = g.mode().gamemode;
                if gm == game_mode_pipe_minigame || gm == game_mode_boss_minigame || gm == game_mode_boxes_minigame {
                    let mapFile = g.game_values.tourstops[g.game_values.tourstopcurrent].pszMapFile.clone();
                    fMiniGameMapFound = g.maplist.findexact(&mapFile, true);

                    if fMiniGameMapFound {
                        let filename = g.maplist.current_filename().to_string();
                        load_match_map(g, &filename);
                        sShortMapName = g.maplist.current_shortmapname().to_string();
                    }
                }
            }

            if g.mode().gamemode == game_mode_pipe_minigame {
                if !fMiniGameMapFound {
                    load_match_map(g, &convert_path(gp.get(), "maps/special/two52_special_pipe_minigame.map"));
                    sShortMapName = "minigamepipe".to_string();
                }
            } else if g.mode().gamemode == game_mode_boss_minigame {
                if !fMiniGameMapFound {
                    let bossType: Boss = g.game_values.gamemodesettings.boss.bosstype;
                    g.bossgamemode.set_boss_type(bossType);
                    match bossType {
                        Boss::Hammer => load_match_map(g, &convert_path(gp.get(), "maps/special/two52_special_hammerboss_minigame.map")),
                        Boss::Bomb => load_match_map(g, &convert_path(gp.get(), "maps/special/two52_special_bombboss_minigame.map")),
                        Boss::Fire => load_match_map(g, &convert_path(gp.get(), "maps/special/two52_special_fireboss_minigame.map")),
                    }
                    sShortMapName = "minigameboss".to_string();
                }
            } else if g.mode().gamemode == game_mode_boxes_minigame {
                if !fMiniGameMapFound {
                    load_match_map(g, &convert_path(gp.get(), "maps/special/two52_special_boxes_minigame.map"));
                    sShortMapName = "minigameboxes".to_string();
                }
            } else if g.game_values.matchtype == MatchType::QuickGame {
                //Load a random map for the quick game
                let szMapName = gp.maplist.random_filename(g);
                load_match_map(g, &szMapName);
                sShortMapName = strip_path_and_extension(&szMapName);

            } else if g.netplay.active {
                // NOTE: for the host, netplay.mapfilepath will be ./data/something
                // while for the other players, it's ~/.smw/net_last.map
                let mapfilepath = g.netplay.mapfilepath.clone();
                load_match_map(g, &mapfilepath);
                sShortMapName = strip_path_and_extension(&g.netplay.mapfilepath);
            } else {
                let filename = g.maplist.current_filename().to_string();
                load_match_map(g, &filename);
                sShortMapName = g.maplist.current_shortmapname().to_string();
            }

            crate::common::global::load_current_map_background(g);

            //Allows all players to start the game
            g.game_values.singleplayermode = -1;

            if g.game_values.music {
                let category = MusicCategory::ALL[g.g_map.musicCategoryID as usize];
                let background = g.g_map.szBackgroundFile.clone();
                gp.musiclist.set_random_music(g, category, &sShortMapName, &background);
                g.backgroundmusic[0] = sfxMusic::from_file_or_silence(g, gp.musiclist.current_music());
                gp.backgroundmusic[0].play(g, g.game_values.playnextmusic, false);
            }
        }

        harness::match_checkpoint(g);
        if !crate::smw::net_v2::match_starting(g) {
            enter_gameplay_tail(g);
        }
    }
}

/// The game host rolls a net game's random mode settings, when it plays with "Randomize" on (`NetGameHost::send_sync_messages`).
pub fn roll_net_game_mode_settings(g: &mut Game) {
        let mut gp = g.ptr();
    if let Some(state) = gp.get().ms.as_mut() {
        if state.mModeOptionsMenu.0.is_some() {
            state.mModeOptionsMenu.set_random_game_mode_settings(g, gp.get().currentgamemode);
        }
    }
}

/// The current mode's options as the mode options menu names them.
pub fn describe_net_game_mode_settings(g: &mut Game) -> Vec<String> {
        let mut gp = g.ptr();
    match gp.ms.as_ref() {
        Some(state) if state.mModeOptionsMenu.0.is_some() => state.mModeOptionsMenu.describe_game_mode_settings(g, gp.currentgamemode),
        _ => Vec::new(),
    }
}

/// Loads the map a match is played on; replay checkpoints (smw/checkpoint.rs) note its file.
fn load_match_map(g: &mut Game, path: &str) {
    let mut gp = g.ptr();
    gp.g_map.load_map(g, path, read_type_full);
    harness::note_match_map(g, path);
}

/// The rest of `MenuState::enter_gameplay`, after the map and its music are loaded. A segment replay
/// (smw/checkpoint.rs) restores the state up to that point and starts the match here.
pub fn enter_gameplay_tail(g: &mut Game) {
        let mut gp = g.ptr();
    g.game_values.appstate = AppState::Game;

    gp.get().g_map.predrawbackground(g, &gp.get().rm.sprite(gp.get().rm.spr_background), &gp.get().rm.sprite(gp.get().rm.spr_backmap[0]));
    gp.get().g_map.predrawforeground(g, &gp.get().rm.sprite(gp.get().rm.spr_frontmap[0]));

    gp.get().g_map.predrawbackground(g, &gp.get().rm.sprite(gp.get().rm.spr_background), &gp.get().rm.sprite(gp.get().rm.spr_backmap[1]));
    gp.get().g_map.predrawforeground(g, &gp.get().rm.sprite(gp.get().rm.spr_frontmap[1]));

    gp.g_map.setup_animated_tiles(g);
    crate::smw::gs_gameplay::load_map_objects(g, false);

    GameStateManager::instance(g).change_state_to(gp.get(), crate::smw::game_state::StateId::Gameplay);
}

impl MenuState {
    /// Handles one menu code from the current menu's input or a tap; true when `update` should return at once.
    fn handle_menu_code(&mut self, g: &mut Game, mut code: MenuCodeEnum, fGenerateMapThumbs: &mut bool) -> bool {
        let mut gp = g.ptr();
        // Shortcut to game start
        if g.netplay.active {
            if g.netplay.client.take_reply(NET_L2P_MATCH_START) {
                code = MENU_CODE_NET_ROOM_GO;
            }
        }

        if MENU_CODE_EXIT_APPLICATION == code {
            self.quit(g);
            return true;
        } else if MENU_CODE_ASK_QUIT == code {
            g.input.quit_dialog = Some(false);
        } else if MENU_CODE_TO_MAIN_MENU == code {
            self.iDisplayError = DISPLAY_ERROR_NONE;
            self.iDisplayErrorTimer = 0;
            net_end_session(g);

            self.mCurrentMenu = MenuId::Main;
        } else if MENU_CODE_BACK_TO_MATCH_SELECTION_MENU == code {
            self.mCurrentMenu = MenuId::MatchSelection;
            self.iUnlockMinigameOptionIndex = 0;
        } else if MENU_CODE_TO_MATCH_SELECTION_MENU == code {
            self.mMatchSelectionMenu.world_map_changed(g);

            self.mCurrentMenu = MenuId::MatchSelection;
            self.current().reset_menu(g);
            self.iUnlockMinigameOptionIndex = 0;
        } else if MENU_CODE_MATCH_SELECTION_START == code || MENU_CODE_QUICK_GAME_START == code {

            if MENU_CODE_QUICK_GAME_START == code {
                g.game_values.matchtype = MatchType::QuickGame;
            } else {
                g.game_values.matchtype = self.mMatchSelectionMenu.get_selected_match_type(g);
            }

            self.mTeamSelectMenu.reset_team_select(g);
            self.mCurrentMenu = MenuId::TeamSelect;
            self.current().reset_menu(g);

            if g.game_values.matchtype != MatchType::Tournament {
                g.game_values.tournamentcontrolteam = -1;
                self.set_controlling_team_for_settings_menu(g, g.game_values.tournamentcontrolteam, false);
            }
        } else if MENU_CODE_MATCH_SELECTION_MATCH_CHANGED == code {
            self.mMatchSelectionMenu.selection_changed(g);
        } else if MENU_CODE_WORLD_MAP_CHANGED == code {
            self.mMatchSelectionMenu.world_map_changed(g);
        } else if MENU_CODE_TO_OPTIONS_MENU == code {
            self.mCurrentMenu = MenuId::Options;
            self.current().reset_menu(g);
        } else if MENU_CODE_BACK_TO_OPTIONS_MENU == code {
            self.mCurrentMenu = MenuId::Options;
        } else if MENU_CODE_TO_NET_SERVERS_MENU == code {
            self.mCurrentMenu = MenuId::NetServers;
            self.current().reset_menu(g);
            net_start_session(g);
        } else if MENU_CODE_BACK_TO_GRAPHIC_OPTIONS_MENU == code {
            self.mCurrentMenu = MenuId::GraphicsOptions;
        } else if MENU_CODE_TO_CONTROLS_MENU == code {
            self.mCurrentMenu = MenuId::PlayerControlsSelect;
            self.current().reset_menu(g);
        } else if MENU_CODE_BACK_TO_CONTROLS_MENU == code {
            self.mCurrentMenu = MenuId::PlayerControlsSelect;
        } else if MENU_CODE_TO_PLAYER_1_CONTROLS == code {
            self.mPlayerControlsMenu.set_player(g, 0);
            self.mCurrentMenu = MenuId::PlayerControls;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_PLAYER_2_CONTROLS == code {
            self.mPlayerControlsMenu.set_player(g, 1);
            self.mCurrentMenu = MenuId::PlayerControls;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_PLAYER_3_CONTROLS == code {
            self.mPlayerControlsMenu.set_player(g, 2);
            self.mCurrentMenu = MenuId::PlayerControls;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_PLAYER_4_CONTROLS == code {
            self.mPlayerControlsMenu.set_player(g, 3);
            self.mCurrentMenu = MenuId::PlayerControls;
            self.current().reset_menu(g);
        } else if MENU_CODE_TOGGLE_FULLSCREEN == code {
            gfx_changefullscreen(g, g.game_values.fullscreen);
            g.blitdest = g.screen;
        } else if MENU_CODE_SHADER_CHANGED == code {
            if let Err(message) = crate::smw::video_settings::apply_shader(g) {
                let field = self.mGraphicsOptionsMenu.miShaderField;
                let failed = *g.video.failed.last().unwrap();
                g.ctl(field).set_current_value(0);
                g.ctl(field).hide_item(failed, true);
                self.show_error_text(&message);
            }
        } else if MENU_CODE_INTEGER_SCALE_CHANGED == code {
            crate::smw::video_settings::apply_integer_scale(g);
        } else if MENU_CODE_TO_GAME_SETUP_MENU == code {
            self.to_game_setup_menu(g);
        } else if MENU_CODE_BACK_TO_GAME_SETUP_MENU == code {
            let mut fNeedTeamAnnouncement = false;
            if g.game_values.matchtype == MatchType::World {
                if g.game_values.tournamentwinner == -2 || (g.game_values.tournamentwinner >= 0 && g.game_values.bonuswheel == 0) {
                    self.reset_tournament_back_to_main_menu(g);
                } else if g.game_values.tournamentwinner >= 0 {
                    g.ctl(self.mBonusWheelMenu.miBonusWheel).reset(g, true);
                    self.mCurrentMenu = MenuId::BonusWheel;
                } else {
                    self.mCurrentMenu = MenuId::World;
                    g.ctl(self.mWorldMenu.miWorldStop).refresh(g, g.game_values.tourstopcurrent as i16);

                    fNeedTeamAnnouncement = true;
                }
            } else if g.game_values.tournamentwinner == -2 {
                //Tied Tour Result
                self.reset_tournament_back_to_main_menu(g);
            } else if g.game_values.tournamentwinner >= 0 {
                //Tournament/Tour Won and Bonus Wheel will be spun
                if g.game_values.bonuswheel == 0 || (g.game_values.matchtype == MatchType::Tour && g.game_values.tourstops[g.game_values.tourstopcurrent - 1].iBonusType == 0) {
                    self.reset_tournament_back_to_main_menu(g);
                } else {
                    g.ctl(self.mBonusWheelMenu.miBonusWheel).reset(g, true);
                    self.mCurrentMenu = MenuId::BonusWheel;
                }
            } else {
                //Next Tour/Tourament Game
                if g.game_values.matchtype == MatchType::Tour {
                    self.mCurrentMenu = MenuId::TourStop;
                } else {
                    self.mCurrentMenu = MenuId::GameSettings;
                }
            }

            self.current().reset_menu(g);

            //Set the controlling team for tournament mode
            if MatchType::Tournament == g.game_values.matchtype && g.game_values.tournamentwinner == -1 {
                self.set_controlling_team_for_settings_menu(g, g.game_values.tournamentcontrolteam, true);
            }

            if fNeedTeamAnnouncement {
                g.ctl(self.mWorldMenu.miWorld).display_team_control_announcement(g);
            }
        } else if MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS == code {
            if g.netplay.active {
                self.mCurrentMenu = MenuId::NetNewRoomSettings;
            } else {
                self.mCurrentMenu = MenuId::GameSettings;
            }
        } else if MENU_CODE_MODE_CHANGED == code {
            g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((self.mGameSettingsMenu.get_current_game_mode_id(g) as usize) as u8));
            self.mGameSettingsMenu.refresh_game_mode_buttons(g);
            self.mNetNewRoomSettingsMenu.refresh_game_mode_buttons(g);
        } else if MENU_CODE_BACK_TEAM_SELECT_MENU == code {
            if g.game_values.matchtype == MatchType::World {
                self.mWorldMenu.open_exit_dialog(g);
            } else if g.game_values.matchtype == MatchType::Tour {
                self.mTourStopMenu.open_exit_dialog(g);
            } else if g.game_values.matchtype == MatchType::Tournament {
                self.mGameSettingsMenu.open_exit_dialog(g);

                if self.iTournamentAITimer > 0 {
                    self.set_controlling_team_for_settings_menu(g, -1, false);
                }
            } else {
                self.mCurrentMenu = MenuId::TeamSelect;
                self.current().reset_menu(g);
            }
        } else if MENU_CODE_SOUND_VOLUME_CHANGED == code {
            g.game_values.sound = g.game_values.soundvolume > 0;
            sfx_setsoundvolume(g, g.game_values.soundvolume as i32);
            if_sound_on_play(g, &mut gp.rm.sfx_coin);
        } else if MENU_CODE_MUSIC_VOLUME_CHANGED == code {
            sfx_setmusicvolume(g, g.game_values.musicvolume as i32);

            if g.game_values.musicvolume == 0 {
                gp.backgroundmusic[2].stop(g);
            } else if g.game_values.musicvolume > 0 && !g.game_values.music {
                gp.backgroundmusic[2].play(g, false, false);
            }

            g.game_values.music = g.game_values.musicvolume > 0;
        } else if MENU_CODE_START_GAME == code {
            self.start_game(g);
        } else if MENU_CODE_EXIT_TOURNAMENT_YES == code || MENU_CODE_EXIT_TOURNAMENT_NO == code {
            self.mGameSettingsMenu.close_exit_dialog(g);

            if MENU_CODE_EXIT_TOURNAMENT_YES == code {
                self.reset_tournament_back_to_main_menu(g);
            } else {
                self.set_controlling_team_for_settings_menu(g, g.game_values.tournamentcontrolteam, false);
            }
        } else if MENU_CODE_EXIT_TOUR_YES == code || MENU_CODE_EXIT_TOUR_NO == code {
            self.mTourStopMenu.close_exit_dialog(g);

            if MENU_CODE_EXIT_TOUR_YES == code {
                self.reset_tournament_back_to_main_menu(g);
            }
        } else if MENU_CODE_EXIT_WORLD_YES == code || MENU_CODE_EXIT_WORLD_NO == code {
            self.mWorldMenu.close_exit_dialog(g);

            if MENU_CODE_EXIT_WORLD_YES == code {
                //Clear out any stored items a player might have
                for iPlayer in 0..4usize {
                    g.game_values.storedpowerups[iPlayer] = -1;
                }

                self.reset_tournament_back_to_main_menu(g);
                g.ctl(self.mWorldMenu.miWorldStop).set_visible(false);
            }
        } else if MENU_CODE_BONUS_DONE == code {
            if g.ctl(self.mBonusWheelMenu.miBonusWheel).get_powerup_selection_done() {
                if g.game_values.matchtype == MatchType::World {
                    if g.game_values.tournamentwinner == -1 {
                        self.mCurrentMenu = MenuId::TournamentScoreboard;
                    } else {
                        self.reset_tournament_back_to_main_menu(g);
                    }
                } else if (g.game_values.matchtype == MatchType::Tour || g.game_values.matchtype == MatchType::Tournament)
                    && g.game_values.tournamentwinner == -1
                    && ((g.game_values.matchtype != MatchType::Tour && g.game_values.bonuswheel == 2)
                        || (g.game_values.matchtype == MatchType::Tour && g.game_values.tourstops[g.game_values.tourstopcurrent - 1].iBonusType != 0))
                {
                    self.mCurrentMenu = MenuId::TournamentScoreboard;
                } else if g.game_values.matchtype == MatchType::SingleGame {
                    self.mCurrentMenu = MenuId::GameSettings;
                    self.current().reset_menu(g);
                    g.ctl(self.mGameSettingsMenu.miMapField).load_current_map(g);
                } else {
                    self.reset_tournament_back_to_main_menu(g);
                }
            }
        } else if MENU_CODE_TO_POWERUP_SELECTION_MENU == code {
            self.mCurrentMenu = MenuId::PowerupDropRates;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_POWERUP_SETTINGS_MENU == code {
            self.mCurrentMenu = MenuId::PowerupSettings;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_GRAPHICS_OPTIONS_MENU == code {
            self.mCurrentMenu = MenuId::GraphicsOptions;
            self.current().reset_menu(g);
            if let Some(message) = g.video.open_error.take() {
                self.show_error_text(&message);
            }
        } else if MENU_CODE_TO_EYECANDY_OPTIONS_MENU == code {
            self.mCurrentMenu = MenuId::EyeCandyOptions;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_SOUND_OPTIONS_MENU == code {
            self.mCurrentMenu = MenuId::SoundOptions;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_STATS_MENU == code {
            self.mStatsMenu.refresh(g);
            self.mCurrentMenu = MenuId::Stats;
            self.current().reset_menu(g);
        } else if MENU_CODE_STATS_RESET == code {
            self.mStatsMenu.open_reset_dialog(g);
            self.current().reset_menu(g);
        } else if MENU_CODE_STATS_RESET_YES == code || MENU_CODE_STATS_RESET_NO == code {
            self.mStatsMenu.close_reset_dialog(g, MENU_CODE_STATS_RESET_YES == code);
        } else if MENU_CODE_TO_GAMEPLAY_OPTIONS_MENU == code {
            self.mCurrentMenu = MenuId::GameplayOptions;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_TEAM_OPTIONS_MENU == code {
            self.mCurrentMenu = MenuId::TeamOptions;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_PROJECTILES_OPTIONS_MENU == code {
            self.mCurrentMenu = MenuId::ProjectileOptions;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_PROJECTILES_LIMITS_MENU == code {
            self.mCurrentMenu = MenuId::ProjectileLimits;
            self.current().reset_menu(g);
        } else if MENU_CODE_TO_MODE_SETTINGS_MENU == code {
            if g.netplay.active {
                self.mCurrentMenu = MenuId::mode_settings(self.mNetNewRoomSettingsMenu.get_selected_game_mode_id(g));
                self.current().reset_menu(g);
            } else {
                for iGameMode in 0..GAMEMODE_LAST as i16 {
                    if g.ctl(self.mGameSettingsMenu.miGoalField[iGameMode as usize]).is_visible() {
                        self.mCurrentMenu = MenuId::mode_settings(iGameMode);
                        self.current().reset_menu(g);
                        break;
                    }
                }
            }
        } else if MENU_CODE_MENU_GRAPHICS_PACK_CHANGED == code {
            gp.rm.load_menu_graphics(g);

            g.blitdest = g.rm.sprite(g.rm.menu_backdrop).get_surface();
            g.rm.sprite(g.rm.menu_shade).setalpha(App::menuTransparency as u8);
            gp.rm.sprite(gp.rm.menu_shade).draw(g, 0, 0);
            g.blitdest = g.screen;
        } else if MENU_CODE_WORLD_GRAPHICS_PACK_CHANGED == code {
            gp.rm.load_world_graphics(g);
        } else if MENU_CODE_GAME_GRAPHICS_PACK_CHANGED == code {
            let packdir = g.gamegraphicspacklist.current_path().to_string_lossy().into_owned();
            let png = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gfx_loadpalette(gp.get(), Path::new(&convert_path_pack(gp.get(), "gfx/packs/palette.png", &packdir)))));
            if let Err(payload) = png {
                let Some(err) = payload.downcast_ref::<String>() else { std::panic::resume_unwind(payload) };
                println!("\nwarning: {} -> falling back to BMP", err);
                gfx_loadpalette(g, Path::new(&convert_path_pack(gp.get(), "gfx/packs/palette.bmp", &packdir)));
            }
            gp.rm.load_game_graphics(g);
        } else if MENU_CODE_SOUND_PACK_CHANGED == code {
            gp.rm.load_game_sounds(g);

            if !g.game_values.soundcapable {
                g.game_values.sound = false;
                g.game_values.music = false;
                g.game_values.soundvolume = 0;
                g.game_values.musicvolume = 0;
            }
        } else if MENU_CODE_WORLD_STAGE_START == code {
            self.mWorldMenu.open_stage_start(g);
        } else if MENU_CODE_WORLD_STAGE_NO_START == code {
            self.mWorldMenu.close_stage_start(g);
        } else if MENU_CODE_WORLD_MUSIC_CHANGED == code {
            let music = self.mSoundOptionsMenu.get_current_world_music_id(g) as usize;
            g.worldmusiclist.set_current(music);
        } else if MENU_CODE_TOUR_STOP_CONTINUE == code || MENU_CODE_TOUR_STOP_CONTINUE_FORCED == code {
            //If this tour stop is forced, we need to load the map first
            if MENU_CODE_TOUR_STOP_CONTINUE_FORCED == code {
                g.ctl(self.mWorldMenu.miWorldStop).refresh(g, g.game_values.tourstopcurrent as i16);
            }

            g.ctl(self.mWorldMenu.miWorld).clear_cloud();

            //Tour bonus house
            if g.game_values.matchtype == MatchType::World && g.game_values.tourstops[g.game_values.tourstopcurrent].iStageType == 1 {
                g.bonushousemode.goal = 0;
                g.game_values.gamemode = Some(crate::common::game_mode::ModeId::BonusHouse);
            } else {
                let iGameMode = g.game_values.tourstops[g.game_values.tourstopcurrent].iMode;

                if iGameMode as GameModeType == game_mode_pipe_minigame {
                    g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Pipe);
                } else if iGameMode as GameModeType == game_mode_boss_minigame {
                    g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Boss);
                } else if iGameMode as GameModeType == game_mode_boxes_minigame {
                    g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Boxes);
                } else {
                    g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((iGameMode as usize) as u8));
                }

                g.mode().goal = g.game_values.tourstops[g.game_values.tourstopcurrent].iGoal;
            }

            self.start_game(g);
        } else if MENU_CODE_RESET_STORED_POWERUPS == code {
            for iPlayer in 0..4usize {
                g.game_values.storedpowerups[iPlayer] = -1;
            }
        } else if MENU_CODE_MAP_CHANGED == code {
            if self.mCurrentMenu == MenuId::NetNewRoomSettings {
                debug_assert!(g.netplay.active);
                g.netplay.mapfilepath = self.mNetNewRoomSettingsMenu.get_current_map_path(g);
                println!("[net] Selected map: {}", g.netplay.mapfilepath);
                self.mNetRoomMenu.set_preview_map_path(g, &gp.netplay.mapfilepath);
            } else if g.game_values.matchtype != MatchType::Tour {
            }
        } else if MENU_CODE_MAP_FILTER_EXIT == code {
            let gv = gp;
            gp.maplist.apply_filters(g, &gv.game_values.pfFilters);

            //If the filtered map list has at least 1 map in it, then allow exiting the filter menu
            if g.maplist.map_in_filtered_set() {
                g.ctl(self.mGameSettingsMenu.miMapField).load_current_map(g);

                self.mGameSettingsMenu.close_map_filters(g);

                self.iDisplayError = DISPLAY_ERROR_NONE;
            } else {
                //otherwise display a message
                self.iDisplayError = DISPLAY_ERROR_MAP_FILTER;
                self.iDisplayErrorTimer = ticks(120) as i16;
            }
        } else if MENU_CODE_TO_MAP_FILTERS == code {
            self.mGameSettingsMenu.open_map_filters(g);
        } else if MENU_CODE_TO_MAP_FILTER_EDIT == code {
            g.ctl(self.mMapFilterEditMenu.miMapBrowser).reset(g, 0);

            self.mCurrentMenu = MenuId::MapFilterEdit;
            self.current().reset_menu(g);
        } else if MENU_CODE_MAP_BROWSER_EXIT == code {
            g.ctl(self.mGameSettingsMenu.miMapField).load_current_map(g);

            self.mCurrentMenu = MenuId::GameSettings;
        } else if MENU_CODE_TO_MAP_BROWSER_THUMBNAILS == code {
            g.ctl(self.mMapFilterEditMenu.miMapBrowser).reset(g, 1);

            self.mCurrentMenu = MenuId::MapFilterEdit;
            self.current().reset_menu(g);
        } else if MENU_CODE_SAVE_ALL_MAP_THUMBNAILS == code {
            // Only the options menu sends this code.
            self.mOptionsMenu.show_thumbnails_popup(g);
            self.current().reset_menu(g);
        } else if MENU_CODE_GENERATE_THUMBS_RESET_YES == code || MENU_CODE_GENERATE_THUMBS_RESET_NO == code {
            self.mOptionsMenu.hide_thumbnails_popup(g);

            if MENU_CODE_GENERATE_THUMBS_RESET_YES == code {
                *fGenerateMapThumbs = true;
            }
        } else if MENU_CODE_HEALTH_MODE_START_LIFE_CHANGED == code {
            self.mModeOptionsMenu.health_mode_start_life_changed(g);
        } else if MENU_CODE_HEALTH_MODE_MAX_LIFE_CHANGED == code {
            self.mModeOptionsMenu.health_mode_max_life_changed(g);
        }

        if g.netplay.active {
            gp.netplay.client.tick_deadline(g);
            let lobby_error = gp.netplay.client.take_lobby_error();
            if let Some(error) = lobby_error {
                g.netplay.operationInProgress = false;
                code = self.lobby_error_code(g, error, code);
            }

            // Override menu code if response has arrived
            if g.netplay.operationInProgress {
                let previousCode = code;
                let client = &mut gp.netplay.client;

                // The CONNECT_OK handler sends the skin before returning, so the lobby gets it first.
                if client.take_reply(NET_RESPONSE_CONNECT_OK) {
                    code = MENU_CODE_TO_NET_LOBBY_MENU;
                } else if client.take_reply(NET_RESPONSE_JOIN_OK) || client.take_reply(NET_RESPONSE_CREATE_OK) {
                    code = MENU_CODE_TO_NET_ROOM_MENU;
                }

                // If we have finished the waiting
                if code != previousCode {
                    g.netplay.operationInProgress = false;
                }
            }

            if MENU_CODE_TO_NET_SERVERLIST == code {
                self.mNetServersMenu.open_server_list(g);
            } else if MENU_CODE_NET_HOST_LAN == code {
                if crate::smw::net::host_on_this_device(g) {
                    self.mNetServersMenu.refresh_scroll(g);
                } else {
                    self.show_error_text("Hosting needs a desktop or handheld build");
                }
            } else if MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU == code {
                self.mCurrentMenu = MenuId::NetEditServers;
                self.current().reset_menu(g);
                self.mNetEditServersMenu.restore(g);
            } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_ADD_BTN == code {
                self.mNetEditServersMenu.on_press_add(g);
            } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_EDIT_BTN == code {
                self.mNetEditServersMenu.on_press_edit(g);
            } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_DELETE_BTN == code {
                self.mNetEditServersMenu.on_press_delete(g);
            } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_SELECT == code {
                self.mNetEditServersMenu.on_entry_select(g);
                self.mNetServersMenu.refresh_scroll(g);
            } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_DIALOG_OK_BTN == code {
                self.mNetEditServersMenu.on_dialog_ok(g);
                self.mNetServersMenu.refresh_scroll(g);
            } else if MENU_CODE_NET_SERVERLIST_EXIT == code || MENU_CODE_NET_CONNECT_ABORT == code {
                if MENU_CODE_NET_SERVERLIST_EXIT == code {
                    g.netplay.currentMenuChanged = true;
                } else {
                    net_start_session(g); // release & restart socket
                    g.netplay.operationInProgress = false;
                }

                self.mNetServersMenu.restore(g);
                self.iDisplayError = DISPLAY_ERROR_NONE;
            } else if MENU_CODE_NET_CONNECT_IN_PROGRESS == code {
                self.mNetServersMenu.connect_in_progress(g);
            } else if MENU_CODE_TO_NET_LOBBY_MENU == code {
                // If we are leaving a room
                if g.netplay.currentRoom.roomID != 0 {
                    gp.netplay.client.send_leave_room_message(g);
                    g.netplay.currentRoom.roomID = 0;
                    g.netplay.joinSuccessful = false;
                    g.netplay.gameRunning = false;
                }
                gp.netplay.client.request_room_list(g);

                // Restore layouts
                self.mNetServersMenu.restore(g);
                self.mNetLobbyMenu.restore(g);
                self.mNetNewRoomMenu.restore(g);

                self.mCurrentMenu = MenuId::NetLobby;
                self.current().reset_menu(g);
            } else if MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU == code {
                self.mCurrentMenu = MenuId::NetNewRoomSettings;
                self.current().reset_menu(g);
            } else if MENU_CODE_NET_ROOM_SETTINGS == code {
                self.room_settings = true;
                self.mCurrentMenu = MenuId::NetNewRoomSettings;
                self.current().reset_menu(g);
            } else if self.room_settings && self.mCurrentMenu == MenuId::NetNewRoomSettings
                && (MENU_CODE_TO_NET_NEW_ROOM_SETTINGS_MENU == code || MENU_CODE_TO_NET_LOBBY_MENU == code)
            {
                self.room_settings = false;
                g.netplay.mapfilepath = self.mNetNewRoomSettingsMenu.get_current_map_path(g);
                gp.netplay.client.send_room_settings(g);
                self.mCurrentMenu = MenuId::NetRoom;
                self.mNetRoomMenu.restore(g);
                self.current().reset_menu(g);
                g.netplay.currentMenuChanged = true;
            } else if MENU_CODE_TO_NET_NEW_ROOM_SETTINGS_MENU == code {
                self.mCurrentMenu = MenuId::NetNewRoom;
                self.current().reset_menu(g);
            } else if MENU_CODE_NET_ROOM_LOCAL_PLAYERS == code {
                gp.netplay.client.send_local_players(g);
            } else if MENU_CODE_NET_CHAT_SEND == code {
                if !g.netplay.mychatmessage.is_empty() {
                    let message = g.netplay.mychatmessage.clone();
                    gp.netplay.client.send_chat_message(g, &message);
                    self.mNetRoomMenu.chat_sent(g);
                } else {
                    code = MENU_CODE_TO_NET_ROOM_MENU;
                }
            } else if MENU_CODE_TO_NET_ROOM_MENU == code {
                self.mCurrentMenu = MenuId::NetRoom;
                self.current().reset_menu(g);
                g.netplay.currentMenuChanged = true;
                g.netplay.operationInProgress = false;

                self.mNetRoomMenu.restore(g); // Restore Room layout
                self.mNetLobbyMenu.restore(g); // Restore Lobby layout
                self.mNetNewRoomMenu.restore(g); // remove 'in progress' dialog
            } else if MENU_CODE_NET_JOIN_ROOM_IN_PROGRESS == code {
                self.mNetLobbyMenu.join_in_progress(g);
            } else if MENU_CODE_NET_JOIN_ROOM_PASSWORD_OK == code {
                self.mNetLobbyMenu.join_with_password(g);
            } else if MENU_CODE_NET_JOIN_ROOM_ABORT == code {
                self.mNetLobbyMenu.abort_join(g);
                self.iDisplayError = DISPLAY_ERROR_NONE;
            } else if MENU_CODE_TO_NET_NEW_ROOM_CREATE_IN_PROGRESS == code {
                self.mNetNewRoomMenu.create_in_progress(g);
            } else if MENU_CODE_TO_NET_NEW_ROOM_CREATE_ABORT == code {
                self.mNetNewRoomMenu.abort_create(g);
                self.iDisplayError = DISPLAY_ERROR_NONE;
            } else if MENU_CODE_TO_NET_ROOM_START_IN_PROGRESS == code {
                self.mNetRoomMenu.start_in_progress(g);
            } else if MENU_CODE_NET_ROOM_GO == code {
                g.netplay.operationInProgress = false;
                g.game_values.matchtype = MatchType::NetGame;
                g.score_cnt = crate::smw::net::apply_room_teams(g);

                g.game_values.tournamentcontrolteam = -1;
                g.game_values.tournamentwinner = -1;

                self.iDisplayError = DISPLAY_ERROR_NONE;
                self.iDisplayErrorTimer = 0;

                self.start_game(g);
            }

            if self.mCurrentMenu == MenuId::NetLobby {
                gp.netplay.client.update_room_list(g);
            }
            if self.mCurrentMenu == MenuId::NetServers {
                self.nearby_timer += 1;
                if self.nearby_timer % TICKS_PER_SECOND as u32 == 1 && crate::smw::net::merge_nearby(g) {
                    self.mNetServersMenu.refresh_scroll(g);
                }
            }

            // on room change
            if g.netplay.currentMenuChanged {
                self.mNetServersMenu.refresh(g);
                self.mNetRoomMenu.refresh(g);

                g.netplay.currentMenuChanged = false;
            }

            if let Some(error) = lobby_error.filter(|_| self.in_net_menu()) {
                self.show_error_text(error.message());
            }
            if self.in_net_menu() {
                if let Some(notice) = g.netplay.match_notice.take() {
                    self.show_error_text(&notice);
                }
            }
        }

        false
    }
}

impl MenuState {
    /// Whether the current menu is one of the netplay menus.
    fn show_error_text(&mut self, text: &str) {
        self.error_text = text.to_string();
        self.iDisplayError = DISPLAY_ERROR_TEXT;
        self.iDisplayErrorTimer = (4 * TICKS_PER_SECOND) as i16;
    }

    fn in_net_menu(&self) -> bool {
        matches!(
            self.mCurrentMenu,
            MenuId::NetServers | MenuId::NetEditServers | MenuId::NetLobby | MenuId::NetNewRoom | MenuId::NetNewRoomSettings | MenuId::NetRoom
        )
    }

    /// The menu code that takes the player from a failed lobby request back to a menu that works.
    fn lobby_error_code(&mut self, g: &mut Game, error: LobbyError, code: MenuCodeEnum) -> MenuCodeEnum {
        if !self.in_net_menu() || self.mCurrentMenu == MenuId::NetEditServers {
            return code;
        }
        if error.disconnects() {
            if self.mCurrentMenu == MenuId::NetServers {
                return MENU_CODE_NET_CONNECT_ABORT;
            }
            g.netplay.currentRoom.roomID = 0;
            g.netplay.joinSuccessful = false;
            g.netplay.gameRunning = false;
            self.mNetServersMenu.restore(g);
            self.mNetLobbyMenu.restore(g);
            self.mNetNewRoomMenu.restore(g);
            self.mCurrentMenu = MenuId::NetServers;
            self.current().reset_menu(g);
            net_start_session(g);
            return MENU_CODE_NONE;
        }
        match self.mCurrentMenu {
            MenuId::NetLobby => MENU_CODE_NET_JOIN_ROOM_ABORT,
            MenuId::NetNewRoom => MENU_CODE_TO_NET_NEW_ROOM_CREATE_ABORT,
            _ => code,
        }
    }
}

impl GameState for MenuState {
    fn init(&mut self, g: &mut Game) -> bool {
        assign(&mut self.mMainMenu, UI_MainMenu::new(g));

        // Options menu
        assign(&mut self.mOptionsMenu, UI_OptionsMenu::new(g));
        assign(&mut self.mGameplayOptionsMenu, UI_GameplayOptionsMenu::new(g));
        assign(&mut self.mTeamOptionsMenu, UI_TeamOptionsMenu::new(g));
        assign(&mut self.mPowerupDropRatesMenu, UI_PowerupDropRatesMenu::new(g));
        assign(&mut self.mPowerupSettingsMenu, UI_PowerupSettingsMenu::new(g));
        assign(&mut self.mProjectileLimitsMenu, UI_ProjectileLimitsMenu::new(g));
        assign(&mut self.mProjectileOptionsMenu, UI_ProjectileOptionsMenu::new(g));
        assign(&mut self.mGraphicsOptionsMenu, UI_GraphicsOptionsMenu::new(g));
        assign(&mut self.mEyeCandyOptionsMenu, UI_EyeCandyOptionsMenu::new(g));
        assign(&mut self.mSoundOptionsMenu, UI_SoundOptionsMenu::new(g));
        assign(&mut self.mStatsMenu, crate::smw::menu::options::stats_menu::UI_StatsMenu::new(g));

        // Controls menu
        assign(&mut self.mPlayerControlsSelectMenu, UI_PlayerControlsSelectMenu::new(g));
        assign(&mut self.mPlayerControlsMenu, UI_PlayerControlsMenu::new(g));

        // Gameplay menu
        assign(&mut self.mModeOptionsMenu, UI_ModeOptionsMenu::new(g));
        assign(&mut self.mMatchSelectionMenu, UI_MatchSelectionMenu::new(g));
        assign(&mut self.mGameSettingsMenu, UI_GameSettingsMenu::new(g));
        assign(&mut self.mMapFilterEditMenu, UI_MapFilterEditMenu::new(g));
        assign(&mut self.mTourStopMenu, UI_TourStopMenu::new(g));
        assign(&mut self.mWorldMenu, UI_WorldMenu::new(g));
        assign(&mut self.mTeamSelectMenu, UI_TeamSelectMenu::new(g));
        assign(&mut self.mTournamentScoreboardMenu, UI_TournamentScoreboardMenu::new(g));
        assign(&mut self.mBonusWheelMenu, UI_BonusWheelMenu::new(g));

        // Multiplayer menu
        assign(&mut self.mNetServersMenu, UI_NetServersMenu::new(g));
        assign(&mut self.mNetEditServersMenu, UI_NetEditServersMenu::new(g));
        assign(&mut self.mNetLobbyMenu, UI_NetLobbyMenu::new(g));
        assign(&mut self.mNetNewRoomMenu, UI_NetNewRoomMenu::new(g));
        assign(&mut self.mNetNewRoomSettingsMenu, UI_NetNewRoomSettingsMenu::new(g, &self.mGameSettingsMenu));
        assign(&mut self.mNetRoomMenu, UI_NetRoomMenu::new(g));

        self.mCurrentMenu = MenuId::Main;

        if let Some(forcedMap) = harness::forced_map(g) {
            if !g.maplist.findexact(forcedMap, false) && !g.maplist.find(forcedMap) {
                eprintln!("[harness] SMW_MAP '{}' not found", forcedMap);
                std::process::exit(2);
            }
            g.ctl(self.mGameSettingsMenu.miMapField).load_current_map(g);
        }


        true
    }

    //---------------------------------------------------------------
    // RUN THE MENU
    //---------------------------------------------------------------

    fn on_enter_state(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        crate::services::services(gp.get()).video.show_cursor(true);
        self.iUnlockMinigameOptionIndex = 0;

        self.iTournamentAIStep = 0;
        self.iTournamentAITimer = 0;

        //Reset the keys each time we switch from menu to game and back
        g.game_values.playerInput.reset_keys();

        self.fNeedMenuMusicReset = false;

        let matchtype = g.game_values.matchtype;
        if g.mode().winningteam > -1
            && g.game_values.tournamentwinner == -1
            && (((matchtype == MatchType::SingleGame || matchtype == MatchType::QuickGame || matchtype == MatchType::MiniGame || matchtype == MatchType::Tournament || matchtype == MatchType::NetGame)
                && g.game_values.bonuswheel == 2)
                || (matchtype == MatchType::Tour && g.game_values.tourstops[g.game_values.tourstopcurrent - 1].iBonusType != 0))
        {
            g.ctl(self.mBonusWheelMenu.miBonusWheel).reset(g, false);
            self.mCurrentMenu = MenuId::BonusWheel;
        } else if matchtype != MatchType::SingleGame && matchtype != MatchType::QuickGame && matchtype != MatchType::MiniGame && matchtype != MatchType::NetGame {
            self.mCurrentMenu = MenuId::TournamentScoreboard;
        } else if matchtype == MatchType::QuickGame {
            //Reset back to main menu after quick game
            self.mCurrentMenu = MenuId::Main;
            self.current().reset_menu(g);
        } else if matchtype == MatchType::NetGame && g.netplay.currentRoom.roomID != 0 {
            self.mCurrentMenu = MenuId::NetRoom;
            self.mNetRoomMenu.restore(g);
            self.current().reset_menu(g);
        } else if matchtype == MatchType::NetGame {
            self.mCurrentMenu = MenuId::NetLobby;
            self.current().reset_menu(g);
            g.netplay.joinSuccessful = false;
            g.netplay.gameRunning = false;
        }

        if g.game_values.matchtype == MatchType::World {
            if g.mode().winningteam > -1 && g.game_values.tournamentwinner == -1 {
                //Stage is completed
                let winningteam = g.mode().winningteam;
                g.ctl(self.mWorldMenu.miWorld).set_controlling_team(g, winningteam);
                g.ctl(self.mWorldMenu.miWorld).set_current_stage_to_completed(g, winningteam);

                self.mWorldMenu.close_stage_start(g);

                //Clear out the stored powerups after a game that had a winner
                if !g.game_values.worldskipscoreboard {
                    //Only clear out the stored powerup if we were allowed to use it in the game
                    if g.mode().has_stored_powerups() {
                        for iPlayer in 0..MAX_PLAYERS as usize {
                            g.game_values.storedpowerups[iPlayer] = -1;
                        }
                    }
                }
            }

            crate::smw::gs_gameplay::update_score_board(g);

            //If we're suposed to skip the scoreboard, then reset back to the world map
            if g.game_values.worldskipscoreboard {
                self.mCurrentMenu = MenuId::World;
                self.current().reset_menu(g);

                g.game_values.worldskipscoreboard = false;
            } else {
                let winningteam = g.mode().winningteam;
                g.ctl(self.mTournamentScoreboardMenu.miTournamentScoreboard).refresh_world_scores(g, winningteam);
            }
        } else if g.game_values.matchtype == MatchType::Tour {
            g.ctl(self.mTournamentScoreboardMenu.miTournamentScoreboard).refresh_tour_scores(g);

            if g.game_values.tourstopcurrent < g.game_values.tourstops.len() {
                g.ctl(self.mTourStopMenu.miTourStop).refresh(g, g.game_values.tourstopcurrent as i16);
            }
        } else if g.game_values.matchtype == MatchType::Tournament {
            g.ctl(self.mTournamentScoreboardMenu.miTournamentScoreboard).stop_swirl();
            if g.mode().winningteam > -1 {
                let winningteam = g.mode().winningteam;
                g.ctl(self.mTournamentScoreboardMenu.miTournamentScoreboard).refresh_tournament_scores(g, winningteam);

                self.choose_next_tournament_control_team(g);
            }
        }

        //Reset game mode back to the current game mode in case we came from boss mode
        g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((g.currentgamemode as usize) as u8));

        //Keep track if we entered the menu loop as part of a tournament, if we exit the tournament
        //we need to reset the menu music back to normal
        let matchtype = g.game_values.matchtype;
        if matchtype != MatchType::SingleGame && matchtype != MatchType::QuickGame && matchtype != MatchType::MiniGame && matchtype != MatchType::NetGame {
            self.fNeedMenuMusicReset = true;
        }

        if g.game_values.music && g.game_values.tournamentwinner < 0 {
            if matchtype == MatchType::SingleGame || matchtype == MatchType::QuickGame || matchtype == MatchType::MiniGame || matchtype == MatchType::NetGame {
                gp.backgroundmusic[2].play(g, false, false);
            } else if matchtype == MatchType::World {
                gp.backgroundmusic[5].play(g, false, false);
            } else {
                gp.backgroundmusic[3].play(g, false, false);
            }
        }

        if matchtype != MatchType::World && matchtype != MatchType::QuickGame && matchtype != MatchType::NetGame {
            if self.mCurrentMenu == MenuId::GameSettings || self.mCurrentMenu == MenuId::TournamentScoreboard {
                g.ctl(self.mGameSettingsMenu.miMapField).load_current_map(g);
            }
        }

        // On return from a game the players stay in their room for the next match.
        if g.netplay.active && g.netplay.currentRoom.roomID != 0 {
            gp.netplay.client.back_in_room(g);
            g.netplay.currentMenuChanged = true;
        } else if g.netplay.active {
            gp.netplay.client.request_room_list(g);
        }
    }

    fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        unsafe {
            if g.netplay.active {
                gp.netplay.client.update(g);
            }

            //Reset the keys that were down the last frame
            g.game_values.playerInput.clear_pressed_keys(1);

            crate::smw::auto_advance::tick(g, matches!(self.mCurrentMenu, MenuId::TournamentScoreboard | MenuId::TourStop | MenuId::BonusWheel));

            let mut fGenerateMapThumbs = false;

            //handle messages
            #[cfg(target_os = "emscripten")]
            g.frame_events.clear();
            while let Some(input) = crate::smw::input::poll(gp.get()) {
                #[cfg(target_os = "emscripten")]
                g.frame_events.push(input.clone());
                if input.event == InputEvent::Quit {
                    self.exit(g);
                    return;
                } else if let InputEvent::Key { key, mods, down: true, .. } = input.event {
                    let sym = key.0;
                    if sym == SDLK_F1 {
                        g.game_values.showfps = !g.game_values.showfps;
                    }

                    if (mods as u32) & (KMOD_LALT as u32 | KMOD_RALT as u32) != 0 {
                        //ALT + F4 = close window
                        if sym == SDLK_F4 {
                            self.exit(g);
                            return;
                        }
                        //ALT + Enter = fullscreen/windowed toggle
                        else if sym == SDLK_RETURN {
                            g.game_values.fullscreen = !g.game_values.fullscreen;
                            gfx_changefullscreen(g, g.game_values.fullscreen);
                            g.blitdest = g.screen;

                            continue;
                        }
                    }

                    if sym == SDLK_INSERT {
                        gfx_take_screenshot(g);
                    }
                }

                gp.game_values.playerInput.update_input(g, &input, 1);

                if let Some((x, y)) = input.event.tap() {
                    if AppState::Menu == g.game_values.appstate {
                        let code = self.current().mouse_click(g, x, y);
                        if self.handle_menu_code(g, code, &mut fGenerateMapThumbs) {
                            return;
                        }
                    }
                }
            }

            //If AI is controlling the tournament menu, select the options
            if g.game_values.matchtype == MatchType::Tournament && self.iTournamentAITimer > 0 && self.mCurrentMenu == MenuId::GameSettings && self.mGameSettingsMenu.is_on_start_btn() {
                self.iTournamentAITimer -= 1;
                if self.iTournamentAITimer == 0 {
                    self.iTournamentAIStep += 1;

                    if self.iTournamentAIStep == 1 {
                        g.ctl(self.mGameSettingsMenu.miMapField).choose_random_map(g);

                        self.iTournamentAITimer = ticks(60) as i16;
                    } else if self.iTournamentAIStep == 2 {
                        g.currentgamemode = RANDOM_INT(gp.get(), GAMEMODE_LAST) as i16;
                        g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((g.currentgamemode as usize) as u8));
                        self.mGameSettingsMenu.game_mode_changed(g, g.currentgamemode);
                        g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((self.mGameSettingsMenu.get_current_game_mode_id(g) as usize) as u8));

                        self.iTournamentAITimer = ticks(60) as i16;
                    } else if self.iTournamentAIStep == 3 {
                        //Choose a goal from the lower values for a quicker game
                        let iRandOption = (RANDOM_INT(gp.get(), 6) + 1) as i16;
                        let goal = g.mode().get_options()[iRandOption as usize].iValue;
                        g.mode().goal = goal;

                        let modeGoal = g.gamemodes[g.currentgamemode as usize].goal;
                        g.ctl(self.mGameSettingsMenu.miGoalField[g.currentgamemode as usize]).set_current_value(modeGoal);

                        self.mModeOptionsMenu.set_random_game_mode_settings(g, g.mode().gamemode as i16);

                        self.iTournamentAITimer = ticks(60) as i16;
                    } else if self.iTournamentAIStep == 4 {
                        self.iTournamentAIStep = 0;
                        self.start_game(g);
                    }
                }
            }

            if AppState::Menu == g.game_values.appstate {
                let code = self.current().send_input(g, Ptr::from_mut(&mut gp.game_values.playerInput));

                if self.handle_menu_code(g, code, &mut fGenerateMapThumbs) {
                    return;
                }
            }

            //--------------- draw everything ----------------------

            //Don't draw backdrop for world
            if self.mCurrentMenu != MenuId::World {
                gp.rm.sprite(gp.rm.menu_backdrop).draw(g, 0, 0);
            } else {
                crate::common::gfx::blit::fill_rect(g.screen, std::ptr::null(), (*g.screen).map_rgb(0, 0, 0));
            }

            self.current().update(g);
            self.current().draw(g);

            if self.iDisplayError > DISPLAY_ERROR_NONE {
                gp.rm.sprite(gp.rm.spr_selectfield).draw_src(g, 70, 400, &rect(0, 0, 484, 32));
                gp.rm.sprite(gp.rm.spr_selectfield).draw_src(g, 554, 400, &rect(496, 0, 16, 32));

                if self.iDisplayError == DISPLAY_ERROR_READ_TOUR_FILE {
                    gp.rm.menu_font_large.draw_centered(g, 320, 405, "Error Reading Tour File!");
                }
                if self.iDisplayError == DISPLAY_ERROR_READ_WORLD_FILE {
                    gp.rm.menu_font_large.draw_centered(g, 320, 405, "Error Reading World File!");
                } else if self.iDisplayError == DISPLAY_ERROR_MAP_FILTER {
                    gp.rm.menu_font_large.draw_centered(g, 320, 405, "No Maps Meet All Filter Conditions!");
                } else if self.iDisplayError == DISPLAY_ERROR_TEXT {
                    gp.rm.menu_font_large.draw_chop_centered(g, 320, 405, 470, &self.error_text);
                }

                self.iDisplayErrorTimer -= 1;
                if self.iDisplayErrorTimer == 0 {
                    self.iDisplayError = DISPLAY_ERROR_NONE;
                }
            }

            if g.game_values.screenfadespeed != 0 {
                g.game_values.screenfade += g.game_values.screenfade_step();

                if g.game_values.screenfade <= 0 {
                    g.game_values.screenfadespeed = 0;
                    g.game_values.screenfade = 0;
                } else if g.game_values.screenfade >= 255 {
                    g.game_values.screenfadespeed = 0;
                    g.game_values.screenfade = 255;
                }
            }

            if g.game_values.screenfade > 0 {
                g.rm.sprite(g.rm.menu_shade).setalpha(g.game_values.screenfade as u8);
                gp.rm.sprite(gp.rm.menu_shade).draw(g, 0, 0);
            }

            if g.game_values.screenfade == 255 {
                if AppState::StartGame == g.game_values.appstate {
                    self.enter_gameplay(g);
                    return;
                } else if AppState::StartWorld == g.game_values.appstate {
                    //Fade to world match type
                    g.game_values.screenfadespeed = -8;

                    self.mCurrentMenu = MenuId::World;
                    self.current().reset_menu(g);

                    gp.backgroundmusic[2].stop(g);
                    let worldMusic = g.worldmusiclist.current_music(g.g_worldmap.get_music_category(), g.g_worldmap.get_world_name()).clone();
                    g.backgroundmusic[5] = sfxMusic::from_file_or_silence(g, &worldMusic);
                    gp.backgroundmusic[5].play(g, false, false);
                    self.fNeedMenuMusicReset = true;

                    g.ctl(self.mWorldMenu.miWorld).display_team_control_announcement(g);

                    g.game_values.appstate = AppState::Menu;
                }
            }

            if fGenerateMapThumbs {
                gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, 160, 176, &rect(0, 0, 160, 64));
                gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, App::screenWidth / 2, 176, &rect(352, 0, 160, 64));
                gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, 160, 240, &rect(0, 416, 160, 64));
                gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, App::screenWidth / 2, App::screenHeight / 2, &rect(352, 416, 160, 64));
                gp.rm.menu_font_large.draw_centered(g, App::screenWidth / 2, 215, "Refreshing Map Thumbnails");
                gp.rm.menu_font_large.draw_centered(g, App::screenWidth / 2, 245, "Please Wait...");
            }

            if fGenerateMapThumbs {
                gp.backgroundmusic[2].toggle_pause(g);

                //Reload map auto filters from live map files (don't use the cache)
                gp.maplist.reload_map_auto_filters(g);

                //Write out all the map thumbnails for the map browser and filter editor
                let mut itr = g.maplist.get_iterator_at(0, false);

                let iMapCount = g.maplist.count() as i16;
                for _iMap in 0..iMapCount {
                    let filename = g.maplist.node_at(itr).filename.clone();
                    let mut szThumbnail = String::from("maps/cache/");
                    szThumbnail += &get_name_from_file_name(&filename, false);
                    szThumbnail += ".png";

                    gp.g_map.load_map(g, &filename, read_type_preview);
                    gp.get().g_map.save_thumbnail(g, &convert_path(gp.get(), &szThumbnail), false);

                    itr += 1;
                }

                gp.backgroundmusic[2].toggle_pause(g);
            }
        }
    }
}
