use bevy::prelude::*;

use crate::core::constants::*;
use crate::game_modes::ctf::Flag;
use crate::game_modes::domination::DominationZone;
use crate::game_modes::king_of_the_hill::HillZone;
use crate::game_modes::tag::TaggedIt;
use crate::game_modes::traits::{GameModeConfig, GameModeType};
use crate::map::components::{CollisionType, MapData};
use crate::objects::components::{Collectible, DamageOnContact, GameObject};
use crate::physics::collision::wrapped_distance_x;
use crate::player::components::{
    Grounded, Player, PlayerAction, PlayerDeathEvent, PlayerInput, PowerupState, Velocity,
};

use super::components::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Perception radius in pixels -- ignore objects beyond this distance.
const PERCEPTION_RADIUS: f32 = 400.0;

/// Distance threshold at which the AI considers itself "close enough" to a target.
const ARRIVE_THRESHOLD: f32 = 24.0;

/// How long (seconds) without positional change before the AI is considered stuck.
const STUCK_THRESHOLD_SECS: f32 = 0.8;

/// Minimum distance delta per stuck-check period to count as "making progress".
const STUCK_DISTANCE_THRESHOLD: f32 = 6.0;

/// Jump cooldown in seconds to prevent bunny-hopping.
const JUMP_COOLDOWN_SECS: f32 = 0.35;

/// Item-use cooldown in seconds.
const ITEM_USE_COOLDOWN_SECS: f32 = 0.5;

/// How far ahead (in tiles) the AI checks for obstacles.
const OBSTACLE_LOOKAHEAD_TILES: i32 = 2;

/// Distance at which the AI starts fleeing from hazards.
const HAZARD_FLEE_DISTANCE: f32 = 80.0;

/// Distance at which the AI considers a player "in melee range".
const MELEE_RANGE: f32 = 48.0;

// ---------------------------------------------------------------------------
// Helper: wrapped 2D distance
// ---------------------------------------------------------------------------

fn wrapped_distance_2d(a: Vec2, b: Vec2) -> f32 {
    let dx = wrapped_distance_x(a.x, b.x);
    let dy = b.y - a.y;
    (dx * dx + dy * dy).sqrt()
}

// ---------------------------------------------------------------------------
// Helper: tile queries
// ---------------------------------------------------------------------------

/// Returns the collision type at a given tile coordinate, or NonSolid if out of bounds.
fn tile_collision_at(map: &MapData, tx: i32, ty: i32) -> CollisionType {
    if tx < 0 || ty < 0 || tx >= MAP_WIDTH as i32 || ty >= MAP_HEIGHT as i32 {
        return CollisionType::NonSolid;
    }
    if map.collision_map.is_empty() {
        return CollisionType::NonSolid;
    }
    let ux = tx as usize;
    let uy = ty as usize;
    if ux < map.collision_map.len() && uy < map.collision_map[ux].len() {
        map.collision_map[ux][uy]
    } else {
        CollisionType::NonSolid
    }
}

fn is_solid(ct: CollisionType) -> bool {
    matches!(ct, CollisionType::Solid | CollisionType::SolidOnTop | CollisionType::Ice)
}

fn is_death_tile(ct: CollisionType) -> bool {
    matches!(
        ct,
        CollisionType::Death | CollisionType::SuperDeath | CollisionType::PlayerDeath
    )
}

// =========================================================================
// SYSTEM: Perception Update
// =========================================================================

