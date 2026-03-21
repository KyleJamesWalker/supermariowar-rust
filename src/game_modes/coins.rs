use bevy::prelude::*;

use super::traits::*;

/// Resource tracking coin collection mode state.
#[derive(Resource, Debug, Default)]
pub struct CoinModeState {
    pub coins: [u32; 4],
}

/// Checks if any player has collected enough coins.
pub fn check_coin_win_condition(
    config: Res<GameModeConfig>,
    state: Res<CoinModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::CoinCollection {
        return;
    }

    for (i, &coins) in state.coins.iter().enumerate() {
        if coins >= config.point_limit {
            round_over.send(RoundOverEvent {
                winning_player: Some(i),
                winning_team: None,
            });
            return;
        }
    }
}

/// Processes coin collection events.
pub fn handle_coin_score(
    config: Res<GameModeConfig>,
    mut state: ResMut<CoinModeState>,
    mut score_events: EventReader<ScoreEvent>,
) {
    if config.mode_type != GameModeType::CoinCollection {
        return;
    }

    for event in score_events.read() {
        if let ScoreReason::CoinCollect = event.reason {
            if event.player_index < 4 {
                state.coins[event.player_index] += event.points as u32;
            }
        }
    }
}
