use bevy::prelude::*;

use crate::audio::sfx::{play_sfx, SfxId, SfxLibrary};
use crate::core::constants::*;
use crate::objects::components::*;
use crate::physics::collision::aabb_overlap;
use crate::map::GameplayEntity;
use crate::physics::components::{Collider, CollisionLayer, GravityAffected};
use crate::player::components::{
    DeathCause, Invincible, Player, PlayerAction, PlayerDeathEvent, Velocity,
};

// ---------------------------------------------------------------------------
// Enemy components
// ---------------------------------------------------------------------------

/// Types of walking/moving enemies.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyType {
    Goomba,
    Koopa,
    KoopaRed,
    Spiny,
    BuzzyBeetle,
    CheepCheep,
    Paragoomba,
    Parakoopa,
    BulletBill,
    Podoboo,
}

/// Enemy state machine.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EnemyState {
    #[default]
    Walking,
    Stunned,
    InShell,
    ShellMoving,
    Dead,
    Falling,
}

/// Enemy behavior and stats.
#[derive(Component, Debug)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub state: EnemyState,
    pub stompable: bool,
    pub kills_on_touch: bool,
    pub speed: f32,
}

impl Enemy {
    pub fn goomba() -> Self {
        Self {
            enemy_type: EnemyType::Goomba,
            state: EnemyState::Walking,
            stompable: true,
            kills_on_touch: true,
            speed: 1.5,
        }
    }

    pub fn koopa() -> Self {
        Self {
            enemy_type: EnemyType::Koopa,
            state: EnemyState::Walking,
            stompable: true,
            kills_on_touch: true,
            speed: 1.5,
        }
    }

    pub fn koopa_red() -> Self {
        Self {
            enemy_type: EnemyType::KoopaRed,
            state: EnemyState::Walking,
            stompable: true,
            kills_on_touch: true,
            speed: 1.2,
        }
    }

    pub fn spiny() -> Self {
        Self {
            enemy_type: EnemyType::Spiny,
            state: EnemyState::Walking,
            stompable: false,
            kills_on_touch: true,
            speed: 1.2,
        }
    }

    pub fn buzzy_beetle() -> Self {
        Self {
            enemy_type: EnemyType::BuzzyBeetle,
            state: EnemyState::Walking,
            stompable: true,
            kills_on_touch: true,
            speed: 1.0,
        }
    }
}

/// Direction the enemy is currently moving.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EnemyFacing {
    #[default]
    Left,
    Right,
}

/// Stun timer for enemies that have been temporarily stunned.
#[derive(Component, Debug)]
pub struct StunTimer {
    pub timer: Timer,
}

/// Shell movement speed when kicked.
#[derive(Component, Debug)]
pub struct ShellSpeed {
    pub speed: f32,
}

impl Default for ShellSpeed {
    fn default() -> Self {
        Self { speed: 7.0 }
    }
}

// ---------------------------------------------------------------------------
// Enemy spawning
// ---------------------------------------------------------------------------

