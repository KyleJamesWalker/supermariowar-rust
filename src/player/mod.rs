pub mod components;
pub mod input;
pub mod systems;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};
use crate::core::GameSystemSet;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        let gameplay_playing = in_state(GameState::Gameplay)
            .and(in_state(GameplayState::Playing));

        app.init_resource::<input::InputMappings>()
            .init_resource::<input::GamepadAssignments>()
            .add_event::<components::PlayerDeathEvent>()
            .add_event::<components::PlayerStompEvent>()
            .add_event::<components::PlayerRespawnEvent>()
            .add_event::<components::PauseEvent>()
            // Input phase — must run during both Playing and Paused so the
            // player can unpause.  AI systems have their own Playing-only guard.
            .add_systems(Update, input::gamepad_connection_system)
            .add_systems(
                Update,
                input::read_player_input
                    .in_set(GameSystemSet::Input)
                    .run_if(in_state(GameState::Gameplay)),
            )
            // Movement phase: timers, movement, jump
            .add_systems(
                Update,
                (
                    systems::frozen_state_system,
                    systems::invincibility_system,
                    systems::player_spawn_protection_system,
                    systems::player_movement,
                    systems::player_jump,
                )
                    .chain()
                    .in_set(GameSystemSet::Movement)
                    .run_if(gameplay_playing.clone()),
            )
            // Combat phase: stomp detection, attacks, powerup use
            .add_systems(
                Update,
                (
                    systems::player_stomp_system,
                    systems::stomp_bounce_system,
                    systems::frozen_shatter_system,
                    systems::attack_system,
                    systems::attack_damage_system,
                    systems::fire_cooldown_tick_system,
                    systems::throw_carried_object_system,
                    systems::carried_object_sync_system,
                )
                    .chain()
                    .in_set(GameSystemSet::Combat)
                    .run_if(gameplay_playing.clone()),
            )
            // Death / respawn / warp phase
            .add_systems(
                Update,
                (
                    systems::vertical_death_system,
                    systems::player_death_system,
                    systems::player_burnup_system,
                    systems::player_respawn_system,
                    systems::warp_system,
                )
                    .chain()
                    .in_set(GameSystemSet::DeathRespawn)
                    .run_if(gameplay_playing.clone()),
            )
            // Stats phase
            .add_systems(
                Update,
                (
                    systems::update_stats_on_stomp,
                    systems::update_stats_on_death,
                )
                    .in_set(GameSystemSet::Scoring)
                    .run_if(gameplay_playing.clone()),
            )
            // Snapshot previous transform at end of frame for next-frame collision checks
            .add_systems(
                PostUpdate,
                systems::update_previous_transform
                    .run_if(gameplay_playing),
            );
    }
}
