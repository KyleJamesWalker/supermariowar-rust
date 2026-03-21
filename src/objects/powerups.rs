use bevy::prelude::*;

use crate::audio::sfx::{play_sfx, SfxId, SfxLibrary, SfxVolume};
use crate::core::constants::*;
use crate::objects::components::*;
use crate::physics::collision::aabb_overlap_wrapped;
use crate::map::GameplayEntity;
use crate::physics::components::{Collider, CollisionLayer, GravityAffected};
use crate::player::components::{
    CapeState, DeathCause, Invincible, Player, PlayerAction, PlayerDeathEvent, PowerupState,
    ShieldState, TailState,
};

// ---------------------------------------------------------------------------
// Powerup components
// ---------------------------------------------------------------------------

/// All powerup item types available in the game.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerupType {
    Mushroom,
    Fire,
    Hammer,
    Boomerang,
    Feather,
    Leaf,
    Bomb,
    Star,
    Poison,
    OneUp,
    TwoUp,
    ThreeUp,
    FiveUp,
    Clock,
    BulletBill,
    Podoboo,
    BobOmb,
    Tanooki,
    MysteryMushroom,
    ExtraGuy,
    Jail,
    Coin,
    ShieldSoft,
    ShieldHard,
    SlowDown,
    IceWand,
}

/// Marker for a spawned powerup entity in the world.
#[derive(Component, Debug)]
pub struct SpawnedPowerup {
    pub powerup_type: PowerupType,
    pub spawning: bool,
    pub spawn_timer: f32,
    pub spawn_rise_target: f32,
}

impl SpawnedPowerup {
    pub fn new(powerup_type: PowerupType) -> Self {
        Self {
            powerup_type,
            spawning: true,
            spawn_timer: 0.0,
            spawn_rise_target: TILE_SIZE,
        }
    }
}

/// Physics behavior categories for powerups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerupPhysics {
    /// Slides along the ground, bounces off walls (mushrooms, poison).
    Sliding,
    /// Floats down slowly (feathers).
    Floating,
    /// Bounces along the ground (stars).
    Bouncing,
    /// Stays in place (coins, static items).
    Static,
}

fn powerup_physics(pt: PowerupType) -> PowerupPhysics {
    match pt {
        PowerupType::Mushroom
        | PowerupType::Poison
        | PowerupType::OneUp
        | PowerupType::TwoUp
        | PowerupType::ThreeUp
        | PowerupType::FiveUp
        | PowerupType::SlowDown => PowerupPhysics::Sliding,

        PowerupType::Feather | PowerupType::Leaf => PowerupPhysics::Floating,

        PowerupType::Star => PowerupPhysics::Bouncing,

        _ => PowerupPhysics::Static,
    }
}

// ---------------------------------------------------------------------------
// Powerup spawning
// ---------------------------------------------------------------------------

