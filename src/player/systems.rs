use bevy::prelude::*;

use crate::core::constants::*;
use crate::map::components::{MapData, WarpDirection};
use crate::physics::components::Collider;
use super::components::*;

// --------------------------------------------------------------------------
// Death bounce velocity (player bounces upward then falls off screen)
// --------------------------------------------------------------------------
const DEATH_BOUNCE_VEL_Y: f32 = 9.0;

// Spawn protection duration in seconds
const SPAWN_PROTECTION_SECS: f32 = 3.0;

// Warp animation speed (progress per second; 1.0 = fully entered/exited)
const WARP_SPEED: f32 = 2.5;

// Stomp detection: how far below the stomper's feet the stomped player's head must be
const STOMP_THRESHOLD_Y: f32 = 8.0;

// How much velocity the stomper gets after a successful stomp
const STOMP_BOUNCE_VEL: f32 = 7.0;

/// Applies horizontal movement based on player input.
pub fn player_movement(
    mut query: Query<(
        &PlayerInput,
        &mut Velocity,
        &mut Facing,
        &mut PlayerAction,
        &Grounded,
        Option<&FrozenState>,
        Option<&WarpState>,
    )>,
) {
    for (input, mut vel, mut facing, mut action, grounded, frozen, warp) in query.iter_mut() {
        // Skip dead, spawning, frozen, or warping players
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            continue;
        }
        if frozen.is_some() || warp.is_some() {
            continue;
        }

        let max_speed = if input.turbo { VEL_TURBO_MOVING } else { VEL_MOVING };
        let accel = VEL_MOVING_ADD;

        // Choose friction based on surface
        let friction = if !grounded.on_ground {
            VEL_AIR_FRICTION
        } else if grounded.on_ice {
            VEL_ICE_FRICTION
        } else {
            VEL_MOVING_FRICTION
        };

        // Skid detection: reversing direction while moving fast on the ground
        let was_moving_right = vel.x > 1.0;
        let was_moving_left = vel.x < -1.0;

        if input.left {
            *facing = Facing::Left;
            vel.x -= accel;
            if vel.x < -max_speed {
                vel.x = -max_speed;
            }
        } else if input.right {
            *facing = Facing::Right;
            vel.x += accel;
            if vel.x > max_speed {
                vel.x = max_speed;
            }
        } else {
            // Apply friction to slow down
            if vel.x > 0.0 {
                vel.x -= friction;
                if vel.x < 0.0 {
                    vel.x = 0.0;
                }
            } else if vel.x < 0.0 {
                vel.x += friction;
                if vel.x > 0.0 {
                    vel.x = 0.0;
                }
            }
        }

        // Update action state for horizontal movement
        if grounded.on_ground && *action != PlayerAction::Jumping {
            // Enter skid state if player reversed direction while running
            let now_moving_left = vel.x < -0.5;
            let now_moving_right = vel.x > 0.5;
            let reversed = (was_moving_right && input.left && now_moving_right)
                || (was_moving_left && input.right && now_moving_left);

            if reversed && *action == PlayerAction::Running {
                *action = PlayerAction::Skidding;
            } else if vel.x.abs() > 0.1 {
                if *action == PlayerAction::Skidding {
                    // Stay skidding until velocity matches facing
                    let aligned = (*facing == Facing::Right && vel.x > 0.0)
                        || (*facing == Facing::Left && vel.x < 0.0);
                    if aligned {
                        *action = PlayerAction::Running;
                    }
                } else {
                    *action = PlayerAction::Running;
                }
            } else {
                *action = PlayerAction::Standing;
            }
        }
    }
}

/// Handles jump initiation and variable jump height.
pub fn player_jump(
    mut query: Query<(
        &PlayerInput,
        &mut Velocity,
        &mut PlayerAction,
        &mut Grounded,
        Option<&FrozenState>,
        Option<&WarpState>,
    )>,
) {
    for (input, mut vel, mut action, mut grounded, frozen, warp) in query.iter_mut() {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            continue;
        }
        if frozen.is_some() || warp.is_some() {
            continue;
        }

        if input.jump_pressed && grounded.on_ground {
            let jump_vel = if input.turbo { VEL_TURBO_JUMP } else { JUMP_VELOCITY };
            vel.y = jump_vel;
            *action = PlayerAction::Jumping;
            grounded.on_ground = false;
        }

        // Variable jump height: releasing jump button reduces upward velocity
        if !input.jump && vel.y > VEL_STOP_JUMP {
            vel.y = VEL_STOP_JUMP;
        }
    }
}

