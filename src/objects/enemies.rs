use bevy::prelude::*;

use crate::audio::sfx::{play_sfx, SfxId, SfxLibrary, SfxVolume};
use crate::core::constants::*;
use crate::objects::components::*;
use crate::physics::collision::aabb_overlap_wrapped;
use crate::map::GameplayEntity;
use crate::physics::components::{Collider, CollisionLayer, GravityAffected};
use crate::player::components::{
    DeathCause, Invincible, Player, PlayerAction, PlayerDeathEvent, ShieldState, SpawnProtection,
    Velocity,
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

/// Timer for shells to re-emerge as walking enemies.
#[derive(Component, Debug)]
pub struct ShellTimer {
    pub timer: Timer,
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
    // Flying enemies don't have gravity
    let gravity = match enemy.enemy_type {
        EnemyType::Paragoomba | EnemyType::CheepCheep | EnemyType::BulletBill | EnemyType::Podoboo => {
            GravityAffected { multiplier: 0.0 }
        }
        _ => GravityAffected::default(),
    };

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
            gravity,
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
    map_data: Res<crate::map::components::MapData>,
    mut query: Query<(
        &mut Enemy,
        &mut EnemyFacing,
        &mut ObjectVelocity,
        &ObjectGrounded,
        &Transform,
    )>,
) {
    let num_cols = map_data.collision_map.len();
    let num_rows = if num_cols > 0 { map_data.collision_map[0].len() } else { 0 };

    for (enemy, mut facing, mut vel, grounded, transform) in query.iter_mut() {
        match enemy.state {
            EnemyState::Walking => {
                let direction = match *facing {
                    EnemyFacing::Left => -1.0,
                    EnemyFacing::Right => 1.0,
                };
                vel.x = enemy.speed * direction;

                // Red koopas turn at platform edges when on the ground
                if enemy.enemy_type == EnemyType::KoopaRed && grounded.0 && num_cols > 0 {
                    // Convert Bevy position to SMW pixel coords for tile lookup
                    let smw_x = transform.translation.x + SCREEN_WIDTH / 2.0;
                    let smw_y = SCREEN_HEIGHT / 2.0 - transform.translation.y;

                    // Current tile position
                    let tile_col = (smw_x / TILE_SIZE).floor() as i32;
                    // Tile row the enemy is standing on (one tile below feet)
                    let tile_row = ((smw_y + TILE_SIZE) / TILE_SIZE).floor() as i32;

                    // Check the tile below and ahead in the facing direction
                    let ahead_col = if direction > 0.0 { tile_col + 1 } else { tile_col - 1 };
                    // Wrap around for screen wrapping
                    let wrapped_col = ((ahead_col % num_cols as i32) + num_cols as i32) as usize % num_cols;

                    if tile_row >= 0 && (tile_row as usize) < num_rows {
                        let below_ahead = map_data.collision_map[wrapped_col][tile_row as usize];
                        // If there's no solid ground ahead, reverse direction
                        if !below_ahead.is_solid() && !below_ahead.is_solid_on_top() {
                            *facing = if direction > 0.0 { EnemyFacing::Left } else { EnemyFacing::Right };
                        }
                    } else if tile_row >= num_rows as i32 {
                        // Below the map — no ground, reverse
                        *facing = if direction > 0.0 { EnemyFacing::Left } else { EnemyFacing::Right };
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
/// Respects GravityAffected multiplier (flying enemies can use 0.0).
pub fn enemy_gravity_system(
    mut query: Query<(&mut ObjectVelocity, &mut Transform, &GravityAffected), With<Enemy>>,
) {
    for (mut vel, mut transform, gravity) in query.iter_mut() {
        // Apply gravity scaled by multiplier (negative = downward in Bevy Y-up)
        vel.y -= GRAVITY * gravity.multiplier;
        if vel.y < -MAX_FALL_SPEED {
            vel.y = -MAX_FALL_SPEED;
        }

        // Apply velocity
        transform.translation.x += vel.x;
        transform.translation.y += vel.y;

        // Screen wrapping (horizontal, Bevy centered coords)
        let half_w = SCREEN_WIDTH / 2.0;
        if transform.translation.x > half_w {
            transform.translation.x -= SCREEN_WIDTH;
        } else if transform.translation.x < -half_w {
            transform.translation.x += SCREEN_WIDTH;
        }
    }
}

/// Detects when a player lands on top of an enemy (stomp). Handles different
/// enemy types: goombas die, koopas enter shell state, spinies hurt the player.
pub fn enemy_stomp_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    sfx_vol: Res<SfxVolume>,
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
        Option<&SpawnProtection>,
    )>,
    mut death_events: EventWriter<EnemyDeathEvent>,
    mut player_death_events: EventWriter<PlayerDeathEvent>,
) {
    for (p_entity, player, mut p_vel, p_transform, p_collider, p_action, invincible, spawn_prot) in
        player_query.iter_mut()
    {
        if *p_action == PlayerAction::Dead || *p_action == PlayerAction::Spawning {
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

            if !aabb_overlap_wrapped(
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
                play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Kick);
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
                // Spiny and similar -- hurt the player unless spawn-protected
                if spawn_prot.is_some() {
                    continue;
                }
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
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Stomp);
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
                    if enemy.state == EnemyState::InShell || enemy.state == EnemyState::ShellMoving {
                        // Kick or re-kick the shell
                        play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Kick);
                        enemy.state = EnemyState::ShellMoving;
                        enemy.kills_on_touch = true;
                        let kick_dir = if p_transform.translation.x < e_transform.translation.x
                        {
                            1.0
                        } else {
                            -1.0
                        };
                        commands.entity(e_entity)
                            .insert(ShellSpeed { speed: 7.0 * kick_dir })
                            .insert(OwnerPlayer { index: player.index })
                            .remove::<ShellTimer>();
                    } else {
                        // Enter shell state
                        play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Stomp);
                        enemy.state = EnemyState::InShell;
                        enemy.kills_on_touch = false;
                        commands.entity(e_entity).insert(ShellTimer {
                            timer: Timer::from_seconds(5.0, TimerMode::Once),
                        });
                    }
                    p_vel.y = VEL_ENEMY_BOUNCE;
                }
                _ => {
                    // Generic stomp for other stompable enemies
                    play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Stomp);
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
    sfx_vol: Res<SfxVolume>,
    mut shell_query: Query<(
        Entity,
        &mut Enemy,
        &ShellSpeed,
        &mut ObjectVelocity,
        &Collider,
        &Transform,
        Option<&OwnerPlayer>,
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
        .filter(|(_, enemy, _, _, _, _, _)| enemy.state == EnemyState::ShellMoving)
        .map(|(entity, _enemy, shell_speed, _vel, collider, transform, owner)| {
            (
                entity,
                shell_speed.speed,
                transform.translation,
                collider.clone(),
                owner.map(|o| o.index),
            )
        })
        .collect();

    // Update shell velocities
    for (_entity, _enemy, shell_speed, mut vel, _collider, _transform, _) in shell_query.iter_mut() {
        if _enemy.state == EnemyState::ShellMoving {
            vel.x = shell_speed.speed;
        }
    }

    // Check shell vs other enemy collisions
    for (_shell_entity, _speed, shell_pos, shell_col, owner) in &shells {
        for (e_entity, mut enemy, mut game_obj, e_collider, e_transform) in
            other_enemies.iter_mut()
        {
            if !game_obj.alive || enemy.state == EnemyState::Dead {
                continue;
            }

            if aabb_overlap_wrapped(&shell_pos, &shell_col, &e_transform.translation, e_collider) {
                play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Kick);
                enemy.state = EnemyState::Dead;
                game_obj.alive = false;
                death_events.send(EnemyDeathEvent {
                    enemy_entity: e_entity,
                    killer_player: *owner,
                });
                commands.entity(e_entity).insert(Lifetime::from_seconds(0.3));
            }
        }
    }

    // Shell-to-shell collision: both shells die when they collide
    for i in 0..shells.len() {
        for j in (i + 1)..shells.len() {
            let (entity_a, _, pos_a, col_a, owner_a) = &shells[i];
            let (entity_b, _, pos_b, col_b, owner_b) = &shells[j];

            if aabb_overlap_wrapped(pos_a, col_a, pos_b, col_b) {
                play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Kick);
                // Kill both shells
                if let Ok((_, mut enemy_a, _, _, _, _, _)) = shell_query.get_mut(*entity_a) {
                    enemy_a.state = EnemyState::Dead;
                }
                if let Ok((_, mut enemy_b, _, _, _, _, _)) = shell_query.get_mut(*entity_b) {
                    enemy_b.state = EnemyState::Dead;
                }
                death_events.send(EnemyDeathEvent {
                    enemy_entity: *entity_a,
                    killer_player: *owner_b,
                });
                death_events.send(EnemyDeathEvent {
                    enemy_entity: *entity_b,
                    killer_player: *owner_a,
                });
                commands.entity(*entity_a).insert(Lifetime::from_seconds(0.3));
                commands.entity(*entity_b).insert(Lifetime::from_seconds(0.3));
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
/// Invincible players kill enemies on side contact instead of being hurt.
pub fn enemy_player_touch_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    sfx_vol: Res<SfxVolume>,
    mut enemy_query: Query<(Entity, &mut Enemy, &mut GameObject, &Collider, &Transform, Option<&OwnerPlayer>)>,
    mut player_query: Query<(
        Entity,
        &Player,
        &mut Velocity,
        &mut PlayerAction,
        &Transform,
        &Collider,
        Option<&Invincible>,
        Option<&SpawnProtection>,
        &mut ShieldState,
    )>,
    mut death_events: EventWriter<EnemyDeathEvent>,
    mut player_death_events: EventWriter<PlayerDeathEvent>,
) {
    for (e_entity, mut enemy, mut game_obj, e_collider, e_transform, owner) in enemy_query.iter_mut() {
        if !game_obj.alive {
            continue;
        }
        if !enemy.kills_on_touch {
            continue;
        }
        if enemy.state == EnemyState::Dead || enemy.state == EnemyState::Stunned {
            continue;
        }

        for (p_entity, player, p_vel, mut p_action, p_transform, p_collider, invincible, spawn_prot, mut shield) in
            player_query.iter_mut()
        {
            if *p_action == PlayerAction::Dead || *p_action == PlayerAction::Spawning {
                continue;
            }

            if !aabb_overlap_wrapped(
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

            // Shell owner immunity: the player who kicked this shell is immune
            if let Some(own) = owner {
                if own.index == player.index {
                    continue;
                }
            }

            // Invincible player kills the enemy on side contact (star power)
            if invincible.is_some() {
                play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::Kick);
                enemy.state = EnemyState::Dead;
                game_obj.alive = false;
                death_events.send(EnemyDeathEvent {
                    enemy_entity: e_entity,
                    killer_player: Some(player.index),
                });
                commands.entity(e_entity).insert(Lifetime::from_seconds(0.3));
                break; // Enemy is dead, stop checking other players
            }

            if spawn_prot.is_some() {
                // Spawn-protected player is immune
                continue;
            }

            // Shield absorbs enemy contact damage
            if *shield != ShieldState::None {
                play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::BlockBreak);
                *shield = ShieldState::None;
                continue;
            }

            // Kill the player
            play_sfx(&mut commands, &sfx, &sfx_vol, SfxId::PlayerDeath);
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

/// Despawns enemies that leave the screen vertically.
pub fn enemy_despawn_offscreen_system(
    mut commands: Commands,
    query: Query<(Entity, &Transform), (With<Enemy>, With<DespawnOffScreen>)>,
) {
    for (entity, transform) in query.iter() {
        if transform.translation.y < -(SCREEN_HEIGHT / 2.0 + TILE_SIZE * 2.0)
            || transform.translation.y > SCREEN_HEIGHT / 2.0 + TILE_SIZE * 4.0
        {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Ticks shell timers and re-emerges Koopas from their shells.
pub fn shell_reemerge_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Enemy, &mut ShellTimer)>,
) {
    for (entity, mut enemy, mut shell_timer) in query.iter_mut() {
        if shell_timer.timer.tick(time.delta()).just_finished() {
            enemy.state = EnemyState::Walking;
            enemy.kills_on_touch = true;
            commands
                .entity(entity)
                .remove::<ShellTimer>()
                .remove::<ShellSpeed>()
                .remove::<OwnerPlayer>();
        }
    }
}

/// Ticks stun timers and recovers enemies to Walking state when expired.
pub fn enemy_stun_recovery_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Enemy, &mut StunTimer)>,
) {
    for (entity, mut enemy, mut stun) in query.iter_mut() {
        if stun.timer.tick(time.delta()).just_finished() {
            enemy.state = EnemyState::Walking;
            commands.entity(entity).remove::<StunTimer>();
        }
    }
}
