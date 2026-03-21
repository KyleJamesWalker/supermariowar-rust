use bevy::prelude::*;

/// Top-level game state controlling which systems run.
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Splash,
    Menu,
    MapSelect,
    ModeSelect,
    Gameplay,
    GameOver,
}

/// Sub-state for gameplay phases.
#[derive(SubStates, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
#[source(GameState = GameState::Gameplay)]
pub enum GameplayState {
    #[default]
    Countdown,
    Playing,
    Paused,
    RoundOver,
    MatchOver,
}

/// Resource tracking the winning player (if any) at end of round/match.
#[derive(Resource, Debug, Default)]
pub struct MatchResult {
    pub winning_team: Option<u8>,
    pub winning_player: Option<usize>,
}
