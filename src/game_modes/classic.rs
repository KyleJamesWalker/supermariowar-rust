use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use crate::player::components::{Player, PlayerDeathEvent};
use super::traits::*;

/// Resource tracking state for Classic elimination mode.
/// In the original C++, Classic starts each player with `goal` lives.
/// Each death decrements the victim's lives. When lives reach 0, the player
/// is eliminated. Last team/player standing wins the round.
/// Frag mode (kill-count) is handled separately.
#[derive(Resource, Debug)]
pub struct ClassicModeState {
    /// Remaining lives per player (counts DOWN from kill_limit)
    pub lives: [i32; MAX_PLAYERS],
    /// Whether each player has been eliminated (lives reached 0)
    pub eliminated: [bool; MAX_PLAYERS],
    pub team_kills: [u32; MAX_PLAYERS],
    pub initialized: bool,
}

impl Default for ClassicModeState {
    fn default() -> Self {
        Self {
            lives: [0; MAX_PLAYERS],
            eliminated: [false; MAX_PLAYERS],
            team_kills: [0; MAX_PLAYERS],
            initialized: false,
        }
    }
}

/// Processes kills in Classic mode: decrements the VICTIM's lives.
/// Also handles self-kills (environment deaths) with a life penalty.
/// In Frag mode: increments the killer's score (original behavior).
pub fn handle_classic_kills(
    config: Res<GameModeConfig>,
    mut state: ResMut<ClassicModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut kill_events: EventReader<PlayerKillEvent>,
    mut death_events: EventReader<PlayerDeathEvent>,
    players: Query<&Player>,
) {
    if config.mode_type != GameModeType::Classic && config.mode_type != GameModeType::Frag {
        return;
    }

    // Initialize lives from kill_limit on first run
    if !state.initialized {
        for i in 0..MAX_PLAYERS {
            state.lives[i] = config.kill_limit as i32;
            scoreboard.score[i] = config.kill_limit as i32;
        }
        state.initialized = true;
    }

    if config.mode_type == GameModeType::Classic {
        // Classic: decrement victim's lives on kill
        for event in kill_events.read() {
            if event.victim >= MAX_PLAYERS {
                continue;
            }

            // Check for team kill
            let same_team = players
                .iter()
                .filter(|p| p.index == event.killer || p.index == event.victim)
                .map(|p| p.team)
                .collect::<Vec<_>>();
            let is_team_kill = same_team.len() == 2 && same_team[0] == same_team[1];

            if is_team_kill && !config.team_kill_counts {
                state.team_kills[event.killer] += 1;
                continue;
            }

            // Decrement VICTIM's lives (Classic = lives count down)
            if !state.eliminated[event.victim] {
                state.lives[event.victim] -= 1;
                scoreboard.score[event.victim] = state.lives[event.victim];
                scoreboard.deaths[event.victim] += 1;
                scoreboard.kills[event.killer] += 1;

                if state.lives[event.victim] <= 0 {
                    state.eliminated[event.victim] = true;
                }
            }
        }

        // Handle self-kills (environment deaths with no killer)
        for event in death_events.read() {
            if event.killer_index.is_some() {
                continue; // Already handled above via PlayerKillEvent bridge
            }
            let idx = event.player_index;
            if idx >= MAX_PLAYERS || state.eliminated[idx] {
                continue;
            }
            state.lives[idx] -= 1;
            scoreboard.score[idx] = state.lives[idx];
            scoreboard.deaths[idx] += 1;
            if state.lives[idx] <= 0 {
                state.eliminated[idx] = true;
            }
        }
    } else {
        // Frag mode: increment killer's score (original behavior)
        for event in kill_events.read() {
            if event.killer >= MAX_PLAYERS || event.victim >= MAX_PLAYERS {
                continue;
            }

            let same_team = players
                .iter()
                .filter(|p| p.index == event.killer || p.index == event.victim)
                .map(|p| p.team)
                .collect::<Vec<_>>();
            let is_team_kill = same_team.len() == 2 && same_team[0] == same_team[1];

            if is_team_kill && !config.team_kill_counts {
                // Team kill penalty: decrement killer's score
                scoreboard.score[event.killer] -= 1;
                state.team_kills[event.killer] += 1;
                continue;
            }

            scoreboard.kills[event.killer] += 1;
            scoreboard.deaths[event.victim] += 1;
            scoreboard.score[event.killer] += 1;
        }

        // Handle self-kills in Frag mode: decrement score
        for event in death_events.read() {
            if event.killer_index.is_some() {
                continue;
            }
            let idx = event.player_index;
            if idx >= MAX_PLAYERS {
                continue;
            }
            scoreboard.score[idx] -= 1;
            scoreboard.deaths[idx] += 1;
        }
    }
}

/// Also listen to legacy ScoreEvent for backward compat.
pub fn handle_classic_score(
    config: Res<GameModeConfig>,
    mut _state: ResMut<ClassicModeState>,
    mut score_events: EventReader<ScoreEvent>,
) {
    if config.mode_type != GameModeType::Classic && config.mode_type != GameModeType::Frag {
        return;
    }
    // Drain events to prevent them from accumulating
    for _event in score_events.read() {}
}

/// Checks win condition:
/// Classic: last player standing (all others eliminated) wins.
/// Frag: first player to reach kill_limit wins.
pub fn check_classic_win_condition(
    config: Res<GameModeConfig>,
    state: Res<ClassicModeState>,
    scoreboard: Res<ScoreBoard>,
    players: Query<&Player>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::Classic && config.mode_type != GameModeType::Frag {
        return;
    }

    if config.mode_type == GameModeType::Classic {
        // Classic: last player standing wins
        let active_players: Vec<usize> = players
            .iter()
            .filter(|p| !state.eliminated[p.index])
            .map(|p| p.index)
            .collect();

        if active_players.len() <= 1 {
            let winner = active_players.first().copied();
            round_over.send(RoundOverEvent {
                winning_player: winner,
                winning_team: None,
            });
        }
    } else {
        // Frag: first to kill_limit
        for (i, &score) in scoreboard.score.iter().enumerate() {
            if score >= config.kill_limit as i32 {
                round_over.send(RoundOverEvent {
                    winning_player: Some(i),
                    winning_team: None,
                });
                return;
            }
        }
    }
}
