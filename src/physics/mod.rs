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
                    // Phase 1: Gravity
                    systems::apply_gravity,
                    // Phase 2: Tile-based collision detection
                    systems::map_collision,
                    // Phase 3: Warp zone detection
                    systems::warp_zone_detection,
                    // Phase 4: Moving platform ride-along
                    systems::moving_platform_system,
                    // Phase 5: Entity-to-entity collision
                    systems::entity_collision_detection,
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayState::Playing)),
            );
    }
}