/// Scans the world for players, items, hazards, and map features near each AI.
pub fn ai_perception_update(
    map_data: Res<MapData>,
    mut ai_query: Query<
        (Entity, &Transform, &Player, &mut AiPerception),
        With<AiControlled>,
    >,
    all_players: Query<(Entity, &Transform, &Player, &PlayerAction), Without<AiControlled>>,
    ai_players: Query<(Entity, &Transform, &Player, &PlayerAction), With<AiControlled>>,
    collectible_query: Query<(Entity, &Transform), With<Collectible>>,
    hazard_query: Query<(&Transform, &DamageOnContact, &GameObject)>,
    config: Res<GameModeConfig>,
    domination_zones: Query<&DominationZone>,
    hill_zones: Query<&HillZone>,
    flags: Query<&Flag>,
    tagged_query: Query<(Entity, &Player), With<TaggedIt>>,
) {
    for (ai_entity, ai_tf, ai_player, mut perception) in ai_query.iter_mut() {
        let ai_pos = ai_tf.translation.truncate();

        // ---- visible players ----
        perception.visible_players.clear();

        // Check non-AI players
        for (entity, tf, player, action) in all_players.iter() {
            if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
                continue;
            }
            let pos = tf.translation.truncate();
            let dist = wrapped_distance_2d(ai_pos, pos);
            if dist > PERCEPTION_RADIUS {
                continue;
            }
            let same_team = player.team == ai_player.team;
            perception.visible_players.push((entity, pos, dist, same_team));
        }

        // Check other AI players (but not self)
        for (entity, tf, player, action) in ai_players.iter() {
            if entity == ai_entity {
                continue;
            }
            if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
                continue;
            }
            let pos = tf.translation.truncate();
            let dist = wrapped_distance_2d(ai_pos, pos);
            if dist > PERCEPTION_RADIUS {
                continue;
            }
            let same_team = player.team == ai_player.team;
            perception.visible_players.push((entity, pos, dist, same_team));
        }

        // Sort by distance ascending
        perception
            .visible_players
            .sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        // ---- visible items ----
        perception.visible_items.clear();
        for (entity, tf) in collectible_query.iter() {
            let pos = tf.translation.truncate();
            let dist = wrapped_distance_2d(ai_pos, pos);
            if dist <= PERCEPTION_RADIUS {
                perception.visible_items.push((entity, pos, dist));
            }
        }
        perception
            .visible_items
            .sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        // ---- nearby hazards ----
        perception.nearby_hazards.clear();
        for (tf, dmg, obj) in hazard_query.iter() {
            if !obj.alive || !dmg.kill {
                continue;
            }
            let pos = tf.translation.truncate();
            let dist = wrapped_distance_2d(ai_pos, pos);
            if dist <= PERCEPTION_RADIUS {
                perception.nearby_hazards.push((pos, dist));
            }
        }
        perception
            .nearby_hazards
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // ---- map tile queries ----
        if map_data.loaded && !map_data.collision_map.is_empty() {
            let tile_x = (ai_pos.x / TILE_SIZE) as i32;
            let tile_y = (ai_pos.y / TILE_SIZE) as i32;

            // Below feet (one tile below center)
            let foot_ty = tile_y + 1;

            // On-edge detection: check one tile ahead on both sides at foot level
            let left_below = tile_collision_at(&map_data, tile_x - 1, foot_ty);
            let right_below = tile_collision_at(&map_data, tile_x + 1, foot_ty);
            let center_below = tile_collision_at(&map_data, tile_x, foot_ty);

            perception.on_edge =
                (is_solid(center_below) && !is_solid(left_below))
                    || (is_solid(center_below) && !is_solid(right_below));

            // Above pit: nothing solid directly below for several tiles
            let mut above_pit = true;
            for dy in 1..=4 {
                let ty = tile_y + dy;
                if ty >= MAP_HEIGHT as i32 {
                    break;
                }
                if is_solid(tile_collision_at(&map_data, tile_x, ty)) {
                    above_pit = false;
                    break;
                }
            }
            perception.above_pit = above_pit;

            // Nearest solid below
            perception.nearest_solid_below = f32::MAX;
            for dy in 1..=(MAP_HEIGHT as i32) {
                let ty = tile_y + dy;
                if ty >= MAP_HEIGHT as i32 {
                    break;
                }
                if is_solid(tile_collision_at(&map_data, tile_x, ty)) {
                    perception.nearest_solid_below = (dy as f32) * TILE_SIZE;
                    break;
                }
            }

            // Path blocked left / right (check tiles at player-height level)
            let mut blocked_left = false;
            let mut blocked_right = false;
            for ahead in 1..=OBSTACLE_LOOKAHEAD_TILES {
                let lt = tile_collision_at(&map_data, tile_x - ahead, tile_y);
                let rt = tile_collision_at(&map_data, tile_x + ahead, tile_y);
                if is_solid(lt) || is_death_tile(lt) {
                    blocked_left = true;
                }
                if is_solid(rt) || is_death_tile(rt) {
                    blocked_right = true;
                }
            }
            perception.path_blocked_left = blocked_left;
            perception.path_blocked_right = blocked_right;
        } else {
            perception.on_edge = false;
            perception.above_pit = false;
            perception.nearest_solid_below = TILE_SIZE;
            perception.path_blocked_left = false;
            perception.path_blocked_right = false;
        }

        // ---- objective position (mode-specific) ----
        perception.objective_position = determine_objective_position(
            &config,
            ai_player,
            ai_pos,
            &domination_zones,
            &hill_zones,
            &flags,
            &tagged_query,
        );
    }
}

