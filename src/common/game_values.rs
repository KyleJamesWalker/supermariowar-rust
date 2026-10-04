//! Port of src/common/GameValues.cpp

use crate::common::eyecandy_styles::{AwardStyle, ScoreboardStyle, SpawnStyle};
use crate::common::file_io::{throw_runtime_error, BinaryFile};
use crate::common::game::perror;
use crate::common::game_mode::{CGameModeTrait, GAMEMODE_LAST};
use crate::common::game_mode_settings::{GameModeSettings, GAMEMODESETTINGS_RAW_SIZE};
use crate::common::gameplay_styles::{BoomerangStyle, ShieldStyle, TeamCollisionStyle, TournamentControlStyle, WarpLockStyle};
use crate::common::global_constants::{MAXTOURNAMENTGAMES, MAX_PLAYERS, NUM_AUTO_FILTERS, NUM_POWERUPS, NUM_POWERUP_PRESETS, WAITTIME};
use crate::common::input::*;
use crate::common::match_types::{MatchType, Minigame};
use crate::common::path::get_home_directory;
use crate::common::sfx::{sfx_setmusicvolume, sfx_setsoundvolume, sfxSound};
use crate::common::version::{Version, GAME_VERSION};
use crate::globals::*;
pub use crate::common::global::game_values;
use sdl2::sys::SDL_KeyCode::*;
use sdl2::sys::SDL_Keycode;

pub const TITLESTRING: &str = "Super Mario War";

pub type TourStopVec = Vec<Ptr<crate::common::world_tour_stop::TourStop>>;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum AppState {
    #[default]
    Splash,
    Menu,
    StartGame,
    StartWorld,
    EndGame,
    Game,
    Quit,
}

//tournament scores
#[derive(Clone, Copy, Default, Debug)]
pub struct TournamentScores {
    pub wins: i16,
    pub type_: [i16; MAXTOURNAMENTGAMES as usize],
    pub total: i16, //used for running total in a tour
}

/// Gameplay-specific variables, which are reset on the start of a game.
#[derive(Clone, Debug)]
pub struct GameplayFlags {
    pub pausegame: bool,
    pub exitinggame: bool,
    pub exityes: bool,

    pub noexit: bool,
    pub noexittimer: i16,
    pub forceexittimer: i16,

    pub teamdeadcounter: i16,

    pub screenshaketimer: i16,
    pub screenshakeplayerid: i16,
    pub screenshaketeamid: i16,
    pub screenshakekillscount: i16,
    pub screenshakekillinair: bool,

    pub slowdownon: i16,
    pub slowdowncounter: i16,

    pub showscoreboard: bool,
    pub scorepercentmove: f32,

    pub playskidsound: bool,
    pub playinvinciblesound: bool,
    pub playflyingsound: bool,

    pub swapplayers: bool,
    pub swapplayersposition: f32,
    pub swapplayersblink: bool,
    pub swapplayersblinkcount: i16,

    pub gamewindx: f32,
    pub gamewindy: f32,

    pub windaffectsplayers: bool,
    pub spinscreen: bool,
    pub reversewalk: bool,
    pub spotlights: bool,
}

impl Default for GameplayFlags {
    fn default() -> Self {
        GameplayFlags {
            pausegame: false,
            exitinggame: false,
            exityes: false,
            noexit: false,
            noexittimer: 0,
            forceexittimer: 0,
            teamdeadcounter: 0,
            screenshaketimer: 0,
            screenshakeplayerid: -1,
            screenshaketeamid: -1,
            screenshakekillscount: 0,
            screenshakekillinair: false,
            slowdownon: -1,
            slowdowncounter: 0,
            showscoreboard: false,
            scorepercentmove: 0.0,
            playskidsound: false,
            playinvinciblesound: false,
            playflyingsound: false,
            swapplayers: false,
            swapplayersposition: 0.0,
            swapplayersblink: false,
            swapplayersblinkcount: 0,
            gamewindx: 0.0,
            gamewindy: 0.0,
            windaffectsplayers: false,
            spinscreen: false,
            reversewalk: false,
            spotlights: false,
        }
    }
}

const fn k(c: sdl2::sys::SDL_KeyCode) -> SDL_Keycode {
    c as SDL_Keycode
}

