use bevy::prelude::*;

use super::traits::*;

/// Marker component for a flag entity.
#[derive(Component, Debug)]
pub struct Flag {
    pub team: u8,
    pub carried_by: Option<usize>,
    pub at_base: bool,
    pub base_position: Vec2,
}

/// Resource tracking capture-the-flag mode state.
#[derive(Resource, Debug, Default)]
pub struct CtfModeState {
    pub captures: [u32; 4],
}

/// Checks if any team has captured enough flags.
pub fn check_ctf_win_condition(
    config: Res<GameModeConfig>,
    state: Res<CtfModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }

    for (i, &captures) in state.captures.iter().enumerate() {
        if captures >= config.point_limit {
            round_over.send(RoundOverEvent {
                winning_player: None,
                winning_team: Some(i as u8),
            });
            return;
        }
    }
}

/// Processes flag capture events.
pub fn handle_ctf_score(
    config: Res<GameModeConfig>,
    mut state: ResMut<CtfModeState>,
    mut score_events: EventReader<ScoreEvent>,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }

    for event in score_events.read() {
        if let ScoreReason::FlagCapture = event.reason {
            if event.player_index < 4 {
                state.captures[event.player_index] += 1;
            }
        }
    }
}