/// Handles player death state: apply death bounce velocity, prevent input.
pub fn player_death_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Player,
        &mut Velocity,
        &mut PlayerAction,
        Option<&BurnupTimer>,
    )>,
    mut death_events: EventReader<PlayerDeathEvent>,
) {
    // Process incoming death events
    for event in death_events.read() {
        if let Ok((entity, mut player, mut vel, mut action, burnup)) =
            query.get_mut(event.player_entity)
        {
            if *action == PlayerAction::Dead {
                continue; // Already dead
            }

            *action = PlayerAction::Dead;
            vel.x = 0.0;
            vel.y = DEATH_BOUNCE_VEL_Y; // Bounce upward
            player.lives -= 1;

            // Add burnup timer if not already present
            if burnup.is_none() {
                commands.entity(entity).insert(BurnupTimer::default());
            }
        }
    }
}

/// Updates the burnup timer and triggers respawn when it finishes.
pub fn player_burnup_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &Player, &PlayerAction, &mut BurnupTimer)>,
    mut respawn_events: EventWriter<PlayerRespawnEvent>,
) {
    for (entity, player, action, mut burnup) in query.iter_mut() {
        if *action != PlayerAction::Dead {
            commands.entity(entity).remove::<BurnupTimer>();
            continue;
        }

        burnup.timer.tick(time.delta());
        if burnup.timer.just_finished() {
            commands.entity(entity).remove::<BurnupTimer>();
            if player.lives > 0 {
                respawn_events.send(PlayerRespawnEvent {
                    player_entity: entity,
                    player_index: player.index,
                });
            }
        }
    }
}

/// Respawns players at a spawn point with protection.
pub fn player_respawn_system(
    mut commands: Commands,
    map_data: Res<MapData>,
    mut query: Query<(
        Entity,
        &Player,
        &mut Velocity,
        &mut PlayerAction,
        &mut Transform,
        &mut Grounded,
    )>,
    mut respawn_events: EventReader<PlayerRespawnEvent>,
) {
    for event in respawn_events.read() {
        if let Ok((entity, player, mut vel, mut action, mut transform, mut grounded)) =
            query.get_mut(event.player_entity)
        {
            let (spawn_x, spawn_y) = pick_spawn_location(&map_data, player.index);

            transform.translation.x = spawn_x;
            transform.translation.y = spawn_y;
            vel.x = 0.0;
            vel.y = 0.0;
            grounded.on_ground = false;
            grounded.on_ice = false;
            *action = PlayerAction::Spawning;

            commands
                .entity(entity)
                .insert(SpawnProtection::new(SPAWN_PROTECTION_SECS));
        }
    }
}

/// Updates spawn protection timers and transitions out of Spawning state.
pub fn player_spawn_protection_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut PlayerAction, &mut SpawnProtection)>,
) {
    for (entity, mut action, mut protection) in query.iter_mut() {
        protection.timer.tick(time.delta());
        if protection.timer.just_finished() {
            if *action == PlayerAction::Spawning {
                *action = PlayerAction::Standing;
            }
            commands.entity(entity).remove::<SpawnProtection>();
        }
    }
}

/// Updates frozen state timers and removes them when expired.
pub fn frozen_state_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut FrozenState)>,
) {
    for (entity, mut frozen) in query.iter_mut() {
        frozen.timer.tick(time.delta());
        if frozen.timer.just_finished() {
            commands.entity(entity).remove::<FrozenState>();
        }
    }
}

/// Updates invincibility timers and removes them when expired.
pub fn invincibility_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Invincible)>,
) {
    for (entity, mut invincible) in query.iter_mut() {
        invincible.timer.tick(time.delta());
        if invincible.timer.just_finished() {
            commands.entity(entity).remove::<Invincible>();
        }
    }
}

