//! Port of src/smw/GSMenu.cpp (reference build: original + tools/cpp-harness.patch).

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
use crate::smw::net::{net_end_session, net_start_session};
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

fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect { x, y, w, h }
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

    pub fn instance(g: &mut Game) -> &'static mut MenuState {
        g.ptr().get().ms.get_or_insert_with(MenuState::new)
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

            let ts: TourStop = parse_tour_stop_line(g, buffer.as_bytes(), &version, false);
            g.game_values.tourstops.push(Ptr::new_box(ts));
        }

        if !g.game_values.tourstops.is_empty() {
            self.mTourStopMenu.miTourStop.refresh(g, g.game_values.tourstopcurrent as i16);

            //For old tours, turn on the bonus wheel at the end
            if version.major == 1 && version.minor == 7 && version.patch == 0 && version.build <= 1 {
                let last = g.game_values.tourstops.len() - 1;
                g.game_values.tourstops[last].iBonusType = 1;
            }
        }

        !g.game_values.tourstops.is_empty()
    }

    fn start_game(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        println!("> StartGame");

        g.game_values.appstate = AppState::StartGame;
        //backgroundonly = false;
        //fastmap = false;

        gp.game_values.write_config(g);

        //Load skins for players
        println!("Loading player skins...");
        for k in 0..4usize {
            if g.game_values.playercontrol[k] > 0 {
                if g.netplay.active {
                    if k == g.netplay.remotePlayerNumber as usize {
                        // local player uses local skin
                        println!("  player {} -> local", k);
                        let skin = gp.rm.load_full_skin(g, gp.game_values.skinids[k], gp.game_values.colorids[k]);
                        g.rm.spr_player[k] = skin;
                    } else {
                        let path = format!("{}net_skin{}.bmp", get_home_directory(g), k);
                        println!("  player {} -> {}", k, path);

                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gp.rm.load_full_skin_path(g, Path::new(&path), k as i16)));
                        match result {
                            Ok(skin) => g.rm.spr_player[k] = skin,
                            Err(_) => {
                                println!("[warning] Could not load netplay skin of player {}, using default", k);
                                let skin = gp.rm.load_full_skin(g, gp.game_values.skinids[k], gp.game_values.colorids[k]);
                                g.rm.spr_player[k] = skin;
                            }
                        }
                    }
                } else if g.game_values.randomskin[k] {
                    g.game_values.skinids[k] = RANDOM_INT(gp.get(), g.skinlist.count() as i32) as i16;
                    let skin = gp.rm.load_full_skin(g, gp.game_values.skinids[k], gp.game_values.colorids[k]);
                    g.rm.spr_player[k] = skin;
                } else {
                    let skin = gp.rm.load_full_skin(g, gp.game_values.skinids[k], gp.game_values.colorids[k]);
                    g.rm.spr_player[k] = skin;
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
                    let sound = sfxSound::from_file(gp.get(), Path::new(&convert_path(g, &szBuffer))).unwrap_or_else(|e| std::panic::panic_any(e));
                    g.rm.sfx_announcer[announcerIndex as usize] = sound;
                }

                announcerIndex += 1;
            }
        }

        //Load soundtrack music if changed
        if g.game_values.loadedmusic as i64 != g.musiclist.current_index() as i64 {
            g.game_values.loadedmusic = g.musiclist.current_index() as i16;
            g.rm.backgroundmusic[1] = sfxMusic::from_file(g, gp.musiclist.music(0)).unwrap_or_else(|e| std::panic::panic_any(e)); //Stage Clear
            g.rm.backgroundmusic[3] = sfxMusic::from_file(g, gp.musiclist.music(2)).unwrap_or_else(|e| std::panic::panic_any(e)); //Tournament Menu
            g.rm.backgroundmusic[4] = sfxMusic::from_file(g, gp.musiclist.music(3)).unwrap_or_else(|e| std::panic::panic_any(e)); //Tournament Over
        }

        gp.rm.backgroundmusic[2].stop(g);
        gp.rm.sfx_announcer[11].play(g);

        g.game_values.screenfade = 8;
        g.game_values.screenfadespeed = 8;
        println!("< StartGame");
    }

    fn set_controlling_team_for_settings_menu(&mut self, g: &mut Game, iControlTeam: i16, fDisplayMessage: bool) {
        self.mGameSettingsMenu.set_controlling_team(iControlTeam);

        self.mModeOptionsMenu.set_controlling_team(iControlTeam);

        self.mGameSettingsMenu.miMapFilterScroll.set_controlling_team(iControlTeam);
        self.mMapFilterEditMenu.miMapBrowser.set_controlling_team(iControlTeam);

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
                self.iTournamentAITimer = 60;
                self.mGameSettingsMenu.set_allow_exit(true);
            }
        }
    }

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
        self.mCurrentMenu.eyeCandy.emplace(EC_Announcement::new(
            Ptr::from_mut(&mut g.rm.menu_font_large),
            Ptr::from_mut(&mut g.rm.spr_announcementicons),
            szMessage,
            iIcon,
            120,
            100,
        ));
    }

    fn exit(&mut self, g: &mut Game) {
        if !cfg!(target_os = "emscripten") {
            self.quit(g);
        }
    }

    /// Not in upstream: `exit` without its web early return. The main menu's Exit button uses it, so the web build
    /// ends the session and the page shows its game-over overlay (web_quit_if_done in main.rs).
    fn quit(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        g.game_values.appstate = AppState::Quit;
        gp.game_values.write_config(g);
    }

    fn reset_tournament_back_to_main_menu(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.mCurrentMenu = menu(self.mMainMenu);
        self.mCurrentMenu.reset_menu(g);

        if g.game_values.matchtype != MatchType::SingleGame && g.game_values.matchtype != MatchType::QuickGame && g.game_values.matchtype != MatchType::MiniGame && g.game_values.matchtype != MatchType::NetGame {
            if self.fNeedMenuMusicReset {
                gp.rm.backgroundmusic[3].stop(g);
                gp.rm.backgroundmusic[2].play(g, false, false);
                self.fNeedMenuMusicReset = false;
            }
        }
    }

    /// The `MENU_CODE_TO_GAME_SETUP_MENU` branch of `MenuState::update`.
    fn to_game_setup_menu(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        println!("MENU_CODE_TO_GAME_SETUP_MENU");
        //Moves teams to the first arrays in the list and counts the number of teams
        g.score_cnt = self.mTeamSelectMenu.get_team_count(g);
        self.iDisplayError = DISPLAY_ERROR_NONE;
        self.iDisplayErrorTimer = 0;
        let mut fErrorReadingTourFile = false;

        if MatchType::MiniGame == g.game_values.matchtype {
            println!(" Match type: Minigame");
            let minigame: Minigame = self.mMatchSelectionMenu.get_minigame();
            match minigame {
                Minigame::PipeCoin => {
                    g.pipegamemode.goal = 50;
                    g.game_values.gamemode = Ptr::from_raw(g.pipegamemode.as_ptr() as *mut dyn CGameModeTrait);
                }
                Minigame::HammerBoss | Minigame::BombBoss | Minigame::FireBoss => {
                    g.game_values.gamemodemenusettings.boss.difficulty = 2;
                    g.game_values.gamemodemenusettings.boss.hitpoints = 5;

                    g.bossgamemode.goal = 5;
                    g.game_values.gamemode = Ptr::from_raw(g.bossgamemode.as_ptr() as *mut dyn CGameModeTrait);
                }
                Minigame::Boxes => {
                    g.boxesgamemode.goal = 10;
                    g.game_values.gamemode = Ptr::from_raw(g.boxesgamemode.as_ptr() as *mut dyn CGameModeTrait);
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
            println!(" Match type: Quick game");
            let iRandomMode = RANDOM_INT(gp.get(), GAMEMODE_LAST) as i16;
            g.game_values.gamemode = g.gamemodes[iRandomMode as usize];

            //Choose a goal from the lower values for a quicker game
            let iRandOption = (RANDOM_INT(gp.get(), 6) + 1) as i16;
            let goal = g.game_values.gamemode.get_options()[iRandOption as usize].iValue;
            g.game_values.gamemode.goal = goal;

            g.game_values.tournamentwinner = -1;

            self.start_game(g);
        } else {
            //Load the tour here if one was selected
            if g.game_values.matchtype == MatchType::Tour {
                println!("  Match type: Tour");
                if !self.read_tour_file(g) {
                    self.iDisplayError = DISPLAY_ERROR_READ_TOUR_FILE;
                    self.iDisplayErrorTimer = 120;
                    fErrorReadingTourFile = true;
                } else {
                    let numGames = g.game_values.tourstops.len() as i16;
                    self.mTournamentScoreboardMenu.miTournamentScoreboard.create_scoreboard(g, gp.score_cnt, numGames, Ptr::from_mut(&mut gp.rm.spr_tour_markers));
                }
            } else if g.game_values.matchtype == MatchType::Tournament {
                println!("  Match type: Tournament");
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
                self.mTournamentScoreboardMenu.miTournamentScoreboard.create_scoreboard(g, gp.score_cnt, numGames, Ptr::from_mut(&mut gp.rm.menu_mode_large));
            } else if g.game_values.matchtype == MatchType::World {
                println!("  Match type: World");
                let this = Ptr::from_mut(self);
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut this = this;
                    gp.get().g_worldmap.init(WorldMap::new_path(g, &gp.get().worldlist.at(gp.get().game_values.worldindex as usize).to_string_lossy(), TILESIZE as i16));

                    this.mTournamentScoreboardMenu.miTournamentScoreboard.create_scoreboard(g, gp.score_cnt, 0, Ptr::from_mut(&mut gp.rm.spr_tour_markers));

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

                    this.mWorldMenu.miWorld.init(g);
                    let team = RANDOM_INT(gp.get(), g.score_cnt as i32) as i16;
                    this.mWorldMenu.miWorld.set_controlling_team(g, team);
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
                g.game_values.tournamentwinner = -1;

                //Setup wins counters for tournament/tour
                for k in 0..4usize {
                    g.game_values.tournament_scores[k].wins = 0;
                    g.game_values.tournament_scores[k].total = 0;
                }

                if MatchType::SingleGame == g.game_values.matchtype || MatchType::Tournament == g.game_values.matchtype || MatchType::NetGame == g.game_values.matchtype {
                    println!("  MatchType::SingleGame || MatchType::Tournament");
                    println!("current map: {}", *self.szCurrentMapName);
                    let szCurrentMapName = (*self.szCurrentMapName).clone();
                    g.maplist.findexact(&szCurrentMapName, false);
                    self.mGameSettingsMenu.miMapField.load_current_map(g);

                    g.game_values.gamemode = g.gamemodes[self.mGameSettingsMenu.get_current_game_mode_id() as usize];

                    for iMode in 0..GAMEMODE_LAST as usize {
                        g.gamemodes[iMode].goal = self.mGameSettingsMenu.miGoalField[iMode].current_value();
                    }

                    if self.mGameSettingsMenu.get_current_game_mode_id() as GameModeType == game_mode_owned {
                        self.mGameSettingsMenu.hide_gm_settings_btn();
                    }

                    self.mCurrentMenu = menu(self.mGameSettingsMenu);
                    self.mCurrentMenu.reset_menu(g);

                    //If it is a tournament, then set the controlling team
                    if MatchType::Tournament == g.game_values.matchtype {
                        self.set_controlling_team_for_settings_menu(g, g.game_values.tournamentcontrolteam, true);
                    }
                } else if MatchType::Tour == g.game_values.matchtype {
                    self.mCurrentMenu = menu(self.mTourStopMenu);
                    self.mCurrentMenu.reset_menu(g);
                } else if MatchType::World == g.game_values.matchtype {
                    g.game_values.screenfadespeed = 8;
                    g.game_values.screenfade = 8;
                    g.game_values.appstate = AppState::StartWorld;

                    //Play enter world sound
                    gp.rm.sfx_enterstage.play(g);
                }

                //Setup items on next menu
                for iGameMode in 0..GAMEMODE_LAST as usize {
                    self.mGameSettingsMenu.miGoalField[iGameMode].hide_item(-1, g.game_values.matchtype == MatchType::Tournament);
                }

                if g.game_values.matchtype == MatchType::World {
                    self.mGameSettingsMenu.set_header_text("World Game Menu");
                } else if g.game_values.matchtype == MatchType::Tour {
                    self.mGameSettingsMenu.set_header_text("Tour Game Menu");
                } else if g.game_values.matchtype == MatchType::Tournament {
                    self.mGameSettingsMenu.set_header_text("Tournament Game Menu");
                } else {
                    self.mGameSettingsMenu.set_header_text("Single Game Menu");
                }
            }
        }
    }

    /// The tournament-control branch of `MenuState::onEnterState`, run when a tournament game was won.
    fn choose_next_tournament_control_team(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        //Set the next controlling team
        match g.game_values.tournamentcontrolstyle {
            TournamentControlStyle::GameWinner => {
                //Winning Team
                g.game_values.tournamentcontrolteam = g.game_values.gamemode.winningteam;
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

    /// The `game_values.screenfade == 255` + `AppState::StartGame` block of `MenuState::update`.
    fn enter_gameplay(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.game_values.matchtype == MatchType::QuickGame {
            self.mModeOptionsMenu.set_random_game_mode_settings(g, g.game_values.gamemode.gamemode as i16);
        } else if g.game_values.matchtype == MatchType::NetGame {
            // Not in the C++ (whose TODO rolls random ones here on every client): the host sent the match's at the sync.
            crate::smw::net::reseed_for_match(g);
        } else {
            crate::smw::gs_gameplay::set_game_mode_settings_from_menu(g);
        }

        if g.game_values.matchtype == MatchType::World && g.game_values.tourstops[g.game_values.tourstopcurrent].iStageType == 1 {
            load_match_map(g, &convert_path(gp.get(), "maps/special/two52_special_bonushouse.map"));
            crate::common::global::load_current_map_background(g);

            if g.game_values.music {
                g.rm.backgroundmusic[0] = sfxMusic::from_file(g, gp.worldmusiclist.current_music(WorldMusicCategory::Bonus, "")).unwrap_or_else(|e| std::panic::panic_any(e));
                gp.rm.backgroundmusic[0].play(g, false, false);
            }
        } else {
            let mut sShortMapName = String::new();

            let mut fMiniGameMapFound = false;

            if g.game_values.matchtype == MatchType::World {
                let gm = g.game_values.gamemode.gamemode;
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

            if g.game_values.gamemode.gamemode == game_mode_pipe_minigame {
                if !fMiniGameMapFound {
                    load_match_map(g, &convert_path(gp.get(), "maps/special/two52_special_pipe_minigame.map"));
                    sShortMapName = "minigamepipe".to_string();
                }
            } else if g.game_values.gamemode.gamemode == game_mode_boss_minigame {
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
            } else if g.game_values.gamemode.gamemode == game_mode_boxes_minigame {
                if !fMiniGameMapFound {
                    load_match_map(g, &convert_path(gp.get(), "maps/special/two52_special_boxes_minigame.map"));
                    sShortMapName = "minigameboxes".to_string();
                }
            } else if g.game_values.matchtype == MatchType::QuickGame {
                //Load a random map for the quick game
                let szMapName = gp.maplist.random_filename(g);
                load_match_map(g, &szMapName);
                sShortMapName = strip_path_and_extension(&szMapName);

                println!("  State: GS_START_GAME, Match type: MatchType::QuickGame");
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
                g.rm.backgroundmusic[0] = sfxMusic::from_file(g, gp.musiclist.current_music()).unwrap_or_else(|e| std::panic::panic_any(e));
                gp.rm.backgroundmusic[0].play(g, g.game_values.playnextmusic, false);
            }
        }

        harness::match_checkpoint(g);
        enter_gameplay_tail(g);
    }
}

/// The game host rolls a net game's random mode settings, when it plays with "Randomize" on (`NetGameHost::send_sync_messages`).
pub fn roll_net_game_mode_settings(g: &mut Game) {
        let mut gp = g.ptr();
    if let Some(state) = gp.get().ms.as_mut() {
        if !state.mModeOptionsMenu.is_null() {
            state.mModeOptionsMenu.set_random_game_mode_settings(g, gp.get().currentgamemode);
        }
    }
}

/// The current mode's options as the mode options menu names them.
pub fn describe_net_game_mode_settings(g: &mut Game) -> Vec<String> {
        let mut gp = g.ptr();
    match gp.ms.as_ref() {
        Some(state) if !state.mModeOptionsMenu.is_null() => state.mModeOptionsMenu.describe_game_mode_settings(g, gp.currentgamemode),
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
    println!("  GS_GAME");

    gp.get().g_map.predrawbackground(g, &gp.get().rm.spr_background, &gp.get().rm.spr_backmap[0]);
    gp.get().g_map.predrawforeground(g, &gp.get().rm.spr_frontmap[0]);

    gp.get().g_map.predrawbackground(g, &gp.get().rm.spr_background, &gp.get().rm.spr_backmap[1]);
    gp.get().g_map.predrawforeground(g, &gp.get().rm.spr_frontmap[1]);

    gp.g_map.setup_animated_tiles(g);
    crate::smw::gs_gameplay::load_map_objects(g, false);

    GameStateManager::instance(g).change_state_to(gp.get(), Ptr::from_mut(GameplayState::instance(gp.get()) as &mut dyn GameState));
}

impl MenuState {
    /// Handles one `MenuCodeEnum` from `mCurrentMenu->SendInput`. Returns true where C++ `return`s from `update`.
    fn handle_menu_code(&mut self, g: &mut Game, mut code: MenuCodeEnum, fGenerateMapThumbs: &mut bool) -> bool {
        let mut gp = g.ptr();
        // Shortcut to game start
        if g.netplay.active {
            if g.netplay.client.take_reply(NET_G2E_GAME_START) {
                code = MENU_CODE_NET_ROOM_GO;
            }
        }

        if MENU_CODE_EXIT_APPLICATION == code {
            self.quit(g);
            return true;
        } else if MENU_CODE_TO_MAIN_MENU == code {
            self.iDisplayError = DISPLAY_ERROR_NONE;
            self.iDisplayErrorTimer = 0;
            net_end_session(g);

            self.mCurrentMenu = menu(self.mMainMenu);
        } else if MENU_CODE_BACK_TO_MATCH_SELECTION_MENU == code {
            self.mCurrentMenu = menu(self.mMatchSelectionMenu);
            self.iUnlockMinigameOptionIndex = 0;
        } else if MENU_CODE_TO_MATCH_SELECTION_MENU == code {
            self.mMatchSelectionMenu.world_map_changed(g);

            self.mCurrentMenu = menu(self.mMatchSelectionMenu);
            self.mCurrentMenu.reset_menu(g);
            self.iUnlockMinigameOptionIndex = 0;
        } else if MENU_CODE_MATCH_SELECTION_START == code || MENU_CODE_QUICK_GAME_START == code {
            println!("itt vagyok");

            if MENU_CODE_QUICK_GAME_START == code {
                g.game_values.matchtype = MatchType::QuickGame;
            } else {
                g.game_values.matchtype = self.mMatchSelectionMenu.get_selected_match_type();
            }

            self.mTeamSelectMenu.reset_team_select(g);
            self.mCurrentMenu = menu(self.mTeamSelectMenu);
            self.mCurrentMenu.reset_menu(g);
            println!("Hello");

            if g.game_values.matchtype != MatchType::Tournament {
                g.game_values.tournamentcontrolteam = -1;
                self.set_controlling_team_for_settings_menu(g, g.game_values.tournamentcontrolteam, false);
            }
        } else if MENU_CODE_MATCH_SELECTION_MATCH_CHANGED == code {
            self.mMatchSelectionMenu.selection_changed(g);
        } else if MENU_CODE_WORLD_MAP_CHANGED == code {
            self.mMatchSelectionMenu.world_map_changed(g);
        } else if MENU_CODE_TO_OPTIONS_MENU == code {
            self.mCurrentMenu = menu(self.mOptionsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_BACK_TO_OPTIONS_MENU == code {
            self.mCurrentMenu = menu(self.mOptionsMenu);
        } else if MENU_CODE_TO_NET_SERVERS_MENU == code {
            self.mCurrentMenu = menu(self.mNetServersMenu);
            self.mCurrentMenu.reset_menu(g);
            net_start_session(g);
        } else if MENU_CODE_BACK_TO_GRAPHIC_OPTIONS_MENU == code {
            self.mCurrentMenu = menu(self.mGraphicsOptionsMenu);
        } else if MENU_CODE_TO_CONTROLS_MENU == code {
            self.mCurrentMenu = menu(self.mPlayerControlsSelectMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_BACK_TO_CONTROLS_MENU == code {
            self.mCurrentMenu = menu(self.mPlayerControlsSelectMenu);
        } else if MENU_CODE_TO_PLAYER_1_CONTROLS == code {
            self.mPlayerControlsMenu.set_player(g, 0);
            self.mCurrentMenu = menu(self.mPlayerControlsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_PLAYER_2_CONTROLS == code {
            self.mPlayerControlsMenu.set_player(g, 1);
            self.mCurrentMenu = menu(self.mPlayerControlsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_PLAYER_3_CONTROLS == code {
            self.mPlayerControlsMenu.set_player(g, 2);
            self.mCurrentMenu = menu(self.mPlayerControlsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_PLAYER_4_CONTROLS == code {
            self.mPlayerControlsMenu.set_player(g, 3);
            self.mCurrentMenu = menu(self.mPlayerControlsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TOGGLE_FULLSCREEN == code {
            gfx_changefullscreen(g, g.game_values.fullscreen);
            g.blitdest = g.screen;
        } else if MENU_CODE_TO_GAME_SETUP_MENU == code {
            self.to_game_setup_menu(g);
        } else if MENU_CODE_BACK_TO_GAME_SETUP_MENU == code {
            let mut fNeedTeamAnnouncement = false;
            if g.game_values.matchtype == MatchType::World {
                if g.game_values.tournamentwinner == -2 || (g.game_values.tournamentwinner >= 0 && g.game_values.bonuswheel == 0) {
                    self.reset_tournament_back_to_main_menu(g);
                } else if g.game_values.tournamentwinner >= 0 {
                    self.mBonusWheelMenu.miBonusWheel.reset(g, true);
                    self.mCurrentMenu = menu(self.mBonusWheelMenu);
                } else {
                    self.mCurrentMenu = menu(self.mWorldMenu);
                    self.mWorldMenu.miWorldStop.refresh(g, g.game_values.tourstopcurrent as i16);

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
                    self.mBonusWheelMenu.miBonusWheel.reset(g, true);
                    self.mCurrentMenu = menu(self.mBonusWheelMenu);
                }
            } else {
                //Next Tour/Tourament Game
                if g.game_values.matchtype == MatchType::Tour {
                    self.mCurrentMenu = menu(self.mTourStopMenu);
                } else {
                    self.mCurrentMenu = menu(self.mGameSettingsMenu);
                }
            }

            self.mCurrentMenu.reset_menu(g);

            //Set the controlling team for tournament mode
            if MatchType::Tournament == g.game_values.matchtype && g.game_values.tournamentwinner == -1 {
                self.set_controlling_team_for_settings_menu(g, g.game_values.tournamentcontrolteam, true);
            }

            if fNeedTeamAnnouncement {
                self.mWorldMenu.miWorld.display_team_control_announcement(g);
            }
        } else if MENU_CODE_BACK_TO_GAME_SETUP_MENU_FROM_MODE_SETTINGS == code {
            if g.netplay.active {
                self.mCurrentMenu = menu(self.mNetNewRoomSettingsMenu);
            } else {
                self.mCurrentMenu = menu(self.mGameSettingsMenu);
            }
        } else if MENU_CODE_MODE_CHANGED == code {
            g.game_values.gamemode = g.gamemodes[self.mGameSettingsMenu.get_current_game_mode_id() as usize];
            self.mGameSettingsMenu.refresh_game_mode_buttons();
            self.mNetNewRoomSettingsMenu.refresh_game_mode_buttons();
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
                self.mCurrentMenu = menu(self.mTeamSelectMenu);
                self.mCurrentMenu.reset_menu(g);
            }
        } else if MENU_CODE_SOUND_VOLUME_CHANGED == code {
            g.game_values.sound = g.game_values.soundvolume > 0;
            sfx_setsoundvolume(g, g.game_values.soundvolume as i32);
            if_sound_on_play(g, &mut gp.rm.sfx_coin);
        } else if MENU_CODE_MUSIC_VOLUME_CHANGED == code {
            sfx_setmusicvolume(g, g.game_values.musicvolume as i32);

            if g.game_values.musicvolume == 0 {
                gp.rm.backgroundmusic[2].stop(g);
            } else if g.game_values.musicvolume > 0 && !g.game_values.music {
                gp.rm.backgroundmusic[2].play(g, false, false);
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
                self.mWorldMenu.miWorldStop.set_visible(false);
            }
        } else if MENU_CODE_BONUS_DONE == code {
            if self.mBonusWheelMenu.miBonusWheel.get_powerup_selection_done() {
                if g.game_values.matchtype == MatchType::World {
                    if g.game_values.tournamentwinner == -1 {
                        self.mCurrentMenu = menu(self.mTournamentScoreboardMenu);
                    } else {
                        self.reset_tournament_back_to_main_menu(g);
                    }
                } else if (g.game_values.matchtype == MatchType::Tour || g.game_values.matchtype == MatchType::Tournament)
                    && g.game_values.tournamentwinner == -1
                    && ((g.game_values.matchtype != MatchType::Tour && g.game_values.bonuswheel == 2)
                        || (g.game_values.matchtype == MatchType::Tour && g.game_values.tourstops[g.game_values.tourstopcurrent - 1].iBonusType != 0))
                {
                    self.mCurrentMenu = menu(self.mTournamentScoreboardMenu);
                } else if g.game_values.matchtype == MatchType::SingleGame {
                    self.mCurrentMenu = menu(self.mGameSettingsMenu);
                    self.mCurrentMenu.reset_menu(g);
                    self.mGameSettingsMenu.miMapField.load_current_map(g);
                } else {
                    self.reset_tournament_back_to_main_menu(g);
                }
            }
        } else if MENU_CODE_TO_POWERUP_SELECTION_MENU == code {
            self.mCurrentMenu = menu(self.mPowerupDropRatesMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_POWERUP_SETTINGS_MENU == code {
            self.mCurrentMenu = menu(self.mPowerupSettingsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_GRAPHICS_OPTIONS_MENU == code {
            self.mCurrentMenu = menu(self.mGraphicsOptionsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_EYECANDY_OPTIONS_MENU == code {
            self.mCurrentMenu = menu(self.mEyeCandyOptionsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_SOUND_OPTIONS_MENU == code {
            self.mCurrentMenu = menu(self.mSoundOptionsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_GAMEPLAY_OPTIONS_MENU == code {
            self.mCurrentMenu = menu(self.mGameplayOptionsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_TEAM_OPTIONS_MENU == code {
            self.mCurrentMenu = menu(self.mTeamOptionsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_PROJECTILES_OPTIONS_MENU == code {
            self.mCurrentMenu = menu(self.mProjectileOptionsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_PROJECTILES_LIMITS_MENU == code {
            self.mCurrentMenu = menu(self.mProjectileLimitsMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_TO_MODE_SETTINGS_MENU == code {
            if g.netplay.active {
                self.mCurrentMenu = self.mModeOptionsMenu.get_options_menu(self.mNetNewRoomSettingsMenu.get_selected_game_mode_id());
                self.mCurrentMenu.reset_menu(g);
            } else {
                for iGameMode in 0..GAMEMODE_LAST as i16 {
                    if self.mGameSettingsMenu.miGoalField[iGameMode as usize].is_visible() {
                        self.mCurrentMenu = self.mModeOptionsMenu.get_options_menu(iGameMode);
                        self.mCurrentMenu.reset_menu(g);
                        break;
                    }
                }
            }
        } else if MENU_CODE_MENU_GRAPHICS_PACK_CHANGED == code {
            gp.rm.load_menu_graphics(g);

            g.blitdest = g.rm.menu_backdrop.get_surface();
            g.rm.menu_shade.setalpha(App::menuTransparency as u8);
            gp.rm.menu_shade.draw(g, 0, 0);
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
            g.worldmusiclist.set_current(self.mSoundOptionsMenu.get_current_world_music_id() as usize);
        } else if MENU_CODE_TOUR_STOP_CONTINUE == code || MENU_CODE_TOUR_STOP_CONTINUE_FORCED == code {
            //If this tour stop is forced, we need to load the map first
            if MENU_CODE_TOUR_STOP_CONTINUE_FORCED == code {
                self.mWorldMenu.miWorldStop.refresh(g, g.game_values.tourstopcurrent as i16);
            }

            self.mWorldMenu.miWorld.clear_cloud();

            //Tour bonus house
            if g.game_values.matchtype == MatchType::World && g.game_values.tourstops[g.game_values.tourstopcurrent].iStageType == 1 {
                g.bonushousemode.goal = 0;
                g.game_values.gamemode = Ptr::from_raw(g.bonushousemode.as_ptr() as *mut dyn CGameModeTrait);
            } else {
                let iGameMode = g.game_values.tourstops[g.game_values.tourstopcurrent].iMode;

                if iGameMode as GameModeType == game_mode_pipe_minigame {
                    g.game_values.gamemode = Ptr::from_raw(g.pipegamemode.as_ptr() as *mut dyn CGameModeTrait);
                } else if iGameMode as GameModeType == game_mode_boss_minigame {
                    g.game_values.gamemode = Ptr::from_raw(g.bossgamemode.as_ptr() as *mut dyn CGameModeTrait);
                } else if iGameMode as GameModeType == game_mode_boxes_minigame {
                    g.game_values.gamemode = Ptr::from_raw(g.boxesgamemode.as_ptr() as *mut dyn CGameModeTrait);
                } else {
                    g.game_values.gamemode = g.gamemodes[iGameMode as usize];
                }

                g.game_values.gamemode.goal = g.game_values.tourstops[g.game_values.tourstopcurrent].iGoal;
            }

            self.start_game(g);
        } else if MENU_CODE_RESET_STORED_POWERUPS == code {
            for iPlayer in 0..4usize {
                g.game_values.storedpowerups[iPlayer] = -1;
            }
        } else if MENU_CODE_MAP_CHANGED == code {
            if self.mCurrentMenu == menu(self.mNetNewRoomSettingsMenu) {
                debug_assert!(g.netplay.active);
                g.netplay.mapfilepath = self.mNetNewRoomSettingsMenu.get_current_map_path(g);
                println!("[net] Selected map: {}", g.netplay.mapfilepath);
                self.mNetRoomMenu.set_preview_map_path(g, &gp.netplay.mapfilepath);
            } else if g.game_values.matchtype != MatchType::Tour {
                self.szCurrentMapName = Ptr::from_mut(&mut self.mGameSettingsMenu.miMapField.szMapName);
            }
        } else if MENU_CODE_MAP_FILTER_EXIT == code {
            let gv = gp;
            gp.maplist.apply_filters(g, &gv.game_values.pfFilters);

            //If the filtered map list has at least 1 map in it, then allow exiting the filter menu
            if g.maplist.map_in_filtered_set() {
                self.mGameSettingsMenu.miMapField.load_current_map(g);
                self.szCurrentMapName = Ptr::from_mut(&mut self.mGameSettingsMenu.miMapField.szMapName);

                self.mGameSettingsMenu.close_map_filters(g);

                self.iDisplayError = DISPLAY_ERROR_NONE;
            } else {
                //otherwise display a message
                self.iDisplayError = DISPLAY_ERROR_MAP_FILTER;
                self.iDisplayErrorTimer = 120;
            }
        } else if MENU_CODE_TO_MAP_FILTERS == code {
            self.mGameSettingsMenu.open_map_filters(g);
        } else if MENU_CODE_TO_MAP_FILTER_EDIT == code {
            self.mMapFilterEditMenu.miMapBrowser.reset(g, 0);

            self.mCurrentMenu = menu(self.mMapFilterEditMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_MAP_BROWSER_EXIT == code {
            self.mGameSettingsMenu.miMapField.load_current_map(g);
            self.szCurrentMapName = Ptr::from_mut(&mut self.mGameSettingsMenu.miMapField.szMapName);

            self.mCurrentMenu = menu(self.mGameSettingsMenu);
            //mCurrentMenu->ResetMenu();
        } else if MENU_CODE_TO_MAP_BROWSER_THUMBNAILS == code {
            self.mMapFilterEditMenu.miMapBrowser.reset(g, 1);

            self.mCurrentMenu = menu(self.mMapFilterEditMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_SAVE_ALL_MAP_THUMBNAILS == code {
            // C++ casts mCurrentMenu to UI_OptionsMenu*; only the options menu emits this code.
            self.mOptionsMenu.show_thumbnails_popup(g);
            self.mCurrentMenu.reset_menu(g);
        } else if MENU_CODE_GENERATE_THUMBS_RESET_YES == code || MENU_CODE_GENERATE_THUMBS_RESET_NO == code {
            self.mOptionsMenu.hide_thumbnails_popup(g);

            if MENU_CODE_GENERATE_THUMBS_RESET_YES == code {
                *fGenerateMapThumbs = true;
            }
        } else if MENU_CODE_HEALTH_MODE_START_LIFE_CHANGED == code {
            self.mModeOptionsMenu.health_mode_start_life_changed();
        } else if MENU_CODE_HEALTH_MODE_MAX_LIFE_CHANGED == code {
            self.mModeOptionsMenu.health_mode_max_life_changed();
        }

        if g.netplay.active {
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
            } else if MENU_CODE_TO_NET_ADDREMOVE_SERVER_MENU == code {
                self.mCurrentMenu = menu(self.mNetEditServersMenu);
                self.mCurrentMenu.reset_menu(g);
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

                self.mCurrentMenu = menu(self.mNetLobbyMenu);
                self.mCurrentMenu.reset_menu(g);
            } else if MENU_CODE_TO_NET_NEW_ROOM_LEVEL_SELECT_MENU == code {
                self.mCurrentMenu = menu(self.mNetNewRoomSettingsMenu);
                self.mCurrentMenu.reset_menu(g);
            } else if MENU_CODE_TO_NET_NEW_ROOM_SETTINGS_MENU == code {
                self.mCurrentMenu = menu(self.mNetNewRoomMenu);
                self.mCurrentMenu.reset_menu(g);
            } else if MENU_CODE_NET_CHAT_SEND == code {
                if !g.netplay.mychatmessage.is_empty() {
                    let message = g.netplay.mychatmessage.clone();
                    gp.netplay.client.send_chat_message(g, &message);
                } else {
                    code = MENU_CODE_TO_NET_ROOM_MENU;
                }
            } else if MENU_CODE_TO_NET_ROOM_MENU == code {
                self.mCurrentMenu = menu(self.mNetRoomMenu);
                self.mCurrentMenu.reset_menu(g);
                g.netplay.currentMenuChanged = true;
                g.netplay.operationInProgress = false;

                self.mNetRoomMenu.restore(g); // Restore Room layout
                self.mNetLobbyMenu.restore(g); // Restore Lobby layout
                self.mNetNewRoomMenu.restore(g); // remove 'in progress' dialog
            } else if MENU_CODE_NET_JOIN_ROOM_IN_PROGRESS == code {
                self.mNetLobbyMenu.join_in_progress(g);
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

                self.mTeamSelectMenu.reset_team_select(g);
                self.mTeamSelectMenu.reset_menu(g);
                g.score_cnt = self.mTeamSelectMenu.get_team_count(g);

                g.game_values.tournamentcontrolteam = -1;
                g.game_values.tournamentwinner = -1;

                self.iDisplayError = DISPLAY_ERROR_NONE;
                self.iDisplayErrorTimer = 0;

                //game_values.noexit = true;
                self.start_game(g);
            }

            // on room change
            if g.netplay.currentMenuChanged {
                self.mNetServersMenu.refresh(g);
                self.mNetRoomMenu.refresh(g);

                g.netplay.currentMenuChanged = false;
            }
        }

        /*if (code != MENU_CODE_NONE)
            printf("Code: %d\n", code);*/
        false
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

        self.mCurrentMenu = menu(self.mMainMenu);

        if let Some(forcedMap) = harness::forced_map(g) {
            if !g.maplist.findexact(forcedMap, false) && !g.maplist.find(forcedMap) {
                eprintln!("[harness] SMW_MAP '{}' not found", forcedMap);
                std::process::exit(2);
            }
            self.mGameSettingsMenu.miMapField.load_current_map(g);
        }

        self.szCurrentMapName = Ptr::from_mut(&mut self.mGameSettingsMenu.miMapField.szMapName);

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
        if g.game_values.gamemode.winningteam > -1
            && g.game_values.tournamentwinner == -1
            && (((matchtype == MatchType::SingleGame || matchtype == MatchType::QuickGame || matchtype == MatchType::MiniGame || matchtype == MatchType::Tournament || matchtype == MatchType::NetGame)
                && g.game_values.bonuswheel == 2)
                || (matchtype == MatchType::Tour && g.game_values.tourstops[g.game_values.tourstopcurrent - 1].iBonusType != 0))
        {
            self.mBonusWheelMenu.miBonusWheel.reset(g, false);
            self.mCurrentMenu = menu(self.mBonusWheelMenu);
        } else if matchtype != MatchType::SingleGame && matchtype != MatchType::QuickGame && matchtype != MatchType::MiniGame && matchtype != MatchType::NetGame {
            self.mCurrentMenu = menu(self.mTournamentScoreboardMenu);
        } else if matchtype == MatchType::QuickGame {
            //Reset back to main menu after quick game
            self.mCurrentMenu = menu(self.mMainMenu);
            self.mCurrentMenu.reset_menu(g);
        } else if matchtype == MatchType::NetGame {
            self.mCurrentMenu = menu(self.mNetLobbyMenu);
            self.mCurrentMenu.reset_menu(g);
            g.netplay.joinSuccessful = false;
            g.netplay.gameRunning = false;
        }

        if g.game_values.matchtype == MatchType::World {
            if g.game_values.gamemode.winningteam > -1 && g.game_values.tournamentwinner == -1 {
                //Stage is completed
                let winningteam = g.game_values.gamemode.winningteam;
                self.mWorldMenu.miWorld.set_controlling_team(g, winningteam);
                self.mWorldMenu.miWorld.set_current_stage_to_completed(g, winningteam);

                self.mWorldMenu.close_stage_start(g);

                //Clear out the stored powerups after a game that had a winner
                if !g.game_values.worldskipscoreboard {
                    //Only clear out the stored powerup if we were allowed to use it in the game
                    if g.game_values.gamemode.has_stored_powerups() {
                        for iPlayer in 0..MAX_PLAYERS as usize {
                            g.game_values.storedpowerups[iPlayer] = -1;
                        }
                    }
                }
            }
            /*else if (game_values.tournamentwinner > -1) //World is completed
            {
                miBonusWheel->Reset(true);
                mCurrentMenu = mBonusWheelMenu;
            }*/

            crate::smw::gs_gameplay::update_score_board(g);

            //If we're suposed to skip the scoreboard, then reset back to the world map
            if g.game_values.worldskipscoreboard {
                self.mCurrentMenu = menu(self.mWorldMenu);
                self.mCurrentMenu.reset_menu(g);

                g.game_values.worldskipscoreboard = false;
            } else {
                let winningteam = g.game_values.gamemode.winningteam;
                self.mTournamentScoreboardMenu.miTournamentScoreboard.refresh_world_scores(g, winningteam);
            }
        } else if g.game_values.matchtype == MatchType::Tour {
            self.mTournamentScoreboardMenu.miTournamentScoreboard.refresh_tour_scores(g);

            if g.game_values.tourstopcurrent < g.game_values.tourstops.len() {
                self.mTourStopMenu.miTourStop.refresh(g, g.game_values.tourstopcurrent as i16);
            }
        } else if g.game_values.matchtype == MatchType::Tournament {
            self.mTournamentScoreboardMenu.miTournamentScoreboard.stop_swirl();
            if g.game_values.gamemode.winningteam > -1 {
                let winningteam = g.game_values.gamemode.winningteam;
                self.mTournamentScoreboardMenu.miTournamentScoreboard.refresh_tournament_scores(g, winningteam);

                self.choose_next_tournament_control_team(g);
            }
        }

        //Reset game mode back to the current game mode in case we came from boss mode
        g.game_values.gamemode = g.gamemodes[g.currentgamemode as usize];

        //Keep track if we entered the menu loop as part of a tournament, if we exit the tournament
        //we need to reset the menu music back to normal
        let matchtype = g.game_values.matchtype;
        if matchtype != MatchType::SingleGame && matchtype != MatchType::QuickGame && matchtype != MatchType::MiniGame && matchtype != MatchType::NetGame {
            self.fNeedMenuMusicReset = true;
        }

        if g.game_values.music && g.game_values.tournamentwinner < 0 {
            if matchtype == MatchType::SingleGame || matchtype == MatchType::QuickGame || matchtype == MatchType::MiniGame || matchtype == MatchType::NetGame {
                gp.rm.backgroundmusic[2].play(g, false, false);
            } else if matchtype == MatchType::World {
                gp.rm.backgroundmusic[5].play(g, false, false);
            } else {
                gp.rm.backgroundmusic[3].play(g, false, false);
            }
        }

        if matchtype != MatchType::World && matchtype != MatchType::QuickGame && matchtype != MatchType::NetGame {
            if self.mCurrentMenu == menu(self.mGameSettingsMenu) || self.mCurrentMenu == menu(self.mTournamentScoreboardMenu) {
                self.mGameSettingsMenu.miMapField.load_current_map(g);
            }
        }

        // On return from a game, refresh room list
        if g.netplay.active {
            if g.netplay.currentRoom.roomID != 0 {
                gp.netplay.client.send_leave_room_message(g);
                g.netplay.currentRoom.roomID = 0;
                g.netplay.joinSuccessful = false;
                g.netplay.gameRunning = false;
            }
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

            let mut fGenerateMapThumbs = false;

            //handle messages
            #[cfg(target_os = "emscripten")]
            g.frame_events.clear();
            while let Some(input) = crate::services::services(gp.get()).events.poll() {
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
                        let code = self.mCurrentMenu.mouse_click(g, x, y);
                        if self.handle_menu_code(g, code, &mut fGenerateMapThumbs) {
                            return;
                        }
                    }
                }
            }

            //If AI is controlling the tournament menu, select the options
            if g.game_values.matchtype == MatchType::Tournament && self.iTournamentAITimer > 0 && self.mCurrentMenu == menu(self.mGameSettingsMenu) && self.mGameSettingsMenu.is_on_start_btn() {
                self.iTournamentAITimer -= 1;
                if self.iTournamentAITimer == 0 {
                    self.iTournamentAIStep += 1;

                    if self.iTournamentAIStep == 1 {
                        self.mGameSettingsMenu.miMapField.choose_random_map(g);

                        self.iTournamentAITimer = 60;
                    } else if self.iTournamentAIStep == 2 {
                        g.currentgamemode = RANDOM_INT(gp.get(), GAMEMODE_LAST) as i16;
                        g.game_values.gamemode = g.gamemodes[g.currentgamemode as usize];
                        self.mGameSettingsMenu.game_mode_changed(g.currentgamemode);
                        g.game_values.gamemode = g.gamemodes[self.mGameSettingsMenu.get_current_game_mode_id() as usize];

                        self.iTournamentAITimer = 60;
                    } else if self.iTournamentAIStep == 3 {
                        //Choose a goal from the lower values for a quicker game
                        let iRandOption = (RANDOM_INT(gp.get(), 6) + 1) as i16;
                        let goal = g.game_values.gamemode.get_options()[iRandOption as usize].iValue;
                        g.game_values.gamemode.goal = goal;

                        let modeGoal = g.gamemodes[g.currentgamemode as usize].goal;
                        self.mGameSettingsMenu.miGoalField[g.currentgamemode as usize].set_current_value(modeGoal);

                        self.mModeOptionsMenu.set_random_game_mode_settings(g, g.game_values.gamemode.gamemode as i16);

                        self.iTournamentAITimer = 60;
                    } else if self.iTournamentAIStep == 4 {
                        self.iTournamentAIStep = 0;
                        self.start_game(g);
                    }
                }
            }

            //Watch for the konami code to unlock the minigames match type
            /*
            if (!game_values.minigameunlocked && mCurrentMenu == mMatchSelectionMenu) {
                ... (commented out in the C++)
            }
            */

            if AppState::Menu == g.game_values.appstate {
                let code = self.mCurrentMenu.send_input(g, Ptr::from_mut(&mut gp.game_values.playerInput));

                if self.handle_menu_code(g, code, &mut fGenerateMapThumbs) {
                    return;
                }
            }

            //--------------- draw everything ----------------------

            //Don't draw backdrop for world
            if self.mCurrentMenu != menu(self.mWorldMenu) {
                gp.rm.menu_backdrop.draw(g, 0, 0);
            } else {
                crate::common::gfx::blit::fill_rect(g.screen, std::ptr::null(), (*g.screen).map_rgb(0, 0, 0));
            }

            self.mCurrentMenu.update(g);
            self.mCurrentMenu.draw(g);

            if self.iDisplayError > DISPLAY_ERROR_NONE {
                gp.rm.spr_selectfield.draw_src(g, 70, 400, &rect(0, 0, 484, 32));
                gp.rm.spr_selectfield.draw_src(g, 554, 400, &rect(496, 0, 16, 32));

                if self.iDisplayError == DISPLAY_ERROR_READ_TOUR_FILE {
                    gp.rm.menu_font_large.draw_centered(g, 320, 405, "Error Reading Tour File!");
                }
                if self.iDisplayError == DISPLAY_ERROR_READ_WORLD_FILE {
                    gp.rm.menu_font_large.draw_centered(g, 320, 405, "Error Reading World File!");
                } else if self.iDisplayError == DISPLAY_ERROR_MAP_FILTER {
                    gp.rm.menu_font_large.draw_centered(g, 320, 405, "No Maps Meet All Filter Conditions!");
                }

                self.iDisplayErrorTimer -= 1;
                if self.iDisplayErrorTimer == 0 {
                    self.iDisplayError = DISPLAY_ERROR_NONE;
                }
            }

            if g.game_values.screenfadespeed != 0 {
                g.game_values.screenfade += g.game_values.screenfadespeed;

                if g.game_values.screenfade <= 0 {
                    g.game_values.screenfadespeed = 0;
                    g.game_values.screenfade = 0;
                } else if g.game_values.screenfade >= 255 {
                    g.game_values.screenfadespeed = 0;
                    g.game_values.screenfade = 255;
                }
            }

            if g.game_values.screenfade > 0 {
                g.rm.menu_shade.setalpha(g.game_values.screenfade as u8);
                gp.rm.menu_shade.draw(g, 0, 0);
            }

            if g.game_values.screenfade == 255 {
                if AppState::StartGame == g.game_values.appstate {
                    self.enter_gameplay(g);
                    return;
                } else if AppState::StartWorld == g.game_values.appstate {
                    //Fade to world match type
                    g.game_values.screenfadespeed = -8;

                    self.mCurrentMenu = menu(self.mWorldMenu);
                    self.mCurrentMenu.reset_menu(g);

                    gp.rm.backgroundmusic[2].stop(g);
                    let worldMusic = g.worldmusiclist.current_music(g.g_worldmap.get_music_category(), g.g_worldmap.get_world_name()).clone();
                    g.rm.backgroundmusic[5] = sfxMusic::from_file(g, &worldMusic).unwrap_or_else(|e| std::panic::panic_any(e));
                    gp.rm.backgroundmusic[5].play(g, false, false);
                    self.fNeedMenuMusicReset = true;

                    self.mWorldMenu.miWorld.display_team_control_announcement(g);

                    g.game_values.appstate = AppState::Menu;
                }
            }

            if fGenerateMapThumbs {
                gp.rm.menu_dialog.draw_src(g, 160, 176, &rect(0, 0, 160, 64));
                gp.rm.menu_dialog.draw_src(g, App::screenWidth / 2, 176, &rect(352, 0, 160, 64));
                gp.rm.menu_dialog.draw_src(g, 160, 240, &rect(0, 416, 160, 64));
                gp.rm.menu_dialog.draw_src(g, App::screenWidth / 2, App::screenHeight / 2, &rect(352, 416, 160, 64));
                gp.rm.menu_font_large.draw_centered(g, App::screenWidth / 2, 215, "Refreshing Map Thumbnails");
                gp.rm.menu_font_large.draw_centered(g, App::screenWidth / 2, 245, "Please Wait...");
            }

            if fGenerateMapThumbs {
                gp.rm.backgroundmusic[2].toggle_pause(g);

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

                gp.rm.backgroundmusic[2].toggle_pause(g);
            }
        }
    }
}
