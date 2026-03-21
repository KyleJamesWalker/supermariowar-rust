pub mod components;
pub mod input;
pub mod systems;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<input::InputMappings>()
            .init_resource::<input::GamepadAssignments>()
            .add_event::<components::PlayerDeathEvent>()
            .add_event::<components::PlayerStompEvent>()
            .add_event::<components::PlayerRespawnEvent>()
            .add_event::<components::PauseEvent>()
            .add_systems(
                Update,
                (
                    // Phase 1: Input
                    input::read_player_input,
                    // Phase 2: State timers
                    (
                        systems::frozen_state_system,
                        systems::invincibility_system,
                        systems::player_spawn_protection_system,
                    ),
                    // Phase 3: Movement
                    (
                        systems::player_movement,
                        systems::player_jump,
                    ),
                    // Phase 4: Apply velocity
                    systems::apply_player_velocity,
                    // Phase 5: Combat / interaction
                    (
                        systems::attack_system,
                        systems::powerup_use_system,
                    ),
                    // Phase 6: Death / respawn / warp
                    (
                        systems::vertical_death_system,
                        systems::player_death_system,
                        systems::player_burnup_system,
                        systems::player_respawn_system,
                        systems::warp_system,
                    ),
                    // Phase 7: Player-to-player collision
                    systems::player_stomp_system,
                    systems::stomp_bounce_system,
                    // Phase 8: Stats
                    (
                        systems::update_stats_on_stomp,
                        systems::update_stats_on_death,
                    ),
                )
                    .chain()
                    .run_if(in_state(GameState::Gameplay))
                    .run_if(in_state(GameplayState::Playing)),
            );
    }
}
