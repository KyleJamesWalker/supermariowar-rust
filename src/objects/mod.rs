pub mod blocks;
pub mod components;
pub mod enemies;
pub mod hazards;
pub mod powerups;
pub mod projectiles;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};
use crate::core::GameSystemSet;

pub struct ObjectsPlugin;

impl Plugin for ObjectsPlugin {
    fn build(&self, app: &mut App) {
        let gameplay_playing = in_state(GameState::Gameplay)
            .and(in_state(GameplayState::Playing));

        // Resources
        app.init_resource::<blocks::OnOffSwitchState>();

        // Events
        app.add_event::<components::BlockHitEvent>()
            .add_event::<components::PowerupCollectEvent>()
            .add_event::<components::EnemyDeathEvent>()
            .add_event::<components::ProjectileHitEvent>()
            .add_event::<components::HazardContactEvent>();

        // Block systems
        app.add_systems(
            Update,
            (
                blocks::block_hit_system,
                blocks::block_bounce_system,
                blocks::donut_block_timer_system,
                blocks::flip_block_system,
                blocks::on_off_block_system,
                blocks::block_bump_animation_system,
            )
                .chain()
                .run_if(gameplay_playing.clone()),
        );

        // Powerup systems: spawn -> movement -> collection -> cleanup
        app.add_systems(
            Update,
            (
                powerups::powerup_spawn_system,
                powerups::powerup_movement_system,
                powerups::powerup_collect_system,
                powerups::powerup_despawn_offscreen_system,
            )
                .chain()
                .run_if(gameplay_playing.clone()),
        );

        // Enemy systems: movement -> shell vel -> gravity -> stomp -> stun -> reemerge -> map collision -> touch -> cleanup
        app.add_systems(
            Update,
            (
                enemies::enemy_movement_system,
                enemies::shell_movement_system,
                enemies::enemy_gravity_system,
                enemies::enemy_stomp_system,
                enemies::enemy_stun_recovery_system,
                enemies::shell_reemerge_system,
                enemies::enemy_map_collision,
                enemies::enemy_player_touch_system,
                enemies::enemy_lifetime_system,
                enemies::enemy_despawn_offscreen_system,
            )
                .chain()
                .run_if(gameplay_playing.clone()),
        );

        // Projectile systems — split across GameSystemSets so they interleave
        // correctly with Physics (object_map_collision):
        //   Movement: fire + move projectiles (before Physics resolves map collisions)
        //   Combat:   collision + bounce + bomb (after Physics sets grounded flags)
        //   DeathRespawn: lifetime + offscreen cleanup
        app.add_systems(
            Update,
            (
                projectiles::fire_projectile_system,
                projectiles::projectile_movement_system,
            )
                .chain()
                .in_set(GameSystemSet::Movement)
                .run_if(gameplay_playing.clone()),
        );
        app.add_systems(
            Update,
            (
                projectiles::projectile_collision_system,
                projectiles::projectile_bounce_system,
                projectiles::bomb_fuse_system,
            )
                .chain()
                .in_set(GameSystemSet::Combat)
                .run_if(gameplay_playing.clone()),
        );
        app.add_systems(
            Update,
            (
                projectiles::projectile_lifetime_system,
                projectiles::projectile_despawn_offscreen_system,
            )
                .chain()
                .in_set(GameSystemSet::DeathRespawn)
                .run_if(gameplay_playing.clone()),
        );

        // Hazard systems: thwomp -> cannon -> piranha -> collision -> cycle
        app.add_systems(
            Update,
            (
                hazards::thwomp_system,
                hazards::bullet_cannon_system,
                hazards::piranha_plant_system,
                hazards::hazard_collision_system,
                hazards::hazard_cycle_system,
            )
                .chain()
                .run_if(gameplay_playing),
        );
    }
}
