use bevy::prelude::*;

use crate::core::constants::*;
use crate::map::components::{CollisionType, MapData, MovingPlatform};
use crate::objects::components::{InteractsWithMap, ObjectGrounded, ObjectVelocity};
use crate::player::components::{
    DeathCause, Grounded, Player, PlayerAction, PlayerDeathEvent, PlayerInput, Velocity, WarpState,
};
use super::collision;
use super::components::*;

// ---------------------------------------------------------------------------
// Coordinate conversion helpers: Bevy (centered, Y-up) ↔ SMW (top-left, Y-down)
// ---------------------------------------------------------------------------

/// Convert Bevy X → SMW pixel X (0..SCREEN_WIDTH)
fn to_smw_x(bevy_x: f32) -> f32 {
    bevy_x + SCREEN_WIDTH / 2.0
}

/// Convert Bevy Y → SMW pixel Y (0..SCREEN_HEIGHT, Y-down)
fn to_smw_y(bevy_y: f32) -> f32 {
    SCREEN_HEIGHT / 2.0 - bevy_y
}

/// Convert SMW pixel X → Bevy X
fn to_bevy_x(smw_x: f32) -> f32 {
    smw_x - SCREEN_WIDTH / 2.0
}

/// Convert SMW pixel Y → Bevy Y
fn to_bevy_y(smw_y: f32) -> f32 {
    SCREEN_HEIGHT / 2.0 - smw_y
}

