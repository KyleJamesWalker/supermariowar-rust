//! Port of src/smw/GSMenu.cpp (reference build: original + tools/cpp-harness.patch).

use smw_platform::{Input, InputEvent};
use crate::common::file_list::{MusicCategory, WorldMusicCategory};
use crate::common::game::App;
use crate::common::game_mode::*;
use crate::common::game_values::{if_sound_on_play, AppState};
use crate::common::gameplay_styles::TournamentControlStyle;
use crate::common::gfx::{gfx_changefullscreen, gfx_loadpalette, gfx_take_screenshot};
use crate::common::global_constants::{MAX_PLAYERS, PANNOUNCER_SOUND_LAST, TILESIZE};
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
use crate::smw::world::{g_worldmap, WorldMap};
use crate::smw::main::{bonushousemode, bossgamemode, boxesgamemode, pipegamemode};
use crate::smw::game_state::{GameState, GameStateManager};
use crate::smw::gs_gameplay::{lookup_team_id, GameplayState};
use crate::smw::harness;
use crate::smw::main::{currentgamemode, gamemodes, score, score_cnt};
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
use crate::smw::net::{net_end_session, net_start_session, netplay};
use sdl2::sys::{SDL_Event, SDL_EventType, SDL_FillRect, SDL_KeyCode, SDL_Keymod, SDL_MapRGB, SDL_Rect};
use std::ops::DerefMut;
use std::path::Path;

pub type DisplayError = i32;
pub const DISPLAY_ERROR_NONE: DisplayError = 0;
pub const DISPLAY_ERROR_READ_TOUR_FILE: DisplayError = 1;
pub const DISPLAY_ERROR_READ_WORLD_FILE: DisplayError = 2;
pub const DISPLAY_ERROR_MAP_FILTER: DisplayError = 3;

/// `menu.get()` as the `UI_Menu*` base, null for an empty slot.
fn menu<T: DerefMut<Target = UI_Menu>>(p: Ptr<T>) -> Ptr<UI_Menu> {
    if p.is_null() {
        Ptr::null()
    } else {
        Ptr::from_mut(&mut **p.get())
    }
}