/// Determines the mode-specific objective position for this AI player.
fn determine_objective_position(
    config: &GameModeConfig,
    ai_player: &Player,
    ai_pos: Vec2,
    domination_zones: &Query<&DominationZone>,
    hill_zones: &Query<&HillZone>,
    flags: &Query<&Flag>,
    tagged_query: &Query<(Entity, &Player), With<TaggedIt>>,
) -> Option<Vec2> {
    match config.mode_type {
        GameModeType::Domination => {
            // Find the nearest unclaimed or enemy-owned zone
            let mut best: Option<(Vec2, f32)> = None;
            for zone in domination_zones.iter() {
                let owned_by_us = zone.owner_team == Some(ai_player.team);
                if owned_by_us {
                    continue;
                }
                let dist = wrapped_distance_2d(ai_pos, zone.center);
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((zone.center, dist));
                }
            }
            // If all zones are ours, defend the nearest one
            if best.is_none() {
                for zone in domination_zones.iter() {
                    let dist = wrapped_distance_2d(ai_pos, zone.center);
                    if best.is_none() || dist < best.unwrap().1 {
                        best = Some((zone.center, dist));
                    }
                }
            }
            best.map(|(pos, _)| pos)
        }
        GameModeType::KingOfTheHill => {
            // Go to the hill
            hill_zones.iter().next().map(|h| h.center)
        }
        GameModeType::CaptureTheFlag => {
            // If carrying enemy flag -> go to own base
            // Otherwise -> go to enemy flag position
            let carrying_flag = flags
                .iter()
                .any(|f| f.carried_by == Some(ai_player.index) && f.team != ai_player.team);

            if carrying_flag {
                // Find own team's flag base
                flags
                    .iter()
                    .find(|f| f.team == ai_player.team)
                    .map(|f| f.base_position)
            } else {
                // Find enemy flag that is at its base (or dropped)
                flags
                    .iter()
                    .find(|f| f.team != ai_player.team && f.carried_by.is_none())
                    .map(|f| f.base_position)
            }
        }
        GameModeType::Tag => {
            // If we are "it", no specific objective position (chase players).
            // If not "it", no objective (flee from "it").
            let _we_are_it = tagged_query
                .iter()
                .any(|(_, p)| p.index == ai_player.index);
            None
        }
        _ => None,
    }
}

// =========================================================================
// SYSTEM: Memory Update
// =========================================================================

/// Tracks last-known player positions, detects stuck state, and ticks cooldowns.
pub fn ai_memory_update(
    time: Res<Time>,
    mut query: Query<
        (&Transform, &mut AiMemory, &AiPerception, &PlayerAction),
        With<AiControlled>,
    >,
) {
    let dt = time.delta_secs();

    for (tf, mut memory, perception, action) in query.iter_mut() {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            memory.stuck_timer = 0.0;
            continue;
        }

        let pos = tf.translation.truncate();

        // Record visible players into the position array by their order of distance
        for (i, &(_, player_pos, _, _)) in perception.visible_players.iter().enumerate() {
            if i >= MAX_PLAYERS {
                break;
            }
            memory.last_known_positions[i] = Some((player_pos, 0.0));
        }
        // Age entries that were not updated
        for i in perception.visible_players.len()..MAX_PLAYERS {
            if let Some((_, ref mut age)) = memory.last_known_positions[i] {
                *age += dt;
                // Forget after 5 seconds
                if *age > 5.0 {
                    memory.last_known_positions[i] = None;
                }
            }
        }

        // Stuck detection
        let moved = (pos - memory.last_position).length();
        if moved < STUCK_DISTANCE_THRESHOLD {
            memory.stuck_timer += dt;
        } else {
            memory.stuck_timer = 0.0;
        }
        memory.last_position = pos;

        // Tick cooldowns
        memory.jump_cooldown = (memory.jump_cooldown - dt).max(0.0);
        memory.item_use_cooldown = (memory.item_use_cooldown - dt).max(0.0);
    }
}