//[Keyboard/Joystick][Game/Menu][NumPlayers][NumKeys]  left, right, jump, down, turbo, powerup, start, cancel
pub static controlkeys: [[[[SDL_Keycode; NUM_KEYS]; 4]; 2]; 2] = [
    [
        [
            [k(SDLK_LEFT), k(SDLK_RIGHT), k(SDLK_UP), k(SDLK_DOWN), k(SDLK_RCTRL), k(SDLK_RSHIFT), k(SDLK_RETURN), k(SDLK_ESCAPE)],
            [k(SDLK_a), k(SDLK_d), k(SDLK_w), k(SDLK_s), k(SDLK_e), k(SDLK_q), k(SDLK_UNKNOWN), k(SDLK_UNKNOWN)],
            [k(SDLK_g), k(SDLK_j), k(SDLK_y), k(SDLK_h), k(SDLK_u), k(SDLK_t), k(SDLK_UNKNOWN), k(SDLK_UNKNOWN)],
            [k(SDLK_l), k(SDLK_QUOTE), k(SDLK_p), k(SDLK_SEMICOLON), k(SDLK_LEFTBRACKET), k(SDLK_o), k(SDLK_UNKNOWN), k(SDLK_UNKNOWN)],
        ],
        //up, down, left, right, select, cancel, random, fast scroll
        [
            [k(SDLK_UP), k(SDLK_DOWN), k(SDLK_LEFT), k(SDLK_RIGHT), k(SDLK_RETURN), k(SDLK_ESCAPE), k(SDLK_SPACE), k(SDLK_LSHIFT)],
            [k(SDLK_w), k(SDLK_s), k(SDLK_a), k(SDLK_d), k(SDLK_e), k(SDLK_q), k(SDLK_UNKNOWN), k(SDLK_UNKNOWN)],
            [k(SDLK_y), k(SDLK_h), k(SDLK_g), k(SDLK_j), k(SDLK_u), k(SDLK_t), k(SDLK_UNKNOWN), k(SDLK_UNKNOWN)],
            [k(SDLK_p), k(SDLK_SEMICOLON), k(SDLK_l), k(SDLK_QUOTE), k(SDLK_LEFTBRACKET), k(SDLK_o), k(SDLK_UNKNOWN), k(SDLK_UNKNOWN)],
        ],
    ],
    //left, right, jump, down, turbo, powerup, start, cancel;
    [
        [
            [JOY_STICK_1_LEFT, JOY_STICK_1_RIGHT, JOY_BUTTON_START, JOY_STICK_1_DOWN, JOY_BUTTON_START + 1, JOY_BUTTON_START + 2, JOY_BUTTON_START + 3, JOY_BUTTON_START + 4],
            [JOY_STICK_1_LEFT, JOY_STICK_1_RIGHT, JOY_BUTTON_START, JOY_STICK_1_DOWN, JOY_BUTTON_START + 1, JOY_BUTTON_START + 2, JOY_BUTTON_START + 3, JOY_BUTTON_START + 4],
            [JOY_STICK_1_LEFT, JOY_STICK_1_RIGHT, JOY_BUTTON_START, JOY_STICK_1_DOWN, JOY_BUTTON_START + 1, JOY_BUTTON_START + 2, JOY_BUTTON_START + 3, JOY_BUTTON_START + 4],
            [JOY_STICK_1_LEFT, JOY_STICK_1_RIGHT, JOY_BUTTON_START, JOY_STICK_1_DOWN, JOY_BUTTON_START + 1, JOY_BUTTON_START + 2, JOY_BUTTON_START + 3, JOY_BUTTON_START + 4],
        ],
        //up, down, left, right, select, cancel, random, fast scroll
        [
            [JOY_STICK_1_UP, JOY_STICK_1_DOWN, JOY_STICK_1_LEFT, JOY_STICK_1_RIGHT, JOY_BUTTON_START, JOY_BUTTON_START + 1, JOY_BUTTON_START + 2, JOY_BUTTON_START + 3],
            [JOY_STICK_1_UP, JOY_STICK_1_DOWN, JOY_STICK_1_LEFT, JOY_STICK_1_RIGHT, JOY_BUTTON_START, JOY_BUTTON_START + 1, JOY_BUTTON_START + 2, JOY_BUTTON_START + 3],
            [JOY_STICK_1_UP, JOY_STICK_1_DOWN, JOY_STICK_1_LEFT, JOY_STICK_1_RIGHT, JOY_BUTTON_START, JOY_BUTTON_START + 1, JOY_BUTTON_START + 2, JOY_BUTTON_START + 3],
            [JOY_STICK_1_UP, JOY_STICK_1_DOWN, JOY_STICK_1_LEFT, JOY_STICK_1_RIGHT, JOY_BUTTON_START, JOY_BUTTON_START + 1, JOY_BUTTON_START + 2, JOY_BUTTON_START + 3],
        ],
    ],
];

/*
0 == poison mushroom
1 == 1up
2 == 2up
3 == 3up
4 == 5up
5 == flower
6 == star
7 == clock
8 == bobomb
9 == pow
10 == bulletbill
11 == hammer
12 == green shell
13 == red shell
14 == spike shell
15 == buzzy shell
16 == mod
17 == feather
18 == mystery mushroom
19 == boomerang
20 == tanooki
21 == ice wand
22 == podoboo
23 == bombs
24 == leaf
25 == pwings
*/
const DEFAULT_POWERUP_PRESETS: [[i16; NUM_POWERUPS as usize]; NUM_POWERUP_PRESETS as usize] = [
    //0  1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25
    [5, 10, 4, 2, 1, 10, 8, 4, 4, 2, 2, 4, 8, 4, 2, 4, 2, 4, 5, 6, 6, 3, 4, 4, 5, 3], //Custom 1
    [5, 10, 4, 2, 1, 10, 8, 4, 4, 2, 2, 4, 8, 4, 2, 4, 2, 4, 5, 6, 6, 3, 4, 4, 5, 3], //Custom 2
    [5, 10, 4, 2, 1, 10, 8, 4, 4, 2, 2, 4, 8, 4, 2, 4, 2, 4, 5, 6, 6, 3, 4, 4, 5, 3], //Custom 3
    [5, 10, 4, 2, 1, 10, 8, 4, 4, 2, 2, 4, 8, 4, 2, 4, 2, 4, 5, 6, 6, 3, 4, 4, 5, 3], //Custom 4
    [5, 10, 4, 2, 1, 10, 8, 4, 4, 2, 2, 4, 8, 4, 2, 4, 2, 4, 5, 6, 6, 3, 4, 4, 5, 3], //Custom 5
    [5, 10, 5, 3, 1, 10, 2, 3, 4, 3, 3, 4, 9, 6, 2, 4, 4, 7, 5, 6, 6, 3, 2, 2, 5, 5], //Balanced
    [5, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 7, 9, 6, 3, 4, 0, 0, 0, 4, 0, 2, 0, 2, 0, 0], //Weapons Only
    [0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0], //Koopa Bros Weapons
    [5, 10, 7, 5, 2, 0, 6, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 5, 0, 3, 0, 0, 0, 8, 6], //Support Items
    [3, 3, 1, 0, 0, 0, 0, 0, 4, 2, 2, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 3, 3, 0, 0], //Booms and Shakes
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 2, 0, 0, 0, 8, 3], //Fly and Glide
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 4, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], //Shells
    [5, 8, 4, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0], //Mushrooms Only
    [3, 5, 0, 0, 0, 5, 2, 0, 0, 0, 3, 0, 6, 4, 1, 3, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0], //Super Mario Bros 1
    [0, 5, 0, 0, 0, 0, 2, 4, 3, 2, 0, 0, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0], //Super Mario Bros 2
    [0, 3, 0, 0, 0, 8, 4, 0, 0, 0, 5, 2, 10, 9, 4, 5, 0, 0, 0, 4, 3, 0, 4, 0, 8, 5], //Super Mario Bros 3
    [0, 10, 0, 0, 0, 10, 6, 0, 0, 0, 2, 0, 8, 4, 2, 4, 0, 4, 0, 0, 0, 0, 5, 0, 0, 0], //Super Mario World
];

