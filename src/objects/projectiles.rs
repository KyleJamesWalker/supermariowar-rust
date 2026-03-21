use bevy::prelude::*;

use crate::audio::sfx::{play_sfx, SfxId, SfxLibrary};
use crate::core::constants::*;
use crate::map::GameplayEntity;
use crate::objects::components::*;
use crate::objects::enemies::{Enemy, EnemyState};
use crate::physics::collision::aabb_overlap;
use crate::physics::components::{Collider, CollisionLayer};
use crate::player::components::{
    DeathCause, Facing, Invincible, Player, PlayerAction, PlayerDeathEvent, PlayerInput,
    PowerupState, SpawnProtection, Velocity,
};

// ---------------------------------------------------------------------------
// Projectile components
// ---------------------------------------------------------------------------

/// Types of projectiles players can fire.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileType {
    Fireball,
    Hammer,
    Boomerang,
    IceBall,
    Bomb,
    Shell,
    ThrowBlock,
    Feather,
    TailSwipe,
    BulletBill,
}

/// Projectile behavior data.
#[derive(Component, Debug)]
pub struct Projectile {
    pub projectile_type: ProjectileType,
    pub owner: usize,
    pub owner_team: u8,
    pub bounces_remaining: u32,
    pub speed: f32,
}

impl Projectile {
    pub fn fireball(owner: usize, owner_team: u8) -> Self {
        Self {
            projectile_type: ProjectileType::Fireball,
            owner,
            owner_team,
            bounces_remaining: 3,
            speed: 6.0,
        }
    }

    pub fn hammer(owner: usize, owner_team: u8) -> Self {
        Self {
            projectile_type: ProjectileType::Hammer,
            owner,
            owner_team,
            bounces_remaining: 0,
            speed: HAMMER_THROW,
        }
    }

    pub fn boomerang(owner: usize, owner_team: u8) -> Self {
        Self {
            projectile_type: ProjectileType::Boomerang,
            owner,
            owner_team,
            bounces_remaining: 0,
            speed: 5.0,
        }
    }

    pub fn iceball(owner: usize, owner_team: u8) -> Self {
        Self {
            projectile_type: ProjectileType::IceBall,
            owner,
            owner_team,
            bounces_remaining: 3,
            speed: 6.0,
        }
    }

    pub fn bomb(owner: usize, owner_team: u8) -> Self {
        Self {
            projectile_type: ProjectileType::Bomb,
            owner,
            owner_team,
            bounces_remaining: 0,
            speed: 5.0,
        }
    }
}

/// Tracks boomerang return behavior.
#[derive(Component, Debug)]
pub struct BoomerangState {
    pub returning: bool,
    pub distance_traveled: f32,
    pub max_distance: f32,
    pub owner_entity: Entity,
}

/// Tracks bomb fuse timer.
#[derive(Component, Debug)]
pub struct BombFuse {
    pub timer: Timer,
    pub explosion_radius: f32,
}