/// Records the attacker when the AI gets killed (for revenge targeting on respawn).
pub fn ai_record_death(
    mut death_events: EventReader<PlayerDeathEvent>,
    mut query: Query<(&Player, &mut AiMemory), With<AiControlled>>,
) {
    for event in death_events.read() {
        for (player, mut memory) in query.iter_mut() {
            if player.index == event.player_index {
                memory.last_hit_by = event.killer_index;
            }
        }
    }
}

// =========================================================================
// SYSTEM: Mode Awareness
// =========================================================================

/// Adjusts perception objective and potentially the current goal based on the
/// active game mode. Runs before goal selection so mode context is available.
pub fn ai_mode_awareness(
    config: Res<GameModeConfig>,
    tagged_query: Query<&Player, With<TaggedIt>>,
    flags: Query<&Flag>,
    mut query: Query<
        (&Player, &mut AiIntent, &AiPerception, &PowerupState),
        With<AiControlled>,
    >,
) {
    for (player, mut intent, perception, powerup) in query.iter_mut() {
        match config.mode_type {
            GameModeType::CaptureTheFlag => {
                let carrying_enemy_flag = flags
                    .iter()
                    .any(|f| f.carried_by == Some(player.index) && f.team != player.team);

                if carrying_enemy_flag {
                    // Priority: return flag to our base
                    if let Some(obj_pos) = perception.objective_position {
                        intent.current_goal = AiGoal::ReturnFlag(obj_pos);
                        continue;
                    }
                }
                // Otherwise, perception already set the objective to enemy flag
                if let Some(obj_pos) = perception.objective_position {
                    intent.current_goal = AiGoal::CaptureObjective(obj_pos);
                }
            }
            GameModeType::Tag => {
                let we_are_it = tagged_query
                    .iter()
                    .any(|p| p.index == player.index);

                if we_are_it {
                    // Chase the nearest non-team player
                    if let Some(&(entity, _, _, _)) = perception
                        .visible_players
                        .iter()
                        .find(|(_, _, _, same_team)| !same_team)
                    {
                        intent.current_goal = AiGoal::Chase(entity);
                    }
                } else {
                    // Flee from "it"
                    if let Some(&(entity, _, _, _)) = perception
                        .visible_players
                        .first()
                    {
                        intent.current_goal = AiGoal::Flee(entity);
                    }
                }
            }
            GameModeType::Domination | GameModeType::KingOfTheHill => {
                if let Some(obj_pos) = perception.objective_position {
                    intent.current_goal = AiGoal::CaptureObjective(obj_pos);
                }
            }
            GameModeType::CoinCollection => {
                // Bias toward collecting items
                if let Some(&(entity, _, _dist)) = perception.visible_items.first() {
                    intent.current_goal = AiGoal::CollectItem(entity);
                }
            }
            GameModeType::Frenzy => {
                // Bias toward collecting powerups; also be aggressive
                if *powerup == PowerupState::None {
                    if let Some(&(entity, _, _)) = perception.visible_items.first() {
                        intent.current_goal = AiGoal::CollectItem(entity);
                    }
                }
            }
            GameModeType::Survival => {
                // Play more conservatively: weight flee higher when lives are low
                if player.lives <= 2 {
                    if let Some(&(entity, _, dist, _same_team)) = perception
                        .visible_players
                        .iter()
                        .find(|(_, _, _, same_team)| !same_team)
                    {
                        if dist < MELEE_RANGE * 2.0 {
                            intent.current_goal = AiGoal::Flee(entity);
                        }
                    }
                }
            }
            _ => {
                // Classic / Frag / TimeLimit / Stomp: default combat behavior
                // handled by goal selection
            }
        }
    }
}