/// Applies velocity to player transform and handles screen wrapping.
/// Runs AFTER gravity and BEFORE map collision so that collision resolution
/// operates on the correct position.
pub fn apply_player_velocity(
    mut query: Query<(&Velocity, &mut Transform), With<Player>>,
) {
    for (vel, mut transform) in query.iter_mut() {
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

/// Applies gravity to all entities that have the GravityAffected component.
pub fn apply_gravity(
    mut query: Query<(&mut Velocity, &GravityAffected, Option<&WarpState>)>,
) {
    for (mut vel, gravity, warp) in query.iter_mut() {
        // Don't apply gravity while warping
        if warp.is_some() {
            continue;
        }
        vel.y -= GRAVITY * gravity.multiplier;
        if vel.y < -MAX_FALL_SPEED {
            vel.y = -MAX_FALL_SPEED;
        }
    }
}

/// Detects collision between players and the map tile grid.
/// Reads from `MapData.collision_map` which is `[col][row]` (column-major).
/// Handles Solid, SolidOnTop, Ice, Death, and SuperDeath tiles.
pub fn map_collision(
    map_data: Res<MapData>,
    mut query: Query<(
        Entity,
        &Player,
        &mut Transform,
        &mut Velocity,
        &mut Grounded,
        &PlayerAction,
        &PlayerInput,
        Option<&WarpState>,
    )>,
    mut death_events: EventWriter<PlayerDeathEvent>,
) {
    let has_collision_map = !map_data.collision_map.is_empty()
        && !map_data.collision_map[0].is_empty();

    for (entity, player, mut transform, mut vel, mut grounded, action, input, warp) in
        query.iter_mut()
    {
        if *action == PlayerAction::Dead {
            continue;
        }
        if warp.is_some() {
            continue;
        }

        // Convert Bevy position → SMW pixel space using local variables.
        // All collision helpers expect SMW coords (top-left origin, Y-down).
        let mut smw_x = to_smw_x(transform.translation.x);
        let mut smw_y = to_smw_y(transform.translation.y);
        let mut smw_vel_y = -vel.y; // Bevy Y-up → SMW Y-down

        if !has_collision_map {
            // Simple floor collision (inline with local vars)
            let floor_y = SCREEN_HEIGHT - PLAYER_HEIGHT - PLAYER_HEIGHT_OFFSET;
            if smw_y >= floor_y {
                smw_y = floor_y;
                smw_vel_y = 0.0;
                grounded.on_ground = true;
            } else {
                grounded.on_ground = false;
            }
            grounded.on_ice = false;

            // Ceiling
            if smw_y < 0.0 {
                smw_y = 0.0;
                if smw_vel_y < 0.0 {
                    smw_vel_y = 0.0;
                }
            }
        } else {
            let num_cols = map_data.collision_map.len();
            let num_rows = map_data.collision_map[0].len();

            // Reset grounded before checking tiles
            grounded.on_ground = false;
            grounded.on_ice = false;

            // Write local SMW position into transform temporarily for collision helpers
            transform.translation.x = smw_x;
            transform.translation.y = smw_y;
            vel.y = smw_vel_y;

            // The player's bounding box in SMW pixel coordinates
            let px = smw_x + PLAYER_WIDTH_OFFSET;
            let py = smw_y + PLAYER_HEIGHT_OFFSET;

            // Vertical collision (floors/ceilings)
            check_vertical_collision(
                &map_data.collision_map,
                num_cols,
                num_rows,
                &mut transform,
                &mut vel,
                &mut grounded,
                input,
                px,
                py,
            );

            // Read back position after vertical correction
            smw_x = transform.translation.x;
            smw_y = transform.translation.y;

            // Recalculate player position after vertical correction
            let px = smw_x + PLAYER_WIDTH_OFFSET;
            let py = smw_y + PLAYER_HEIGHT_OFFSET;

            // Horizontal collision (walls)
            check_horizontal_collision(
                &map_data.collision_map,
                num_cols,
                num_rows,
                &mut transform,
                &mut vel,
                px,
                py,
            );

            // Read back after horizontal correction
            smw_x = transform.translation.x;
            smw_y = transform.translation.y;
            smw_vel_y = vel.y;

            // Death tile check
            let px = smw_x + PLAYER_WIDTH_OFFSET;
            let py = smw_y + PLAYER_HEIGHT_OFFSET;
            if check_death_tiles(&map_data.collision_map, num_cols, num_rows, px, py) {
                death_events.send(PlayerDeathEvent {
                    player_entity: entity,
                    player_index: player.index,
                    killer_index: None,
                    cause: DeathCause::Environment,
                });
            }

            // Ceiling collision (top of screen, SMW Y=0)
            if smw_y < 0.0 {
                smw_y = 0.0;
                if smw_vel_y < 0.0 {
                    smw_vel_y = 0.0;
                }
            }
        }

        // Convert SMW pixel space → Bevy position (write back once)
        transform.translation.x = to_bevy_x(smw_x);
        transform.translation.y = to_bevy_y(smw_y);
        vel.y = -smw_vel_y; // SMW Y-down → Bevy Y-up
    }
}

/// Simple floor collision fallback when no map data is loaded.
fn simple_floor_collision(
    transform: &mut Transform,
    vel: &mut Velocity,
    grounded: &mut Grounded,
) {
    let floor_y = SCREEN_HEIGHT - PLAYER_HEIGHT - PLAYER_HEIGHT_OFFSET;
    if transform.translation.y >= floor_y {
        transform.translation.y = floor_y;
        vel.y = 0.0;
        grounded.on_ground = true;
    } else {
        grounded.on_ground = false;
    }
    grounded.on_ice = false;

    // Ceiling
    if transform.translation.y < 0.0 {
        transform.translation.y = 0.0;
        if vel.y < 0.0 {
            vel.y = 0.0;
        }
    }
}

/// Gets the collision type at tile coordinate (tile_x, tile_y).
/// `collision_map` is `[col][row]` (column-major).
fn get_collision(
    collision_map: &[Vec<CollisionType>],
    num_cols: usize,
    num_rows: usize,
    tile_x: usize,
    tile_y: usize,
) -> CollisionType {
    if tile_x < num_cols && tile_y < num_rows {
        collision_map[tile_x][tile_y]
    } else {
        CollisionType::NonSolid
    }
}

/// Checks vertical collision against tiles (floors, ceilings, one-way platforms).
fn check_vertical_collision(
    collision_map: &[Vec<CollisionType>],
    num_cols: usize,
    num_rows: usize,
    transform: &mut Transform,
    vel: &mut Velocity,
    grounded: &mut Grounded,
    input: &PlayerInput,
    px: f32,
    py: f32,
) {
    // Tiles the player's feet span horizontally
    let left_tile = ((px / TILE_SIZE).floor() as i32).max(0) as usize;
    let right_tile = (((px + PLAYER_WIDTH) / TILE_SIZE).floor() as i32).max(0) as usize;

    if vel.y >= 0.0 {
        // Moving downward or stationary: check floor beneath feet
        let foot_y = py + PLAYER_HEIGHT;
        let foot_tile_y = (foot_y / TILE_SIZE).floor() as i32;

        if foot_tile_y >= 0 && (foot_tile_y as usize) < num_rows {
            let ty = foot_tile_y as usize;
            for tx in left_tile..=(right_tile.min(num_cols.saturating_sub(1))) {
                let tile = get_collision(collision_map, num_cols, num_rows, tx, ty);
                match tile {
                    CollisionType::Solid | CollisionType::Ice => {
                        let tile_top = ty as f32 * TILE_SIZE;
                        if foot_y >= tile_top
                            && foot_y <= tile_top + vel.y.abs() + 1.0
                        {
                            transform.translation.y =
                                tile_top - PLAYER_HEIGHT - PLAYER_HEIGHT_OFFSET;
                            vel.y = 0.0;
                            grounded.on_ground = true;
                            if tile == CollisionType::Ice {
                                grounded.on_ice = true;
                            }
                        }
                    }
                    CollisionType::SolidOnTop => {
                        // Drop through: pressing down on a passthrough platform
                        if !input.down {
                            let tile_top = ty as f32 * TILE_SIZE;
                            // Only land if feet are at or just below the tile top
                            if foot_y >= tile_top && foot_y <= tile_top + vel.y.abs() + 1.0 {
                                transform.translation.y =
                                    tile_top - PLAYER_HEIGHT - PLAYER_HEIGHT_OFFSET;
                                vel.y = 0.0;
                                grounded.on_ground = true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    } else {
        // Moving upward: check ceiling above head
        let head_tile_y = ((py / TILE_SIZE).floor() as i32).max(0) as usize;

        if head_tile_y < num_rows {
            for tx in left_tile..=(right_tile.min(num_cols.saturating_sub(1))) {
                let tile = get_collision(collision_map, num_cols, num_rows, tx, head_tile_y);
                if tile == CollisionType::Solid || tile == CollisionType::Ice {
                    let tile_bottom = (head_tile_y as f32 + 1.0) * TILE_SIZE;
                    if py <= tile_bottom {
                        transform.translation.y = tile_bottom - PLAYER_HEIGHT_OFFSET;
                        vel.y = 0.0;
                    }
                }
                // SolidOnTop tiles do NOT block from below
            }
        }
    }
}

/// Checks horizontal collision against solid tiles (walls).
fn check_horizontal_collision(
    collision_map: &[Vec<CollisionType>],
    num_cols: usize,
    num_rows: usize,
    transform: &mut Transform,
    vel: &mut Velocity,
    px: f32,
    py: f32,
) {
    // Tiles the player's body spans vertically
    let top_tile = ((py / TILE_SIZE).floor() as i32).max(0) as usize;
    let bottom_tile = (((py + PLAYER_HEIGHT - 1.0) / TILE_SIZE).floor() as i32).max(0) as usize;

    if vel.x > 0.0 {
        // Moving right: check right edge
        let right_edge = px + PLAYER_WIDTH;
        let right_tile_x = (right_edge / TILE_SIZE).floor() as i32;

        if right_tile_x >= 0 && (right_tile_x as usize) < num_cols {
            let tx = right_tile_x as usize;
            for ty in top_tile..=(bottom_tile.min(num_rows.saturating_sub(1))) {
                let tile = get_collision(collision_map, num_cols, num_rows, tx, ty);
                if tile == CollisionType::Solid || tile == CollisionType::Ice {
                    let tile_left = tx as f32 * TILE_SIZE;
                    if right_edge > tile_left {
                        transform.translation.x = tile_left - PLAYER_WIDTH - PLAYER_WIDTH_OFFSET;
                        vel.x = 0.0;
                    }
                }
            }
        }
    } else if vel.x < 0.0 {
        // Moving left: check left edge
        let left_tile_x = ((px / TILE_SIZE).floor() as i32).max(0) as usize;

        if left_tile_x < num_cols {
            for ty in top_tile..=(bottom_tile.min(num_rows.saturating_sub(1))) {
                let tile = get_collision(collision_map, num_cols, num_rows, left_tile_x, ty);
                if tile == CollisionType::Solid || tile == CollisionType::Ice {
                    let tile_right = (left_tile_x as f32 + 1.0) * TILE_SIZE;
                    if px < tile_right {
                        transform.translation.x = tile_right - PLAYER_WIDTH_OFFSET;
                        vel.x = 0.0;
                    }
                }
            }
        }
    }
}

/// Checks if the player is touching any Death or SuperDeath tiles.
fn check_death_tiles(
    collision_map: &[Vec<CollisionType>],
    num_cols: usize,
    num_rows: usize,
    px: f32,
    py: f32,
) -> bool {
    let left_tile = ((px / TILE_SIZE).floor() as i32).max(0) as usize;
    let right_tile = (((px + PLAYER_WIDTH) / TILE_SIZE).floor() as i32).max(0) as usize;
    let top_tile = ((py / TILE_SIZE).floor() as i32).max(0) as usize;
    let bottom_tile = (((py + PLAYER_HEIGHT) / TILE_SIZE).floor() as i32).max(0) as usize;

    for tx in left_tile..=(right_tile.min(num_cols.saturating_sub(1))) {
        for ty in top_tile..=(bottom_tile.min(num_rows.saturating_sub(1))) {
            let tile = get_collision(collision_map, num_cols, num_rows, tx, ty);
            if tile == CollisionType::Death
                || tile == CollisionType::SuperDeath
                || tile == CollisionType::PlayerDeath
            {
                return true;
            }
        }
    }
    false
}

/// Detects warp zone entry: when a player presses down on a warp entrance tile.
/// Checks against `MapData.warp_data` and `MapData.warp_exits` to find entrance/exit pairs.
pub fn warp_zone_detection(
    mut commands: Commands,
    map_data: Res<MapData>,
    query: Query<
        (
            Entity,
            &Transform,
            &PlayerInput,
            &PlayerAction,
            Option<&WarpState>,
        ),
        With<Player>,
    >,
) {
    if map_data.warp_exits.is_empty() {
        return;
    }

    let has_warp_data =
        !map_data.warp_data.is_empty() && !map_data.warp_data[0].is_empty();
    if !has_warp_data {
        return;
    }

    let warp_cols = map_data.warp_data.len();
    let warp_rows = map_data.warp_data[0].len();

    for (entity, transform, input, action, existing_warp) in query.iter() {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            continue;
        }
        if existing_warp.is_some() {
            continue;
        }
        if !input.down {
            continue;
        }

        // Convert Bevy position to SMW pixel space for tile lookups
        let smw_x = to_smw_x(transform.translation.x);
        let smw_y = to_smw_y(transform.translation.y);
        let center_x = smw_x + PLAYER_WIDTH_OFFSET + PLAYER_WIDTH * 0.5;
        let foot_y = smw_y + PLAYER_HEIGHT_OFFSET + PLAYER_HEIGHT;
        let tile_x_i = (center_x / TILE_SIZE).floor() as i32;
        let tile_y_i = (foot_y / TILE_SIZE).floor() as i32;
        if tile_x_i < 0 || tile_y_i < 0 {
            continue;
        }
        let tile_x = tile_x_i as usize;
        let tile_y = tile_y_i as usize;

        if tile_x >= warp_cols || tile_y >= warp_rows {
            continue;
        }

        let warp_info = &map_data.warp_data[tile_x][tile_y];
        if warp_info.connection < 0 {
            continue; // No warp at this tile
        }

        // Find the matching warp exit by connection id
        let connection = warp_info.connection;
        if let Some(exit) = map_data
            .warp_exits
            .iter()
            .find(|e| e.connection == connection && e.id != warp_info.id)
        {
            // exit.x/y are pixel coordinates (player exit position in SMW space)
            let dest_x = to_bevy_x(exit.x as f32);
            let dest_y = to_bevy_y(exit.y as f32);
            let direction =
                crate::map::components::WarpDirection::from_raw_enter(warp_info.direction);
            let exit_direction = exit.direction;

            commands.entity(entity).insert(WarpState::new_entering(
                direction, exit_direction, warp_info.id as u32, dest_x, dest_y,
            ));
        }
    }
}

/// Entity-to-entity collision detection using Collider components.
/// Sends CollisionEvents when entities with colliders overlap.
pub fn entity_collision_detection(
    query: Query<(Entity, &Transform, &Collider, &CollisionLayer)>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    let entities: Vec<_> = query.iter().collect();

    for i in 0..entities.len() {
        for j in (i + 1)..entities.len() {
            let (e_a, t_a, c_a, l_a) = &entities[i];
            let (e_b, t_b, c_b, l_b) = &entities[j];

            // Skip same-layer collisions for projectiles
            if *l_a == &CollisionLayer::Projectile && *l_b == &CollisionLayer::Projectile {
                continue;
            }

            if let Some(pen) = collision::aabb_penetration(
                &t_a.translation,
                c_a,
                &t_b.translation,
                c_b,
            ) {
                collision_events.send(CollisionEvent {
                    entity_a: *e_a,
                    entity_b: *e_b,
                    layer_a: **l_a,
                    layer_b: **l_b,
                    penetration: pen,
                });
            }
        }
    }
}

/// Applies ride-along velocity from moving platforms to players standing on them.
/// Checks platform proximity independently of tile-based grounding, since players
/// on platforms are not over solid tiles and would otherwise fall through.
pub fn moving_platform_system(
    platform_query: Query<(Entity, &MovingPlatform, &Transform, &Collider)>,
    mut player_query: Query<
        (Entity, &mut Transform, &mut Grounded, &mut Velocity),
        (With<Player>, Without<MovingPlatform>),
    >,
    mut commands: Commands,
) {
    for (player_entity, mut player_transform, mut grounded, mut vel) in player_query.iter_mut() {
        // Skip players moving upward (jumping) — they shouldn't snap to platforms
        if vel.y > 1.0 {
            continue;
        }

        let player_px = player_transform.translation.x;
        // In Bevy Y-up, player's visual bottom is at translation.y (collider offset_y = 0)
        let player_bottom = player_transform.translation.y;

        let mut found_platform = false;

        for (platform_entity, platform, plat_transform, plat_collider) in platform_query.iter() {
            let (plat_left, _plat_min_y, plat_right, plat_max_y) =
                plat_collider.world_rect(&plat_transform.translation);

            // In Bevy Y-up, plat_max_y = visual top of platform
            // Allow slightly larger tolerance for falling players and fast platforms
            let tolerance = if grounded.on_ground { 2.0 } else { 4.0 + vel.y.abs() };
            let upper_tolerance = 2.0 + platform.vel_y.abs();
            let on_top = player_bottom >= plat_max_y - tolerance
                && player_bottom <= plat_max_y + upper_tolerance;
            let horizontally_on =
                player_px + PLAYER_WIDTH > plat_left && player_px < plat_right;

            if on_top && horizontally_on {
                // Snap player to platform top and carry them with the platform
                player_transform.translation.y = plat_max_y;
                player_transform.translation.x += platform.vel_x;
                player_transform.translation.y += platform.vel_y;

                // Zero vertical velocity so gravity doesn't pull player off
                vel.y = 0.0;
                grounded.on_ground = true;

                commands
                    .entity(player_entity)
                    .insert(RidingPlatform { platform_entity });
                found_platform = true;
                break;
            }
        }

        if !found_platform {
            commands.entity(player_entity).remove::<RidingPlatform>();
        }
    }
}

// ---------------------------------------------------------------------------
// Object map collision (enemies, powerups, projectiles)
// ---------------------------------------------------------------------------

/// Applies map tile collision to non-player objects that have InteractsWithMap.
/// Replaces the hardcoded floor/wall checks in individual object systems with
/// proper tile-based collision using the same collision map as player physics.
pub fn object_map_collision(
    map_data: Res<MapData>,
    mut query: Query<
        (&mut ObjectVelocity, &mut Transform, &mut ObjectGrounded, &Collider),
        With<InteractsWithMap>,
    >,
) {
    if map_data.collision_map.is_empty() {
        return;
    }

    let num_cols = map_data.collision_map.len();
    let num_rows = map_data.collision_map[0].len();

    for (mut vel, mut transform, mut grounded, collider) in query.iter_mut() {
        // Object dimensions (use collider size or default to TILE_SIZE)
        let obj_w = collider.width;
        let obj_h = collider.height;

        // Convert Bevy position → SMW pixel space for tile lookups
        let smw_x = to_smw_x(transform.translation.x);
        let smw_y = to_smw_y(transform.translation.y);

        // --- Vertical collision (floor) ---
        // In SMW Y-down: feet = smw_y + obj_h
        let foot_y = smw_y + obj_h;
        let foot_tile_y = (foot_y / TILE_SIZE).floor() as i32;
        let left_tile = ((smw_x / TILE_SIZE).floor() as i32).max(0) as usize;
        let right_tile = (((smw_x + obj_w) / TILE_SIZE).floor() as i32).max(0) as usize;

        grounded.0 = false;

        // Check floor (object moving downward in SMW = vel.y negative in Bevy)
        if vel.y <= 0.0 && foot_tile_y >= 0 && (foot_tile_y as usize) < num_rows {
            let ty = foot_tile_y as usize;
            for tx in left_tile..=(right_tile.min(num_cols.saturating_sub(1))) {
                let tile = get_collision(&map_data.collision_map, num_cols, num_rows, tx, ty);
                match tile {
                    CollisionType::Solid | CollisionType::SolidOnTop | CollisionType::Ice => {
                        let tile_top_smw = ty as f32 * TILE_SIZE;
                        if foot_y >= tile_top_smw && foot_y <= tile_top_smw + vel.y.abs() + 2.0 {
                            // Snap to top of tile (convert back to Bevy)
                            let corrected_smw_y = tile_top_smw - obj_h;
                            transform.translation.y = to_bevy_y(corrected_smw_y);
                            vel.y = 0.0;
                            grounded.0 = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        // --- Vertical collision (ceiling) ---
        // Check ceiling (object moving upward in Bevy = vel.y positive)
        if vel.y > 0.0 {
            let head_smw_y = to_smw_y(transform.translation.y);
            let head_tile_y = (head_smw_y / TILE_SIZE).floor() as i32;
            if head_tile_y >= 0 && (head_tile_y as usize) < num_rows {
                let ty = head_tile_y as usize;
                for tx in left_tile..=(right_tile.min(num_cols.saturating_sub(1))) {
                    let tile = get_collision(&map_data.collision_map, num_cols, num_rows, tx, ty);
                    if tile == CollisionType::Solid || tile == CollisionType::Ice {
                        let tile_bottom_smw = (ty as f32 + 1.0) * TILE_SIZE;
                        if head_smw_y <= tile_bottom_smw {
                            // Push object below the tile (in SMW space)
                            transform.translation.y = to_bevy_y(tile_bottom_smw);
                            vel.y = 0.0;
                        }
                    }
                }
            }
        }

        // --- Horizontal collision (walls) ---
        // Recalculate SMW position after vertical correction
        let smw_x = to_smw_x(transform.translation.x);
        let smw_y = to_smw_y(transform.translation.y);
        let top_tile = ((smw_y / TILE_SIZE).floor() as i32).max(0) as usize;
        let bottom_tile =
            (((smw_y + obj_h - 1.0) / TILE_SIZE).floor() as i32).max(0) as usize;

        if vel.x > 0.0 {
            // Moving right: check right edge
            let right_edge = smw_x + obj_w;
            let right_tile_x = (right_edge / TILE_SIZE).floor() as i32;
            if right_tile_x >= 0 && (right_tile_x as usize) < num_cols {
                let tx = right_tile_x as usize;
                for ty in top_tile..=(bottom_tile.min(num_rows.saturating_sub(1))) {
                    let tile =
                        get_collision(&map_data.collision_map, num_cols, num_rows, tx, ty);
                    if tile == CollisionType::Solid || tile == CollisionType::Ice {
                        let tile_left_smw = tx as f32 * TILE_SIZE;
                        if right_edge >= tile_left_smw {
                            transform.translation.x =
                                to_bevy_x(tile_left_smw - obj_w);
                            vel.x = -vel.x; // Bounce off wall
                        }
                    }
                }
            }
        } else if vel.x < 0.0 {
            // Moving left: check left edge
            let left_tile_x = (smw_x / TILE_SIZE).floor() as i32;
            if left_tile_x >= 0 && (left_tile_x as usize) < num_cols {
                let tx = left_tile_x as usize;
                for ty in top_tile..=(bottom_tile.min(num_rows.saturating_sub(1))) {
                    let tile =
                        get_collision(&map_data.collision_map, num_cols, num_rows, tx, ty);
                    if tile == CollisionType::Solid || tile == CollisionType::Ice {
                        let tile_right_smw = (tx as f32 + 1.0) * TILE_SIZE;
                        if smw_x <= tile_right_smw {
                            transform.translation.x = to_bevy_x(tile_right_smw);
                            vel.x = -vel.x; // Bounce off wall
                        }
                    }
                }
            }
        }

        // --- Fallback: screen-edge floor when below all tiles ---
        if !grounded.0 {
            let floor_y = -(SCREEN_HEIGHT / 2.0 - TILE_SIZE);
            if transform.translation.y <= floor_y {
                transform.translation.y = floor_y;
                vel.y = 0.0;
                grounded.0 = true;
            }
        }
    }
}