/// `unique_ptr = std::make_unique<T>()`: the new object exists before the old one is deleted.
fn assign<T>(slot: &mut Ptr<T>, value: Box<T>) {
    let old = *slot;
    *slot = Ptr::from_box(value);
    old.delete();
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> SDL_Rect {
    SDL_Rect { x, y, w, h }
}

pub struct MenuState {
    mCurrentMenu: Ptr<UI_Menu>,
    mMainMenu: Ptr<UI_MainMenu>,

    // Options menu
    mOptionsMenu: Ptr<UI_OptionsMenu>,
    mGameplayOptionsMenu: Ptr<UI_GameplayOptionsMenu>,
    mTeamOptionsMenu: Ptr<UI_TeamOptionsMenu>,
    mPowerupDropRatesMenu: Ptr<UI_PowerupDropRatesMenu>,
    mPowerupSettingsMenu: Ptr<UI_PowerupSettingsMenu>,
    mProjectileLimitsMenu: Ptr<UI_ProjectileLimitsMenu>,
    mProjectileOptionsMenu: Ptr<UI_ProjectileOptionsMenu>,
    mGraphicsOptionsMenu: Ptr<UI_GraphicsOptionsMenu>,
    mEyeCandyOptionsMenu: Ptr<UI_EyeCandyOptionsMenu>,
    mSoundOptionsMenu: Ptr<UI_SoundOptionsMenu>,

    // Controls menu
    mPlayerControlsSelectMenu: Ptr<UI_PlayerControlsSelectMenu>,
    mPlayerControlsMenu: Ptr<UI_PlayerControlsMenu>,

    // Gameplay menus
    mModeOptionsMenu: Ptr<UI_ModeOptionsMenu>,
    mMatchSelectionMenu: Ptr<UI_MatchSelectionMenu>,
    mGameSettingsMenu: Ptr<UI_GameSettingsMenu>,
    mMapFilterEditMenu: Ptr<UI_MapFilterEditMenu>,
    mTourStopMenu: Ptr<UI_TourStopMenu>,
    mWorldMenu: Ptr<UI_WorldMenu>,
    mTeamSelectMenu: Ptr<UI_TeamSelectMenu>,
    mTournamentScoreboardMenu: Ptr<UI_TournamentScoreboardMenu>,
    mBonusWheelMenu: Ptr<UI_BonusWheelMenu>,

    // Multiplayer menus
    mNetServersMenu: Ptr<UI_NetServersMenu>,
    mNetEditServersMenu: Ptr<UI_NetEditServersMenu>,
    mNetLobbyMenu: Ptr<UI_NetLobbyMenu>,
    mNetNewRoomMenu: Ptr<UI_NetNewRoomMenu>,
    mNetNewRoomSettingsMenu: Ptr<UI_NetNewRoomSettingsMenu>,
    mNetRoomMenu: Ptr<UI_NetRoomMenu>,

    iDisplayError: DisplayError,
    iDisplayErrorTimer: i16,
    fNeedMenuMusicReset: bool,

    /// Aliases the map field's name buffer, like the C++ `const char*` into `char szMapName[256]`.
    szCurrentMapName: Ptr<String>,

    iUnlockMinigameOptionIndex: i16,

    iTournamentAIStep: i16,
    iTournamentAITimer: i16,

    _alias: Aliased,
}

static mut ms: Option<MenuState> = None;

/// The browser cannot block for input, so a control field being rebound reads this frame's events here.
#[cfg(target_os = "emscripten")]
pub static mut frame_events: Vec<Input> = Vec::new();

impl MenuState {
    fn new() -> Self {
        MenuState {
            mCurrentMenu: Ptr::null(),
            mMainMenu: Ptr::null(),
            mOptionsMenu: Ptr::null(),
            mGameplayOptionsMenu: Ptr::null(),
            mTeamOptionsMenu: Ptr::null(),
            mPowerupDropRatesMenu: Ptr::null(),
            mPowerupSettingsMenu: Ptr::null(),
            mProjectileLimitsMenu: Ptr::null(),
            mProjectileOptionsMenu: Ptr::null(),
            mGraphicsOptionsMenu: Ptr::null(),
            mEyeCandyOptionsMenu: Ptr::null(),
            mSoundOptionsMenu: Ptr::null(),
            mPlayerControlsSelectMenu: Ptr::null(),
            mPlayerControlsMenu: Ptr::null(),
            mModeOptionsMenu: Ptr::null(),
            mMatchSelectionMenu: Ptr::null(),
            mGameSettingsMenu: Ptr::null(),
            mMapFilterEditMenu: Ptr::null(),
            mTourStopMenu: Ptr::null(),
            mWorldMenu: Ptr::null(),
            mTeamSelectMenu: Ptr::null(),
            mTournamentScoreboardMenu: Ptr::null(),
            mBonusWheelMenu: Ptr::null(),
            mNetServersMenu: Ptr::null(),
            mNetEditServersMenu: Ptr::null(),
            mNetLobbyMenu: Ptr::null(),
            mNetNewRoomMenu: Ptr::null(),
            mNetNewRoomSettingsMenu: Ptr::null(),
            mNetRoomMenu: Ptr::null(),
            iDisplayError: DISPLAY_ERROR_NONE,
            iDisplayErrorTimer: 0,
            fNeedMenuMusicReset: false,
            szCurrentMapName: Ptr::null(),
            iUnlockMinigameOptionIndex: 0,
            iTournamentAIStep: 0,
            iTournamentAITimer: 0,
            _alias: Aliased::new(),
        }
    }

    pub fn instance() -> &'static mut MenuState {
        unsafe { ms.get_or_insert_with(MenuState::new) }
    }

    pub fn harness_menu_name(&self) -> &'static str {
        let names: [(Ptr<UI_Menu>, &'static str); 28] = [
            (menu(self.mMainMenu), "main"),
            (menu(self.mOptionsMenu), "options"),
            (menu(self.mGameplayOptionsMenu), "gameplay_options"),
            (menu(self.mTeamOptionsMenu), "team_options"),
            (menu(self.mPowerupDropRatesMenu), "powerup_drop_rates"),
            (menu(self.mPowerupSettingsMenu), "powerup_settings"),
            (menu(self.mProjectileLimitsMenu), "projectile_limits"),
            (menu(self.mProjectileOptionsMenu), "projectile_options"),
            (menu(self.mGraphicsOptionsMenu), "graphics_options"),
            (menu(self.mEyeCandyOptionsMenu), "eyecandy_options"),
            (menu(self.mSoundOptionsMenu), "sound_options"),
            (menu(self.mPlayerControlsSelectMenu), "player_controls_select"),
            (menu(self.mPlayerControlsMenu), "player_controls"),
            (menu(self.mModeOptionsMenu), "mode_options"),
            (menu(self.mMatchSelectionMenu), "match_selection"),
            (menu(self.mGameSettingsMenu), "game_settings"),
            (menu(self.mMapFilterEditMenu), "map_filter_edit"),
            (menu(self.mTourStopMenu), "tour_stop"),
            (menu(self.mWorldMenu), "world"),
            (menu(self.mTeamSelectMenu), "team_select"),
            (menu(self.mTournamentScoreboardMenu), "tournament_scoreboard"),
            (menu(self.mBonusWheelMenu), "bonus_wheel"),
            (menu(self.mNetServersMenu), "net_servers"),
            (menu(self.mNetEditServersMenu), "net_edit_servers"),
            (menu(self.mNetLobbyMenu), "net_lobby"),
            (menu(self.mNetNewRoomMenu), "net_new_room"),
            (menu(self.mNetNewRoomSettingsMenu), "net_new_room_settings"),
            (menu(self.mNetRoomMenu), "net_room"),
        ];
        for (m, name) in names.iter() {
            if *m == self.mCurrentMenu {
                return name;
            }
        }
        "unknown"
    }

    pub fn harness_focus_index(&self) -> i32 {
        if !self.mCurrentMenu.is_null() {
            self.mCurrentMenu.current_focus_index()
        } else {
            -1
        }
    }

    pub fn harness_modifying(&self) -> bool {
        !self.mCurrentMenu.is_null() && self.mCurrentMenu.is_modifying()
    }

    fn read_tour_file(&mut self) -> bool {
        unsafe {
            reset_tour_stops();

            let fp = std::fs::File::open(tourlist.at(game_values.tourindex as usize)).expect("fopen tour file");
            let ignorable_leads: &[u8] = b" #\n\r\t";

            let mut fReadVersion = false;
            let mut version = Version::default();
            for buffer in fgets_lines(fp) {
                if game_values.tourstops.len() >= 10 {
                    break;
                }

                // strchr also matches the terminating NUL
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

                let ts: TourStop = parse_tour_stop_line(buffer.as_bytes(), &version, false);
                game_values.tourstops.push(Ptr::new_box(ts));
            }

            if !game_values.tourstops.is_empty() {
                self.mTourStopMenu.miTourStop.refresh(game_values.tourstopcurrent as i16);

                //For old tours, turn on the bonus wheel at the end
                if version.major == 1 && version.minor == 7 && version.patch == 0 && version.build <= 1 {
                    let last = game_values.tourstops.len() - 1;
                    game_values.tourstops[last].iBonusType = 1;
                }
            }

            !game_values.tourstops.is_empty()
        }
    }

    fn start_game(&mut self) {
        unsafe {
            println!("> StartGame");

            game_values.appstate = AppState::StartGame;
            //backgroundonly = false;
            //fastmap = false;

            game_values.write_config();

            //Load skins for players
            println!("Loading player skins...");
            for k in 0..4usize {
                if game_values.playercontrol[k] > 0 {
                    if netplay.active {
                        if k == netplay.remotePlayerNumber as usize {
                            // local player uses local skin
                            println!("  player {} -> local", k);
                            let skin = rm.load_full_skin(game_values.skinids[k], game_values.colorids[k]);
                            rm.spr_player[k] = skin;
                        } else {
                            let path = format!("{}net_skin{}.bmp", get_home_directory(), k);
                            println!("  player {} -> {}", k, path);

                            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rm.load_full_skin_path(Path::new(&path), k as i16)));
                            match result {
                                Ok(skin) => rm.spr_player[k] = skin,
                                Err(_) => {
                                    println!("[warning] Could not load netplay skin of player {}, using default", k);
                                    let skin = rm.load_full_skin(game_values.skinids[k], game_values.colorids[k]);
                                    rm.spr_player[k] = skin;
                                }
                            }
                        }
                    } else if game_values.randomskin[k] {
                        game_values.skinids[k] = RANDOM_INT(skinlist.count() as i32) as i16;
                        let skin = rm.load_full_skin(game_values.skinids[k], game_values.colorids[k]);
                        rm.spr_player[k] = skin;
                    } else {
                        let skin = rm.load_full_skin(game_values.skinids[k], game_values.colorids[k]);
                        rm.spr_player[k] = skin;
                    }
                }
            }

            //Load announcer sounds if changed
            if game_values.loadedannouncer != announcerlist.current_index() {
                game_values.loadedannouncer = announcerlist.current_index();

                //Delete the old sounds
                for k in 0..PANNOUNCER_SOUND_LAST as usize {
                    rm.sfx_announcer[k] = sfxSound::new();
                }

                let announcerfile = std::fs::File::open(announcerlist.current_path()).expect("fopen announcer file");

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
                        let sound = sfxSound::from_file(Path::new(&convert_path(&szBuffer))).unwrap_or_else(|e| std::panic::panic_any(e));
                        rm.sfx_announcer[announcerIndex as usize] = sound;
                    }

                    announcerIndex += 1;
                }
            }

            //Load soundtrack music if changed
            if game_values.loadedmusic as i64 != musiclist.current_index() as i64 {
                game_values.loadedmusic = musiclist.current_index() as i16;
                rm.backgroundmusic[1] = sfxMusic::from_file(musiclist.music(0)).unwrap_or_else(|e| std::panic::panic_any(e)); //Stage Clear
                rm.backgroundmusic[3] = sfxMusic::from_file(musiclist.music(2)).unwrap_or_else(|e| std::panic::panic_any(e)); //Tournament Menu
                rm.backgroundmusic[4] = sfxMusic::from_file(musiclist.music(3)).unwrap_or_else(|e| std::panic::panic_any(e)); //Tournament Over
            }

            rm.backgroundmusic[2].stop();
            rm.sfx_announcer[11].play();

            game_values.screenfade = 8;
            game_values.screenfadespeed = 8;
            println!("< StartGame");
        }
    }

    fn set_controlling_team_for_settings_menu(&mut self, iControlTeam: i16, fDisplayMessage: bool) {
        unsafe {
            self.mGameSettingsMenu.set_controlling_team(iControlTeam);

            self.mModeOptionsMenu.set_controlling_team(iControlTeam);

            self.mGameSettingsMenu.miMapFilterScroll.set_controlling_team(iControlTeam);
            self.mMapFilterEditMenu.miMapBrowser.set_controlling_team(iControlTeam);

            if fDisplayMessage {
                self.display_controlling_team_message(iControlTeam);
            }

            //Scan controlling team members and if they are only bots, then let the cpu control the selection
            self.iTournamentAITimer = 0;
            self.iTournamentAIStep = 0;
            self.mGameSettingsMenu.set_allow_exit(false);

            if iControlTeam >= 0 {
                let mut fNeedAI = true;
                for iPlayer in 0..game_values.teamcounts[iControlTeam as usize] {
                    if game_values.playercontrol[game_values.teamids[iControlTeam as usize][iPlayer as usize] as usize] == 1 {
                        fNeedAI = false;
                        break;
                    }
                }

                if fNeedAI {
                    self.iTournamentAITimer = 60;
                    self.mGameSettingsMenu.set_allow_exit(true);
                }
            }
        }
    }

    fn display_controlling_team_message(&mut self, iControlTeam: i16) {
        unsafe {
            //Display the team that is in control of selecting the next game
            let szMessage = if iControlTeam < 0 {
                "All Teams Are In Control".to_string()
            } else if game_values.teamcounts[iControlTeam as usize] <= 1 {
                format!("Player {} Is In Control", game_values.teamids[iControlTeam as usize][0] + 1)
            } else {
                format!("Team {} Is In Control", iControlTeam + 1)
            };

            let iIcon: i16 = if iControlTeam < 0 { 4 } else { game_values.colorids[game_values.teamids[iControlTeam as usize][0] as usize] };
            self.mCurrentMenu.eyeCandy.emplace(EC_Announcement::new(
                Ptr::from_mut(&mut rm.menu_font_large),
                Ptr::from_mut(&mut rm.spr_announcementicons),
                szMessage,
                iIcon,
                120,
                100,
            ));
        }
    }

    fn exit(&mut self) {
        #[cfg(target_os = "emscripten")]
        return;

        self.quit();
    }

    /// Not in upstream: `exit` without its web early return. The main menu's Exit button uses it, so the web build
    /// ends the session and the page shows its game-over overlay (web_quit_if_done in main.rs).
    fn quit(&mut self) {
        unsafe {
            game_values.appstate = AppState::Quit;
            game_values.write_config();
        }
    }

    fn reset_tournament_back_to_main_menu(&mut self) {
        unsafe {
            self.mCurrentMenu = menu(self.mMainMenu);
            self.mCurrentMenu.reset_menu();

            if game_values.matchtype != MatchType::SingleGame && game_values.matchtype != MatchType::QuickGame && game_values.matchtype != MatchType::MiniGame && game_values.matchtype != MatchType::NetGame {
                if self.fNeedMenuMusicReset {
                    rm.backgroundmusic[3].stop();
                    rm.backgroundmusic[2].play(false, false);
                    self.fNeedMenuMusicReset = false;
                }
            }
        }
    }

    /// The `MENU_CODE_TO_GAME_SETUP_MENU` branch of `MenuState::update`.
    fn to_game_setup_menu(&mut self) {
        unsafe {
            println!("MENU_CODE_TO_GAME_SETUP_MENU");
            //Moves teams to the first arrays in the list and counts the number of teams
            score_cnt = self.mTeamSelectMenu.get_team_count();
            self.iDisplayError = DISPLAY_ERROR_NONE;
            self.iDisplayErrorTimer = 0;
            let mut fErrorReadingTourFile = false;

            if MatchType::MiniGame == game_values.matchtype {
                println!(" Match type: Minigame");
                let minigame: Minigame = self.mMatchSelectionMenu.get_minigame();
                match minigame {
                    Minigame::PipeCoin => {
                        pipegamemode.goal = 50;
                        game_values.gamemode = Ptr::from_raw(pipegamemode.as_ptr() as *mut dyn CGameModeTrait);
                    }
                    Minigame::HammerBoss | Minigame::BombBoss | Minigame::FireBoss => {
                        game_values.gamemodemenusettings.boss.difficulty = 2;
                        game_values.gamemodemenusettings.boss.hitpoints = 5;

                        bossgamemode.goal = 5;
                        game_values.gamemode = Ptr::from_raw(bossgamemode.as_ptr() as *mut dyn CGameModeTrait);
                    }
                    Minigame::Boxes => {
                        boxesgamemode.goal = 10;
                        game_values.gamemode = Ptr::from_raw(boxesgamemode.as_ptr() as *mut dyn CGameModeTrait);
                    }
                }
                match minigame {
                    Minigame::HammerBoss => {
                        game_values.gamemodemenusettings.boss.bosstype = Boss::Hammer;
                    }
                    Minigame::BombBoss => {
                        game_values.gamemodemenusettings.boss.bosstype = Boss::Bomb;
                    }
                    Minigame::FireBoss => {
                        game_values.gamemodemenusettings.boss.bosstype = Boss::Fire;
                    }
                    _ => {}
                }
                self.start_game();
            } else if MatchType::QuickGame == game_values.matchtype {
                println!(" Match type: Quick game");
                let iRandomMode = RANDOM_INT(GAMEMODE_LAST) as i16;
                game_values.gamemode = gamemodes[iRandomMode as usize];

                //Choose a goal from the lower values for a quicker game
                let iRandOption = (RANDOM_INT(6) + 1) as i16;
                let goal = game_values.gamemode.get_options()[iRandOption as usize].iValue;
                game_values.gamemode.goal = goal;

                game_values.tournamentwinner = -1;

                self.start_game();
            } else {
                //Load the tour here if one was selected
                if game_values.matchtype == MatchType::Tour {
                    println!("  Match type: Tour");
                    if !self.read_tour_file() {
                        self.iDisplayError = DISPLAY_ERROR_READ_TOUR_FILE;
                        self.iDisplayErrorTimer = 120;
                        fErrorReadingTourFile = true;
                    } else {
                        let numGames = game_values.tourstops.len() as i16;
                        self.mTournamentScoreboardMenu.miTournamentScoreboard.create_scoreboard(score_cnt, numGames, Ptr::from_mut(&mut rm.spr_tour_markers));
                    }
                } else if game_values.matchtype == MatchType::Tournament {
                    println!("  Match type: Tournament");
                    //Set who is controlling the tournament menu
                    if game_values.tournamentcontrolstyle == TournamentControlStyle::Random || game_values.tournamentcontrolstyle == TournamentControlStyle::RandomLoser {
                        //Random
                        game_values.tournamentcontrolteam = RANDOM_INT(score_cnt as i32) as i16;
                    } else if game_values.tournamentcontrolstyle == TournamentControlStyle::RoundRobin {
                        //Round robin
                        game_values.tournamentcontrolteam = 0;
                    } else {
                        //The first game of the tournament is controlled by all players
                        game_values.tournamentcontrolteam = -1;
                    }

                    game_values.tournamentnextcontrol = 1; //if round robin is selected, choose the next team next

                    let numGames = game_values.tournamentgames;
                    self.mTournamentScoreboardMenu.miTournamentScoreboard.create_scoreboard(score_cnt, numGames, Ptr::from_mut(&mut rm.menu_mode_large));
                } else if game_values.matchtype == MatchType::World {
                    println!("  Match type: World");
                    let this = Ptr::from_mut(self);
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        let mut this = this;
                        g_worldmap.init(WorldMap::new_path(&worldlist.at(game_values.worldindex as usize).to_string_lossy(), TILESIZE as i16));

                        this.mTournamentScoreboardMenu.miTournamentScoreboard.create_scoreboard(score_cnt, 0, Ptr::from_mut(&mut rm.spr_tour_markers));

                        g_worldmap.set_initial_powerups();

                        //If a player had a stored powerup from another game, add it to their inventory
                        for iPlayer in 0..4i16 {
                            if game_values.storedpowerups[iPlayer as usize] != -1 {
                                let iTeamId = lookup_team_id(iPlayer) as usize;
                                if game_values.worldpowerupcount[iTeamId] < 32 {
                                    let count = game_values.worldpowerupcount[iTeamId] as usize;
                                    game_values.worldpowerupcount[iTeamId] += 1;
                                    game_values.worldpowerups[iTeamId][count] = game_values.storedpowerups[iPlayer as usize];
                                }

                                game_values.storedpowerups[iPlayer as usize] = -1;
                            }
                        }

                        this.mWorldMenu.miWorld.init();
                        let team = RANDOM_INT(score_cnt as i32) as i16;
                        this.mWorldMenu.miWorld.set_controlling_team(team);
                    }));
                    if result.is_err() {
                        self.iDisplayError = DISPLAY_ERROR_READ_WORLD_FILE;
                        self.iDisplayErrorTimer = 120;
                        fErrorReadingTourFile = true;
                    }
                }

                if !fErrorReadingTourFile {
                    println!("  !fErrorReadingTourFile");
                    self.mTournamentScoreboardMenu.eyeCandy.clean();

                    //Initialize tournament values
                    game_values.tournamentwinner = -1;

                    //Setup wins counters for tournament/tour
                    for k in 0..4usize {
                        game_values.tournament_scores[k].wins = 0;
                        game_values.tournament_scores[k].total = 0;
                    }

                    if MatchType::SingleGame == game_values.matchtype || MatchType::Tournament == game_values.matchtype || MatchType::NetGame == game_values.matchtype {
                        println!("  MatchType::SingleGame || MatchType::Tournament");
                        println!("current map: {}", *self.szCurrentMapName);
                        let szCurrentMapName = (*self.szCurrentMapName).clone();
                        maplist.findexact(&szCurrentMapName, false);
                        self.mGameSettingsMenu.miMapField.load_current_map();

                        game_values.gamemode = gamemodes[self.mGameSettingsMenu.get_current_game_mode_id() as usize];

                        for iMode in 0..GAMEMODE_LAST as usize {
                            gamemodes[iMode].goal = self.mGameSettingsMenu.miGoalField[iMode].current_value();
                        }

                        if self.mGameSettingsMenu.get_current_game_mode_id() as GameModeType == game_mode_owned {
                            self.mGameSettingsMenu.hide_gm_settings_btn();
                        }

                        self.mCurrentMenu = menu(self.mGameSettingsMenu);
                        self.mCurrentMenu.reset_menu();

                        //If it is a tournament, then set the controlling team
                        if MatchType::Tournament == game_values.matchtype {
                            self.set_controlling_team_for_settings_menu(game_values.tournamentcontrolteam, true);
                        }
                    } else if MatchType::Tour == game_values.matchtype {
                        self.mCurrentMenu = menu(self.mTourStopMenu);
                        self.mCurrentMenu.reset_menu();
                    } else if MatchType::World == game_values.matchtype {
                        game_values.screenfadespeed = 8;
                        game_values.screenfade = 8;
                        game_values.appstate = AppState::StartWorld;

                        //Play enter world sound
                        rm.sfx_enterstage.play();
                    }

                    //Setup items on next menu
                    for iGameMode in 0..GAMEMODE_LAST as usize {
                        self.mGameSettingsMenu.miGoalField[iGameMode].hide_item(-1, game_values.matchtype == MatchType::Tournament);
                    }

                    if game_values.matchtype == MatchType::World {
                        self.mGameSettingsMenu.set_header_text("World Game Menu");
                    } else if game_values.matchtype == MatchType::Tour {
                        self.mGameSettingsMenu.set_header_text("Tour Game Menu");
                    } else if game_values.matchtype == MatchType::Tournament {
                        self.mGameSettingsMenu.set_header_text("Tournament Game Menu");
                    } else {
                        self.mGameSettingsMenu.set_header_text("Single Game Menu");
                    }
                }
            }
        }
    }

    /// The tournament-control branch of `MenuState::onEnterState`, run when a tournament game was won.
    fn choose_next_tournament_control_team(&mut self) {
        unsafe {
            //Set the next controlling team
            match game_values.tournamentcontrolstyle {
                TournamentControlStyle::GameWinner => {
                    //Winning Team
                    game_values.tournamentcontrolteam = game_values.gamemode.winningteam;
                }

                TournamentControlStyle::GameLoser => {
                    //Losing Team
                    let mut iNumInPlace: i16 = 0;
                    let mut iInPlace: [i16; 4] = [0; 4];
                    let mut iLowestPlace: i16 = 0;
                    for iScore in 0..score_cnt {
                        if score[iScore as usize].place == iLowestPlace {
                            iInPlace[iNumInPlace as usize] = iScore;
                            iNumInPlace += 1;
                        } else if score[iScore as usize].place > iLowestPlace {
                            iNumInPlace = 1;
                            iInPlace[0] = iScore;
                            iLowestPlace = score[iScore as usize].place;
                        }
                    }

                    game_values.tournamentcontrolteam = iInPlace[RANDOM_INT(iNumInPlace as i32) as usize];
                }

                TournamentControlStyle::LeadingTeams => {
                    //Tournament Ahead Teams
                    let mut iNumInPlace: i16 = 0;
                    let mut iInPlace: [i16; 4] = [0; 4];
                    let mut iMostWins: i16 = 0;
                    for iTeam in 0..score_cnt {
                        if game_values.tournament_scores[iTeam as usize].wins == iMostWins {
                            iInPlace[iNumInPlace as usize] = iTeam;
                            iNumInPlace += 1;
                        } else if game_values.tournament_scores[iTeam as usize].wins > iMostWins {
                            iNumInPlace = 1;
                            iInPlace[0] = iTeam;
                            iMostWins = game_values.tournament_scores[iTeam as usize].wins;
                        }
                    }

                    game_values.tournamentcontrolteam = iInPlace[RANDOM_INT(iNumInPlace as i32) as usize];
                }

                TournamentControlStyle::TrailingTeams => {
                    //Tournament Behind Teams
                    let mut iNumInPlace: i16 = 0;
                    let mut iInPlace: [i16; 4] = [0, 0, 0, 0];
                    let mut iLeastWins: i16 = 20; //Most possible wins are 10

                    for iTeam in 0..score_cnt {
                        if game_values.tournament_scores[iTeam as usize].wins == iLeastWins {
                            iInPlace[iNumInPlace as usize] = iTeam;
                            iNumInPlace += 1;
                        } else if game_values.tournament_scores[iTeam as usize].wins < iLeastWins {
                            iNumInPlace = 1;
                            iInPlace[0] = iTeam;
                            iLeastWins = game_values.tournament_scores[iTeam as usize].wins;
                        }
                    }

                    game_values.tournamentcontrolteam = iInPlace[RANDOM_INT(iNumInPlace as i32) as usize];
                }

                TournamentControlStyle::Random => {
                    //Random
                    game_values.tournamentcontrolteam = RANDOM_INT(score_cnt as i32) as i16;
                }

                TournamentControlStyle::RandomLoser => {
                    //Random Losing Team
                    let mut iNumInPlace: i16 = 0;
                    let mut iInPlace: [i16; 4] = [0, 0, 0, 0];
                    let mut iWinner: i16 = 0;

                    for iTeam in 0..score_cnt {
                        if score[iTeam as usize].place == 0 {
                            iWinner = iTeam;
                            break;
                        }
                    }

                    for iTeam in 0..score_cnt {
                        if iTeam == iWinner {
                            continue;
                        }

                        iInPlace[iNumInPlace as usize] = iTeam;
                        iNumInPlace += 1;
                    }

                    game_values.tournamentcontrolteam = iInPlace[RANDOM_INT(iNumInPlace as i32) as usize];
                }

                TournamentControlStyle::RoundRobin => {
                    //Round Robin
                    game_values.tournamentcontrolteam = game_values.tournamentnextcontrol;

                    game_values.tournamentnextcontrol += 1;
                    if game_values.tournamentnextcontrol >= score_cnt {
                        game_values.tournamentnextcontrol = 0;
                    }
                }

                _ => {
                    game_values.tournamentcontrolteam = -1;
                }
            }
        }
    }

    /// The `game_values.screenfade == 255` + `AppState::StartGame` block of `MenuState::update`.
    fn enter_gameplay(&mut self) {
        unsafe {
            if game_values.matchtype == MatchType::QuickGame {
                self.mModeOptionsMenu.set_random_game_mode_settings(game_values.gamemode.gamemode as i16);
            } else if game_values.matchtype == MatchType::NetGame {
                // Not in the C++ (whose TODO rolls random ones here on every client): the host sent its own at the sync.
                crate::smw::net::reseed_for_match();
            } else {
                crate::smw::gs_gameplay::set_game_mode_settings_from_menu();
            }

            if game_values.matchtype == MatchType::World && game_values.tourstops[game_values.tourstopcurrent].iStageType == 1 {
                load_match_map(&convert_path("maps/special/two52_special_bonushouse.map"));
                crate::common::global::load_current_map_background();

                if game_values.music {
                    rm.backgroundmusic[0] = sfxMusic::from_file(worldmusiclist.current_music(WorldMusicCategory::Bonus, "")).unwrap_or_else(|e| std::panic::panic_any(e));
                    rm.backgroundmusic[0].play(false, false);
                }
            } else {
                let mut sShortMapName = String::new();

                let mut fMiniGameMapFound = false;

                if game_values.matchtype == MatchType::World {
                    let gm = game_values.gamemode.gamemode;
                    if gm == game_mode_pipe_minigame || gm == game_mode_boss_minigame || gm == game_mode_boxes_minigame {
                        let mapFile = game_values.tourstops[game_values.tourstopcurrent].pszMapFile.clone();
                        fMiniGameMapFound = maplist.findexact(&mapFile, true);

                        if fMiniGameMapFound {
                            let filename = maplist.current_filename().to_string();
                            load_match_map(&filename);
                            sShortMapName = maplist.current_shortmapname().to_string();
                        }
                    }
                }

                if game_values.gamemode.gamemode == game_mode_pipe_minigame {
                    if !fMiniGameMapFound {
                        load_match_map(&convert_path("maps/special/two52_special_pipe_minigame.map"));
                        sShortMapName = "minigamepipe".to_string();
                    }
                } else if game_values.gamemode.gamemode == game_mode_boss_minigame {
                    if !fMiniGameMapFound {
                        let bossType: Boss = game_values.gamemodesettings.boss.bosstype;
                        bossgamemode.set_boss_type(bossType);
                        match bossType {
                            Boss::Hammer => load_match_map(&convert_path("maps/special/two52_special_hammerboss_minigame.map")),
                            Boss::Bomb => load_match_map(&convert_path("maps/special/two52_special_bombboss_minigame.map")),
                            Boss::Fire => load_match_map(&convert_path("maps/special/two52_special_fireboss_minigame.map")),
                        }
                        sShortMapName = "minigameboss".to_string();
                    }
                } else if game_values.gamemode.gamemode == game_mode_boxes_minigame {
                    if !fMiniGameMapFound {
                        load_match_map(&convert_path("maps/special/two52_special_boxes_minigame.map"));
                        sShortMapName = "minigameboxes".to_string();
                    }
                } else if game_values.matchtype == MatchType::QuickGame {
                    //Load a random map for the quick game
                    let szMapName = maplist.random_filename();
                    load_match_map(&szMapName);
                    sShortMapName = strip_path_and_extension(&szMapName);

                    println!("  State: GS_START_GAME, Match type: MatchType::QuickGame");
                } else if netplay.active {
                    // NOTE: for the host, netplay.mapfilepath will be ./data/something
                    // while for the other players, it's ~/.smw/net_last.map
                    load_match_map(&netplay.mapfilepath);
                    sShortMapName = strip_path_and_extension(&netplay.mapfilepath);
                } else {
                    let filename = maplist.current_filename().to_string();
                    load_match_map(&filename);
                    sShortMapName = maplist.current_shortmapname().to_string();
                }

                crate::common::global::load_current_map_background();

                //Allows all players to start the game
                game_values.singleplayermode = -1;

                if game_values.music {
                    let category = MusicCategory::ALL[g_map.musicCategoryID as usize];
                    let background = g_map.szBackgroundFile.clone();
                    musiclist.set_random_music(category, &sShortMapName, &background);
                    rm.backgroundmusic[0] = sfxMusic::from_file(musiclist.current_music()).unwrap_or_else(|e| std::panic::panic_any(e));
                    rm.backgroundmusic[0].play(game_values.playnextmusic, false);
                }
            }

            harness::match_checkpoint();
            enter_gameplay_tail();
        }
    }
}

