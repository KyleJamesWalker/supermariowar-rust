use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use crate::player::components::PlayerDeathEvent;
use super::traits::*;

/// Resource tracking state for Stomp mode.
/// Only kills via stomping count -- projectile kills do not award points.
#[derive(Resource, Debug)]
pub struct StompModeState {
    pub stomps: [u32; MAX_PLAYERS],
    /// Current combo streak per player (resets when touching ground without a stomp).
    pub combo: [u32; MAX_PLAYERS],
    /// Cumulative bonus points earned from combos.
    pub combo_bonus: [u32; MAX_PLAYERS],
}

impl Default for StompModeState {
    fn default() -> Self {
        Self {
            stomps: [0; MAX_PLAYERS],
            combo: [0; MAX_PLAYERS],
            combo_bonus: [0; MAX_PLAYERS],
        }
    }
}

/// Processes PlayerKillEvent -- only stomps score.
pub fn handle_stomp_kills(
    config: Res<GameModeConfig>,
    mut state: ResMut<StompModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut kill_events: EventReader<PlayerKillEvent>,
) {
    if config.mode_type != GameModeType::Stomp {
        return;
    }

    for event in kill_events.read() {
        if event.killer >= MAX_PLAYERS || event.victim >= MAX_PLAYERS {
            continue;
        }

        if event.method != KillMethod::Stomp {
            // Non-stomp kills are ignored for scoring in this mode
            continue;
        }

        state.stomps[event.killer] += 1;
        state.combo[event.killer] += 1;

        // Combo bonus: each consecutive stomp without landing gives +1 extra point
        let combo_count = state.combo[event.killer];
        let bonus = if combo_count > 1 { combo_count - 1 } else { 0 };
        state.combo_bonus[event.killer] += bonus;

        let total_points = 1 + bonus as i32;
        scoreboard.kills[event.killer] += 1;
        scoreboard.deaths[event.victim] += 1;
        scoreboard.score[event.killer] += total_points;
    }
}

/// Reset combo when a player lands on the ground (detected via PlayerDeathEvent for victim,
/// or could be driven by a grounded event in the future). For now, we reset combo on death.
pub fn reset_stomp_combo_on_death(
    config: Res<GameModeConfig>,
    mut state: ResMut<StompModeState>,
    mut death_events: EventReader<PlayerDeathEvent>,
) {
    if config.mode_type != GameModeType::Stomp {
        return;
    }

    for event in death_events.read() {
        if event.player_index < MAX_PLAYERS {
            state.combo[event.player_index] = 0;
        }
    }
}

/// Checks if any player has reached the kill limit via stomps.
pub fn check_stomp_win_condition(
    config: Res<GameModeConfig>,
    state: Res<StompModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::Stomp {
        return;
    }

    for (i, &stomps) in state.stomps.iter().enumerate() {
        if stomps >= config.kill_limit {
            round_over.send(RoundOverEvent {
                winning_player: Some(i),
                winning_team: None,
            });
            return;
        }
    }
}