impl Default for BombFuse {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
            explosion_radius: TILE_SIZE * 2.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Projectile spawning
// ---------------------------------------------------------------------------

/// Spawns a projectile entity at the given position with the specified type.
pub fn spawn_projectile(
    commands: &mut Commands,
    projectile: Projectile,
    x: f32,
    y: f32,
    vel_x: f32,
    vel_y: f32,
) -> Entity {
    let id = commands
        .spawn((
            GameObject {
                object_type: ObjectType::Projectile,
                alive: true,
            },
            projectile,
            ObjectVelocity { x: vel_x, y: vel_y },
            ObjectGrounded(false),
            Collider::new(16.0, 16.0),
            CollisionLayer::Projectile,
            DamageOnContact {
                kill: true,
                owner_player: None,
            },
            InteractsWithMap,
            DespawnOffScreen,
            Lifetime::from_seconds(8.0),
            Transform::from_xyz(x, y, crate::graphics::sprites::z_order::PROJECTILES),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ))
        .id();
    commands.entity(id).insert(GameplayEntity);
    id
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns projectiles when a player with the appropriate powerup presses use_item.
pub fn fire_projectile_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    query: Query<(
        Entity,
        &Player,
        &PlayerInput,
        &PowerupState,
        &Facing,
        &Transform,
        &PlayerAction,
    )>,
) {
    for (p_entity, player, input, powerup, facing, transform, action) in query.iter() {
        if *action == PlayerAction::Dead {
            continue;
        }
        if !input.use_item {
            continue;
        }

        let dir = match facing {
            Facing::Right => 1.0,
            Facing::Left => -1.0,
        };

        let spawn_x = transform.translation.x + dir * PLAYER_WIDTH;
        let spawn_y = transform.translation.y + PLAYER_HEIGHT * 0.5;

        match powerup {
            PowerupState::Fire => {
                play_sfx(&mut commands, &sfx, SfxId::Fireball);
                let proj = Projectile::fireball(player.index, player.team);
                let vel_x = proj.speed * dir;
                let vel_y = -2.0; // Slight downward angle
                spawn_projectile(&mut commands, proj, spawn_x, spawn_y, vel_x, vel_y);
            }
            PowerupState::Hammer => {
                play_sfx(&mut commands, &sfx, SfxId::Fireball);
                let proj = Projectile::hammer(player.index, player.team);
                let vel_x = proj.speed * dir * 0.7;
                let vel_y = HAMMER_THROW; // Thrown upward in parabolic arc (positive = up in Bevy)
                let entity = spawn_projectile(
                    &mut commands,
                    proj,
                    spawn_x,
                    spawn_y,
                    vel_x,
                    vel_y,
                );
                // Hammer needs gravity
                commands.entity(entity).insert(
                    crate::physics::components::GravityAffected { multiplier: 1.0 },
                );
            }
            PowerupState::Boomerang => {
                play_sfx(&mut commands, &sfx, SfxId::Fireball);
                let proj = Projectile::boomerang(player.index, player.team);
                let vel_x = proj.speed * dir;
                let entity = spawn_projectile(
                    &mut commands,
                    proj,
                    spawn_x,
                    spawn_y,
                    vel_x,
                    0.0,
                );
                commands.entity(entity).insert(BoomerangState {
                    returning: false,
                    distance_traveled: 0.0,
                    max_distance: SCREEN_WIDTH * 0.4,
                    owner_entity: p_entity,
                });
            }
            PowerupState::Bomb => {
                play_sfx(&mut commands, &sfx, SfxId::Fireball);
                let proj = Projectile::bomb(player.index, player.team);
                let vel_x = proj.speed * dir * 0.5;
                let vel_y = proj.speed; // Thrown upward (positive = up in Bevy)
                let entity = spawn_projectile(
                    &mut commands,
                    proj,
                    spawn_x,
                    spawn_y,
                    vel_x,
                    vel_y,
                );
                commands.entity(entity).insert(BombFuse::default());
                commands.entity(entity).insert(
                    crate::physics::components::GravityAffected { multiplier: 1.0 },
                );
            }
            _ => {
                // No projectile for other powerup states
            }
        }
    }
}

/// Updates projectile positions based on their type-specific movement rules.
/// Floor/wall collision is handled by `object_map_collision` in the physics
/// plugin. This system applies gravity, velocity, and type-specific behaviors.
pub fn projectile_movement_system(
    mut commands: Commands,
    _time: Res<Time>,
    mut query: Query<(
        Entity,
        &Projectile,
        &mut ObjectVelocity,
        &mut Transform,
        &ObjectGrounded,
        Option<&mut BoomerangState>,
    )>,
    owner_query: Query<&Transform, (With<Player>, Without<Projectile>)>,
) {
    for (entity, proj, mut vel, mut transform, grounded, boomerang) in query.iter_mut() {
        match proj.projectile_type {
            ProjectileType::Fireball | ProjectileType::IceBall => {
                // Gravity (negative = downward in Bevy Y-up)
                vel.y -= GRAVITY;
                if vel.y < -MAX_FALL_SPEED {
                    vel.y = -MAX_FALL_SPEED;
                }

                transform.translation.x += vel.x;
                transform.translation.y += vel.y;

                // Bounce when landing (object_map_collision sets grounded + zeros vel.y)
                if grounded.0 && vel.y == 0.0 {
                    vel.y = FIREBALL_BOUNCE;
                }
            }
            ProjectileType::Hammer => {
                // Hammer arcs: apply gravity manually (GravityAffected doesn't
                // work on ObjectVelocity)
                vel.y -= GRAVITY;
                if vel.y < -MAX_FALL_SPEED {
                    vel.y = -MAX_FALL_SPEED;
                }
                transform.translation.x += vel.x;
                transform.translation.y += vel.y;
            }
            ProjectileType::Boomerang => {
                if let Some(mut boom) = boomerang {
                    boom.distance_traveled += vel.x.abs();

                    if !boom.returning && boom.distance_traveled >= boom.max_distance {
                        boom.returning = true;
                    }

                    if boom.returning {
                        // Curve back toward the owner
                        if let Ok(owner_transform) = owner_query.get(boom.owner_entity) {
                            let diff_x =
                                owner_transform.translation.x - transform.translation.x;
                            let diff_y =
                                owner_transform.translation.y - transform.translation.y;
                            let dist = (diff_x * diff_x + diff_y * diff_y).sqrt().max(1.0);

                            vel.x = (diff_x / dist) * proj.speed * 1.5;
                            vel.y = (diff_y / dist) * proj.speed * 1.5;

                            // Despawn when returning to owner
                            if dist < TILE_SIZE {
                                commands.entity(entity).despawn_recursive();
                            }
                        }
                    }

                    transform.translation.x += vel.x;
                    transform.translation.y += vel.y;
                } else {
                    transform.translation.x += vel.x;
                    transform.translation.y += vel.y;
                }
            }
            ProjectileType::Bomb => {
                // Apply gravity
                vel.y -= GRAVITY;
                if vel.y < -MAX_FALL_SPEED {
                    vel.y = -MAX_FALL_SPEED;
                }
                transform.translation.x += vel.x;
                transform.translation.y += vel.y;

                // Apply friction when grounded (floor snap by object_map_collision)
                if grounded.0 {
                    vel.x *= 0.8;
                }
            }
            ProjectileType::Shell | ProjectileType::ThrowBlock => {
                // Gravity for falling shells/blocks
                vel.y -= GRAVITY;
                if vel.y < -MAX_FALL_SPEED {
                    vel.y = -MAX_FALL_SPEED;
                }
                transform.translation.x += vel.x;
                transform.translation.y += vel.y;
                // Floor collision handled by object_map_collision
            }
            _ => {
                transform.translation.x += vel.x;
                transform.translation.y += vel.y;
            }
        }

        // Screen wrapping (horizontal, Bevy centered coords)
        let half_w = SCREEN_WIDTH / 2.0;
        if transform.translation.x > half_w {
            transform.translation.x -= SCREEN_WIDTH;
        } else if transform.translation.x < -half_w {
            transform.translation.x += SCREEN_WIDTH;
        }
    }
}

/// Checks projectile collisions against players and enemies.
pub fn projectile_collision_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    projectile_query: Query<(
        Entity,
        &Projectile,
        &Collider,
        &Transform,
        &GameObject,
    )>,
    mut player_query: Query<(
        Entity,
        &Player,
        &mut Velocity,
        &mut PlayerAction,
        &Transform,
        &Collider,
        Option<&Invincible>,
        Option<&SpawnProtection>,
    )>,
    mut enemy_query: Query<(
        Entity,
        &mut Enemy,
        &mut GameObject,
        &Collider,
        &Transform,
    ), Without<Projectile>>,
    mut hit_events: EventWriter<ProjectileHitEvent>,
    mut player_death_events: EventWriter<PlayerDeathEvent>,
    mut death_events: EventWriter<EnemyDeathEvent>,
) {
    for (proj_entity, proj, proj_collider, proj_transform, proj_obj) in projectile_query.iter() {
        if !proj_obj.alive {
            continue;
        }

        // Check vs players
        for (p_entity, player, _p_vel, mut p_action, p_transform, p_collider, invincible, spawn_prot) in
            player_query.iter_mut()
        {
            // Don't hit the owner or teammates
            if player.index == proj.owner {
                continue;
            }
            if player.team == proj.owner_team {
                continue;
            }
            if *p_action == PlayerAction::Dead || *p_action == PlayerAction::Spawning {
                continue;
            }

            if !aabb_overlap(
                &proj_transform.translation,
                proj_collider,
                &p_transform.translation,
                p_collider,
            ) {
                continue;
            }

            hit_events.send(ProjectileHitEvent {
                projectile_entity: proj_entity,
                hit_entity: p_entity,
                projectile_owner: proj.owner,
            });

            if invincible.is_some() || spawn_prot.is_some() {
                // Invincible or spawn-protected player destroys the projectile
                commands.entity(proj_entity).despawn_recursive();
                break;
            }

            match proj.projectile_type {
                ProjectileType::IceBall => {
                    // Freeze the player instead of killing
                    play_sfx(&mut commands, &sfx, SfxId::Transform);
                    commands.entity(p_entity).insert(
                        crate::player::components::FrozenState::new(3.0),
                    );
                    commands.entity(proj_entity).despawn_recursive();
                }
                _ => {
                    // Kill the player
                    play_sfx(&mut commands, &sfx, SfxId::PlayerDeath);
                    *p_action = PlayerAction::Dead;
                    player_death_events.send(PlayerDeathEvent {
                        player_entity: p_entity,
                        player_index: player.index,
                        killer_index: Some(proj.owner),
                        cause: DeathCause::Killed { by: proj.owner },
                    });
                    commands.entity(proj_entity).despawn_recursive();
                }
            }
            break;
        }

        // Check vs enemies
        for (e_entity, mut enemy, mut game_obj, e_collider, e_transform) in
            enemy_query.iter_mut()
        {
            if !game_obj.alive || enemy.state == EnemyState::Dead {
                continue;
            }

            if !aabb_overlap(
                &proj_transform.translation,
                proj_collider,
                &e_transform.translation,
                e_collider,
            ) {
                continue;
            }

            hit_events.send(ProjectileHitEvent {
                projectile_entity: proj_entity,
                hit_entity: e_entity,
                projectile_owner: proj.owner,
            });

            match proj.projectile_type {
                ProjectileType::IceBall => {
                    // Freeze the enemy
                    play_sfx(&mut commands, &sfx, SfxId::Transform);
                    enemy.state = EnemyState::Stunned;
                    enemy.kills_on_touch = false;
                    commands.entity(e_entity).insert(
                        crate::objects::enemies::StunTimer {
                            timer: Timer::from_seconds(5.0, TimerMode::Once),
                        },
                    );
                    commands.entity(proj_entity).despawn_recursive();
                }
                _ => {
                    // Kill the enemy
                    play_sfx(&mut commands, &sfx, SfxId::Kick);
                    enemy.state = EnemyState::Dead;
                    game_obj.alive = false;
                    death_events.send(EnemyDeathEvent {
                        enemy_entity: e_entity,
                        killer_player: Some(proj.owner),
                    });
                    commands.entity(e_entity).insert(Lifetime::from_seconds(0.3));
                    commands.entity(proj_entity).despawn_recursive();
                }
            }
            break;
        }
    }
}