/// Detects player-to-player stomps (one player falling onto another from above).
pub fn player_stomp_system(
    query: Query<(
        Entity,
        &Player,
        &Transform,
        &Velocity,
        &PlayerAction,
        &Collider,
        Option<&Invincible>,
        Option<&SpawnProtection>,
    )>,
    mut stomp_events: EventWriter<PlayerStompEvent>,
    mut death_events: EventWriter<PlayerDeathEvent>,
) {
    let players: Vec<_> = query.iter().collect();

    for i in 0..players.len() {
        for j in (i + 1)..players.len() {
            let (e_a, p_a, t_a, v_a, act_a, col_a, inv_a, sp_a) = &players[i];
            let (e_b, p_b, t_b, v_b, act_b, col_b, inv_b, sp_b) = &players[j];

            // Skip if either player is dead, spawning, or has protection
            if **act_a == PlayerAction::Dead
                || **act_a == PlayerAction::Spawning
                || **act_b == PlayerAction::Dead
                || **act_b == PlayerAction::Spawning
            {
                continue;
            }
            if inv_a.is_some() || sp_a.is_some() || inv_b.is_some() || sp_b.is_some() {
                continue;
            }

            // Same team check
            if p_a.team == p_b.team {
                continue;
            }

            // Check AABB overlap (with screen wrapping consideration)
            if !players_overlap(t_a, col_a, t_b, col_b) {
                continue;
            }

            // In Bevy Y-up: pos + offset_y = visual bottom, pos + offset_y + height = visual top
            let a_visual_bottom = t_a.translation.y + col_a.offset_y;
            let a_visual_top = a_visual_bottom + col_a.height;
            let b_visual_bottom = t_b.translation.y + col_b.offset_y;
            let b_visual_top = b_visual_bottom + col_b.height;

            // A stomps B: A is falling (vel.y < 0) and A's bottom is near B's top
            if v_a.y < 0.0 && (a_visual_bottom - b_visual_top).abs() < STOMP_THRESHOLD_Y {
                stomp_events.send(PlayerStompEvent {
                    stomper_entity: *e_a,
                    stomped_entity: *e_b,
                    stomper_index: p_a.index,
                    stomped_index: p_b.index,
                });
                death_events.send(PlayerDeathEvent {
                    player_entity: *e_b,
                    player_index: p_b.index,
                    killer_index: Some(p_a.index),
                    cause: DeathCause::Killed { by: p_a.index },
                });
                continue;
            }

            // B stomps A: B is falling (vel.y < 0) and B's bottom is near A's top
            if v_b.y < 0.0 && (b_visual_bottom - a_visual_top).abs() < STOMP_THRESHOLD_Y {
                stomp_events.send(PlayerStompEvent {
                    stomper_entity: *e_b,
                    stomped_entity: *e_a,
                    stomper_index: p_b.index,
                    stomped_index: p_a.index,
                });
                death_events.send(PlayerDeathEvent {
                    player_entity: *e_a,
                    player_index: p_a.index,
                    killer_index: Some(p_b.index),
                    cause: DeathCause::Killed { by: p_b.index },
                });
            }
        }
    }
}

/// After a stomp is detected, give the stomper a bounce.
pub fn stomp_bounce_system(
    mut query: Query<(Entity, &mut Velocity)>,
    mut stomp_events: EventReader<PlayerStompEvent>,
) {
    for event in stomp_events.read() {
        if let Ok((_entity, mut vel)) = query.get_mut(event.stomper_entity) {
            vel.y = STOMP_BOUNCE_VEL;
        }
    }
}

/// Vertical death detection: players falling off the bottom of the screen.
pub fn vertical_death_system(
    query: Query<(Entity, &Player, &Transform, &PlayerAction)>,
    mut death_events: EventWriter<PlayerDeathEvent>,
) {
    for (entity, player, transform, action) in query.iter() {
        if *action == PlayerAction::Dead {
            continue;
        }
        if transform.translation.y < -(SCREEN_HEIGHT / 2.0 + PLAYER_HEIGHT) {
            death_events.send(PlayerDeathEvent {
                player_entity: entity,
                player_index: player.index,
                killer_index: None,
                cause: DeathCause::Environment,
            });
        }
    }
}

/// Cape/tail attack system: activates attack hitbox when player uses item with Feather/Leaf.
pub fn attack_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &PlayerInput,
        &PowerupState,
        &PlayerAction,
        Option<&mut CapeState>,
        Option<&mut TailState>,
    )>,
) {
    for (entity, input, powerup, action, cape, tail) in query.iter_mut() {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            continue;
        }

        match powerup {
            PowerupState::Feather => {
                if let Some(mut cape) = cape {
                    cape.timer.tick(time.delta());
                    if cape.timer.finished() {
                        cape.active = false;
                    }
                    if input.use_item && !cape.active {
                        cape.active = true;
                        cape.timer.reset();
                    }
                } else if input.use_item {
                    let mut state = CapeState::default();
                    state.active = true;
                    commands.entity(entity).insert(state);
                }
            }
            PowerupState::Leaf => {
                if let Some(mut tail) = tail {
                    tail.timer.tick(time.delta());
                    if tail.timer.finished() {
                        tail.active = false;
                    }
                    if input.use_item && !tail.active {
                        tail.active = true;
                        tail.timer.reset();
                    }
                } else if input.use_item {
                    let mut state = TailState::default();
                    state.active = true;
                    commands.entity(entity).insert(state);
                }
            }
            _ => {}
        }
    }
}