/// Spawns a powerup entity at the given world position.
pub fn spawn_powerup(commands: &mut Commands, powerup_type: PowerupType, x: f32, y: f32) -> Entity {
    let gravity_mult = match powerup_physics(powerup_type) {
        PowerupPhysics::Sliding => 1.0,
        PowerupPhysics::Floating => 0.15,
        PowerupPhysics::Bouncing => 1.0,
        PowerupPhysics::Static => 0.0,
    };

    let initial_vx = match powerup_physics(powerup_type) {
        PowerupPhysics::Sliding => 2.0,
        PowerupPhysics::Bouncing => 2.0,
        _ => 0.0,
    };

    let mut entity = commands.spawn((
        GameObject {
            object_type: ObjectType::Powerup,
            alive: true,
        },
        SpawnedPowerup::new(powerup_type),
        powerup_type,
        ObjectVelocity {
            x: initial_vx,
            y: 0.0,
        },
        ObjectGrounded(false),
        Collectible,
        Collider::new(TILE_SIZE, TILE_SIZE),
        CollisionLayer::Powerup,
        InteractsWithMap,
        DespawnOffScreen,
        Transform::from_xyz(x, y, crate::graphics::sprites::z_order::OBJECTS + 3.0),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));
    entity.insert(GameplayEntity);

    if gravity_mult > 0.0 {
        entity.insert(GravityAffected {
            multiplier: gravity_mult,
        });
    }

    entity.id()
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Animates a powerup emerging from its block (rising upward for a short duration).
pub fn powerup_spawn_system(
    time: Res<Time>,
    mut query: Query<(&mut SpawnedPowerup, &mut Transform, &mut ObjectVelocity)>,
) {
    let dt = time.delta_secs();

    for (mut spawned, mut transform, mut vel) in query.iter_mut() {
        if !spawned.spawning {
            continue;
        }

        spawned.spawn_timer += dt * 60.0;

        // Rise up animation: move upward until one tile above spawn point
        let rise_speed = 1.0;
        transform.translation.y += rise_speed;
        spawned.spawn_rise_target -= rise_speed;

        if spawned.spawn_rise_target <= 0.0 {
            spawned.spawning = false;
            // Start normal movement after spawn animation
            vel.y = 0.0;
        } else {
            // Suppress velocity during spawn
            vel.x = 0.0;
            vel.y = 0.0;
        }
    }
}

/// Moves powerups: applies gravity and velocity. Floor/wall collision is
/// handled by `object_map_collision` in the physics plugin. This system only
/// handles type-specific behaviors (floating drift, bounce on landing).
pub fn powerup_movement_system(
    time: Res<Time>,
    mut query: Query<(
        &SpawnedPowerup,
        &mut ObjectVelocity,
        &mut Transform,
        &ObjectGrounded,
    )>,
) {
    let dt = time.delta_secs();

    for (spawned, mut vel, mut transform, grounded) in query.iter_mut() {
        if spawned.spawning {
            continue;
        }

        let physics = powerup_physics(spawned.powerup_type);

        match physics {
            PowerupPhysics::Sliding => {
                // Gravity + movement
                vel.y -= GRAVITY;
                if vel.y < -MAX_FALL_SPEED {
                    vel.y = -MAX_FALL_SPEED;
                }
                transform.translation.x += vel.x;
                transform.translation.y += vel.y;
                // Wall bounce and floor snap handled by object_map_collision

                // Screen wrapping (horizontal, Bevy centered coords)
                let half_w = SCREEN_WIDTH / 2.0;
                if transform.translation.x > half_w {
                    transform.translation.x -= SCREEN_WIDTH;
                } else if transform.translation.x < -half_w {
                    transform.translation.x += SCREEN_WIDTH;
                }
            }
            PowerupPhysics::Floating => {
                // Gentle float downward with sine-wave horizontal drift
                vel.y -= GRAVITY * 0.15;
                if vel.y < -2.0 {
                    vel.y = -2.0;
                }
                let drift = (time.elapsed_secs() * 3.0).sin() * 0.5;
                transform.translation.x += drift * dt * 60.0;
                transform.translation.y += vel.y;

                // Screen wrapping
                let half_w = SCREEN_WIDTH / 2.0;
                if transform.translation.x > half_w {
                    transform.translation.x -= SCREEN_WIDTH;
                } else if transform.translation.x < -half_w {
                    transform.translation.x += SCREEN_WIDTH;
                }
            }
            PowerupPhysics::Bouncing => {
                // Gravity + movement
                vel.y -= GRAVITY;
                if vel.y < -MAX_FALL_SPEED {
                    vel.y = -MAX_FALL_SPEED;
                }
                transform.translation.x += vel.x;
                transform.translation.y += vel.y;

                // Bounce on landing (object_map_collision sets grounded + zeros vel.y)
                if grounded.0 && vel.y == 0.0 {
                    vel.y = VEL_POWERUP_BOUNCE;
                }

                // Screen wrapping
                let half_w = SCREEN_WIDTH / 2.0;
                if transform.translation.x > half_w {
                    transform.translation.x -= SCREEN_WIDTH;
                } else if transform.translation.x < -half_w {
                    transform.translation.x += SCREEN_WIDTH;
                }
            }
            PowerupPhysics::Static => {
                // No movement
            }
        }
    }
}

/// Detects when a player overlaps a powerup and applies the corresponding effect.
pub fn powerup_collect_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    sfx_vol: Res<SfxVolume>,
    powerup_query: Query<(
        Entity,
        &SpawnedPowerup,
        &Collider,
        &Transform,
        &GameObject,
    )>,
    mut player_query: Query<(
        Entity,
        &mut Player,
        &mut PowerupState,
        &mut ShieldState,
        &PlayerAction,
        &Transform,
        &Collider,
    )>,
    mut collect_events: EventWriter<PowerupCollectEvent>,
    mut death_events: EventWriter<PlayerDeathEvent>,
) {
    for (pu_entity, spawned, pu_collider, pu_transform, game_obj) in powerup_query.iter() {
        if !game_obj.alive || spawned.spawning {
            continue;
        }

        for (p_entity, mut player, mut powerup_state, mut shield_state, action, p_transform, p_collider) in
            player_query.iter_mut()
        {
            if *action == PlayerAction::Dead {
                continue;
            }

            if !aabb_overlap_wrapped(
                &p_transform.translation,
                p_collider,
                &pu_transform.translation,
                pu_collider,
            ) {
                continue;
            }

            // Collect the powerup
            collect_events.send(PowerupCollectEvent {
                powerup_entity: pu_entity,
                player_entity: p_entity,
                player_index: player.index,
            });

            match spawned.powerup_type {
                PowerupType::Mushroom => {
                    // Mushroom is a generic powerup in the original C++, NOT extra life.
                    // ExtraGuy/OneUp is the extra-life powerup.
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                }
                PowerupType::Fire => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *powerup_state = PowerupState::Fire;
                }
                PowerupType::Hammer => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *powerup_state = PowerupState::Hammer;
                }
                PowerupType::Boomerang => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *powerup_state = PowerupState::Boomerang;
                }
                PowerupType::IceWand => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *powerup_state = PowerupState::Ice;
                }
                PowerupType::Star => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Invincible);
                    *powerup_state = PowerupState::Star;
                    commands.entity(p_entity).insert(Invincible {
                        timer: Timer::from_seconds(10.0, TimerMode::Once),
                    });
                }
                PowerupType::Coin => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Coin);
                    player.score += 1;
                }
                PowerupType::ShieldSoft => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *shield_state = ShieldState::Soft;
                }
                PowerupType::ShieldHard => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *shield_state = ShieldState::Hard;
                }
                PowerupType::Poison => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PlayerDeath);
                    // Kill the player
                    commands.entity(p_entity).insert(PlayerAction::Dead);
                    death_events.send(PlayerDeathEvent {
                        player_entity: p_entity,
                        player_index: player.index,
                        killer_index: None,
                        cause: DeathCause::Environment,
                    });
                }
                PowerupType::OneUp => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::ExtraLife);
                    player.lives += 1;
                }
                PowerupType::TwoUp => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::ExtraLife);
                    player.lives += 2;
                }
                PowerupType::ThreeUp => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::ExtraLife);
                    player.lives += 3;
                }
                PowerupType::FiveUp => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::ExtraLife);
                    player.lives += 5;
                }
                PowerupType::Feather => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *powerup_state = PowerupState::Feather;
                }
                PowerupType::Leaf => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *powerup_state = PowerupState::Leaf;
                }
                PowerupType::Bomb | PowerupType::BobOmb => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *powerup_state = PowerupState::Bomb;
                }
                PowerupType::Jail => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                    *powerup_state = PowerupState::Jail;
                }
                PowerupType::ExtraGuy => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::ExtraLife);
                    player.lives += 1;
                }
                PowerupType::Clock
                | PowerupType::BulletBill
                | PowerupType::Podoboo
                | PowerupType::Tanooki
                | PowerupType::MysteryMushroom
                | PowerupType::SlowDown => {
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PowerupCollect);
                }
            }

            // Only strip ability state when collecting a WEAPON powerup that replaces it.
            // Utility pickups (coins, shields, extra life, clock, etc.) should NOT
            // remove existing combat abilities like cape, tail, or invincibility.
            let is_weapon_powerup = matches!(
                spawned.powerup_type,
                PowerupType::Fire
                    | PowerupType::Hammer
                    | PowerupType::Boomerang
                    | PowerupType::IceWand
                    | PowerupType::Feather
                    | PowerupType::Leaf
                    | PowerupType::Bomb
                    | PowerupType::BobOmb
            );
            if is_weapon_powerup {
                if spawned.powerup_type != PowerupType::Feather {
                    commands.entity(p_entity).remove::<CapeState>();
                }
                if spawned.powerup_type != PowerupType::Leaf {
                    commands.entity(p_entity).remove::<TailState>();
                }
            }
            // Star invincibility is NEVER removed by collecting other powerups
            // (it expires on its own timer)

            // Despawn the collected powerup
            commands.entity(pu_entity).despawn_recursive();

            // Only one player can collect a powerup per frame
            break;
        }
    }
}

/// Removes powerup entities that have fallen off the bottom of the screen.
pub fn powerup_despawn_offscreen_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (With<SpawnedPowerup>, With<DespawnOffScreen>)>,
) {
    for (entity, transform) in query.iter() {
        if transform.translation.y < -(SCREEN_HEIGHT / 2.0 + TILE_SIZE * 2.0)
            || transform.translation.y > SCREEN_HEIGHT / 2.0 + TILE_SIZE * 4.0
        {
            commands.entity(entity).despawn_recursive();
        }
    }
}
