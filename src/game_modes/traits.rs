use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;

/// Configuration for a game mode instance.
#[derive(Resource, Debug, Clone)]
pub struct GameModeConfig {
    pub mode_type: GameModeType,
    pub kill_limit: u32,
    pub time_limit_secs: u32,
    pub point_limit: u32,
    pub lives: u32,
    pub frenzy_rate: f32,
    pub team_kill_counts: bool,
    pub sudden_death: bool,
    pub zone_change_interval_secs: f32,
    pub flag_auto_return_secs: f32,
    pub coin_spawn_interval_secs: f32,
    pub coins_dropped_on_kill: u32,
}

impl Default for GameModeConfig {
    fn default() -> Self {
        Self {
            mode_type: GameModeType::Classic,
            kill_limit: 10,
            time_limit_secs: 60,
            point_limit: 5,
            lives: 5,
            frenzy_rate: 2.0,
            team_kill_counts: false,
            sudden_death: true,
            zone_change_interval_secs: 30.0,
            flag_auto_return_secs: 15.0,
            coin_spawn_interval_secs: 3.0,
            coins_dropped_on_kill: 3,
        }
    }
}

/// All available game mode types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameModeType {
    Classic,
    Frag,
    TimeLimit,
    Jail,
    CoinCollection,
    Stomp,
    Eggs,
    CaptureTheFlag,
    Chicken,
    Tag,
    Star,
    Domination,
    KingOfTheHill,
    Race,
    Owned,
    Frenzy,
    Survival,
    Greed,
    Health,
    CardCollection,
    Chase,
    ShyGuyTag,
}

// ---------------------------------------------------------------------------
// Kill method enum used to distinguish how a kill happened
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillMethod {
    Stomp,
    Projectile,
    Hazard,
    Star,
    Environment,
    Bomb,
    Fire,
    Hammer,
    Boomerang,
    Shell,
    Other,
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// Fired when one player kills another.
#[derive(Event, Debug)]
pub struct PlayerKillEvent {
    pub killer: usize,
    pub victim: usize,
    pub method: KillMethod,
}

/// Fired when a player collects an item.
#[derive(Event, Debug)]
pub struct ItemCollectEvent {
    pub player_index: usize,
    pub item_type: CollectibleItemType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectibleItemType {
    Coin,
    Powerup,
    Flag,
    Key,
    Custom,
}

/// Fired for mode-specific objective completions.
#[derive(Event, Debug)]
pub struct ObjectiveEvent {
    pub player_index: usize,
    pub objective_type: ObjectiveType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveType {
    FlagCapture,
    ZoneCaptured,
    ZoneHeld,
    TagTransfer,
    Elimination,
    Custom,
}

/// Legacy score event kept for backward compatibility with any external consumers.
#[derive(Event, Debug)]
pub struct ScoreEvent {
    pub player_index: usize,
    pub points: i32,
    pub reason: ScoreReason,
}

#[derive(Debug, Clone, Copy)]
pub enum ScoreReason {
    Kill,
    Stomp,
    CoinCollect,
    FlagCapture,
    Objective,
    Bonus,
}

/// Fired when the current round ends.
#[derive(Event, Debug)]
pub struct RoundOverEvent {
    pub winning_player: Option<usize>,
    pub winning_team: Option<u8>,
}

/// Fired when the start-of-round countdown finishes.
#[derive(Event, Debug)]
pub struct CountdownFinishedEvent;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Per-player score tracking across the entire match.
#[derive(Resource, Debug, Clone)]
pub struct ScoreBoard {
    pub kills: [u32; MAX_PLAYERS],
    pub deaths: [u32; MAX_PLAYERS],
    pub score: [i32; MAX_PLAYERS],
    pub objective_count: [u32; MAX_PLAYERS],
}

impl Default for ScoreBoard {
    fn default() -> Self {
        Self {
            kills: [0; MAX_PLAYERS],
            deaths: [0; MAX_PLAYERS],
            score: [0; MAX_PLAYERS],
            objective_count: [0; MAX_PLAYERS],
        }
    }
}

/// Countdown timer for time-based modes.
#[derive(Resource, Debug)]
pub struct GameTimer {
    pub timer: Timer,
    pub expired: bool,
}

impl GameTimer {
    pub fn new(seconds: f32) -> Self {
        Self {
            timer: Timer::from_seconds(seconds, TimerMode::Once),
            expired: false,
        }
    }

    pub fn remaining_secs(&self) -> f32 {
        self.timer.remaining_secs()
    }
}

impl Default for GameTimer {
    fn default() -> Self {
        Self::new(60.0)
    }
}

/// Round / match progression tracker.
#[derive(Resource, Debug, Clone)]
pub struct RoundManager {
    pub current_round: u32,
    pub total_rounds: u32,
    /// Per-player wins across rounds.
    pub round_wins: [u32; MAX_PLAYERS],
    /// Per-team wins across rounds (index = team id).
    pub team_round_wins: [u32; MAX_PLAYERS],
    pub match_finished: bool,
}

impl Default for RoundManager {
    fn default() -> Self {
        Self {
            current_round: 1,
            total_rounds: 3,
            round_wins: [0; MAX_PLAYERS],
            team_round_wins: [0; MAX_PLAYERS],
            match_finished: false,
        }
    }
}

/// Countdown at the start of each round (3-2-1-GO!).
#[derive(Resource, Debug)]
pub struct RoundCountdown {
    pub timer: Timer,
    pub phase: CountdownPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountdownPhase {
    Three,
    Two,
    One,
    Go,
    Done,
}

impl Default for RoundCountdown {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            phase: CountdownPhase::Three,
        }
    }
}

/// Timer used during RoundOver state to show scores before transitioning.
#[derive(Resource, Debug)]
pub struct RoundOverTimer {
    pub timer: Timer,
}

impl Default for RoundOverTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        }
    }
}