pub fn if_sound_on_play(sfx: &mut sfxSound) {
    unsafe {
        if game_values.sound {
            sfx.play();
        }
    }
}

pub fn default_powerup_setting(presetIdx: usize, powerupIdx: usize) -> i16 {
    DEFAULT_POWERUP_PRESETS[presetIdx][powerupIdx]
}

// the real configuration class
pub struct CGameConfig {
    pub soundvolume: i16,
    pub musicvolume: i16,
    pub fullscreen: bool,
    pub spawnstyle: SpawnStyle,
    pub awardstyle: AwardStyle,
    pub teamcollision: TeamCollisionStyle,
    pub shellttl: i16,
    pub blueblockttl: i16,
    pub redblockttl: i16,
    pub grayblockttl: i16,
    pub bombslimit: i16,
    pub bonuswheel: i16,
    pub fireballttl: i16,
    pub fireballlimit: i16,
    pub boomerangstyle: BoomerangStyle,
    pub boomeranglife: i16,
    pub boomeranglimit: i16,
    pub cpudifficulty: i16,
    pub warplockstyle: WarpLockStyle,
    pub warplocktime: i16,
    pub hammerttl: i16,
    pub hammerdelay: i16,
    pub hammerlimit: i16,
    pub hammerpower: bool,
    pub featherjumps: i16,
    pub featherlimit: i16,
    pub suicidetime: i16,
    pub deadteamnotice: bool,
    pub framelimiter: i16,
    pub hiddenblockrespawn: i16,
    pub inputConfiguration: [[CInputPlayerControl; 2]; 4], //[NumPlayers][Keyboard/Joystick]
    pub itemrespawntime: i16,
    pub keeppowerup: bool,
    pub leaflimit: i16,
    pub pwingslimit: i16,
    pub tanookilimit: i16,
    pub playercontrol: [i16; 4],
    pub playerInput: CPlayerInput,
    pub playnextmusic: bool, //automatically advance to the next music track after one finishes
    pub outofboundstime: i16,
    pub overridepowerupsettings: i16,
    pub secretsenabled: bool,
    pub gamemodemenusettings: GameModeSettings,
    pub music: bool,
    pub pointspeed: i16,
    pub poweruppreset: i16,
    pub allPowerupPresets: [[i16; NUM_POWERUPS as usize]; NUM_POWERUP_PRESETS as usize],
    pub randomskin: [bool; 4],
    pub scoreboardstyle: ScoreboardStyle,
    pub screencrunch: bool,
    pub shieldtime: i16,
    pub shieldstyle: ShieldStyle,
    pub showwinningcrown: bool,
    pub skinids: [i16; 4],
    pub wandlimit: i16,
    pub tournamentcontrolstyle: TournamentControlStyle, //ID for the player selected control style
    pub wandfreezetime: i16,
    pub toplayer: bool,
    pub teamcolors: bool,
    pub sound: bool,
    pub startgamecountdown: bool,
    pub startmodedisplay: bool,
    pub storedpowerupdelay: i16,
    pub swapstyle: i16,
    pub respawn: i16,
    pub _alias: Aliased,
}

// these are actually the program options
pub struct CGameValues {
    pub cgame_config: CGameConfig,

    pub showfps: bool,
    pub frameadvance: bool,
    pub autokill: bool,

    pub appstate: AppState,

    pub flags: GameplayFlags,
    pub gamemode: Ptr<dyn CGameModeTrait>,

    pub screenfade: i16,
    pub screenfadespeed: i16,

    pub matchtype: MatchType, //The currently selected match type: quick game, single, tournament, tour, world, minigame

    pub teamids: [[i16; 3]; 4],
    pub teamcounts: [i16; 4],

    pub tournamentgames: i16,  //How many games that are played in this tournament
    pub tournamentwinner: i16, //-2 for a tied tournament (for tours), -1 for no winner yet, 0 or greater for the team that has won the tournament

    pub tournamentcontrolteam: i16, //The team ID that currently has control
    pub tournamentnextcontrol: i16, //For round robin control style

    pub selectedminigame: Minigame,

    pub tourindex: i16,
    pub tourstopcurrent: usize,
    pub tourstops: TourStopVec,
    pub worldindex: i16,

    pub storedpowerups: [i16; 4],
    pub gamepowerups: [i16; 4],
    pub powerupweights: [i16; NUM_POWERUPS as usize],