// =========================================================================
// SYSTEM: Goal Selection
// =========================================================================

/// Chooses a high-level goal based on perception, memory, and difficulty.
pub fn ai_goal_selection(
    time: Res<Time>,
    mut query: Query<(
        &mut AiControlled,
        &mut AiIntent,
        &AiPerception,
        &AiMemory,
        &Transform,
        &PowerupState,
        &Player,
        &PlayerAction,
    )>,
) {
    let dt = time.delta_secs();

    for (mut ai, mut intent, perception, memory, tf, powerup, _player, action) in query.iter_mut() {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            intent.move_direction = 0.0;
            intent.want_jump = false;
            intent.want_turbo = false;
            intent.want_down = false;
            intent.want_use_item = false;
            continue;
        }

        // Decision timer gating -- only re-evaluate periodically
        ai.decision_timer += dt;
        if ai.decision_timer < ai.decision_interval {
            continue;
        }
        ai.decision_timer = 0.0;

        // Tick reaction cooldown
        ai.reaction_cooldown = (ai.reaction_cooldown - ai.decision_interval).max(0.0);

        let _ai_pos = tf.translation.truncate();

        // ---- Phase 1: Immediate hazard avoidance (overrides everything) ----
        if let Some(&(hazard_pos, hazard_dist)) = perception.nearby_hazards.first() {
            if hazard_dist < HAZARD_FLEE_DISTANCE {
                intent.current_goal = AiGoal::AvoidHazard(hazard_pos);
                continue;
            }
        }

        // ---- Phase 2: Check if mode awareness already set a strong goal ----
        // Keep mode-set goals for CTF flag carrying, Tag, etc.
        let mode_goal_is_strong = matches!(
            intent.current_goal,
            AiGoal::ReturnFlag(_)
        );
        if mode_goal_is_strong {
            continue;
        }

        // ---- Phase 3: If stuck, pick an unstuck goal ----
        if memory.stuck_timer > STUCK_THRESHOLD_SECS {
            // Reverse patrol direction and jump to try to get unstuck
            intent.current_goal = AiGoal::Patrol;
            continue;
        }

        // ---- Phase 4: Item collection vs combat ----
        let nearest_enemy = perception
            .visible_players
            .iter()
            .find(|(_, _, _, same_team)| !*same_team);
        let nearest_item = perception.visible_items.first();

        // Score items vs combat
        let item_score = if let Some(&(_, _, item_dist)) = nearest_item {
            // Closer items are more attractive; difficulty scales priority
            let base = 1.0 - (item_dist / PERCEPTION_RADIUS).min(1.0);
            base * ai.item_priority
        } else {
            0.0
        };

        let combat_score = if let Some(&(_, _, enemy_dist, _)) = nearest_enemy {
            let base = 1.0 - (enemy_dist / PERCEPTION_RADIUS).min(1.0);
            base * ai.aggression
        } else {
            0.0
        };

        // Check if we have no powerup and should get one first
        let want_powerup = *powerup == PowerupState::None && item_score > 0.2;

        // ---- Phase 5: Choose goal ----
        // Mode-specific objectives get a bonus
        let objective_score = if perception.objective_position.is_some() {
            0.6 // moderate pull toward objectives
        } else {
            0.0
        };

        if want_powerup && item_score > combat_score && item_score > objective_score {
            if let Some(&(entity, _, _)) = nearest_item {
                intent.current_goal = AiGoal::CollectItem(entity);
                continue;
            }
        }

        if objective_score > combat_score && objective_score > item_score {
            if let Some(obj_pos) = perception.objective_position {
                intent.current_goal = AiGoal::CaptureObjective(obj_pos);
                continue;
            }
        }

        if let Some(&(entity, _, enemy_dist, _)) = nearest_enemy {
            // Flee if we have no powerup and enemy is very close, and we're not aggressive
            let should_flee = *powerup == PowerupState::None
                && enemy_dist < MELEE_RANGE
                && ai.aggression < 0.5;

            if should_flee {
                intent.current_goal = AiGoal::Flee(entity);
            } else {
                intent.current_goal = AiGoal::Chase(entity);
            }
            continue;
        }

        // If items are visible, go collect
        if let Some(&(entity, _, _)) = nearest_item {
            intent.current_goal = AiGoal::CollectItem(entity);
            continue;
        }

        // Nothing visible -- patrol
        intent.current_goal = AiGoal::Patrol;
    }
}

