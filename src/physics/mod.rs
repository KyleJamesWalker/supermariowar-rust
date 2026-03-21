pub mod collision;
pub mod components;
pub mod systems;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};
use crate::core::GameSystemSet;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        let gameplay_playing = in_state(GameState::Gameplay)
            .and(in_state(GameplayState::Playing));

        app.add_event::<components::CollisionEvent>()
            .add_systems(
                Update,
                (
                    // Gravity → apply velocity → map collision → object collision → warps → platforms → entity collision
                    systems::apply_gravity,
                    systems::apply_player_velocity,
                    systems::map_collision,
                    systems::object_map_collision,
                    systems::warp_zone_detection,
                    systems::moving_platform_system,
                    systems::entity_collision_detection,
                )
                    .chain()
                    .in_set(GameSystemSet::Physics)
                    .run_if(gameplay_playing),
            );
    }
}