    pub worldpowerups: [[i16; 32]; 4],
    pub worldpowerupcount: [i16; 4],

    pub worldpointsbonus: i16,

    pub colorids: [i16; 4],

    pub bulletbilltimer: [i16; 4],
    pub bulletbillspawntimer: [i16; 4],

    pub loadedannouncer: usize,
    pub loadedmusic: i16,

    pub cputurn: i16,

    pub tournament_scores: [TournamentScores; 4],

    pub gamemodesettings: GameModeSettings,

    pub soundcapable: bool,

    pub pfFilters: Vec<bool>,
    pub piFilterIcons: Vec<i16>,
    pub selectedmapfilter: i16,
    pub fNeedWriteFilters: bool,
    pub fFiltersOn: bool,

    pub singleplayermode: i16,

    pub worldskipscoreboard: bool,

    pub windaffectsplayers: bool,
    pub spinscreen: bool,
    pub reversewalk: bool,
    pub spotlights: bool,

    pub unlocksecret1part1: [bool; 4],
    pub unlocksecret1part2: i16,
    pub unlocksecret2part1: bool,
    pub unlocksecret2part2: i16,
    pub unlocksecret3part1: [i16; 4],
    pub unlocksecret3part2: [i16; 4],
    pub unlocksecretunlocked: [bool; 4],

    pub _alias: Aliased,
}

crate::impl_base!(CGameValues => cgame_config: CGameConfig);

impl CGameConfig {
    /// Zero-initialized like the C++ global, which has static storage.
    pub fn new() -> Self {
        CGameConfig { _alias: Aliased::new(),
            soundvolume: 0,
            musicvolume: 0,
            fullscreen: false,
            spawnstyle: SpawnStyle::default(),
            awardstyle: AwardStyle::default(),
            teamcollision: TeamCollisionStyle::default(),
            shellttl: 0,
            blueblockttl: 0,
            redblockttl: 0,
            grayblockttl: 0,
            bombslimit: 0,
            bonuswheel: 0,
            fireballttl: 0,
            fireballlimit: 0,
            boomerangstyle: BoomerangStyle::default(),
            boomeranglife: 0,
            boomeranglimit: 0,
            cpudifficulty: 0,
            warplockstyle: WarpLockStyle::default(),
            warplocktime: 0,
            hammerttl: 0,
            hammerdelay: 0,
            hammerlimit: 0,
            hammerpower: false,
            featherjumps: 0,
            featherlimit: 0,
            suicidetime: 0,
            deadteamnotice: false,
            framelimiter: 0,
            hiddenblockrespawn: 0,
            inputConfiguration: [[CInputPlayerControl::default(); 2]; 4],
            itemrespawntime: 0,
            keeppowerup: false,
            leaflimit: 0,
            pwingslimit: 0,
            tanookilimit: 0,
            playercontrol: [0; 4],
            playerInput: CPlayerInput::new(),
            playnextmusic: false,
            outofboundstime: 0,
            overridepowerupsettings: 0,
            secretsenabled: false,
            gamemodemenusettings: GameModeSettings::new(),
            music: false,
            pointspeed: 0,
            poweruppreset: 0,
            allPowerupPresets: [[0; NUM_POWERUPS as usize]; NUM_POWERUP_PRESETS as usize],
            randomskin: [false; 4],
            scoreboardstyle: ScoreboardStyle::default(),
            screencrunch: false,
            shieldtime: 0,
            shieldstyle: ShieldStyle::default(),
            showwinningcrown: false,
            skinids: [0; 4],
            wandlimit: 0,
            tournamentcontrolstyle: TournamentControlStyle::default(),
            wandfreezetime: 0,
            toplayer: false,
            teamcolors: false,
            sound: false,
            startgamecountdown: false,
            startmodedisplay: false,
            storedpowerupdelay: 0,
            swapstyle: 0,
            respawn: 0,
        }
    }

}

impl CGameValues {
    /// Zero-initialized plus the in-class member initializers, like the C++ global.
    pub fn new() -> Self {
        CGameValues {
            cgame_config: CGameConfig::new(),
            showfps: false,
            frameadvance: false,
            autokill: false,
            appstate: AppState::Splash,
            flags: GameplayFlags::default(),
            gamemode: Ptr::null(),
            screenfade: 0,
            screenfadespeed: 0,
            matchtype: MatchType::default(),
            teamids: [[0; 3]; 4],
            teamcounts: [0; 4],
            tournamentgames: 0,
            tournamentwinner: 0,
            tournamentcontrolteam: 0,
            tournamentnextcontrol: 0,
            selectedminigame: Minigame::default(),
            tourindex: 0,
            tourstopcurrent: 0,
            tourstops: TourStopVec::new(),
            worldindex: 0,
            storedpowerups: [0; 4],
            gamepowerups: [0; 4],
            powerupweights: [0; NUM_POWERUPS as usize],
            worldpowerups: [[0; 32]; 4],
            worldpowerupcount: [0; 4],
            worldpointsbonus: 0,
            colorids: [0; 4],
            bulletbilltimer: [0; 4],
            bulletbillspawntimer: [0; 4],
            loadedannouncer: 0,
            loadedmusic: 0,
            cputurn: 0,
            tournament_scores: [TournamentScores::default(); 4],
            gamemodesettings: GameModeSettings::new(),
            soundcapable: false,
            pfFilters: Vec::new(),
            piFilterIcons: Vec::new(),
            selectedmapfilter: 0,
            fNeedWriteFilters: false,
            fFiltersOn: false,
            singleplayermode: 0,
            worldskipscoreboard: false,
            windaffectsplayers: false,
            spinscreen: false,
            reversewalk: false,
            spotlights: false,
            unlocksecret1part1: [false; 4],
            unlocksecret1part2: 0,
            unlocksecret2part1: false,
            unlocksecret2part2: 0,
            unlocksecret3part1: [0; 4],
            unlocksecret3part2: [0; 4],
            unlocksecretunlocked: [false; 4],
            _alias: Aliased::new(),
        }
    }

