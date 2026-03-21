use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use crate::player::components::Player;
use super::traits::*;

/// Resource tracking state for Classic (frag) deathmatch mode.
#[derive(Resource, Debug)]
pub struct ClassicModeState {
    pub kills: [u32; MAX_PLAYERS],
    pub team_kills: [u32; MAX_PLAYERS],
}

impl Default for ClassicModeState {
    fn default() -> Self {
        Self {
            kills: [0; MAX_PLAYERS],
            team_kills: [0; MAX_PLAYERS],
        }
    }
}

/// Processes PlayerKillEvent in classic mode: increments the killer's kill count
/// unless team kills are disabled and both players share a team.
pub fn handle_classic_kills(
    config: Res<GameModeConfig>,
    mut state: ResMut<ClassicModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut kill_events: EventReader<PlayerKillEvent>,
    players: Query<&Player>,
) {
    if config.mode_type != GameModeType::Classic && config.mode_type != GameModeType::Frag {
        return;
    }

    for event in kill_events.read() {
        if event.killer >= MAX_PLAYERS || event.victim >= MAX_PLAYERS {
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

        state.kills[event.killer] += 1;
        scoreboard.kills[event.killer] += 1;
        scoreboard.deaths[event.victim] += 1;
        scoreboard.score[event.killer] += 1;
    }
}

/// Also listen to legacy ScoreEvent for backward compat.
pub fn handle_classic_score(
    config: Res<GameModeConfig>,
    mut state: ResMut<ClassicModeState>,
    mut score_events: EventReader<ScoreEvent>,
) {
    if config.mode_type != GameModeType::Classic && config.mode_type != GameModeType::Frag {
        return;
    }

    for event in score_events.read() {
        if let ScoreReason::Kill = event.reason {
            if event.player_index < MAX_PLAYERS {
                state.kills[event.player_index] += event.points.max(0) as u32;
            }
        }
    }
}

/// Checks if any player has reached the kill limit.
pub fn check_classic_win_condition(
    config: Res<GameModeConfig>,
    state: Res<ClassicModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::Classic && config.mode_type != GameModeType::Frag {
        return;
    }

    for (i, &kills) in state.kills.iter().enumerate() {
        if kills >= config.kill_limit {
            round_over.send(RoundOverEvent {
                winning_player: Some(i),
                winning_team: None,
            });
            return;
        }
    }
}