/// Spawns an enemy entity at the given world position.
pub fn spawn_enemy(commands: &mut Commands, enemy: Enemy, x: f32, y: f32) -> Entity {
    let entity = commands
        .spawn((
            GameObject {
                object_type: ObjectType::Enemy,
                alive: true,
            },
            enemy,
            EnemyFacing::Left,
            ObjectVelocity { x: 0.0, y: 0.0 },
            ObjectGrounded(false),
            Collider::new(TILE_SIZE, TILE_SIZE),
            CollisionLayer::Enemy,
            GravityAffected::default(),
            InteractsWithMap,
            DespawnOffScreen,
            DamageOnContact {
                kill: true,
                owner_player: None,
            },
            GameplayEntity,
            Transform::from_xyz(x, y, crate::graphics::sprites::z_order::OBJECTS + 2.0),
        ))
        .id();

    commands.entity(entity).insert((
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

    entity
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Moves walking enemies back and forth. Green koopas walk off edges; red koopas
/// and other types turn at ledge edges.
pub fn enemy_movement_system(
    mut query: Query<(
        &mut Enemy,
        &mut EnemyFacing,
        &mut ObjectVelocity,
        &ObjectGrounded,
        &Transform,
    )>,
) {
    for (enemy, mut facing, mut vel, grounded, transform) in query.iter_mut() {
        match enemy.state {
            EnemyState::Walking => {
                let direction = match *facing {
                    EnemyFacing::Left => -1.0,
                    EnemyFacing::Right => 1.0,
                };
                vel.x = enemy.speed * direction;

                // Red koopas turn at edges when on the ground
                // This is a simplified edge detection using screen boundaries
                if enemy.enemy_type == EnemyType::KoopaRed && grounded.0 {
                    let left_edge = -(SCREEN_WIDTH / 2.0) + TILE_SIZE;
                    let right_edge = SCREEN_WIDTH / 2.0 - TILE_SIZE * 2.0;
                    if transform.translation.x <= left_edge {
                        *facing = EnemyFacing::Right;
                    } else if transform.translation.x >= right_edge {
                        *facing = EnemyFacing::Left;
                    }
                }
            }
            EnemyState::InShell | EnemyState::Stunned => {
                // Stationary
                vel.x = 0.0;
            }
            EnemyState::ShellMoving => {
                // Shell speed handled in shell_movement_system
            }
            EnemyState::Dead | EnemyState::Falling => {
                // No horizontal movement for dead/falling enemies
                vel.x = 0.0;
            }
        }
    }
}

/// Applies gravity to enemies, making them fall when not on solid ground.
pub fn enemy_gravity_system(
    mut query: Query<(&mut ObjectVelocity, &mut Transform), With<Enemy>>,
) {
    for (mut vel, mut transform) in query.iter_mut() {
        // Apply gravity (negative = downward in Bevy Y-up)
        vel.y -= GRAVITY;
        if vel.y < -MAX_FALL_SPEED {
            vel.y = -MAX_FALL_SPEED;
        }

        // Apply velocity
        transform.translation.x += vel.x;
        transform.translation.y += vel.y;

        // Screen wrapping (horizontal, centered coords)
        // Floor/wall collision handled by object_map_collision in physics plugin
        let wrap_right = SCREEN_WIDTH / 2.0 + TILE_SIZE;
        let wrap_left = -(SCREEN_WIDTH / 2.0 + TILE_SIZE);
        if transform.translation.x > wrap_right {
            transform.translation.x = wrap_left;
        } else if transform.translation.x < wrap_left {
            transform.translation.x = wrap_right;
        }
    }
}

/// Detects when a player lands on top of an enemy (stomp). Handles different
/// enemy types: goombas die, koopas enter shell state, spinies hurt the player.
pub fn enemy_stomp_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    mut enemy_query: Query<(
        Entity,
        &mut Enemy,
        &Collider,
        &Transform,
        &mut GameObject,
    )>,
    mut player_query: Query<(
        Entity,
        &Player,
        &mut Velocity,
        &Transform,
        &Collider,
        &PlayerAction,
        Option<&Invincible>,
    )>,
    mut death_events: EventWriter<EnemyDeathEvent>,
    mut player_death_events: EventWriter<PlayerDeathEvent>,
) {
    for (p_entity, player, mut p_vel, p_transform, p_collider, p_action, invincible) in
        player_query.iter_mut()
    {
        if *p_action == PlayerAction::Dead {
            continue;
        }

        // Player must be falling to stomp (negative Y = falling in Bevy's Y-up system)
        if p_vel.y >= 0.0 {
            continue; // Skip if player is NOT falling
        }

        for (e_entity, mut enemy, e_collider, e_transform, mut game_obj) in
            enemy_query.iter_mut()
        {
            if !game_obj.alive {
                continue;
            }
            if enemy.state == EnemyState::Dead {
                continue;
            }

            if !aabb_overlap(
                &p_transform.translation,
                p_collider,
                &e_transform.translation,
                e_collider,
            ) {
                continue;
            }

            // Check that the player is above the enemy (stomp from top).
            // In Bevy Y-up: visual_bottom = pos + offset_y, visual_top = bottom + height
            let player_visual_bottom = p_transform.translation.y + p_collider.offset_y;
            let enemy_visual_mid =
                e_transform.translation.y + e_collider.offset_y + e_collider.height * 0.5;

            if player_visual_bottom < enemy_visual_mid {
                // Player's feet are below the enemy midpoint -- not a valid stomp
                continue;
            }

            // Invincible players kill any enemy on contact
            if invincible.is_some() {
                play_sfx(&mut commands, &sfx, SfxId::Kick);
                enemy.state = EnemyState::Dead;
                game_obj.alive = false;
                death_events.send(EnemyDeathEvent {
                    enemy_entity: e_entity,
                    killer_player: Some(player.index),
                });
                p_vel.y = VEL_ENEMY_BOUNCE;
                commands.entity(e_entity).despawn_recursive();
                continue;
            }

            if !enemy.stompable {
                // Spiny and similar -- hurt the player instead
                commands.entity(p_entity).insert(PlayerAction::Dead);
                player_death_events.send(PlayerDeathEvent {
                    player_entity: p_entity,
                    player_index: player.index,
                    killer_index: None,
                    cause: DeathCause::Environment,
                });
                continue;
            }

            match enemy.enemy_type {
                EnemyType::Goomba | EnemyType::Paragoomba => {
                    play_sfx(&mut commands, &sfx, SfxId::Stomp);
                    enemy.state = EnemyState::Dead;
                    game_obj.alive = false;
                    death_events.send(EnemyDeathEvent {
                        enemy_entity: e_entity,
                        killer_player: Some(player.index),
                    });
                    // Bounce player upward
                    p_vel.y = VEL_ENEMY_BOUNCE;
                    // Despawn after brief squish animation (using lifetime)
                    commands.entity(e_entity).insert(Lifetime::from_seconds(0.3));
                }
                EnemyType::Koopa
                | EnemyType::KoopaRed
                | EnemyType::BuzzyBeetle
                | EnemyType::Parakoopa => {
                    if enemy.state == EnemyState::InShell {
                        // Kick the shell
                        play_sfx(&mut commands, &sfx, SfxId::Kick);
                        enemy.state = EnemyState::ShellMoving;
                        enemy.kills_on_touch = true;
                        let kick_dir = if p_transform.translation.x < e_transform.translation.x
                        {
                            1.0
                        } else {
                            -1.0
                        };
                        commands.entity(e_entity).insert(ShellSpeed {
                            speed: 7.0 * kick_dir,
                        });
                        commands.entity(e_entity).insert(OwnerPlayer {
                            index: player.index,
                        });
                    } else {
                        // Enter shell state
                        play_sfx(&mut commands, &sfx, SfxId::Stomp);
                        enemy.state = EnemyState::InShell;
                        enemy.kills_on_touch = false;
                    }
                    p_vel.y = VEL_ENEMY_BOUNCE;
                }
                _ => {
                    // Generic stomp for other stompable enemies
                    play_sfx(&mut commands, &sfx, SfxId::Stomp);
                    enemy.state = EnemyState::Dead;
                    game_obj.alive = false;
                    p_vel.y = VEL_ENEMY_BOUNCE;
                    death_events.send(EnemyDeathEvent {
                        enemy_entity: e_entity,
                        killer_player: Some(player.index),
                    });
                    commands.entity(e_entity).insert(Lifetime::from_seconds(0.3));
                }
            }
        }
    }
}

/// Moves kicked shells at high speed and kills other enemies on contact.
pub fn shell_movement_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    mut shell_query: Query<(
        Entity,
        &mut Enemy,
        &ShellSpeed,
        &mut ObjectVelocity,
        &Collider,
        &Transform,
    )>,
    mut other_enemies: Query<
        (Entity, &mut Enemy, &mut GameObject, &Collider, &Transform),
        Without<ShellSpeed>,
    >,
    mut death_events: EventWriter<EnemyDeathEvent>,
) {
    // Collect shell data first to avoid borrow conflicts
    let shells: Vec<(Entity, f32, Vec3, Collider, Option<usize>)> = shell_query
        .iter()
        .filter(|(_, enemy, _, _, _, _)| enemy.state == EnemyState::ShellMoving)
        .map(|(entity, _enemy, shell_speed, _vel, collider, transform)| {
            // Try to get owner from the entity -- we check later
            (
                entity,
                shell_speed.speed,
                transform.translation,
                collider.clone(),
                None::<usize>,
            )
        })
        .collect();

    // Update shell velocities
    for (_entity, _enemy, shell_speed, mut vel, _collider, _transform) in shell_query.iter_mut() {
        if _enemy.state == EnemyState::ShellMoving {
            vel.x = shell_speed.speed;
        }
    }

    // Check shell vs other enemy collisions
    for (_shell_entity, _speed, shell_pos, shell_col, _owner) in &shells {
        for (e_entity, mut enemy, mut game_obj, e_collider, e_transform) in
            other_enemies.iter_mut()
        {
            if !game_obj.alive || enemy.state == EnemyState::Dead {
                continue;
            }

            if aabb_overlap(&shell_pos, &shell_col, &e_transform.translation, e_collider) {
                play_sfx(&mut commands, &sfx, SfxId::Kick);
                enemy.state = EnemyState::Dead;
                game_obj.alive = false;
                death_events.send(EnemyDeathEvent {
                    enemy_entity: e_entity,
                    killer_player: None,
                });
                commands.entity(e_entity).insert(Lifetime::from_seconds(0.3));
            }
        }
    }
}

/// Handles enemy collision with map boundaries (simplified wall bounce).
/// Syncs enemy facing direction to their current velocity.
/// Wall collision (velocity reversal) is handled by `object_map_collision`.
pub fn enemy_map_collision(
    mut query: Query<(&mut EnemyFacing, &ObjectVelocity), With<Enemy>>,
) {
    for (mut facing, vel) in query.iter_mut() {
        if vel.x > 0.0 && *facing != EnemyFacing::Right {
            *facing = EnemyFacing::Right;
        } else if vel.x < 0.0 && *facing != EnemyFacing::Left {
            *facing = EnemyFacing::Left;
        }
    }
}

/// Kills players on lateral contact with walking/shell-moving enemies.
/// Does not trigger if the player is invincible or the enemy is in a passive state.
pub fn enemy_player_touch_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    enemy_query: Query<(&Enemy, &Collider, &Transform, &GameObject)>,
    mut player_query: Query<(
        Entity,
        &Player,
        &mut Velocity,
        &mut PlayerAction,
        &Transform,
        &Collider,
        Option<&Invincible>,
    )>,
    _death_events: EventWriter<EnemyDeathEvent>,
    mut player_death_events: EventWriter<PlayerDeathEvent>,
) {
    for (enemy, e_collider, e_transform, game_obj) in enemy_query.iter() {
        if !game_obj.alive {
            continue;
        }
        if !enemy.kills_on_touch {
            continue;
        }
        if enemy.state == EnemyState::Dead || enemy.state == EnemyState::Stunned {
            continue;
        }

        for (p_entity, player, p_vel, mut p_action, p_transform, p_collider, invincible) in
            player_query.iter_mut()
        {
            if *p_action == PlayerAction::Dead {
                continue;
            }

            if !aabb_overlap(
                &p_transform.translation,
                p_collider,
                &e_transform.translation,
                e_collider,
            ) {
                continue;
            }

            // Check that this is a lateral touch, not a stomp (player not above enemy).
            // In Bevy Y-up: visual_bottom = pos + offset_y (feet)
            let player_visual_bottom = p_transform.translation.y + p_collider.offset_y;
            let enemy_visual_mid =
                e_transform.translation.y + e_collider.offset_y + e_collider.height * 0.5;

            // If player's feet are above enemy midpoint and falling, this is a stomp
            // handled by enemy_stomp_system, so skip here
            if player_visual_bottom >= enemy_visual_mid && p_vel.y < 0.0 {
                continue;
            }

            if invincible.is_some() {
                // Invincible player kills the enemy (handled in stomp system)
                continue;
            }

            // Kill the player
            play_sfx(&mut commands, &sfx, SfxId::PlayerDeath);
            *p_action = PlayerAction::Dead;
            player_death_events.send(PlayerDeathEvent {
                player_entity: p_entity,
                player_index: player.index,
                killer_index: None,
                cause: DeathCause::Environment,
            });
        }
    }
}

/// Despawns dead enemies after their lifetime expires.
pub fn enemy_lifetime_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Lifetime), With<Enemy>>,
) {
    for (entity, mut lifetime) in query.iter_mut() {
        if lifetime.timer.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Despawns enemies that fall off the bottom of the screen.
pub fn enemy_despawn_offscreen_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (With<Enemy>, With<DespawnOffScreen>)>,
) {
    for (entity, transform) in query.iter() {
        if transform.translation.y < -(SCREEN_HEIGHT / 2.0 + TILE_SIZE * 2.0) {
            commands.entity(entity).despawn_recursive();
        }
    }
}