    pub fn init(&mut self) {
        //set standard game values
        self.playercontrol[0] = 1;
        self.playercontrol[1] = 1;
        self.showfps = false;
        self.frameadvance = false;
        self.autokill = false;
        self.framelimiter = WAITTIME as i16;
        self.sound = true;
        self.music = true;
        self.appstate = AppState::Splash;
        self.fullscreen = false;

        self.awardstyle = AwardStyle::Fireworks;
        self.spawnstyle = SpawnStyle::Swirl;
        self.tournamentgames = 2;
        self.tournamentwinner = -1;
        self.selectedminigame = Minigame::PipeCoin;
        self.matchtype = MatchType::SingleGame;
        self.tourindex = 0;
        self.tourstopcurrent = 0;
        self.worldindex = 0;
        self.teamcollision = TeamCollisionStyle::Off;
        self.screencrunch = true;
        self.toplayer = true;
        self.loadedannouncer = usize::MAX; // -1 into size_t
        self.loadedmusic = -1;
        self.scoreboardstyle = ScoreboardStyle::Top;
        self.teamcolors = true;
        self.cputurn = -1;
        self.shieldtime = 62;
        self.shieldstyle = ShieldStyle::SoftWithStomp;
        self.musicvolume = 128;
        self.soundvolume = 128;
        self.respawn = 2;
        self.itemrespawntime = 1860; //default item respawn is 30 seconds (30 * 62 fps)
        self.hiddenblockrespawn = 1860; //default item respawn is 30 seconds
        self.outofboundstime = 5;
        self.warplockstyle = WarpLockStyle::ExitOnly; // Lock Warp Exit Only
        self.warplocktime = 186; // 3 seconds
        self.suicidetime = 310; // 5 seconds (the reference build is Release)
        self.cpudifficulty = 2;
        self.fireballttl = 310; // 5 seconds
        self.shellttl = 496; // 8 seconds
        self.blueblockttl = 310; // 5 seconds
        self.redblockttl = 310; // 5 seconds
        self.grayblockttl = 310; // 5 seconds
        self.hammerdelay = 25; // 0.4 second
        self.hammerttl = 49; // 0.8 second
        self.hammerpower = true; //hammers die on first hit
        self.fireballlimit = 0; //Unlimited
        self.hammerlimit = 0; //Unlimited
        self.boomerangstyle = BoomerangStyle::SMB3; //SMB3 style
        self.boomeranglife = 248; // 4 seconds of zelda boomerang
        self.boomeranglimit = 0; //Unlimited
        self.featherjumps = 1; //Allow one extra cape jump
        self.featherlimit = 0; //Unlimited
        self.leaflimit = 0; //Unlimited
        self.pwingslimit = 0; //Unlimited
        self.tanookilimit = 0; //Unlimited
        self.bombslimit = 0; //Unlimited
        self.wandfreezetime = 310; //5 seconds of freeze time
        self.wandlimit = 0; //Unlimited
        self.storedpowerupdelay = 4;
        self.bonuswheel = 1;
        self.keeppowerup = false;
        self.showwinningcrown = false;
        self.startgamecountdown = true;
        self.startmodedisplay = true;
        self.deadteamnotice = true;
        self.playnextmusic = false;
        self.pointspeed = 20;
        self.swapstyle = 1; //Blink then swap
        self.worldpointsbonus = -1; //no world multiplier until player uses item to boost it
        self.singleplayermode = -1;
        self.worldskipscoreboard = false;
        self.overridepowerupsettings = 0;
        self.secretsenabled = false;
        self.poweruppreset = 0;
        self.tournamentcontrolstyle = TournamentControlStyle::All;

        let filtercount = unsafe { filterslist.count() };
        self.pfFilters.resize(NUM_AUTO_FILTERS as usize + filtercount, false);
        self.piFilterIcons.resize(NUM_AUTO_FILTERS as usize + filtercount, 0);
        self.fNeedWriteFilters = false;

        for iPlayer in 0..4usize {
            self.storedpowerups[iPlayer] = -1;
            self.gamepowerups[iPlayer] = -1;
            self.teamids[iPlayer][0] = iPlayer as i16;
            self.teamcounts[iPlayer] = 1;
            self.skinids[iPlayer] = 0;
            self.colorids[iPlayer] = iPlayer as i16;
            self.randomskin[iPlayer] = false;

            //Setup the default key/button input configurations
            for iInputType in 0..2usize {
                //for keyboard/joystick
                self.inputConfiguration[iPlayer][iInputType].iDevice = iInputType as i16 - 1;

                for iInputState in 0..2usize {
                    //for game/menu
                    for iKey in 0..NUM_KEYS {
                        self.inputConfiguration[iPlayer][iInputType].inputGameControls[iInputState].keys[iKey] =
                            controlkeys[iInputType][iInputState][iPlayer][iKey];
                    }
                }
            }

            //Set the players input to the default configuration (will be overwritten by options.bin settings)
            self.cgame_config.playerInput.inputControls[iPlayer] = Ptr::from_mut(&mut self.cgame_config.inputConfiguration[iPlayer][0]);
        }

        //Set the default powerup weights for bonus wheel and [?] boxes
        for iPreset in 0..NUM_POWERUP_PRESETS as usize {
            for iPowerup in 0..NUM_POWERUPS as usize {
                self.allPowerupPresets[iPreset][iPowerup] = default_powerup_setting(iPreset, iPowerup);
            }
        }
    }

