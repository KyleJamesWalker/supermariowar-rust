//! Game options and session state (`CGameValues`), with the options.bin and controls.sdl2.bin formats.

use crate::common::keys::*;
use crate::game::Game;
use crate::common::eyecandy_styles::{AwardStyle, ScoreboardStyle, SpawnStyle};
use crate::common::file_io::{throw_runtime_error, BinaryFile};
use crate::common::game::perror;
use crate::common::game_mode::{CGameModeTrait, GAMEMODE_LAST};
use crate::common::game_mode_settings::{GameModeSettings, GAMEMODESETTINGS_RAW_SIZE};
use crate::common::gameplay_styles::{BoomerangStyle, ShieldStyle, TeamCollisionStyle, TournamentControlStyle, WarpLockStyle};
use crate::common::global_constants::{MAXTOURNAMENTGAMES, MAX_PLAYERS, NUM_AUTO_FILTERS, NUM_POWERUPS, NUM_POWERUP_PRESETS, frame_rate_cap, ticks_from_disk, ticks_to_disk, FRAME_RATE_DISPLAY};
use crate::common::input::*;
use crate::common::match_types::{MatchType, Minigame};
use crate::common::path::get_home_directory;
use crate::common::sfx::{sfx_setmusicvolume, sfx_setsoundvolume, sfxSound};
use crate::common::version::{Version, GAME_VERSION};
use crate::globals::*;
use Keycode;

/// The window title.
pub const TITLESTRING: &str = "Super Mario War";

/// The stops of the loaded tour, in play order.
pub type TourStopVec = Vec<Box<crate::common::world_tour_stop::TourStop>>;

/// Which top-level screen the main loop runs.
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

crate::enum_from_u8!(AppState, 7);

//tournament scores
/// A team's tournament or tour results.
#[derive(Clone, Copy, Default, Debug)]
pub struct TournamentScores {
    /// Games won.
    pub wins: i16,
    /// Per game played, the team's place.
    pub type_: [i16; MAXTOURNAMENTGAMES as usize],
    pub total: i16, //used for running total in a tour
}

/// Gameplay-specific variables, which are reset on the start of a game.
#[derive(Clone, Debug)]
pub struct GameplayFlags {
    /// The pause menu is open.
    pub pausegame: bool,
    /// The exit prompt is open.
    pub exitinggame: bool,
    pub exityes: bool,

    /// The game cannot be exited while set; `noexittimer` clears it when it runs out, in ticks.
    pub noexit: bool,
    pub noexittimer: i16,
    pub forceexittimer: i16,

    /// How many teams have been removed so far; `CScore::order` takes the next value.
    pub teamdeadcounter: i16,

    /// Ticks left of a screen shake, and the player and team who caused it.
    pub screenshaketimer: i16,
    pub screenshakeplayerid: i16,
    pub screenshaketeamid: i16,
    pub screenshakekillscount: i16,
    pub screenshakekillinair: bool,

    /// The team whose slow-down powerup is active, or -1.
    pub slowdownon: i16,
    pub slowdowncounter: i16,

    /// The scoreboard is moving to its end-of-game layout; `scorepercentmove` is how far, 0 to 1.
    pub showscoreboard: bool,
    pub scorepercentmove: f32,

    pub playskidsound: bool,
    pub playinvinciblesound: bool,
    pub playflyingsound: bool,

    /// The players are being swapped by a mystery mushroom.
    pub swapplayers: bool,
    pub swapplayersposition: f32,
    pub swapplayersblink: bool,
    pub swapplayersblinkcount: i16,