/// Fires a projectile when the player presses use_item with a projectile-type powerup.
pub fn powerup_use_system(
    mut commands: Commands,
    query: Query<(
        &PlayerInput,
        &Player,
        &PowerupState,
        &Facing,
        &Transform,
        &PlayerAction,
    )>,
) {
    use crate::objects::components::ObjectVelocity;
    use crate::objects::projectiles::{Projectile, ProjectileType};
    use crate::physics::components::CollisionLayer;

    for (input, player, powerup, facing, transform, action) in query.iter() {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            continue;
        }
        if !input.use_item {
            continue;
        }

        let dir = match facing {
            Facing::Right => 1.0,
            Facing::Left => -1.0,
        };

        let spawn_x = transform.translation.x + dir * (PLAYER_WIDTH * 0.5);
        let spawn_y = transform.translation.y + PLAYER_HEIGHT * 0.3;

        match powerup {
            PowerupState::Fire => {
                let speed = 6.0_f32;
                commands.spawn((
                    Projectile::fireball(player.index),
                    ObjectVelocity {
                        x: speed * dir,
                        y: 0.0,
                    },
                    Transform::from_xyz(spawn_x, spawn_y, 11.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    Collider::new(8.0, 8.0),
                    CollisionLayer::Projectile,
                ));
            }
            PowerupState::Hammer => {
                let speed = 8.5_f32;
                commands.spawn((
                    Projectile::hammer(player.index),
                    ObjectVelocity {
                        x: speed * dir,
                        y: HAMMER_THROW,
                    },
                    Transform::from_xyz(spawn_x, spawn_y, 11.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    Collider::new(16.0, 16.0),
                    CollisionLayer::Projectile,
                ));
            }
            PowerupState::Boomerang => {
                let speed = 5.0_f32;
                commands.spawn((
                    Projectile::boomerang(player.index),
                    ObjectVelocity {
                        x: speed * dir,
                        y: 0.0,
                    },
                    Transform::from_xyz(spawn_x, spawn_y, 11.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    Collider::new(16.0, 16.0),
                    CollisionLayer::Projectile,
                ));
            }
            PowerupState::Bomb => {
                commands.spawn((
                    Projectile {
                        projectile_type: ProjectileType::Bomb,
                        owner: player.index,
                        bounces_remaining: 0,
                        speed: 4.0,
                    },
                    ObjectVelocity {
                        x: 4.0 * dir,
                        y: 3.0,
                    },
                    Transform::from_xyz(spawn_x, spawn_y, 11.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    Collider::new(16.0, 16.0),
                    CollisionLayer::Projectile,
                ));
            }
            _ => {}
        }
    }
}

/// Warp pipe entry/exit animation system.
pub fn warp_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut WarpState,
        &mut Transform,
        &mut Velocity,
        &mut PlayerAction,
    )>,
) {
    for (entity, mut warp, mut transform, mut vel, mut action) in query.iter_mut() {
        vel.x = 0.0;
        vel.y = 0.0;

        warp.progress += WARP_SPEED * time.delta_secs();

        let movement = WARP_SPEED * TILE_SIZE * time.delta_secs();

        if warp.entering {
            // Animate player moving into the pipe (Bevy Y-up)
            match warp.direction {
                WarpDirection::Down => {
                    transform.translation.y -= movement;
                }
                WarpDirection::Up => {
                    transform.translation.y += movement;
                }
                WarpDirection::Left => {
                    transform.translation.x -= movement;
                }
                WarpDirection::Right => {
                    transform.translation.x += movement;
                }
                WarpDirection::Undefined => {}
            }

            if warp.progress >= 1.0 {
                // Teleport to destination and start exiting
                transform.translation.x = warp.dest_x;
                transform.translation.y = warp.dest_y;
                warp.entering = false;
                warp.progress = 0.0;
            }
        } else {
            // Animate player moving out of the pipe (reverse direction, Bevy Y-up)
            match warp.direction {
                WarpDirection::Down => {
                    transform.translation.y += movement;
                }
                WarpDirection::Up => {
                    transform.translation.y -= movement;
                }
                WarpDirection::Left => {
                    transform.translation.x += movement;
                }
                WarpDirection::Right => {
                    transform.translation.x -= movement;
                }
                WarpDirection::Undefined => {}
            }

            if warp.progress >= 1.0 {
                commands.entity(entity).remove::<WarpState>();
                *action = PlayerAction::Standing;
            }
        }
    }
}

/// Updates player stats when stomp events occur.
pub fn update_stats_on_stomp(
    mut query: Query<(&Player, &mut PlayerStats)>,
    mut stomp_events: EventReader<PlayerStompEvent>,
) {
    for event in stomp_events.read() {
        for (player, mut stats) in query.iter_mut() {
            if player.index == event.stomper_index {
                stats.stomps += 1;
                stats.kills += 1;
            }
            if player.index == event.stomped_index {
                stats.deaths += 1;
            }
        }
    }
}

/// Updates player stats when death events occur (non-stomp deaths like falling).
pub fn update_stats_on_death(
    mut query: Query<(&Player, &mut PlayerStats)>,
    mut death_events: EventReader<PlayerDeathEvent>,
) {
    for event in death_events.read() {
        for (player, mut stats) in query.iter_mut() {
            if player.index == event.player_index {
                stats.deaths += 1;
                if event.killer_index.is_none() {
                    stats.suicides += 1;
                }
            }
        }
    }
}

// --------------------------------------------------------------------------
// Helper functions
// --------------------------------------------------------------------------

/// Pick a spawn location from the map's spawn areas, or use a fallback.
/// Returns Bevy coordinates (centered, Y-up).
fn pick_spawn_location(map_data: &MapData, player_index: usize) -> (f32, f32) {
    // spawn_areas[0] = "All players" spawn areas
    if !map_data.spawn_areas.is_empty() && !map_data.spawn_areas[0].is_empty() {
        let areas = &map_data.spawn_areas[0];
        let area = &areas[player_index % areas.len()];
        // Convert SMW pixel coords to Bevy coords
        let smw_x = (area.left as f32 + area.width as f32 / 2.0) * TILE_SIZE;
        let smw_y = area.top as f32 * TILE_SIZE;
        (smw_x - SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0 - smw_y)
    } else {
        // Fallback: distribute players across the top of the screen (Bevy coords)
        let smw_x = (SCREEN_WIDTH / (MAX_PLAYERS as f32 + 1.0)) * (player_index as f32 + 1.0);
        let smw_y = TILE_SIZE * 2.0;
        (smw_x - SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0 - smw_y)
    }
}

/// Check if two players' colliders overlap, accounting for screen wrapping.
fn players_overlap(
    t_a: &Transform,
    col_a: &Collider,
    t_b: &Transform,
    col_b: &Collider,
) -> bool {
    // Direct overlap check
    if aabb_overlap_direct(&t_a.translation, col_a, &t_b.translation, col_b) {
        return true;
    }

    // Check with screen wrapping (player A wrapped, Bevy centered coords)
    let wrap_amount = SCREEN_WIDTH + PLAYER_WIDTH * 2.0;
    let mut wrapped_a = t_a.translation;
    if wrapped_a.x < 0.0 {
        wrapped_a.x += wrap_amount;
    } else {
        wrapped_a.x -= wrap_amount;
    }
    if aabb_overlap_direct(&wrapped_a, col_a, &t_b.translation, col_b) {
        return true;
    }

    false
}

fn aabb_overlap_direct(pos_a: &Vec3, col_a: &Collider, pos_b: &Vec3, col_b: &Collider) -> bool {
    let a_left = pos_a.x + col_a.offset_x;
    let a_right = a_left + col_a.width;
    let a_top = pos_a.y + col_a.offset_y;
    let a_bottom = a_top + col_a.height;

    let b_left = pos_b.x + col_b.offset_x;
    let b_right = b_left + col_b.width;
    let b_top = pos_b.y + col_b.offset_y;
    let b_bottom = b_top + col_b.height;

    a_left < b_right && a_right > b_left && a_top < b_bottom && a_bottom > b_top
}