    pub fn reset_gameplay_settings(&mut self) {
        self.flags = GameplayFlags::default();

        self.screenfade = 255;
        self.screenfadespeed = -8;

        //Initialize game mode
        self.gamemode.get().init();
    }

    pub fn reset_secret_counters(&mut self) {
        //Reset Secret Counters
        for iPlayer in 0..4 {
            self.unlocksecret1part1[iPlayer] = false;
        }

        self.unlocksecret1part2 = 0;
        self.unlocksecret2part1 = false;
        self.unlocksecret2part2 = 0;
        self.unlocksecret3part1 = [0; 4];
        self.unlocksecret3part2 = [0; 4];
        self.unlocksecretunlocked = [false; 4];
    }
}


/// `CInputPlayerControl` as clang lays it out: `short iDevice`, 2 bytes padding, `SDL_Keycode keys[2][8]`.
const CINPUTPLAYERCONTROL_RAW_SIZE: usize = 68;

fn input_control_from_raw(c: &mut CInputPlayerControl, b: &[u8]) {
    c.iDevice = i16::from_le_bytes([b[0], b[1]]);
    for s in 0..2 {
        for k in 0..NUM_KEYS {
            let o = 4 + (s * NUM_KEYS + k) * 4;
            c.inputGameControls[s].keys[k] = i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        }
    }
}

fn input_control_to_raw(c: &CInputPlayerControl, b: &mut [u8]) {
    b[0..2].copy_from_slice(&c.iDevice.to_le_bytes());
    for s in 0..2 {
        for k in 0..NUM_KEYS {
            let o = 4 + (s * NUM_KEYS + k) * 4;
            b[o..o + 4].copy_from_slice(&c.inputGameControls[s].keys[k].to_le_bytes());
        }
    }
}

/// `try { f } catch (std::exception const& error) { perror(error.what()); }`; `None` when it threw.
fn catch_runtime_error<R>(f: impl FnOnce() -> R) -> Option<R> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(r) => Some(r),
        Err(e) => {
            let what = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            perror(&what);
            None
        }
    }
}

impl CGameConfig {
    pub fn read_binary_config(&mut self) {
        let ok = catch_runtime_error(|| {
            let options_path = get_home_directory() + "options.bin";
            let mut options = BinaryFile::new(&options_path, "rb");
            if !options.is_open() {
                throw_runtime_error(&format!("Could not open {}", options_path));
            }

            self.read_options(&mut options)
        });
        if ok != Some(true) {
            return;
        }

        catch_runtime_error(|| {
            let controls_path = get_home_directory() + "controls.sdl2.bin";
            let mut controls = BinaryFile::new(&controls_path, "rb");
            if !controls.is_open() {
                throw_runtime_error(&format!("Could not open {}", controls_path));
            }

            self.read_controls(&mut controls);
        });
    }

