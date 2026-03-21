use bevy::prelude::*;

use crate::player::components::Player;
use super::traits::*;

/// Resource tracking state for Classic (frag) deathmatch mode.
#[derive(Resource, Debug, Default)]
pub struct ClassicModeState {
    pub kills: [u32; 4],
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

/// Processes kill events in classic mode.
pub fn handle_classic_score(
    config: Res<GameModeConfig>,
    mut state: ResMut<ClassicModeState>,
    mut score_events: EventReader<ScoreEvent>,
) {
    if config.mode_type != GameModeType::Classic && config.mode_type != GameModeType::Frag {
        return;
    }

    for event in score_events.read() {
        if event.player_index < 4 {
            state.kills[event.player_index] += event.points as u32;
        }
    }
}
