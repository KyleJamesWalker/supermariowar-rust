use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use crate::player::components::{DeathCause, PlayerDeathEvent};
use super::traits::*;

/// Resource tracking state for Survival mode.
/// Each player starts with N lives. Last player standing wins.
#[derive(Resource, Debug)]
pub struct SurvivalModeState {
    pub lives: [i32; MAX_PLAYERS],
    /// Whether the player is still in the game (not eliminated).
    pub alive: [bool; MAX_PLAYERS],
    /// Number of players that are still in the game.
    pub alive_count: u32,
    /// Total number of players that started this round.
    pub total_players: u32,
    /// Track eliminations for scoring.
    pub eliminations: [u32; MAX_PLAYERS],
    pub initialised: bool,
}

impl Default for SurvivalModeState {
    fn default() -> Self {
        Self {
            lives: [0; MAX_PLAYERS],
            alive: [false; MAX_PLAYERS],
            alive_count: 0,
            total_players: 0,
            eliminations: [0; MAX_PLAYERS],
            initialised: false,
        }
    }
}

/// Initialise lives for all players at the start of a round.
pub fn survival_init(
    config: Res<GameModeConfig>,
    mut state: ResMut<SurvivalModeState>,
    players: Query<&crate::player::components::Player>,
) {
    if config.mode_type != GameModeType::Survival {
        return;
    }
    if state.initialised {
        return;
    }

    let mut count = 0u32;
    for player in players.iter() {
        if player.index < MAX_PLAYERS {
            state.lives[player.index] = config.lives as i32;
            state.alive[player.index] = true;
            count += 1;
        }
    }
    state.alive_count = count;
    state.total_players = count;
    state.initialised = true;
}

/// When a player is killed, decrement their lives. If they hit 0, they are eliminated.
pub fn handle_survival_death(
    config: Res<GameModeConfig>,
    mut state: ResMut<SurvivalModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut death_events: EventReader<PlayerDeathEvent>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::Survival {
        return;
    }

    for event in death_events.read() {
        let idx = event.player_index;
        if idx >= MAX_PLAYERS || !state.alive[idx] {
            continue;
        }

        state.lives[idx] -= 1;
        scoreboard.deaths[idx] += 1;

        if state.lives[idx] <= 0 {
            state.alive[idx] = false;
            state.alive_count = state.alive_count.saturating_sub(1);

            // Credit the killer with an elimination point
            if let DeathCause::Killed { by } = event.cause {
                if by < MAX_PLAYERS {
                    state.eliminations[by] += 1;
                    scoreboard.score[by] += 1;
                    scoreboard.objective_count[by] += 1;
                }
            }
        }

        // Check if only one (or zero) players remain
        if state.alive_count <= 1 && state.total_players > 1 {
            let winner = state
                .alive
                .iter()
                .enumerate()
                .find(|(_, &a)| a)
                .map(|(i, _)| i);

            round_over.send(RoundOverEvent {
                winning_player: winner,
                winning_team: None,
            });
            return;
        }
    }
}

/// Kill events also feed into death tracking (bridge from PlayerKillEvent).
pub fn handle_survival_kills(
    config: Res<GameModeConfig>,
    mut kill_events: EventReader<PlayerKillEvent>,
    mut death_events: EventWriter<PlayerDeathEvent>,
    players: Query<(Entity, &crate::player::components::Player)>,
) {
    if config.mode_type != GameModeType::Survival {
        return;
    }

    for event in kill_events.read() {
        // Look up the victim's entity
        let victim_entity = players
            .iter()
            .find(|(_, p)| p.index == event.victim)
            .map(|(e, _)| e);

        if let Some(entity) = victim_entity {
            death_events.send(PlayerDeathEvent {
                player_entity: entity,
                player_index: event.victim,
                killer_index: Some(event.killer),
                cause: DeathCause::Killed { by: event.killer },
            });
        }
    }
}