    fn read_options(&mut self, options: &mut BinaryFile) -> bool {
        unsafe {
            let version = Version {
                major: options.read_i32() as u8,
                minor: options.read_i32() as u8,
                patch: options.read_i32() as u8,
                build: options.read_i32() as u8,
            };

            if GAME_VERSION != version {
                println!("Old options.bin detected. Skipped reading it.");
                return false;
            }

            self.spawnstyle = SpawnStyle::from_u8(options.read_u8());
            self.awardstyle = AwardStyle::from_u8(options.read_u8());
            self.teamcollision = TeamCollisionStyle::from_u8(options.read_u8());
            self.screencrunch = options.read_u8() != 0;
            self.toplayer = options.read_u8() != 0;
            self.scoreboardstyle = ScoreboardStyle::from_u8(options.read_u8());
            self.teamcolors = options.read_u8() != 0;
            self.sound = options.read_u8() != 0;
            self.music = options.read_u8() != 0;
            self.musicvolume = options.read_u8() as i16;
            self.soundvolume = options.read_u8() as i16;
            self.respawn = options.read_u8() as i16;
            self.outofboundstime = options.read_u8() as i16;
            self.cpudifficulty = options.read_u8() as i16;
            self.framelimiter = options.read_u8() as i16;
            self.bonuswheel = options.read_u8() as i16;
            self.keeppowerup = options.read_u8() != 0;
            self.showwinningcrown = options.read_u8() != 0;
            self.playnextmusic = options.read_u8() != 0;
            self.pointspeed = options.read_u8() as i16;
            self.swapstyle = options.read_u8() as i16;
            self.overridepowerupsettings = options.read_u8() as i16;
            self.secretsenabled = options.read_u8() != 0;
            self.startgamecountdown = options.read_u8() != 0;
            self.deadteamnotice = options.read_u8() != 0;
            self.tournamentcontrolstyle = TournamentControlStyle::from_u8(options.read_u8());
            self.startmodedisplay = options.read_u8() != 0;

            self.shieldtime = options.read_i16();
            self.shieldstyle = ShieldStyle::from_u8(options.read_i16() as u8);
            self.itemrespawntime = options.read_i16();
            self.hiddenblockrespawn = options.read_i16();
            self.fireballttl = options.read_i16();
            self.fireballlimit = options.read_i16();
            self.hammerdelay = options.read_i16();
            self.hammerttl = options.read_i16();
            self.hammerpower = options.read_i16() != 0;
            self.hammerlimit = options.read_i16();
            self.boomerangstyle = BoomerangStyle::from_u8(options.read_i16() as u8);
            self.boomeranglife = options.read_i16();
            self.boomeranglimit = options.read_i16();
            self.featherjumps = options.read_i16();
            self.featherlimit = options.read_i16();
            self.leaflimit = options.read_i16();
            self.pwingslimit = options.read_i16();
            self.tanookilimit = options.read_i16();
            self.bombslimit = options.read_i16();
            self.wandfreezetime = options.read_i16();
            self.wandlimit = options.read_i16();
            self.shellttl = options.read_i16();
            self.blueblockttl = options.read_i16();
            self.redblockttl = options.read_i16();
            self.grayblockttl = options.read_i16();
            self.storedpowerupdelay = options.read_i16();
            self.warplockstyle = WarpLockStyle::from_u8(options.read_i16() as u8);
            self.warplocktime = options.read_i16();
            self.suicidetime = options.read_i16();

            self.poweruppreset = options.read_i16();
            let mut presets = [0i16; (NUM_POWERUP_PRESETS * NUM_POWERUPS) as usize];
            options.read_i16_array(&mut presets);
            for (i, v) in presets.iter().enumerate() {
                self.allPowerupPresets[i / NUM_POWERUPS as usize][i % NUM_POWERUPS as usize] = *v;
            }

            self.fullscreen = options.read_bool();

            for iGameMode in 0..GAMEMODE_LAST as usize {
                crate::smw::main::gamemodes[iGameMode].goal = options.read_i16();
            }

            let mut gms = [0u8; GAMEMODESETTINGS_RAW_SIZE];
            options.read_raw(&mut gms);
            self.gamemodemenusettings.from_raw_bytes(&gms);

            let mut raw = [0u8; 8];
            options.read_raw(&mut raw);
            for i in 0..4 {
                self.skinids[i] = i16::from_le_bytes([raw[2 * i], raw[2 * i + 1]]);
            }
            let mut raw = [0u8; 4];
            options.read_raw(&mut raw);
            for i in 0..4 {
                self.randomskin[i] = raw[i] != 0;
            }
            let mut raw = [0u8; 8];
            options.read_raw(&mut raw);
            for i in 0..4 {
                self.playercontrol[i] = i16::from_le_bytes([raw[2 * i], raw[2 * i + 1]]);
            }

            for iPlayer in 0..MAX_PLAYERS as usize {
                if self.skinids[iPlayer] as i64 >= skinlist.count() as i64 || self.skinids[iPlayer] < 0 {
                    self.skinids[iPlayer] = 0;
                }
            }

            announcerlist.set_current_index(options.read_u8() as usize);
            musiclist.set_current(options.read_u8() as usize);
            worldmusiclist.set_current(options.read_u8() as usize);
            soundpacklist.set_current_index(options.read_u8() as usize);
            menugraphicspacklist.set_current_index(options.read_u8() as usize);
            worldgraphicspacklist.set_current_index(options.read_u8() as usize);
            gamegraphicspacklist.set_current_index(options.read_u8() as usize);

            sfx_setmusicvolume(self.musicvolume as i32);
            sfx_setsoundvolume(self.soundvolume as i32);
            true
        }
    }

    fn read_controls(&mut self, controls: &mut BinaryFile) {
        unsafe {
            let mut raw = [0u8; CINPUTPLAYERCONTROL_RAW_SIZE * 8];
            controls.read_raw(&mut raw);
            for p in 0..4 {
                for d in 0..2 {
                    let o = (p * 2 + d) * CINPUTPLAYERCONTROL_RAW_SIZE;
                    input_control_from_raw(&mut self.inputConfiguration[p][d], &raw[o..o + CINPUTPLAYERCONTROL_RAW_SIZE]);
                }
            }

            for iPlayer in 0..MAX_PLAYERS as usize {
                let mut iDevice = controls.read_i16();

                if iDevice >= joystickcount {
                    iDevice = DEVICE_KEYBOARD;
                }

                self.playerInput.inputControls[iPlayer] =
                    Ptr::from_mut(&mut self.inputConfiguration[iPlayer][if iDevice == DEVICE_KEYBOARD { 0 } else { 1 }]);
            }
        }
    }

