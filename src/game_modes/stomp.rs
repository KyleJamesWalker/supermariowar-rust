use bevy::prelude::*;

use crate::core::constants::*;
use crate::objects::components::EnemyDeathEvent;
use crate::objects::enemies::{Enemy, EnemyType, spawn_enemy};
use crate::player::components::PlayerDeathEvent;
use super::traits::*;

/// Resource tracking state for Stomp mode.
/// Only kills via stomping count -- projectile kills do not award points.
#[derive(Resource, Debug)]
pub struct StompModeState {
    pub stomps: [u32; MAX_PLAYERS],
    /// Current combo streak per player (resets when touching ground without a stomp).
    pub combo: [u32; MAX_PLAYERS],
    /// Cumulative bonus points earned from combos.
    pub combo_bonus: [u32; MAX_PLAYERS],
    /// Timer for enemy spawning (seconds until next spawn).
    pub spawn_timer: f32,
    /// Whether initial enemies have been spawned.
    pub initialized: bool,
}

impl Default for StompModeState {
    fn default() -> Self {
        Self {
            stomps: [0; MAX_PLAYERS],
            combo: [0; MAX_PLAYERS],
            combo_bonus: [0; MAX_PLAYERS],
            spawn_timer: 0.0,
            initialized: false,
        }
    }
}

/// C++ stomp mode spawn rate: spawns every 60-120 frames (~1-2 seconds at 60fps).
const STOMP_SPAWN_RATE_MIN: f32 = 1.0;
const STOMP_SPAWN_RATE_MAX: f32 = 2.0;
/// Maximum enemies on screen at once.
const STOMP_MAX_ENEMIES: usize = 12;

/// Enemy weights for stomp mode (matching C++ defaults).
/// [Goomba, Koopa, CheepCheep, KoopaRed, Spiny, BuzzyBeetle, Paragoomba, Parakoopa]
const STOMP_ENEMY_WEIGHTS: [(EnemyType, u32); 8] = [
    (EnemyType::Goomba, 5),
    (EnemyType::Koopa, 3),
    (EnemyType::CheepCheep, 1),
    (EnemyType::KoopaRed, 3),
    (EnemyType::Spiny, 2),
    (EnemyType::BuzzyBeetle, 2),
    (EnemyType::Paragoomba, 1),
    (EnemyType::Parakoopa, 1),
];

/// Spawns enemies periodically during Stomp mode (like C++ Stomp::think()).
pub fn stomp_enemy_spawn_system(
    time: Res<Time>,
    config: Res<GameModeConfig>,
    mut state: ResMut<StompModeState>,
    mut commands: Commands,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    if config.mode_type != GameModeType::Stomp {
        return;
    }

    // Don't exceed max enemies on screen
    let current_enemy_count = enemy_query.iter().count();
    if current_enemy_count >= STOMP_MAX_ENEMIES {
        return;
    }

    state.spawn_timer -= time.delta_secs();

    if !state.initialized {
        // Spawn a few initial enemies immediately
        state.initialized = true;
        for _ in 0..3 {
            spawn_random_stomp_enemy(&mut commands);
        }
        state.spawn_timer = rand::random::<f32>() * (STOMP_SPAWN_RATE_MAX - STOMP_SPAWN_RATE_MIN) + STOMP_SPAWN_RATE_MIN;
        return;
    }

    if state.spawn_timer <= 0.0 {
        spawn_random_stomp_enemy(&mut commands);
        // Reset timer: random interval matching C++ behavior
        state.spawn_timer = rand::random::<f32>() * (STOMP_SPAWN_RATE_MAX - STOMP_SPAWN_RATE_MIN) + STOMP_SPAWN_RATE_MIN;
    }
}

/// Picks a weighted random enemy type and spawns it at a random screen-top position.
fn spawn_random_stomp_enemy(commands: &mut Commands) {
    let total_weight: u32 = STOMP_ENEMY_WEIGHTS.iter().map(|(_, w)| w).sum();
    let mut roll = rand::random::<u32>() % total_weight;
    let mut selected_type = EnemyType::Goomba;
    for (etype, weight) in &STOMP_ENEMY_WEIGHTS {
        if roll < *weight {
            selected_type = *etype;
            break;
        }
        roll -= *weight;
    }

    // Random X position across screen, spawn at top
    let x = (rand::random::<f32>() * SCREEN_WIDTH) - SCREEN_WIDTH / 2.0;
    let y = SCREEN_HEIGHT / 2.0 - TILE_SIZE; // Just below top edge

    let enemy = Enemy::new(selected_type);
    spawn_enemy(commands, enemy, x, y);
}

/// Processes EnemyDeathEvent -- ENEMY kills score in Stomp mode, not player kills.
/// In the original C++, playerkilledplayer returns Normal with no scoring.
/// Scoring comes exclusively from killing spawned enemies via playerextraguy.
pub fn handle_stomp_kills(
    config: Res<GameModeConfig>,
    mut state: ResMut<StompModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut enemy_death_events: EventReader<EnemyDeathEvent>,
    mut _kill_events: EventReader<PlayerKillEvent>,
) {
    if config.mode_type != GameModeType::Stomp {
        return;
    }

    // Drain player kill events without scoring (player-vs-player kills don't count)
    for _event in _kill_events.read() {}

    // Score ENEMY kills
    for event in enemy_death_events.read() {
        let Some(killer_idx) = event.killer_player else {
            continue;
        };
        if killer_idx >= MAX_PLAYERS {
            continue;
        }

        state.stomps[killer_idx] += 1;
        state.combo[killer_idx] += 1;

        // Combo bonus: each consecutive stomp without landing gives +1 extra point
        let combo_count = state.combo[killer_idx];
        let bonus = if combo_count > 1 { combo_count - 1 } else { 0 };
        state.combo_bonus[killer_idx] += bonus;

        let total_points = 1 + bonus as i32;
        scoreboard.kills[killer_idx] += 1;
        scoreboard.score[killer_idx] += total_points;
    }
}

/// Reset combo when a player lands on the ground (detected via PlayerDeathEvent for victim,
/// or could be driven by a grounded event in the future). For now, we reset combo on death.
pub fn reset_stomp_combo_on_death(
    config: Res<GameModeConfig>,
    mut state: ResMut<StompModeState>,
    mut death_events: EventReader<PlayerDeathEvent>,
) {
    if config.mode_type != GameModeType::Stomp {
        return;
    }

    for event in death_events.read() {
        if event.player_index < MAX_PLAYERS {
            state.combo[event.player_index] = 0;
        }
    }
}

/// Checks if any player has reached the kill limit via stomps.
pub fn check_stomp_win_condition(
    config: Res<GameModeConfig>,
    state: Res<StompModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::Stomp {
        return;
    }

    for (i, &stomps) in state.stomps.iter().enumerate() {
        if stomps >= config.kill_limit {
            round_over.send(RoundOverEvent {
                winning_player: Some(i),
                winning_team: None,
            });
            return;
        }
    }
}
