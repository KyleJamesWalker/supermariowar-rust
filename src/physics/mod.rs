pub mod collision;
pub mod components;
pub mod systems;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<components::CollisionEvent>()
            .add_systems(
                Update,
                (
                    systems::apply_gravity,
                    systems::map_collision,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayState::Playing)),
            );
    }
}