    fn write_options(&self, options: &mut BinaryFile) {
        unsafe {
            options.write_i32(GAME_VERSION.major as i32);
            options.write_i32(GAME_VERSION.minor as i32);
            options.write_i32(GAME_VERSION.patch as i32);
            options.write_i32(GAME_VERSION.build as i32);

            options.write_u8(self.spawnstyle as u8);
            options.write_u8(self.awardstyle as u8);
            options.write_u8(self.teamcollision as u8);
            options.write_u8(self.screencrunch as u8);
            options.write_u8(self.toplayer as u8);
            options.write_u8(self.scoreboardstyle as u8);
            options.write_u8(self.teamcolors as u8);
            options.write_u8(self.sound as u8);
            options.write_u8(self.music as u8);
            options.write_u8(self.musicvolume as u8);
            options.write_u8(self.soundvolume as u8);
            options.write_u8(self.respawn as u8);
            options.write_u8(self.outofboundstime as u8);
            options.write_u8(self.cpudifficulty as u8);
            options.write_u8(self.framelimiter as u8);
            options.write_u8(self.bonuswheel as u8);
            options.write_u8(self.keeppowerup as u8);
            options.write_u8(self.showwinningcrown as u8);
            options.write_u8(self.playnextmusic as u8);
            options.write_u8(self.pointspeed as u8);
            options.write_u8(self.swapstyle as u8);
            options.write_u8(self.overridepowerupsettings as u8);
            options.write_u8(self.secretsenabled as u8);
            options.write_u8(self.startgamecountdown as u8);
            options.write_u8(self.deadteamnotice as u8);
            options.write_u8(self.tournamentcontrolstyle as u8);
            options.write_u8(self.startmodedisplay as u8);

            options.write_i16(self.shieldtime);
            options.write_i16(self.shieldstyle as i16);
            options.write_i16(self.itemrespawntime);
            options.write_i16(self.hiddenblockrespawn);
            options.write_i16(self.fireballttl);
            options.write_i16(self.fireballlimit);
            options.write_i16(self.hammerdelay);
            options.write_i16(self.hammerttl);
            options.write_i16(self.hammerpower as i16);
            options.write_i16(self.hammerlimit);
            options.write_i16(self.boomerangstyle as i16);
            options.write_i16(self.boomeranglife);
            options.write_i16(self.boomeranglimit);
            options.write_i16(self.featherjumps);
            options.write_i16(self.featherlimit);
            options.write_i16(self.leaflimit);
            options.write_i16(self.pwingslimit);
            options.write_i16(self.tanookilimit);
            options.write_i16(self.bombslimit);
            options.write_i16(self.wandfreezetime);
            options.write_i16(self.wandlimit);
            options.write_i16(self.shellttl);
            options.write_i16(self.blueblockttl);
            options.write_i16(self.redblockttl);
            options.write_i16(self.grayblockttl);
            options.write_i16(self.storedpowerupdelay);
            options.write_i16(self.warplockstyle as i16);
            options.write_i16(self.warplocktime);
            options.write_i16(self.suicidetime);

            options.write_i16(self.poweruppreset);
            let mut presets = Vec::with_capacity((NUM_POWERUP_PRESETS * NUM_POWERUPS * 2) as usize);
            for row in self.allPowerupPresets.iter() {
                for v in row.iter() {
                    presets.extend_from_slice(&v.to_le_bytes());
                }
            }
            options.write_raw(&presets);

            options.write_bool(self.fullscreen);

            for k in 0..GAMEMODE_LAST as usize {
                options.write_i16(crate::smw::main::gamemodes[k].goal);
            }

            options.write_raw(&self.gamemodemenusettings.to_raw_bytes());

            let mut raw = Vec::new();
            for v in self.skinids.iter() {
                raw.extend_from_slice(&v.to_le_bytes());
            }
            options.write_raw(&raw);
            let raw: Vec<u8> = self.randomskin.iter().map(|&b| b as u8).collect();
            options.write_raw(&raw);
            let mut raw = Vec::new();
            for v in self.playercontrol.iter() {
                raw.extend_from_slice(&v.to_le_bytes());
            }
            options.write_raw(&raw);

            options.write_u8(announcerlist.current_index() as u8);
            options.write_u8(musiclist.current_index() as u8);
            options.write_u8(worldmusiclist.current_index() as u8);
            options.write_u8(soundpacklist.current_index() as u8);
            options.write_u8(menugraphicspacklist.current_index() as u8);
            options.write_u8(worldgraphicspacklist.current_index() as u8);
            options.write_u8(gamegraphicspacklist.current_index() as u8);
        }
    }

    fn write_controls(&self, controls: &mut BinaryFile) {
        let mut raw = [0u8; CINPUTPLAYERCONTROL_RAW_SIZE * 8];
        for p in 0..4 {
            for d in 0..2 {
                let o = (p * 2 + d) * CINPUTPLAYERCONTROL_RAW_SIZE;
                input_control_to_raw(&self.inputConfiguration[p][d], &mut raw[o..o + CINPUTPLAYERCONTROL_RAW_SIZE]);
            }
        }
        controls.write_raw(&raw);

        for iPlayer in 0..4 {
            controls.write_i16(self.playerInput.inputControls[iPlayer].iDevice);
        }
    }

    /// Not in the C++: the bytes `write_config` would write to options.bin and controls.sdl2.bin.
    pub fn config_bytes(&self) -> (Vec<u8>, Vec<u8>) {
        let mut options = BinaryFile::memory("options.bin", Vec::new());
        self.write_options(&mut options);
        let mut controls = BinaryFile::memory("controls.sdl2.bin", Vec::new());
        self.write_controls(&mut controls);
        (options.into_bytes(), controls.into_bytes())
    }

    /// Not in the C++: `read_binary_config` from the bytes `config_bytes` returned.
    pub fn read_config_bytes(&mut self, options: Vec<u8>, controls: Vec<u8>) {
        if catch_runtime_error(|| self.read_options(&mut BinaryFile::memory("options.bin", options))) == Some(true) {
            catch_runtime_error(|| self.read_controls(&mut BinaryFile::memory("controls.sdl2.bin", controls)));
        }
    }

    pub fn write_config(&self) {
        let ok = catch_runtime_error(|| {
            let options_path = get_home_directory() + "options.bin";
            let mut options = BinaryFile::new(&options_path, "wb");
            if !options.is_open() {
                throw_runtime_error(&format!("Could not open {}", options_path));
            }

            self.write_options(&mut options);
        });
        if ok.is_none() {
            return;
        }

        let ok = catch_runtime_error(|| {
            let controls_path = get_home_directory() + "controls.sdl2.bin";
            let mut controls = BinaryFile::new(&controls_path, "wb");
            if !controls.is_open() {
                throw_runtime_error(&format!("Could not open {}", controls_path));
            }

            self.write_controls(&mut controls);
        });
        if ok.is_none() {
            return;
        }

        unsafe {
            maplist.write_filters();
            maplist.write_map_summary_cache();
        }
    }
}