    /// The wind's push on players and objects, in pixels per tick.
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

//[Keyboard/Joystick][Game/Menu][NumPlayers][NumKeys]  left, right, jump, down, turbo, powerup, start, cancel
/// The default bindings, by input kind, control set, player and slot.
pub static controlkeys: [[[[Keycode; NUM_KEYS]; 4]; 2]; 2] = [
    [
        [
            [SDLK_LEFT, SDLK_RIGHT, SDLK_UP, SDLK_DOWN, SDLK_RCTRL, SDLK_RSHIFT, SDLK_RETURN, SDLK_ESCAPE],
            [SDLK_a, SDLK_d, SDLK_w, SDLK_s, SDLK_e, SDLK_q, SDLK_UNKNOWN, SDLK_UNKNOWN],
            [SDLK_g, SDLK_j, SDLK_y, SDLK_h, SDLK_u, SDLK_t, SDLK_UNKNOWN, SDLK_UNKNOWN],
            [SDLK_l, SDLK_QUOTE, SDLK_p, SDLK_SEMICOLON, SDLK_LEFTBRACKET, SDLK_o, SDLK_UNKNOWN, SDLK_UNKNOWN],
        ],
        //up, down, left, right, select, cancel, random, fast scroll
        [
            [SDLK_UP, SDLK_DOWN, SDLK_LEFT, SDLK_RIGHT, SDLK_RETURN, SDLK_ESCAPE, SDLK_SPACE, SDLK_LSHIFT],
            [SDLK_w, SDLK_s, SDLK_a, SDLK_d, SDLK_e, SDLK_q, SDLK_UNKNOWN, SDLK_UNKNOWN],
            [SDLK_y, SDLK_h, SDLK_g, SDLK_j, SDLK_u, SDLK_t, SDLK_UNKNOWN, SDLK_UNKNOWN],
            [SDLK_p, SDLK_SEMICOLON, SDLK_l, SDLK_QUOTE, SDLK_LEFTBRACKET, SDLK_o, SDLK_UNKNOWN, SDLK_UNKNOWN],
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
/// The built-in powerup weights per preset, indexed as listed above; 0 never spawns.
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

/// Plays `sfx` if sound is on.
pub fn if_sound_on_play(g: &mut Game, sfx: &mut sfxSound) {
    if g.game_values.sound {
        sfx.play(g);
    }
}

/// Built-in preset `presetIdx`'s weight for powerup `powerupIdx`.
pub fn default_powerup_setting(presetIdx: usize, powerupIdx: usize) -> i16 {
    DEFAULT_POWERUP_PRESETS[presetIdx][powerupIdx]
}

/// The options saved in options.bin and controls.sdl2.bin.
#[derive(Clone)]
pub struct CGameConfig {
    /// 0 to 128.
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
    /// The CPU players' skill, 0 (easiest) to 4.
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
    /// The "Frame Rate" setting: `FRAME_RATE_DISPLAY` or one of `FRAME_RATE_CAPS`.
    pub framelimiter: i16,
    /// Ticks a revealed hidden block stays before hiding again; 0 never hides.
    pub hiddenblockrespawn: i16,
    pub inputConfiguration: [[CInputPlayerControl; 2]; 4], //[NumPlayers][Keyboard/Joystick]
    /// Ticks before an emptied powerup block refills; 0 never.
    pub itemrespawntime: i16,
    pub keeppowerup: bool,
    pub leaflimit: i16,
    pub pwingslimit: i16,
    pub tanookilimit: i16,
    /// Per player: 0 off, 1 human, 2 CPU.
    pub playercontrol: [i16; 4],
    pub playerInput: CPlayerInput,
    pub playnextmusic: bool, //automatically advance to the next music track after one finishes
    /// Seconds a player may stay above the screen top before dying; 0 for no limit.
    pub outofboundstime: i16,
    /// 1 makes every powerup block use the global weights, ignoring the map's own.
    pub overridepowerupsettings: i16,
    pub secretsenabled: bool,
    pub gamemodemenusettings: GameModeSettings,
    pub music: bool,
    /// Ticks per point in the timed scoring modes (area, king of the hill, chicken), before `ticks` scaling.
    pub pointspeed: i16,
    pub poweruppreset: i16,
    pub allPowerupPresets: [[i16; NUM_POWERUPS as usize]; NUM_POWERUP_PRESETS as usize],
    /// Per player: pick a random skin each game.
    pub randomskin: [bool; 4],
    pub scoreboardstyle: ScoreboardStyle,
    pub screencrunch: bool,
    pub shieldtime: i16,
    pub shieldstyle: ShieldStyle,
    pub showwinningcrown: bool,
    /// Per player: index into `Game::skinlist`.
    pub skinids: [i16; 4],
    pub wandlimit: i16,
    pub tournamentcontrolstyle: TournamentControlStyle, //ID for the player selected control style
    pub wandfreezetime: i16,
    pub toplayer: bool,
    pub stats: crate::smw::stats::MatchStats,
    pub teamcolors: bool,
    pub sound: bool,
    pub startgamecountdown: bool,
    pub startmodedisplay: bool,
    /// How fast the stored-powerup ring closes when one is used.
    pub storedpowerupdelay: i16,
    /// Mystery mushroom swap: 0 walk, 1 blink then swap, 2 instant.
    pub swapstyle: i16,
    /// Respawn delay in half-seconds; 0 is instant.
    pub respawn: i16,
    pub _alias: Aliased,
}

/// Program options plus the state of the current session, match and game.
#[derive(Clone)]
pub struct CGameValues {
    pub cgame_config: CGameConfig,

    /// Draw the frame rate overlay.
    pub showfps: bool,
    pub frameadvance: bool,
    pub autokill: bool,

    pub appstate: AppState,

    pub flags: GameplayFlags,
    /// The current game mode, or `None` before one is chosen.
    pub gamemode: Option<crate::common::game_mode::ModeId>,

    /// The screen fade's opacity, 0 to 255, and its change per 62.5 Hz tick (negative fades in).
    pub screenfade: i16,
    pub screenfadespeed: i16,

    pub matchtype: MatchType, //The currently selected match type: quick game, single, tournament, tour, world, minigame

    /// Per team, its players' ids; `teamcounts` says how many are in use.
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

    /// Per player, the powerup stored for the next game, or -1; `gamepowerups` is what each holds in the current one.
    pub storedpowerups: [i16; 4],
    pub gamepowerups: [i16; 4],
    pub powerupweights: [i16; NUM_POWERUPS as usize],

    /// Per team, the world map items held; `worldpowerupcount` says how many are in use.
    pub worldpowerups: [[i16; 32]; 4],
    pub worldpowerupcount: [i16; 4],

    /// The world point multiplier item in play, or -1.
    pub worldpointsbonus: i16,

    /// Per player, the color.
    pub colorids: [i16; 4],

    /// Per player, ticks left of the bullet bill powerup and until its next bill.
    pub bulletbilltimer: [i16; 4],
    pub bulletbillspawntimer: [i16; 4],

    /// The index in `announcerlist` and `musiclist` loaded into sounds, or -1 / `usize::MAX` for none.
    pub loadedannouncer: usize,
    pub loadedmusic: i16,

    /// Cycles 0 to 3 each tick; a CPU player thinks when it matches its id, so every fourth tick (second on Very Hard).
    pub cputurn: i16,

    pub tournament_scores: [TournamentScores; 4],

    /// Mode settings for the current game; `gamemodemenusettings` holds the menu's copy.
    pub gamemodesettings: GameModeSettings,

    /// The audio device opened.
    pub soundcapable: bool,

    /// Per map filter, whether it is on, and its icon.
    pub pfFilters: Vec<bool>,
    pub piFilterIcons: Vec<i16>,
    pub selectedmapfilter: i16,
    pub fNeedWriteFilters: bool,
    pub fFiltersOn: bool,

    /// The player who entered a world map bonus house alone, or -1.
    pub singleplayermode: i16,

    pub worldskipscoreboard: bool,

    pub windaffectsplayers: bool,
    pub spinscreen: bool,
    pub reversewalk: bool,
    pub spotlights: bool,

    /// Progress toward the hidden secrets, which `secretsenabled` turns on.
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
    /// Zero-initialized; `init` sets the defaults.
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
            stats: crate::smw::stats::MatchStats::new(),
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
    /// Tour stop `i`, for code that holds it while it also uses `g`.
    pub fn tour_stop(&mut self, i: usize) -> Ptr<crate::common::world_tour_stop::TourStop> {
        Ptr::from_mut(&mut *self.tourstops[i])
    }

    /// The tour stop being played.
    pub fn current_tour_stop(&mut self) -> Ptr<crate::common::world_tour_stop::TourStop> {
        self.tour_stop(self.tourstopcurrent)
    }

    /// Zero-initialized; `init` sets the defaults.
    pub fn new() -> Self {
        CGameValues {
            cgame_config: CGameConfig::new(),
            showfps: false,
            frameadvance: false,
            autokill: false,
            appstate: AppState::Splash,
            flags: GameplayFlags::default(),
            gamemode: None,
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

    /// Sets every option to its default; `read_binary_config` then loads the saved ones.
    pub fn init(&mut self, g: &mut Game) {
        //set standard game values
        self.playercontrol[0] = 1;
        self.playercontrol[1] = 1;
        self.showfps = false;
        self.frameadvance = false;
        self.autokill = false;
        self.framelimiter = FRAME_RATE_DISPLAY;
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
        self.loadedannouncer = usize::MAX; // none loaded
        self.loadedmusic = -1;
        self.scoreboardstyle = ScoreboardStyle::Top;
        self.teamcolors = true;
        self.cputurn = -1;
        self.shieldtime = 60;
        self.shieldstyle = ShieldStyle::SoftWithStomp;
        self.musicvolume = 128;
        self.soundvolume = 128;
        self.respawn = 2;
        self.itemrespawntime = 1800; //default item respawn is 30 seconds
        self.hiddenblockrespawn = 1800; //default item respawn is 30 seconds
        self.outofboundstime = 5;
        self.warplockstyle = WarpLockStyle::ExitOnly; // Lock Warp Exit Only
        self.warplocktime = 180; // 3 seconds
        self.suicidetime = 300; // 5 seconds
        self.cpudifficulty = 2;
        self.fireballttl = 300; // 5 seconds
        self.shellttl = 480; // 8 seconds
        self.blueblockttl = 300; // 5 seconds
        self.redblockttl = 300; // 5 seconds
        self.grayblockttl = 300; // 5 seconds
        self.hammerdelay = 24; // 0.4 second
        self.hammerttl = 48; // 0.8 second
        self.hammerpower = true; //hammers die on first hit
        self.fireballlimit = 0; //Unlimited
        self.hammerlimit = 0; //Unlimited
        self.boomerangstyle = BoomerangStyle::SMB3; //SMB3 style
        self.boomeranglife = 240; // 4 seconds of zelda boomerang
        self.boomeranglimit = 0; //Unlimited
        self.featherjumps = 1; //Allow one extra cape jump
        self.featherlimit = 0; //Unlimited
        self.leaflimit = 0; //Unlimited
        self.pwingslimit = 0; //Unlimited
        self.tanookilimit = 0; //Unlimited
        self.bombslimit = 0; //Unlimited
        self.wandfreezetime = 300; //5 seconds of freeze time
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

        let filtercount = g.filterslist.count();
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
            self.cgame_config.playerInput.inputControls[iPlayer] = Some(((iPlayer) as u8, (0) as u8));
        }

        //Set the default powerup weights for bonus wheel and [?] boxes
        for iPreset in 0..NUM_POWERUP_PRESETS as usize {
            for iPowerup in 0..NUM_POWERUPS as usize {
                self.allPowerupPresets[iPreset][iPowerup] = default_powerup_setting(iPreset, iPowerup);
            }
        }
    }

    /// The change in `screenfade` this tick. `screenfadespeed` is per 62.5 Hz tick, and a fade starts at 255 going down
    /// or 8 going up, so the steps taken so far follow from `screenfade`; one tick in 24 takes two.
    pub fn screenfade_step(&self) -> i16 {
        let speed = self.screenfadespeed as i32;
        let start = if speed < 0 { 255 } else { 8 };
        let steps = (self.screenfade as i32 - start).abs() / speed.abs().max(1);
        (if steps % 25 == 23 { 2 * speed } else { speed }) as i16
    }

    /// Clears the gameplay flags, starts the fade in, and initializes the game mode.
    pub fn reset_gameplay_settings(&mut self, g: &mut Game) {
        self.flags = GameplayFlags::default();

        self.screenfade = 255;
        self.screenfadespeed = -8;

        //Initialize game mode
        g.mode().get().init(g);
    }

    /// Clears all progress toward the secrets.
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


/// `CInputPlayerControl` as stored in controls.sdl2.bin: `i16 iDevice`, 2 bytes padding, `i32 keys[2][8]`.
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

/// Runs `f`, printing the message of a `throw_runtime_error` panic in it to stderr; `None` when it panicked.
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
    /// Loads options.bin, then controls.sdl2.bin, from the settings directory; skips both if options.bin is missing or from another version.
    pub fn read_binary_config(&mut self, g: &mut Game) {
        let ok = catch_runtime_error(|| {
            let options_path = get_home_directory(g) + "options.bin";
            let mut options = BinaryFile::new(&options_path, "rb");
            if !options.is_open() {
                throw_runtime_error(&format!("Could not open {}", options_path));
            }

            self.read_options(g, &mut options)
        });
        if ok != Some(true) {
            return;
        }

        catch_runtime_error(|| {
            let controls_path = get_home_directory(g) + "controls.sdl2.bin";
            let mut controls = BinaryFile::new(&controls_path, "rb");
            if !controls.is_open() {
                throw_runtime_error(&format!("Could not open {}", controls_path));
            }

            self.read_controls(g, &mut controls);
        });
    }

    /// Reads options.bin's fields; false, with nothing read past the version, when it is from another version.
    fn read_options(&mut self, g: &mut Game, options: &mut BinaryFile) -> bool {
        let mut gp = g.ptr();
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
        if frame_rate_cap(self.framelimiter).is_none() {
            self.framelimiter = FRAME_RATE_DISPLAY;
        }
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

        self.shieldtime = ticks_from_disk(options.read_i16());
        self.shieldstyle = ShieldStyle::from_u8(options.read_i16() as u8);
        self.itemrespawntime = ticks_from_disk(options.read_i16());
        self.hiddenblockrespawn = ticks_from_disk(options.read_i16());
        self.fireballttl = ticks_from_disk(options.read_i16());
        self.fireballlimit = options.read_i16();
        self.hammerdelay = ticks_from_disk(options.read_i16());
        self.hammerttl = ticks_from_disk(options.read_i16());
        self.hammerpower = options.read_i16() != 0;
        self.hammerlimit = options.read_i16();
        self.boomerangstyle = BoomerangStyle::from_u8(options.read_i16() as u8);
        self.boomeranglife = ticks_from_disk(options.read_i16());
        self.boomeranglimit = options.read_i16();
        self.featherjumps = options.read_i16();
        self.featherlimit = options.read_i16();
        self.leaflimit = options.read_i16();
        self.pwingslimit = options.read_i16();
        self.tanookilimit = options.read_i16();
        self.bombslimit = options.read_i16();
        self.wandfreezetime = ticks_from_disk(options.read_i16());
        self.wandlimit = options.read_i16();
        self.shellttl = ticks_from_disk(options.read_i16());
        self.blueblockttl = ticks_from_disk(options.read_i16());
        self.redblockttl = ticks_from_disk(options.read_i16());
        self.grayblockttl = ticks_from_disk(options.read_i16());
        self.storedpowerupdelay = options.read_i16();
        self.warplockstyle = WarpLockStyle::from_u8(options.read_i16() as u8);
        self.warplocktime = ticks_from_disk(options.read_i16());
        self.suicidetime = ticks_from_disk(options.read_i16());

        self.poweruppreset = options.read_i16();
        let mut presets = [0i16; (NUM_POWERUP_PRESETS * NUM_POWERUPS) as usize];
        options.read_i16_array(&mut presets);
        for (i, v) in presets.iter().enumerate() {
            self.allPowerupPresets[i / NUM_POWERUPS as usize][i % NUM_POWERUPS as usize] = *v;
        }

        self.fullscreen = options.read_bool();

        for iGameMode in 0..GAMEMODE_LAST as usize {
            gp.gamemodes[iGameMode].goal = options.read_i16();
        }

        let mut gms = [0u8; GAMEMODESETTINGS_RAW_SIZE];
        options.read_raw(&mut gms);
        self.gamemodemenusettings.from_raw_bytes(&gms);
        for v in self.gamemodemenusettings.durations_mut() {
            *v = ticks_from_disk(*v);
        }

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
            if self.skinids[iPlayer] as i64 >= g.skinlist.count() as i64 || self.skinids[iPlayer] < 0 {
                self.skinids[iPlayer] = 0;
            }
        }

        g.announcerlist.set_current_index(options.read_u8() as usize);
        g.musiclist.set_current(options.read_u8() as usize);
        g.worldmusiclist.set_current(options.read_u8() as usize);
        g.soundpacklist.set_current_index(options.read_u8() as usize);
        g.menugraphicspacklist.set_current_index(options.read_u8() as usize);
        g.worldgraphicspacklist.set_current_index(options.read_u8() as usize);
        g.gamegraphicspacklist.set_current_index(options.read_u8() as usize);

        sfx_setmusicvolume(g, self.musicvolume as i32);
        sfx_setsoundvolume(g, self.soundvolume as i32);
        true
    }

    /// Reads every player's bindings, then each player's device; a device not connected falls back to the keyboard.
    fn read_controls(&mut self, g: &mut Game, controls: &mut BinaryFile) {
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

            if iDevice as i32 >= g.devices.len() as i32 {
                iDevice = DEVICE_KEYBOARD;
            }

            self.playerInput.inputControls[iPlayer] = Some((iPlayer as u8, if iDevice == DEVICE_KEYBOARD { 0 } else { 1 }));
        }
    }

    fn write_options(&self, g: &mut Game, options: &mut BinaryFile) {
        let mut gp = g.ptr();
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

        options.write_i16(ticks_to_disk(self.shieldtime));
        options.write_i16(self.shieldstyle as i16);
        options.write_i16(ticks_to_disk(self.itemrespawntime));
        options.write_i16(ticks_to_disk(self.hiddenblockrespawn));
        options.write_i16(ticks_to_disk(self.fireballttl));
        options.write_i16(self.fireballlimit);
        options.write_i16(ticks_to_disk(self.hammerdelay));
        options.write_i16(ticks_to_disk(self.hammerttl));
        options.write_i16(self.hammerpower as i16);
        options.write_i16(self.hammerlimit);
        options.write_i16(self.boomerangstyle as i16);
        options.write_i16(ticks_to_disk(self.boomeranglife));
        options.write_i16(self.boomeranglimit);
        options.write_i16(self.featherjumps);
        options.write_i16(self.featherlimit);
        options.write_i16(self.leaflimit);
        options.write_i16(self.pwingslimit);
        options.write_i16(self.tanookilimit);
        options.write_i16(self.bombslimit);
        options.write_i16(ticks_to_disk(self.wandfreezetime));
        options.write_i16(self.wandlimit);
        options.write_i16(ticks_to_disk(self.shellttl));
        options.write_i16(ticks_to_disk(self.blueblockttl));
        options.write_i16(ticks_to_disk(self.redblockttl));
        options.write_i16(ticks_to_disk(self.grayblockttl));
        options.write_i16(self.storedpowerupdelay);
        options.write_i16(self.warplockstyle as i16);
        options.write_i16(ticks_to_disk(self.warplocktime));
        options.write_i16(ticks_to_disk(self.suicidetime));

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
            options.write_i16(gp.gamemodes[k].goal);
        }

        let mut gms = self.gamemodemenusettings.clone();
        for v in gms.durations_mut() {
            *v = ticks_to_disk(*v);
        }
        options.write_raw(&gms.to_raw_bytes());

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

        options.write_u8(g.announcerlist.current_index() as u8);
        options.write_u8(g.musiclist.current_index() as u8);
        options.write_u8(g.worldmusiclist.current_index() as u8);
        options.write_u8(g.soundpacklist.current_index() as u8);
        options.write_u8(g.menugraphicspacklist.current_index() as u8);
        options.write_u8(g.worldgraphicspacklist.current_index() as u8);
        options.write_u8(g.gamegraphicspacklist.current_index() as u8);
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
            let (set, kind) = self.playerInput.inputControls[iPlayer].expect("player input set");
            controls.write_i16(self.inputConfiguration[set as usize][kind as usize].iDevice);
        }
    }

    /// The bytes `write_config` would write to options.bin and controls.sdl2.bin.
    pub fn config_bytes(&self, g: &mut Game) -> (Vec<u8>, Vec<u8>) {
        let mut options = BinaryFile::memory("options.bin", Vec::new());
        self.write_options(g, &mut options);
        let mut controls = BinaryFile::memory("controls.sdl2.bin", Vec::new());
        self.write_controls(&mut controls);
        (options.into_bytes(), controls.into_bytes())
    }

    /// `read_binary_config` from the bytes `config_bytes` returned.
    pub fn read_config_bytes(&mut self, g: &mut Game, options: Vec<u8>, controls: Vec<u8>) {
        if catch_runtime_error(|| self.read_options(g, &mut BinaryFile::memory("options.bin", options))) == Some(true) {
            catch_runtime_error(|| self.read_controls(g, &mut BinaryFile::memory("controls.sdl2.bin", controls)));
        }
    }

    /// Saves options.bin, controls.sdl2.bin, the map filters and the map summary cache.
    pub fn write_config(&self, g: &mut Game) {
        let mut gp = g.ptr();
        let ok = catch_runtime_error(|| {
            let options_path = get_home_directory(g) + "options.bin";
            let mut options = BinaryFile::new(&options_path, "wb");
            if !options.is_open() {
                throw_runtime_error(&format!("Could not open {}", options_path));
            }

            self.write_options(g, &mut options);
        });
        if ok.is_none() {
            return;
        }

        let ok = catch_runtime_error(|| {
            let controls_path = get_home_directory(g) + "controls.sdl2.bin";
            let mut controls = BinaryFile::new(&controls_path, "wb");
            if !controls.is_open() {
                throw_runtime_error(&format!("Could not open {}", controls_path));
            }

            self.write_controls(&mut controls);
        });
        if ok.is_none() {
            return;
        }

        gp.maplist.write_filters(g);
        gp.maplist.write_map_summary_cache(g);
    }
}