// =========================================================================
// SYSTEM: Navigation
// =========================================================================

/// Converts the current AiGoal into concrete movement intentions (direction, jump, etc.).
pub fn ai_navigation(
    map_data: Res<MapData>,
    all_transforms: Query<&Transform>,
    mut query: Query<(
        &mut AiIntent,
        &mut AiPathfinding,
        &AiPerception,
        &AiMemory,
        &AiControlled,
        &Transform,
        &Grounded,
        &Velocity,
        &PlayerAction,
    )>,
) {
    for (mut intent, mut pathfinding, perception, memory, ai, tf, grounded, vel, action) in
        query.iter_mut()
    {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            continue;
        }

        let ai_pos = tf.translation.truncate();

        // Determine the navigation target position based on the current goal
        let nav_target: Option<Vec2> = match &intent.current_goal {
            AiGoal::Chase(entity) | AiGoal::Flee(entity) | AiGoal::CollectItem(entity) => {
                all_transforms
                    .get(*entity)
                    .ok()
                    .map(|t| t.translation.truncate())
            }
            AiGoal::CaptureObjective(pos)
            | AiGoal::ReturnFlag(pos)
            | AiGoal::DefendZone(pos)
            | AiGoal::AvoidHazard(pos) => Some(*pos),
            AiGoal::Patrol => None,
        };

        pathfinding.target_position = nav_target;

        // Clone the goal so we can match on it without borrowing intent immutably.
        let goal = intent.current_goal.clone();

        match goal {
            AiGoal::Patrol => {
                navigate_patrol(&mut intent, &ai_pos, perception, memory, &map_data, grounded);
            }
            AiGoal::AvoidHazard(hazard_pos) => {
                navigate_avoid(&mut intent, &ai_pos, hazard_pos, perception, grounded, &map_data);
            }
            AiGoal::Flee(_) => {
                if let Some(target_pos) = nav_target {
                    navigate_flee(
                        &mut intent,
                        &ai_pos,
                        target_pos,
                        perception,
                        grounded,
                        ai,
                        &map_data,
                    );
                } else {
                    navigate_patrol(&mut intent, &ai_pos, perception, memory, &map_data, grounded);
                }
            }
            AiGoal::Chase(_) => {
                if let Some(target_pos) = nav_target {
                    navigate_toward(
                        &mut intent,
                        &mut pathfinding,
                        &ai_pos,
                        target_pos,
                        perception,
                        grounded,
                        vel,
                        ai,
                        &map_data,
                        true,
                    );
                }
            }
            AiGoal::CollectItem(_)
            | AiGoal::CaptureObjective(_)
            | AiGoal::ReturnFlag(_)
            | AiGoal::DefendZone(_) => {
                if let Some(target_pos) = nav_target {
                    navigate_toward(
                        &mut intent,
                        &mut pathfinding,
                        &ai_pos,
                        target_pos,
                        perception,
                        grounded,
                        vel,
                        ai,
                        &map_data,
                        false,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Navigation helpers
// ---------------------------------------------------------------------------

fn navigate_patrol(
    intent: &mut AiIntent,
    ai_pos: &Vec2,
    perception: &AiPerception,
    memory: &AiMemory,
    _map_data: &MapData,
    grounded: &Grounded,
) {
    let dir = memory.patrol_direction;
    intent.move_direction = dir;
    intent.want_turbo = false;
    intent.want_jump = false;
    intent.want_down = false;

    // Reverse at screen edges
    if ai_pos.x < TILE_SIZE * 2.0 && dir < 0.0 {
        intent.move_direction = 1.0;
    } else if ai_pos.x > SCREEN_WIDTH - TILE_SIZE * 2.0 && dir > 0.0 {
        intent.move_direction = -1.0;
    }

    // Reverse at blocked paths
    if perception.path_blocked_left && dir < 0.0 {
        intent.move_direction = 1.0;
        intent.want_jump = grounded.on_ground;
    } else if perception.path_blocked_right && dir > 0.0 {
        intent.move_direction = -1.0;
        intent.want_jump = grounded.on_ground;
    }

    // Don't walk into pits
    if perception.on_edge && perception.above_pit {
        intent.move_direction = -intent.move_direction;
    }

    // Jump if stuck
    if memory.stuck_timer > STUCK_THRESHOLD_SECS && grounded.on_ground {
        intent.want_jump = true;
        intent.move_direction = -intent.move_direction;
    }
}

fn navigate_avoid(
    intent: &mut AiIntent,
    ai_pos: &Vec2,
    hazard_pos: Vec2,
    _perception: &AiPerception,
    grounded: &Grounded,
    _map_data: &MapData,
) {
    // Move away from the hazard
    let dx = wrapped_distance_x(ai_pos.x, hazard_pos.x);
    let flee_dir = if dx.abs() < 1.0 {
        // Hazard is right on top -- pick a direction
        if ai_pos.x < SCREEN_WIDTH * 0.5 { 1.0 } else { -1.0 }
    } else {
        -dx.signum()
    };
    intent.move_direction = flee_dir;
    intent.want_turbo = true;

    // Jump if hazard is at the same height or we're on edge
    let dy = hazard_pos.y - ai_pos.y;
    if dy.abs() < TILE_SIZE * 2.0 && grounded.on_ground {
        intent.want_jump = true;
    }
}

fn navigate_flee(
    intent: &mut AiIntent,
    ai_pos: &Vec2,
    threat_pos: Vec2,
    perception: &AiPerception,
    grounded: &Grounded,
    _ai: &AiControlled,
    _map_data: &MapData,
) {
    let dx = wrapped_distance_x(ai_pos.x, threat_pos.x);
    let flee_dir = if dx.abs() < 1.0 {
        if ai_pos.x < SCREEN_WIDTH * 0.5 { 1.0 } else { -1.0 }
    } else {
        -dx.signum()
    };

    intent.move_direction = flee_dir;
    intent.want_turbo = true;

    // Jump if blocked or near edges
    if (perception.path_blocked_left && flee_dir < 0.0)
        || (perception.path_blocked_right && flee_dir > 0.0)
    {
        intent.want_jump = grounded.on_ground;
    }

    // Don't flee into pits
    if perception.on_edge && perception.above_pit {
        intent.move_direction = -intent.move_direction;
        intent.want_jump = grounded.on_ground;
    }
}

fn navigate_toward(
    intent: &mut AiIntent,
    pathfinding: &mut AiPathfinding,
    ai_pos: &Vec2,
    target_pos: Vec2,
    perception: &AiPerception,
    grounded: &Grounded,
    _vel: &Velocity,
    ai: &AiControlled,
    map_data: &MapData,
    is_combat: bool,
) {
    let dx = wrapped_distance_x(ai_pos.x, target_pos.x);
    let dy = target_pos.y - ai_pos.y;
    let abs_dx = dx.abs();

    // Apply accuracy jitter: less accurate AIs add noise to direction
    let direction = if abs_dx < ARRIVE_THRESHOLD {
        // Close enough horizontally
        0.0
    } else {
        let base_dir = dx.signum();
        // At lower accuracy, occasionally choose wrong direction
        let noise = rand::random::<f32>();
        if noise > ai.accuracy {
            -base_dir // mistake
        } else {
            base_dir
        }
    };

    intent.move_direction = direction;

    // Turbo when far away
    intent.want_turbo = abs_dx > TILE_SIZE * 4.0;

    // Jump decisions
    pathfinding.jump_needed = false;
    pathfinding.should_drop_through = false;

    if grounded.on_ground {
        // Jump if target is above us
        if dy < -TILE_SIZE && abs_dx < TILE_SIZE * 4.0 {
            pathfinding.jump_needed = true;
        }

        // Jump if obstacle ahead
        if direction > 0.0 && perception.path_blocked_right {
            pathfinding.jump_needed = true;
        } else if direction < 0.0 && perception.path_blocked_left {
            pathfinding.jump_needed = true;
        }

        // Jump over gaps when chasing
        if perception.on_edge && perception.above_pit && abs_dx > TILE_SIZE {
            pathfinding.jump_needed = true;
            intent.want_turbo = true;
        }
    }

    // Drop through platforms when target is significantly below
    if grounded.on_ground && dy > TILE_SIZE * 2.0 && abs_dx < TILE_SIZE * 3.0 {
        // Check if the tile below is a SolidOnTop platform
        let tile_x = (ai_pos.x / TILE_SIZE) as i32;
        let tile_y = (ai_pos.y / TILE_SIZE) as i32 + 1;
        let below = tile_collision_at(map_data, tile_x, tile_y);
        if below == CollisionType::SolidOnTop {
            pathfinding.should_drop_through = true;
        }
    }

    intent.want_jump = pathfinding.jump_needed;
    intent.want_down = pathfinding.should_drop_through;

    // Combat: use item when in range and have a powerup
    if is_combat && abs_dx < TILE_SIZE * 5.0 && abs_dx > 0.0 {
        intent.want_use_item = true;
    }
}

// =========================================================================
// SYSTEM: Apply Intent -> PlayerInput
// =========================================================================

/// Converts `AiIntent` into `PlayerInput` so that the normal movement systems
/// process the AI player identically to a human player.
pub fn ai_apply_intent(
    mut query: Query<(
        &AiIntent,
        &AiControlled,
        &mut AiMemory,
        &mut PlayerInput,
        &Grounded,
        &PlayerAction,
    )>,
) {
    for (intent, ai, mut memory, mut input, grounded, action) in query.iter_mut() {
        if *action == PlayerAction::Dead || *action == PlayerAction::Spawning {
            // Clear input for dead/spawning players
            *input = PlayerInput::default();
            continue;
        }

        // Difficulty-based input dropping: randomly skip inputs to simulate mistakes
        let drop = rand::random::<f32>() < ai.difficulty.input_drop_chance();
        if drop {
            // On a dropped frame, do nothing (release all buttons)
            *input = PlayerInput::default();
            continue;
        }

        // Movement direction -> left/right booleans
        input.left = intent.move_direction < -0.1;
        input.right = intent.move_direction > 0.1;

        // Jump with cooldown to prevent bunny-hopping
        let can_jump = memory.jump_cooldown <= 0.0 && grounded.on_ground;
        if intent.want_jump && can_jump {
            input.jump = true;
            input.jump_pressed = true;
            memory.jump_cooldown = JUMP_COOLDOWN_SECS;
        } else if intent.want_jump && !grounded.on_ground {
            // Hold jump while in the air for variable jump height
            input.jump = true;
            input.jump_pressed = false;
        } else {
            input.jump = false;
            input.jump_pressed = false;
        }

        input.down = intent.want_down;
        input.turbo = intent.want_turbo;

        // Item use with cooldown
        if intent.want_use_item && memory.item_use_cooldown <= 0.0 {
            input.use_item = true;
            memory.item_use_cooldown = ITEM_USE_COOLDOWN_SECS;
        } else {
            input.use_item = false;
        }

        // Never press pause
        input.pause_pressed = false;

        // Update patrol direction in memory based on movement
        if intent.move_direction.abs() > 0.1 {
            memory.patrol_direction = intent.move_direction.signum();
        }

        // Stuck detection response: if stuck, force a direction change + jump
        if memory.stuck_timer > STUCK_THRESHOLD_SECS * 2.0 && grounded.on_ground {
            // Emergency unstuck: jump and reverse
            input.jump = true;
            input.jump_pressed = true;
            input.left = !input.left;
            input.right = !input.right;
            memory.jump_cooldown = JUMP_COOLDOWN_SECS;
        }
    }
}
