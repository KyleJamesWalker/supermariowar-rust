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
                    // Platforms first (so rider velocity includes platform delta) → gravity → velocity → collisions → warps
                    systems::moving_platform_system,
                    systems::apply_gravity,
                    systems::apply_player_velocity,
                    systems::map_collision,
                    systems::object_map_collision,
                    systems::warp_zone_detection,
                    systems::entity_collision_detection,
                    systems::player_pushback_system,
                )
                    .chain()
                    .in_set(GameSystemSet::Physics)
                    .run_if(gameplay_playing),
            );
    }
}
