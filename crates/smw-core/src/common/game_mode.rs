//! The game-mode base type, its virtual interface, and the `game_mode_*` ids.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::{Aliased, Ptr};
use crate::smw::gamemodes::game_mode as gm;
use crate::smw::rollback::Fnv;
use crate::smw::player::CPlayer;
use std::any::Any;
use std::ops::{Deref, DerefMut};

/// How many goal choices each mode's settings menu offers.
pub const GAMEMODE_NUM_OPTIONS: usize = 21;

/// A game mode id; one of the `game_mode_*` constants. Minigames use ids from 999 up.
pub type GameModeType = i32;
pub const game_mode_classic: GameModeType = 0;
pub const game_mode_frag: GameModeType = 1;
pub const game_mode_timelimit: GameModeType = 2;
pub const game_mode_jail: GameModeType = 3;
pub const game_mode_coins: GameModeType = 4;
pub const game_mode_stomp: GameModeType = 5;
pub const game_mode_eggs: GameModeType = 6;
pub const game_mode_ctf: GameModeType = 7;
pub const game_mode_chicken: GameModeType = 8;
pub const game_mode_tag: GameModeType = 9;
pub const game_mode_star: GameModeType = 10;
pub const game_mode_domination: GameModeType = 11;
pub const game_mode_koth: GameModeType = 12;
pub const game_mode_race: GameModeType = 13;
pub const game_mode_owned: GameModeType = 14;
pub const game_mode_frenzy: GameModeType = 15;
pub const game_mode_survival: GameModeType = 16;
pub const game_mode_greed: GameModeType = 17;
pub const game_mode_health: GameModeType = 18;
pub const game_mode_collection: GameModeType = 19;
pub const game_mode_chase: GameModeType = 20;
pub const game_mode_shyguytag: GameModeType = 21;
/// How many standard modes there are; `Game::gamemodes` has one per id below this.
pub const GAMEMODE_LAST: GameModeType = 22;
pub const game_mode_bonus: GameModeType = 999;
pub const game_mode_pipe_minigame: GameModeType = 1000;
pub const game_mode_boss_minigame: GameModeType = 1001;
pub const game_mode_boxes_minigame: GameModeType = 1002;

/// One goal choice for a mode: the label shown and the goal value it sets.
#[derive(Clone, Debug, Default)]
pub struct SModeOption {
    pub szName: String,
    pub iValue: i16,
}

/// What every game mode has: its id, goal and win state.
#[derive(Clone)]

pub struct CGameMode {
    /// The winning team once the game is over, or -1.
    pub winningteam: i16,
    pub gameover: bool,
    pub gamemode: GameModeType,
    /// Whether the near-the-goal warning sound has played this game.
    pub playedwarningsound: bool,
    /// The mode's goal from its settings; -1 is the "Unlimited" choice.
    pub goal: i16,

    pub szModeName: String,
    pub szGoalName: String,
    pub modeOptions: [SModeOption; GAMEMODE_NUM_OPTIONS],

    /// Lowest score wins.
    pub fReverseScoring: bool,

    pub _alias: Aliased,
}

impl CGameMode {
    pub fn getgamemode(&self) -> GameModeType {
        self.gamemode
    }
    pub fn get_mode_name(&self) -> &str {
        &self.szModeName
    }
    pub fn get_goal_name(&self) -> &str {
        &self.szGoalName
    }
    pub fn get_options(&mut self) -> &mut [SModeOption; GAMEMODE_NUM_OPTIONS] {
        &mut self.modeOptions
    }
    pub fn get_reverse_scoring(&self) -> bool {
        self.fReverseScoring
    }

    /// The base fields of `CGameModeTrait::harness_hash`.
    pub fn harness_hash(&self, h: &mut Fnv) {
        h.u32(self.gamemode as u32);
        h.i16s(&[self.winningteam, self.goal]);
        h.bools(&[self.gameover, self.playedwarningsound, self.fReverseScoring]);
    }
}

/// Which mode `CGameValues::gamemode` names: a standard mode by `Game::gamemodes` index, or a minigame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModeId {
    Standard(u8),
    BonusHouse,
    Pipe,
    Boss,
    Boxes,
}

/// Cloning a mode behind `dyn CGameModeTrait`, for `Game::clone_match`.
pub trait CloneGameMode {
    fn clone_box(&self) -> Box<dyn CGameModeTrait>;
}

impl<T: CGameModeTrait + Clone> CloneGameMode for T {
    fn clone_box(&self) -> Box<dyn CGameModeTrait> {
        Box::new(self.clone())
    }
}

/// Virtual interface of `CGameMode`. Default bodies live in `smw::gamemodes::game_mode`.
pub trait CGameModeTrait: Any + CloneGameMode {
    fn gm(&self) -> &CGameMode;
    fn gm_mut(&mut self) -> &mut CGameMode;
    fn as_any(&mut self) -> &mut dyn Any;

    fn init(&mut self, g: &mut Game) {
        gm::cgamemode_init(g, self)
    }
    fn think(&mut self, g: &mut Game) {
        gm::cgamemode_think(g, self)
    }
    /// Feeds the mode's simulated state into a desync checksum; overrides add their own fields to the base ones.
    fn harness_hash(&self, h: &mut Fnv) {
        self.gm().harness_hash(h);
    }
    fn draw_background(&mut self, g: &mut Game) {}
    fn draw_foreground(&mut self, g: &mut Game) {}
    /// What a kill of `other` by `inflictor` does under this mode's rules.
    fn playerkilledplayer(&mut self, g: &mut Game, inflictor: PlayerId, other: PlayerId, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledplayer(g, self, inflictor, other, style)
    }
    /// What a death with no killer to credit does under this mode's rules.
    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(g, self, player, style)
    }
    /// `player` picked up an extra life of kind `iType`.
    fn playerextraguy(&mut self, g: &mut Game, player: PlayerId, iType: i16) {
        gm::cgamemode_playerextraguy(g, self, player, iType)
    }
    /// Ends the game if `_player`'s team has reached the goal; `Removed` when it did.
    fn check_winner(&mut self, g: &mut Game, _player: PlayerId) -> PlayerKillType {
        PlayerKillType::Normal
    }
    /// Whether players keep their stored powerups in this mode.
    fn has_stored_powerups(&mut self) -> bool {
        true
    }
}

impl Deref for dyn CGameModeTrait {
    type Target = CGameMode;
    #[inline(always)]
    fn deref(&self) -> &CGameMode {
        self.gm()
    }
}

impl DerefMut for dyn CGameModeTrait {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut CGameMode {
        self.gm_mut()
    }
}
