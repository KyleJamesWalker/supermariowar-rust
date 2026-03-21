use bevy::prelude::*;

/// Configuration for a game mode instance.
#[derive(Resource, Debug, Clone)]
pub struct GameModeConfig {
    pub mode_type: GameModeType,
    pub kill_limit: u32,
    pub time_limit: u32,
    pub point_limit: u32,
}

impl Default for GameModeConfig {
    fn default() -> Self {
        Self {
            mode_type: GameModeType::Classic,
            kill_limit: 10,
            time_limit: 0,
            point_limit: 5,
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

/// Event emitted when a player scores a point/kill in the current mode.
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

/// Event emitted when the current round ends.
#[derive(Event, Debug)]
pub struct RoundOverEvent {
    pub winning_player: Option<usize>,
    pub winning_team: Option<u8>,
}
