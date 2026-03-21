pub mod classic;
pub mod coins;
pub mod ctf;
pub mod traits;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};

pub struct GameModesPlugin;

impl Plugin for GameModesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<traits::GameModeConfig>()
            .init_resource::<classic::ClassicModeState>()
            .init_resource::<coins::CoinModeState>()
            .init_resource::<ctf::CtfModeState>()
            .add_event::<traits::ScoreEvent>()
            .add_event::<traits::RoundOverEvent>()
            .add_systems(
                Update,
                (
                    classic::handle_classic_score,
                    classic::check_classic_win_condition,
                    coins::handle_coin_score,
                    coins::check_coin_win_condition,
                    ctf::handle_ctf_score,
                    ctf::check_ctf_win_condition,
                )
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayState::Playing)),
            );
    }
}
