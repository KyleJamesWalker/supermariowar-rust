pub mod components;
pub mod input;
pub mod systems;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<input::InputMappings>()
            .add_systems(
                Update,
                (
                    input::read_player_input,
                    systems::player_movement,
                    systems::player_jump,
                    systems::apply_player_velocity,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayState::Playing)),
            );
    }
}