/// Handles fireball/iceball bounce counting. When a fireball has exhausted its
/// bounces or hits a wall, it is despawned.
pub fn projectile_bounce_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Projectile,
        &ObjectVelocity,
        &ObjectGrounded,
    )>,
) {
    for (entity, mut proj, _vel, grounded) in query.iter_mut() {
        match proj.projectile_type {
            ProjectileType::Fireball | ProjectileType::IceBall => {
                // Count bounces when hitting ground
                if grounded.0 && proj.bounces_remaining > 0 {
                    proj.bounces_remaining -= 1;
                }
                if proj.bounces_remaining == 0 && grounded.0 {
                    commands.entity(entity).despawn_recursive();
                }
                // Wall bounce handled by object_map_collision (reverses velocity)
            }
            _ => {}
        }
    }
}

/// Ticks bomb fuse timers and triggers explosions.
pub fn bomb_fuse_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    time: Res<Time>,
    mut query: Query<(Entity, &mut BombFuse, &Projectile, &Transform, &Collider)>,
    mut player_query: Query<(
        Entity,
        &Player,
        &mut PlayerAction,
        &mut Velocity,
        &Transform,
        &Collider,
        Option<&Invincible>,
    ), With<Player>>,
    mut bomb_death_events: EventWriter<PlayerDeathEvent>,
    mut enemy_query: Query<(
        Entity,
        &mut Enemy,
        &mut GameObject,
        &Collider,
        &Transform,
    ), Without<Player>>,
    mut death_events: EventWriter<EnemyDeathEvent>,
) {
    for (bomb_entity, mut fuse, proj, bomb_transform, _bomb_collider) in query.iter_mut() {
        if !fuse.timer.tick(time.delta()).just_finished() {
            continue;
        }

        play_sfx(&mut commands, &sfx, SfxId::BlockBreak);

        let blast_collider = Collider::new(
            fuse.explosion_radius * 2.0,
            fuse.explosion_radius * 2.0,
        ).with_offset(-fuse.explosion_radius, -fuse.explosion_radius);

        // Damage players in blast radius (except owner)
        for (p_entity, player, mut p_action, mut p_vel, p_transform, p_collider, invincible) in
            player_query.iter_mut()
        {
            if *p_action == PlayerAction::Dead {
                continue;
            }
            if invincible.is_some() {
                continue;
            }

            if aabb_overlap(
                &bomb_transform.translation,
                &blast_collider,
                &p_transform.translation,
                p_collider,
            ) {
                *p_action = PlayerAction::Dead;
                // Pushback away from explosion
                let diff_x = p_transform.translation.x - bomb_transform.translation.x;
                p_vel.x = if diff_x > 0.0 { VEL_PUSHBACK } else { -VEL_PUSHBACK };
                bomb_death_events.send(PlayerDeathEvent {
                    player_entity: p_entity,
                    player_index: player.index,
                    killer_index: Some(proj.owner),
                    cause: DeathCause::Killed { by: proj.owner },
                });
            }
        }

        // Damage enemies in blast radius
        for (e_entity, mut enemy, mut game_obj, e_collider, e_transform) in
            enemy_query.iter_mut()
        {
            if !game_obj.alive || enemy.state == EnemyState::Dead {
                continue;
            }

            if aabb_overlap(
                &bomb_transform.translation,
                &blast_collider,
                &e_transform.translation,
                e_collider,
            ) {
                enemy.state = EnemyState::Dead;
                game_obj.alive = false;
                death_events.send(EnemyDeathEvent {
                    enemy_entity: e_entity,
                    killer_player: Some(proj.owner),
                });
                commands.entity(e_entity).insert(Lifetime::from_seconds(0.3));
            }
        }

        commands.entity(bomb_entity).despawn_recursive();
    }
}

/// Despawns projectiles when their lifetime expires.
pub fn projectile_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Lifetime), With<Projectile>>,
) {
    for (entity, mut lifetime) in query.iter_mut() {
        if lifetime.timer.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Despawns projectiles that fall off screen.
pub fn projectile_despawn_offscreen_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (With<Projectile>, With<DespawnOffScreen>)>,
) {
    for (entity, transform) in query.iter() {
        if transform.translation.y < -(SCREEN_HEIGHT / 2.0 + TILE_SIZE * 2.0)
            || transform.translation.y > SCREEN_HEIGHT / 2.0 + TILE_SIZE * 4.0
        {
            commands.entity(entity).despawn_recursive();
        }
    }
}