/// Loads the map a match is played on; replay checkpoints (smw/checkpoint.rs) note its file.
fn load_match_map(path: &str) {
    unsafe { g_map.load_map(path, read_type_full) };
    harness::note_match_map(path);
}

/// The rest of `MenuState::enter_gameplay`, after the map and its music are loaded. A segment replay
/// (smw/checkpoint.rs) restores the state up to that point and starts the match here.
pub fn enter_gameplay_tail() {
    unsafe {
        game_values.appstate = AppState::Game;
        println!("  GS_GAME");

        g_map.predrawbackground(&rm.spr_background, &rm.spr_backmap[0]);
        g_map.predrawforeground(&rm.spr_frontmap[0]);

        g_map.predrawbackground(&rm.spr_background, &rm.spr_backmap[1]);
        g_map.predrawforeground(&rm.spr_frontmap[1]);

        g_map.setup_animated_tiles();
        crate::smw::gs_gameplay::load_map_objects(false);

        GameStateManager::instance().change_state_to(Ptr::from_mut(GameplayState::instance() as &mut dyn GameState));
    }
}

impl MenuState {
    /// Handles one `MenuCodeEnum` from `mCurrentMenu->SendInput`. Returns true where C++ `return`s from `update`.
    fn handle_menu_code(&mut self, mut code: MenuCodeEnum, fGenerateMapThumbs: &mut bool) -> bool {
        unsafe {
            // Shortcut to game start
            if netplay.active {
                let lastSendType = netplay.client.lastSentMessage.packageType;
                let lastRecvType = netplay.client.lastReceivedMessage.packageType;

                if lastSendType == NET_P2G_SYNC_OK && lastRecvType == NET_G2E_GAME_START {
                    code = MENU_CODE_NET_ROOM_GO;
                }
            }

            if MENU_CODE_EXIT_APPLICATION == code {
                self.quit();
                return true;
            } else if MENU_CODE_TO_MAIN_MENU == code {
                self.iDisplayError = DISPLAY_ERROR_NONE;
                self.iDisplayErrorTimer = 0;
                net_end_session();

                self.mCurrentMenu = menu(self.mMainMenu);
            } else if MENU_CODE_BACK_TO_MATCH_SELECTION_MENU == code {
                self.mCurrentMenu = menu(self.mMatchSelectionMenu);
                self.iUnlockMinigameOptionIndex = 0;
            } else if MENU_CODE_TO_MATCH_SELECTION_MENU == code {
                self.mMatchSelectionMenu.world_map_changed();

                self.mCurrentMenu = menu(self.mMatchSelectionMenu);
                self.mCurrentMenu.reset_menu();
                self.iUnlockMinigameOptionIndex = 0;
            } else if MENU_CODE_MATCH_SELECTION_START == code || MENU_CODE_QUICK_GAME_START == code {
                println!("itt vagyok");

                if MENU_CODE_QUICK_GAME_START == code {
                    game_values.matchtype = MatchType::QuickGame;
                } else {
                    game_values.matchtype = self.mMatchSelectionMenu.get_selected_match_type();
                }

                self.mTeamSelectMenu.reset_team_select();
                self.mCurrentMenu = menu(self.mTeamSelectMenu);
                self.mCurrentMenu.reset_menu();
                println!("Hello");

                if game_values.matchtype != MatchType::Tournament {
                    game_values.tournamentcontrolteam = -1;
                    self.set_controlling_team_for_settings_menu(game_values.tournamentcontrolteam, false);
                }
            } else if MENU_CODE_MATCH_SELECTION_MATCH_CHANGED == code {
                self.mMatchSelectionMenu.selection_changed();
            } else if MENU_CODE_WORLD_MAP_CHANGED == code {
                self.mMatchSelectionMenu.world_map_changed();
            } else if MENU_CODE_TO_OPTIONS_MENU == code {
                self.mCurrentMenu = menu(self.mOptionsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_BACK_TO_OPTIONS_MENU == code {
                self.mCurrentMenu = menu(self.mOptionsMenu);
            } else if MENU_CODE_TO_NET_SERVERS_MENU == code {
                self.mCurrentMenu = menu(self.mNetServersMenu);
                self.mCurrentMenu.reset_menu();
                net_start_session();
            } else if MENU_CODE_BACK_TO_GRAPHIC_OPTIONS_MENU == code {
                self.mCurrentMenu = menu(self.mGraphicsOptionsMenu);
            } else if MENU_CODE_TO_CONTROLS_MENU == code {
                self.mCurrentMenu = menu(self.mPlayerControlsSelectMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_BACK_TO_CONTROLS_MENU == code {
                self.mCurrentMenu = menu(self.mPlayerControlsSelectMenu);
            } else if MENU_CODE_TO_PLAYER_1_CONTROLS == code {
                self.mPlayerControlsMenu.set_player(0);
                self.mCurrentMenu = menu(self.mPlayerControlsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_PLAYER_2_CONTROLS == code {
                self.mPlayerControlsMenu.set_player(1);
                self.mCurrentMenu = menu(self.mPlayerControlsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_PLAYER_3_CONTROLS == code {
                self.mPlayerControlsMenu.set_player(2);
                self.mCurrentMenu = menu(self.mPlayerControlsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_PLAYER_4_CONTROLS == code {
                self.mPlayerControlsMenu.set_player(3);
                self.mCurrentMenu = menu(self.mPlayerControlsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TOGGLE_FULLSCREEN == code {
                gfx_changefullscreen(game_values.fullscreen);
                blitdest = screen;
            } else if MENU_CODE_TO_GAME_SETUP_MENU == code {
                self.to_game_setup_menu();
            } else if MENU_CODE_BACK_TO_GAME_SETUP_MENU == code {
                let mut fNeedTeamAnnouncement = false;
                if game_values.matchtype == MatchType::World {
                    if game_values.tournamentwinner == -2 || (game_values.tournamentwinner >= 0 && game_values.bonuswheel == 0) {
                        self.reset_tournament_back_to_main_menu();
                    } else if game_values.tournamentwinner >= 0 {
                        self.mBonusWheelMenu.miBonusWheel.reset(true);
                        self.mCurrentMenu = menu(self.mBonusWheelMenu);
                    } else {
                        self.mCurrentMenu = menu(self.mWorldMenu);
                        self.mWorldMenu.miWorldStop.refresh(game_values.tourstopcurrent as i16);

                        fNeedTeamAnnouncement = true;
                    }
                } else if game_values.tournamentwinner == -2 {
                    //Tied Tour Result
                    self.reset_tournament_back_to_main_menu();
                } else if game_values.tournamentwinner >= 0 {
                    //Tournament/Tour Won and Bonus Wheel will be spun
                    if game_values.bonuswheel == 0 || (game_values.matchtype == MatchType::Tour && game_values.tourstops[game_values.tourstopcurrent - 1].iBonusType == 0) {
                        self.reset_tournament_back_to_main_menu();
                    } else {
                        self.mBonusWheelMenu.miBonusWheel.reset(true);
                        self.mCurrentMenu = menu(self.mBonusWheelMenu);
                    }
                } else {
                    //Next Tour/Tourament Game
                    if game_values.matchtype == MatchType::Tour {
                        self.mCurrentMenu = menu(self.mTourStopMenu);
                    } else {
                        self.mCurrentMenu = menu(self.mGameSettingsMenu);
                    }
                }

                self.mCurrentMenu.reset_menu();

                //Set the controlling team for tournament mode
                if MatchType::Tournament == game_values.matchtype && game_values.tournamentwinner == -1 {
                    self.set_controlling_team_for_settings_menu(game_values.tournamentcontrolteam, true);
                }

                if fNeedTeamAnnouncement {
                    self.mWorldMenu.miWorld.display_team_control_announcement();
                }
            } else if MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS == code {
                if netplay.active {
                    self.mCurrentMenu = menu(self.mNetNewRoomSettingsMenu);
                } else {
                    self.mCurrentMenu = menu(self.mGameSettingsMenu);
                }
            } else if MENU_CODE_MODE_CHANGED == code {
                game_values.gamemode = gamemodes[self.mGameSettingsMenu.get_current_game_mode_id() as usize];
                self.mGameSettingsMenu.refresh_game_mode_buttons();
                self.mNetNewRoomSettingsMenu.refresh_game_mode_buttons();
            } else if MENU_CODE_BACK_TEAM_SELECT_MENU == code {
                if game_values.matchtype == MatchType::World {
                    self.mWorldMenu.open_exit_dialog();
                } else if game_values.matchtype == MatchType::Tour {
                    self.mTourStopMenu.open_exit_dialog();
                } else if game_values.matchtype == MatchType::Tournament {
                    self.mGameSettingsMenu.open_exit_dialog();

                    if self.iTournamentAITimer > 0 {
                        self.set_controlling_team_for_settings_menu(-1, false);
                    }
                } else {
                    self.mCurrentMenu = menu(self.mTeamSelectMenu);
                    self.mCurrentMenu.reset_menu();
                }
            } else if MENU_CODE_SOUND_VOLUME_CHANGED == code {
                game_values.sound = game_values.soundvolume > 0;
                sfx_setsoundvolume(game_values.soundvolume as i32);
                if_sound_on_play(&mut rm.sfx_coin);
            } else if MENU_CODE_MUSIC_VOLUME_CHANGED == code {
                sfx_setmusicvolume(game_values.musicvolume as i32);

                if game_values.musicvolume == 0 {
                    rm.backgroundmusic[2].stop();
                } else if game_values.musicvolume > 0 && !game_values.music {
                    rm.backgroundmusic[2].play(false, false);
                }

                game_values.music = game_values.musicvolume > 0;
            } else if MENU_CODE_START_GAME == code {
                self.start_game();
            } else if MENU_CODE_EXIT_TOURNAMENT_YES == code || MENU_CODE_EXIT_TOURNAMENT_NO == code {
                self.mGameSettingsMenu.close_exit_dialog();

                if MENU_CODE_EXIT_TOURNAMENT_YES == code {
                    self.reset_tournament_back_to_main_menu();
                } else {
                    self.set_controlling_team_for_settings_menu(game_values.tournamentcontrolteam, false);
                }
            } else if MENU_CODE_EXIT_TOUR_YES == code || MENU_CODE_EXIT_TOUR_NO == code {
                self.mTourStopMenu.close_exit_dialog();

                if MENU_CODE_EXIT_TOUR_YES == code {
                    self.reset_tournament_back_to_main_menu();
                }
            } else if MENU_CODE_EXIT_WORLD_YES == code || MENU_CODE_EXIT_WORLD_NO == code {
                self.mWorldMenu.close_exit_dialog();

                if MENU_CODE_EXIT_WORLD_YES == code {
                    //Clear out any stored items a player might have
                    for iPlayer in 0..4usize {
                        game_values.storedpowerups[iPlayer] = -1;
                    }

                    self.reset_tournament_back_to_main_menu();
                    self.mWorldMenu.miWorldStop.set_visible(false);
                }
            } else if MENU_CODE_BONUS_DONE == code {
                if self.mBonusWheelMenu.miBonusWheel.get_powerup_selection_done() {
                    if game_values.matchtype == MatchType::World {
                        if game_values.tournamentwinner == -1 {
                            self.mCurrentMenu = menu(self.mTournamentScoreboardMenu);
                        } else {
                            self.reset_tournament_back_to_main_menu();
                        }
                    } else if (game_values.matchtype == MatchType::Tour || game_values.matchtype == MatchType::Tournament)
                        && game_values.tournamentwinner == -1
                        && ((game_values.matchtype != MatchType::Tour && game_values.bonuswheel == 2)
                            || (game_values.matchtype == MatchType::Tour && game_values.tourstops[game_values.tourstopcurrent - 1].iBonusType != 0))
                    {
                        self.mCurrentMenu = menu(self.mTournamentScoreboardMenu);
                    } else if game_values.matchtype == MatchType::SingleGame {
                        self.mCurrentMenu = menu(self.mGameSettingsMenu);
                        self.mCurrentMenu.reset_menu();
                        self.mGameSettingsMenu.miMapField.load_current_map();
                    } else {
                        self.reset_tournament_back_to_main_menu();
                    }
                }
            } else if MENU_CODE_TO_POWERUP_SELECTION_MENU == code {
                self.mCurrentMenu = menu(self.mPowerupDropRatesMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_POWERUP_SETTINGS_MENU == code {
                self.mCurrentMenu = menu(self.mPowerupSettingsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_GRAPHICS_OPTIONS_MENU == code {
                self.mCurrentMenu = menu(self.mGraphicsOptionsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_EYECANDY_OPTIONS_MENU == code {
                self.mCurrentMenu = menu(self.mEyeCandyOptionsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_SOUND_OPTIONS_MENU == code {
                self.mCurrentMenu = menu(self.mSoundOptionsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_GAMEPLAY_OPTIONS_MENU == code {
                self.mCurrentMenu = menu(self.mGameplayOptionsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_TEAM_OPTIONS_MENU == code {
                self.mCurrentMenu = menu(self.mTeamOptionsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_PROJECTILES_OPTIONS_MENU == code {
                self.mCurrentMenu = menu(self.mProjectileOptionsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_PROJECTILES_LIMITS_MENU == code {
                self.mCurrentMenu = menu(self.mProjectileLimitsMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_TO_MODE_SETTINGS_MENU == code {
                if netplay.active {
                    self.mCurrentMenu = self.mModeOptionsMenu.get_options_menu(self.mNetNewRoomSettingsMenu.get_selected_game_mode_id());
                    self.mCurrentMenu.reset_menu();
                } else {
                    for iGameMode in 0..GAMEMODE_LAST as i16 {
                        if self.mGameSettingsMenu.miGoalField[iGameMode as usize].is_visible() {
                            self.mCurrentMenu = self.mModeOptionsMenu.get_options_menu(iGameMode);
                            self.mCurrentMenu.reset_menu();
                            break;
                        }
                    }
                }
            } else if MENU_CODE_MENU_GRAPHICS_PACK_CHANGED == code {
                rm.load_menu_graphics();

                blitdest = rm.menu_backdrop.get_surface();
                rm.menu_shade.setalpha(App::menuTransparency as u8);
                rm.menu_shade.draw(0, 0);
                blitdest = screen;
            } else if MENU_CODE_WORLD_GRAPHICS_PACK_CHANGED == code {
                rm.load_world_graphics();
            } else if MENU_CODE_GAME_GRAPHICS_PACK_CHANGED == code {
                let packdir = gamegraphicspacklist.current_path().to_string_lossy().into_owned();
                let png = std::panic::catch_unwind(|| gfx_loadpalette(Path::new(&convert_path_pack("gfx/packs/palette.png", &packdir))));
                if let Err(payload) = png {
                    let Some(err) = payload.downcast_ref::<String>() else { std::panic::resume_unwind(payload) };
                    println!("\nwarning: {} -> falling back to BMP", err);
                    gfx_loadpalette(Path::new(&convert_path_pack("gfx/packs/palette.bmp", &packdir)));
                }
                rm.load_game_graphics();
            } else if MENU_CODE_SOUND_PACK_CHANGED == code {
                rm.load_game_sounds();

                if !game_values.soundcapable {
                    game_values.sound = false;
                    game_values.music = false;
                    game_values.soundvolume = 0;
                    game_values.musicvolume = 0;
                }
            } else if MENU_CODE_WORLD_STAGE_START == code {
                self.mWorldMenu.open_stage_start();
            } else if MENU_CODE_WORLD_STAGE_NO_START == code {
                self.mWorldMenu.close_stage_start();
            } else if MENU_CODE_WORLD_MUSIC_CHANGED == code {
                worldmusiclist.set_current(self.mSoundOptionsMenu.get_current_world_music_id() as usize);
            } else if MENU_CODE_TOUR_STOP_CONTINUE == code || MENU_CODE_TOUR_STOP_CONTINUE_FORCED == code {
                //If this tour stop is forced, we need to load the map first
                if MENU_CODE_TOUR_STOP_CONTINUE_FORCED == code {
                    self.mWorldMenu.miWorldStop.refresh(game_values.tourstopcurrent as i16);
                }

                self.mWorldMenu.miWorld.clear_cloud();

                //Tour bonus house
                if game_values.matchtype == MatchType::World && game_values.tourstops[game_values.tourstopcurrent].iStageType == 1 {
                    bonushousemode.goal = 0;
                    game_values.gamemode = Ptr::from_raw(bonushousemode.as_ptr() as *mut dyn CGameModeTrait);
                } else {
                    let iGameMode = game_values.tourstops[game_values.tourstopcurrent].iMode;

                    if iGameMode as GameModeType == game_mode_pipe_minigame {
                        game_values.gamemode = Ptr::from_raw(pipegamemode.as_ptr() as *mut dyn CGameModeTrait);
                    } else if iGameMode as GameModeType == game_mode_boss_minigame {
                        game_values.gamemode = Ptr::from_raw(bossgamemode.as_ptr() as *mut dyn CGameModeTrait);
                    } else if iGameMode as GameModeType == game_mode_boxes_minigame {
                        game_values.gamemode = Ptr::from_raw(boxesgamemode.as_ptr() as *mut dyn CGameModeTrait);
                    } else {
                        game_values.gamemode = gamemodes[iGameMode as usize];
                    }

                    game_values.gamemode.goal = game_values.tourstops[game_values.tourstopcurrent].iGoal;
                }

                self.start_game();
            } else if MENU_CODE_RESET_STORED_POWERUPS == code {
                for iPlayer in 0..4usize {
                    game_values.storedpowerups[iPlayer] = -1;
                }
            } else if MENU_CODE_MAP_CHANGED == code {
                if self.mCurrentMenu == menu(self.mNetNewRoomSettingsMenu) {
                    debug_assert!(netplay.active);
                    netplay.mapfilepath = self.mNetNewRoomSettingsMenu.get_current_map_path();
                    println!("[net] Selected map: {}", netplay.mapfilepath);
                    self.mNetRoomMenu.set_preview_map_path(&netplay.mapfilepath);
                } else if game_values.matchtype != MatchType::Tour {
                    self.szCurrentMapName = Ptr::from_mut(&mut self.mGameSettingsMenu.miMapField.szMapName);
                }
            } else if MENU_CODE_MAP_FILTER_EXIT == code {
                maplist.apply_filters(&game_values.pfFilters);

                //If the filtered map list has at least 1 map in it, then allow exiting the filter menu
                if maplist.map_in_filtered_set() {
                    self.mGameSettingsMenu.miMapField.load_current_map();
                    self.szCurrentMapName = Ptr::from_mut(&mut self.mGameSettingsMenu.miMapField.szMapName);

                    self.mGameSettingsMenu.close_map_filters();

                    self.iDisplayError = DISPLAY_ERROR_NONE;
                } else {
                    //otherwise display a message
                    self.iDisplayError = DISPLAY_ERROR_MAP_FILTER;
                    self.iDisplayErrorTimer = 120;
                }
            } else if MENU_CODE_TO_MAP_FILTERS == code {
                self.mGameSettingsMenu.open_map_filters();
            } else if MENU_CODE_TO_MAP_FILTER_EDIT == code {
                self.mMapFilterEditMenu.miMapBrowser.reset(0);

                self.mCurrentMenu = menu(self.mMapFilterEditMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_MAP_BROWSER_EXIT == code {
                self.mGameSettingsMenu.miMapField.load_current_map();
                self.szCurrentMapName = Ptr::from_mut(&mut self.mGameSettingsMenu.miMapField.szMapName);

                self.mCurrentMenu = menu(self.mGameSettingsMenu);
                //mCurrentMenu->ResetMenu();
            } else if MENU_CODE_TO_MAP_BROWSER_THUMBNAILS == code {
                self.mMapFilterEditMenu.miMapBrowser.reset(1);

                self.mCurrentMenu = menu(self.mMapFilterEditMenu);
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_SAVE_ALL_MAP_THUMBNAILS == code {
                // C++ casts mCurrentMenu to UI_OptionsMenu*; only the options menu emits this code.
                self.mOptionsMenu.show_thumbnails_popup();
                self.mCurrentMenu.reset_menu();
            } else if MENU_CODE_GENERATE_THUMBS_RESET_YES == code || MENU_CODE_GENERATE_THUMBS_RESET_NO == code {
                self.mOptionsMenu.hide_thumbnails_popup();

                if MENU_CODE_GENERATE_THUMBS_RESET_YES == code {
                    *fGenerateMapThumbs = true;
                }
            } else if MENU_CODE_HEALTH_MODE_START_LIFE_CHANGED == code {
                self.mModeOptionsMenu.health_mode_start_life_changed();
            } else if MENU_CODE_HEALTH_MODE_MAX_LIFE_CHANGED == code {
                self.mModeOptionsMenu.health_mode_max_life_changed();
            }

            if netplay.active {
                let lastSent = netplay.client.lastSentMessage;
                let lastRecv = netplay.client.lastReceivedMessage;

                // Override menu code if response has arrived
                if netplay.operationInProgress {
                    let previousCode = code;

                    // if sent connect request,
                    // received connect_ok
                    // then sent skin change
                    if lastSent.packageType == NET_NOTICE_SKIN_CHANGE && lastRecv.packageType == NET_RESPONSE_CONNECT_OK && lastSent.timestamp >= lastRecv.timestamp {
                        code = MENU_CODE_TO_NET_LOBBY_MENU;
                    } else if lastSent.packageType == NET_REQUEST_JOIN_ROOM && lastRecv.packageType == NET_RESPONSE_JOIN_OK && lastSent.timestamp < lastRecv.timestamp {
                        // ensure that the incoming message arrived after the request
                        code = MENU_CODE_TO_NET_ROOM_MENU;
                    } else if lastSent.packageType == NET_NOTICE_GAMEMODESETTINGS && lastRecv.packageType == NET_RESPONSE_CREATE_OK {
                        code = MENU_CODE_TO_NET_ROOM_MENU;
                    }

                    // If we have finished the waiting
                    if code != previousCode {
                        netplay.operationInProgress = false;
                    }
                }

                if MENU_CODE_TO_NET_SERVERLIST == code {
                    self.mNetServersMenu.open_server_list();
                } else if MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU == code {
                    self.mCurrentMenu = menu(self.mNetEditServersMenu);
                    self.mCurrentMenu.reset_menu();
                    self.mNetEditServersMenu.restore();
                } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_ADD_BTN == code {
                    self.mNetEditServersMenu.on_press_add();
                } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_EDIT_BTN == code {
                    self.mNetEditServersMenu.on_press_edit();
                } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_DELETE_BTN == code {
                    self.mNetEditServersMenu.on_press_delete();
                } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_SELECT == code {
                    self.mNetEditServersMenu.on_entry_select();
                    self.mNetServersMenu.refresh_scroll();
                } else if MENU_CODE_NET_ADDREMOVE_SERVER_ON_DIALOG_OK_BTN == code {
                    self.mNetEditServersMenu.on_dialog_ok();
                    self.mNetServersMenu.refresh_scroll();
                } else if MENU_CODE_NET_SERVERLIST_EXIT == code || MENU_CODE_NET_CONNECT_ABORT == code {
                    if MENU_CODE_NET_SERVERLIST_EXIT == code {
                        netplay.currentMenuChanged = true;
                    } else {
                        net_start_session(); // release & restart socket
                        netplay.operationInProgress = false;
                    }

                    self.mNetServersMenu.restore();
                    self.iDisplayError = DISPLAY_ERROR_NONE;
                } else if MENU_CODE_NET_CONNECT_IN_PROGRESS == code {
                    self.mNetServersMenu.connect_in_progress();
                } else if MENU_CODE_TO_NET_LOBBY_MENU == code {
                    // If we are leaving a room
                    if netplay.currentRoom.roomID != 0 {
                        netplay.client.send_leave_room_message();
                        netplay.currentRoom.roomID = 0;
                        netplay.joinSuccessful = false;
                        netplay.gameRunning = false;
                    }
                    netplay.client.request_room_list();

                    // Restore layouts
                    self.mNetServersMenu.restore();
                    self.mNetLobbyMenu.restore();
                    self.mNetNewRoomMenu.restore();

                    self.mCurrentMenu = menu(self.mNetLobbyMenu);
                    self.mCurrentMenu.reset_menu();
                } else if MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU == code {
                    self.mCurrentMenu = menu(self.mNetNewRoomSettingsMenu);
                    self.mCurrentMenu.reset_menu();
                } else if MENU_CODE_TO_NET_NEW_ROOM_SETTINGS_MENU == code {
                    self.mCurrentMenu = menu(self.mNetNewRoomMenu);
                    self.mCurrentMenu.reset_menu();
                } else if MENU_CODE_NET_CHAT_SEND == code {
                    if !netplay.mychatmessage.is_empty() {
                        let message = netplay.mychatmessage.clone();
                        netplay.client.send_chat_message(&message);
                    } else {
                        code = MENU_CODE_TO_NET_ROOM_MENU;
                    }
                } else if MENU_CODE_TO_NET_ROOM_MENU == code {
                    self.mCurrentMenu = menu(self.mNetRoomMenu);
                    self.mCurrentMenu.reset_menu();
                    netplay.currentMenuChanged = true;
                    netplay.operationInProgress = false;

                    self.mNetRoomMenu.restore(); // Restore Room layout
                    self.mNetLobbyMenu.restore(); // Restore Lobby layout
                    self.mNetNewRoomMenu.restore(); // remove 'in progress' dialog
                } else if MENU_CODE_NET_JOIN_ROOM_IN_PROGRESS == code {
                    self.mNetLobbyMenu.join_in_progress();
                } else if MENU_CODE_NET_JOIN_ROOM_ABORT == code {
                    self.mNetLobbyMenu.abort_join();
                    self.iDisplayError = DISPLAY_ERROR_NONE;
                } else if MENU_CODE_TO_NET_NEW_ROOM_CREATE_IN_PROGRESS == code {
                    self.mNetNewRoomMenu.create_in_progress();
                } else if MENU_CODE_TO_NET_NEW_ROOM_CREATE_ABORT == code {
                    self.mNetNewRoomMenu.abort_create();
                    self.iDisplayError = DISPLAY_ERROR_NONE;
                } else if MENU_CODE_TO_NET_ROOM_START_IN_PROGRESS == code {
                    self.mNetRoomMenu.start_in_progress();
                } else if MENU_CODE_NET_ROOM_GO == code {
                    netplay.operationInProgress = false;
                    game_values.matchtype = MatchType::NetGame;

                    self.mTeamSelectMenu.reset_team_select();
                    self.mTeamSelectMenu.reset_menu();
                    score_cnt = self.mTeamSelectMenu.get_team_count();

                    game_values.tournamentcontrolteam = -1;
                    game_values.tournamentwinner = -1;

                    self.iDisplayError = DISPLAY_ERROR_NONE;
                    self.iDisplayErrorTimer = 0;

                    //game_values.noexit = true;
                    self.start_game();
                }

                // on room change
                if netplay.currentMenuChanged {
                    self.mNetServersMenu.refresh();
                    self.mNetRoomMenu.refresh();

                    netplay.currentMenuChanged = false;
                }
            }

            /*if (code != MENU_CODE_NONE)
                printf("Code: %d\n", code);*/
            false
        }
    }
}

impl GameState for MenuState {
    fn init(&mut self) -> bool {
        unsafe {
            assign(&mut self.mMainMenu, UI_MainMenu::new());

            // Options menu
            assign(&mut self.mOptionsMenu, UI_OptionsMenu::new());
            assign(&mut self.mGameplayOptionsMenu, UI_GameplayOptionsMenu::new());
            assign(&mut self.mTeamOptionsMenu, UI_TeamOptionsMenu::new());
            assign(&mut self.mPowerupDropRatesMenu, UI_PowerupDropRatesMenu::new());
            assign(&mut self.mPowerupSettingsMenu, UI_PowerupSettingsMenu::new());
            assign(&mut self.mProjectileLimitsMenu, UI_ProjectileLimitsMenu::new());
            assign(&mut self.mProjectileOptionsMenu, UI_ProjectileOptionsMenu::new());
            assign(&mut self.mGraphicsOptionsMenu, UI_GraphicsOptionsMenu::new());
            assign(&mut self.mEyeCandyOptionsMenu, UI_EyeCandyOptionsMenu::new());
            assign(&mut self.mSoundOptionsMenu, UI_SoundOptionsMenu::new());

            // Controls menu
            assign(&mut self.mPlayerControlsSelectMenu, UI_PlayerControlsSelectMenu::new());
            assign(&mut self.mPlayerControlsMenu, UI_PlayerControlsMenu::new());

            // Gameplay menu
            assign(&mut self.mModeOptionsMenu, UI_ModeOptionsMenu::new());
            assign(&mut self.mMatchSelectionMenu, UI_MatchSelectionMenu::new());
            assign(&mut self.mGameSettingsMenu, UI_GameSettingsMenu::new());
            assign(&mut self.mMapFilterEditMenu, UI_MapFilterEditMenu::new());
            assign(&mut self.mTourStopMenu, UI_TourStopMenu::new());
            assign(&mut self.mWorldMenu, UI_WorldMenu::new());
            assign(&mut self.mTeamSelectMenu, UI_TeamSelectMenu::new());
            assign(&mut self.mTournamentScoreboardMenu, UI_TournamentScoreboardMenu::new());
            assign(&mut self.mBonusWheelMenu, UI_BonusWheelMenu::new());

            // Multiplayer menu
            assign(&mut self.mNetServersMenu, UI_NetServersMenu::new());
            assign(&mut self.mNetEditServersMenu, UI_NetEditServersMenu::new());
            assign(&mut self.mNetLobbyMenu, UI_NetLobbyMenu::new());
            assign(&mut self.mNetNewRoomMenu, UI_NetNewRoomMenu::new());
            assign(&mut self.mNetNewRoomSettingsMenu, UI_NetNewRoomSettingsMenu::new(&self.mGameSettingsMenu));
            assign(&mut self.mNetRoomMenu, UI_NetRoomMenu::new());

            self.mCurrentMenu = menu(self.mMainMenu);

            if let Some(forcedMap) = harness::forced_map() {
                if !maplist.findexact(forcedMap, false) && !maplist.find(forcedMap) {
                    eprintln!("[harness] SMW_MAP '{}' not found", forcedMap);
                    std::process::exit(2);
                }
                self.mGameSettingsMenu.miMapField.load_current_map();
            }

            self.szCurrentMapName = Ptr::from_mut(&mut self.mGameSettingsMenu.miMapField.szMapName);

            true
        }
    }

    //---------------------------------------------------------------
    // RUN THE MENU
    //---------------------------------------------------------------

    fn on_enter_state(&mut self) {
        unsafe {
            self.iUnlockMinigameOptionIndex = 0;

            self.iTournamentAIStep = 0;
            self.iTournamentAITimer = 0;

            //Reset the keys each time we switch from menu to game and back
            game_values.playerInput.reset_keys();

            self.fNeedMenuMusicReset = false;

            let matchtype = game_values.matchtype;
            if game_values.gamemode.winningteam > -1
                && game_values.tournamentwinner == -1
                && (((matchtype == MatchType::SingleGame || matchtype == MatchType::QuickGame || matchtype == MatchType::MiniGame || matchtype == MatchType::Tournament || matchtype == MatchType::NetGame)
                    && game_values.bonuswheel == 2)
                    || (matchtype == MatchType::Tour && game_values.tourstops[game_values.tourstopcurrent - 1].iBonusType != 0))
            {
                self.mBonusWheelMenu.miBonusWheel.reset(false);
                self.mCurrentMenu = menu(self.mBonusWheelMenu);
            } else if matchtype != MatchType::SingleGame && matchtype != MatchType::QuickGame && matchtype != MatchType::MiniGame && matchtype != MatchType::NetGame {
                self.mCurrentMenu = menu(self.mTournamentScoreboardMenu);
            } else if matchtype == MatchType::QuickGame {
                //Reset back to main menu after quick game
                self.mCurrentMenu = menu(self.mMainMenu);
                self.mCurrentMenu.reset_menu();
            } else if matchtype == MatchType::NetGame {
                self.mCurrentMenu = menu(self.mNetLobbyMenu);
                self.mCurrentMenu.reset_menu();
                netplay.joinSuccessful = false;
                netplay.gameRunning = false;
            }

            if game_values.matchtype == MatchType::World {
                if game_values.gamemode.winningteam > -1 && game_values.tournamentwinner == -1 {
                    //Stage is completed
                    let winningteam = game_values.gamemode.winningteam;
                    self.mWorldMenu.miWorld.set_controlling_team(winningteam);
                    self.mWorldMenu.miWorld.set_current_stage_to_completed(winningteam);

                    self.mWorldMenu.close_stage_start();

                    //Clear out the stored powerups after a game that had a winner
                    if !game_values.worldskipscoreboard {
                        //Only clear out the stored powerup if we were allowed to use it in the game
                        if game_values.gamemode.has_stored_powerups() {
                            for iPlayer in 0..MAX_PLAYERS as usize {
                                game_values.storedpowerups[iPlayer] = -1;
                            }
                        }
                    }
                }
                /*else if (game_values.tournamentwinner > -1) //World is completed
                {
                    miBonusWheel->Reset(true);
                    mCurrentMenu = mBonusWheelMenu;
                }*/

                crate::smw::gs_gameplay::update_score_board();

                //If we're suposed to skip the scoreboard, then reset back to the world map
                if game_values.worldskipscoreboard {
                    self.mCurrentMenu = menu(self.mWorldMenu);
                    self.mCurrentMenu.reset_menu();

                    game_values.worldskipscoreboard = false;
                } else {
                    let winningteam = game_values.gamemode.winningteam;
                    self.mTournamentScoreboardMenu.miTournamentScoreboard.refresh_world_scores(winningteam);
                }
            } else if game_values.matchtype == MatchType::Tour {
                self.mTournamentScoreboardMenu.miTournamentScoreboard.refresh_tour_scores();

                if game_values.tourstopcurrent < game_values.tourstops.len() {
                    self.mTourStopMenu.miTourStop.refresh(game_values.tourstopcurrent as i16);
                }
            } else if game_values.matchtype == MatchType::Tournament {
                self.mTournamentScoreboardMenu.miTournamentScoreboard.stop_swirl();
                if game_values.gamemode.winningteam > -1 {
                    let winningteam = game_values.gamemode.winningteam;
                    self.mTournamentScoreboardMenu.miTournamentScoreboard.refresh_tournament_scores(winningteam);

                    self.choose_next_tournament_control_team();
                }
            }

            //Reset game mode back to the current game mode in case we came from boss mode
            game_values.gamemode = gamemodes[currentgamemode as usize];

            //Keep track if we entered the menu loop as part of a tournament, if we exit the tournament
            //we need to reset the menu music back to normal
            let matchtype = game_values.matchtype;
            if matchtype != MatchType::SingleGame && matchtype != MatchType::QuickGame && matchtype != MatchType::MiniGame && matchtype != MatchType::NetGame {
                self.fNeedMenuMusicReset = true;
            }

            if game_values.music && game_values.tournamentwinner < 0 {
                if matchtype == MatchType::SingleGame || matchtype == MatchType::QuickGame || matchtype == MatchType::MiniGame || matchtype == MatchType::NetGame {
                    rm.backgroundmusic[2].play(false, false);
                } else if matchtype == MatchType::World {
                    rm.backgroundmusic[5].play(false, false);
                } else {
                    rm.backgroundmusic[3].play(false, false);
                }
            }

            if matchtype != MatchType::World && matchtype != MatchType::QuickGame && matchtype != MatchType::NetGame {
                if self.mCurrentMenu == menu(self.mGameSettingsMenu) || self.mCurrentMenu == menu(self.mTournamentScoreboardMenu) {
                    self.mGameSettingsMenu.miMapField.load_current_map();
                }
            }

            // On return from a game, refresh room list
            if netplay.active {
                if netplay.currentRoom.roomID != 0 {
                    netplay.client.send_leave_room_message();
                    netplay.currentRoom.roomID = 0;
                    netplay.joinSuccessful = false;
                    netplay.gameRunning = false;
                }
                netplay.client.request_room_list();
            }
        }
    }

    fn update(&mut self) {
        unsafe {
            if netplay.active {
                netplay.client.update();
            }

            //Reset the keys that were down the last frame
            game_values.playerInput.clear_pressed_keys(1);

            //handle messages
            #[cfg(target_os = "emscripten")]
            frame_events.clear();
            while let Some(input) = smw_sdl2::events::poll_input() {
                #[cfg(target_os = "emscripten")]
                frame_events.push(input.clone());
                if input.event == InputEvent::Quit {
                    self.exit();
                    return;
                } else if let InputEvent::Key { key, mods, down: true, .. } = input.event {
                    let sym = key.0;
                    if sym == SDL_KeyCode::SDLK_F1 as i32 {
                        game_values.showfps = !game_values.showfps;
                    }

                    if (mods as u32) & (SDL_Keymod::KMOD_LALT as u32 | SDL_Keymod::KMOD_RALT as u32) != 0 {
                        //ALT + F4 = close window
                        if sym == SDL_KeyCode::SDLK_F4 as i32 {
                            self.exit();
                            return;
                        }
                        //ALT + Enter = fullscreen/windowed toggle
                        else if sym == SDL_KeyCode::SDLK_RETURN as i32 {
                            game_values.fullscreen = !game_values.fullscreen;
                            gfx_changefullscreen(game_values.fullscreen);
                            blitdest = screen;

                            continue;
                        }
                    }

                    if sym == SDL_KeyCode::SDLK_INSERT as i32 {
                        gfx_take_screenshot();
                    }
                }

                game_values.playerInput.update_input(&input, 1);
            }

            //If AI is controlling the tournament menu, select the options
            if game_values.matchtype == MatchType::Tournament && self.iTournamentAITimer > 0 && self.mCurrentMenu == menu(self.mGameSettingsMenu) && self.mGameSettingsMenu.is_on_start_btn() {
                self.iTournamentAITimer -= 1;
                if self.iTournamentAITimer == 0 {
                    self.iTournamentAIStep += 1;

                    if self.iTournamentAIStep == 1 {
                        self.mGameSettingsMenu.miMapField.choose_random_map();

                        self.iTournamentAITimer = 60;
                    } else if self.iTournamentAIStep == 2 {
                        currentgamemode = RANDOM_INT(GAMEMODE_LAST) as i16;
                        game_values.gamemode = gamemodes[currentgamemode as usize];
                        self.mGameSettingsMenu.game_mode_changed(currentgamemode);
                        game_values.gamemode = gamemodes[self.mGameSettingsMenu.get_current_game_mode_id() as usize];

                        self.iTournamentAITimer = 60;
                    } else if self.iTournamentAIStep == 3 {
                        //Choose a goal from the lower values for a quicker game
                        let iRandOption = (RANDOM_INT(6) + 1) as i16;
                        let goal = game_values.gamemode.get_options()[iRandOption as usize].iValue;
                        game_values.gamemode.goal = goal;

                        let modeGoal = gamemodes[currentgamemode as usize].goal;
                        self.mGameSettingsMenu.miGoalField[currentgamemode as usize].set_current_value(modeGoal);

                        self.mModeOptionsMenu.set_random_game_mode_settings(game_values.gamemode.gamemode as i16);

                        self.iTournamentAITimer = 60;
                    } else if self.iTournamentAIStep == 4 {
                        self.iTournamentAIStep = 0;
                        self.start_game();
                    }
                }
            }

            //Watch for the konami code to unlock the minigames match type
            /*
            if (!game_values.minigameunlocked && mCurrentMenu == mMatchSelectionMenu) {
                ... (commented out in the C++)
            }
            */

            let mut fGenerateMapThumbs = false;
            if AppState::Menu == game_values.appstate {
                let code = self.mCurrentMenu.send_input(Ptr::from_mut(&mut game_values.playerInput));

                if self.handle_menu_code(code, &mut fGenerateMapThumbs) {
                    return;
                }
            }

            //--------------- draw everything ----------------------

            //Don't draw backdrop for world
            if self.mCurrentMenu != menu(self.mWorldMenu) {
                rm.menu_backdrop.draw(0, 0);
            } else {
                SDL_FillRect(screen, std::ptr::null(), SDL_MapRGB((*screen).format, 0, 0, 0));
            }

            self.mCurrentMenu.update();
            self.mCurrentMenu.draw();

            if self.iDisplayError > DISPLAY_ERROR_NONE {
                rm.spr_selectfield.draw_src(70, 400, &rect(0, 0, 484, 32));
                rm.spr_selectfield.draw_src(554, 400, &rect(496, 0, 16, 32));

                if self.iDisplayError == DISPLAY_ERROR_READ_TOUR_FILE {
                    rm.menu_font_large.draw_centered(320, 405, "Error Reading Tour File!");
                }
                if self.iDisplayError == DISPLAY_ERROR_READ_WORLD_FILE {
                    rm.menu_font_large.draw_centered(320, 405, "Error Reading World File!");
                } else if self.iDisplayError == DISPLAY_ERROR_MAP_FILTER {
                    rm.menu_font_large.draw_centered(320, 405, "No Maps Meet All Filter Conditions!");
                }

                self.iDisplayErrorTimer -= 1;
                if self.iDisplayErrorTimer == 0 {
                    self.iDisplayError = DISPLAY_ERROR_NONE;
                }
            }

            if game_values.screenfadespeed != 0 {
                game_values.screenfade += game_values.screenfadespeed;

                if game_values.screenfade <= 0 {
                    game_values.screenfadespeed = 0;
                    game_values.screenfade = 0;
                } else if game_values.screenfade >= 255 {
                    game_values.screenfadespeed = 0;
                    game_values.screenfade = 255;
                }
            }

            if game_values.screenfade > 0 {
                rm.menu_shade.setalpha(game_values.screenfade as u8);
                rm.menu_shade.draw(0, 0);
            }

            if game_values.screenfade == 255 {
                if AppState::StartGame == game_values.appstate {
                    self.enter_gameplay();
                    return;
                } else if AppState::StartWorld == game_values.appstate {
                    //Fade to world match type
                    game_values.screenfadespeed = -8;

                    self.mCurrentMenu = menu(self.mWorldMenu);
                    self.mCurrentMenu.reset_menu();

                    rm.backgroundmusic[2].stop();
                    let worldMusic = worldmusiclist.current_music(g_worldmap.get_music_category(), g_worldmap.get_world_name()).clone();
                    rm.backgroundmusic[5] = sfxMusic::from_file(&worldMusic).unwrap_or_else(|e| std::panic::panic_any(e));
                    rm.backgroundmusic[5].play(false, false);
                    self.fNeedMenuMusicReset = true;

                    self.mWorldMenu.miWorld.display_team_control_announcement();

                    game_values.appstate = AppState::Menu;
                }
            }

            if fGenerateMapThumbs {
                rm.menu_dialog.draw_src(160, 176, &rect(0, 0, 160, 64));
                rm.menu_dialog.draw_src(App::screenWidth / 2, 176, &rect(352, 0, 160, 64));
                rm.menu_dialog.draw_src(160, 240, &rect(0, 416, 160, 64));
                rm.menu_dialog.draw_src(App::screenWidth / 2, App::screenHeight / 2, &rect(352, 416, 160, 64));
                rm.menu_font_large.draw_centered(App::screenWidth / 2, 215, "Refreshing Map Thumbnails");
                rm.menu_font_large.draw_centered(App::screenWidth / 2, 245, "Please Wait...");
            }

            if fGenerateMapThumbs {
                rm.backgroundmusic[2].toggle_pause();

                //Reload map auto filters from live map files (don't use the cache)
                maplist.reload_map_auto_filters();

                //Write out all the map thumbnails for the map browser and filter editor
                let mut itr = maplist.get_iterator_at(0, false);

                let iMapCount = maplist.count() as i16;
                for _iMap in 0..iMapCount {
                    let filename = maplist.node_at(itr).filename.clone();
                    let mut szThumbnail = String::from("maps/cache/");
                    szThumbnail += &get_name_from_file_name(&filename, false);
                    szThumbnail += ".png";

                    g_map.load_map(&filename, read_type_preview);
                    g_map.save_thumbnail(&convert_path(&szThumbnail), false);

                    itr += 1;
                }

                rm.backgroundmusic[2].toggle_pause();
            }
        }
    }
}
