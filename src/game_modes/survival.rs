use bevy::prelude::*;

use crate::core::constants::*;
use crate::objects::enemies::{Enemy, EnemyType, spawn_enemy};
use crate::player::components::{DeathCause, PlayerDeathEvent};
use super::traits::*;

/// Resource tracking state for Survival mode.
/// Each player starts with N lives. Last player standing wins.
#[derive(Resource, Debug)]
pub struct SurvivalModeState {
    pub lives: [i32; MAX_PLAYERS],
    /// Whether the player is still in the game (not eliminated).
    pub alive: [bool; MAX_PLAYERS],
    /// Number of players that are still in the game.
    pub alive_count: u32,
    /// Total number of players that started this round.
    pub total_players: u32,
    /// Track eliminations for scoring.
    pub eliminations: [u32; MAX_PLAYERS],
    pub initialised: bool,
    /// Timer for enemy spawning.
    pub spawn_timer: f32,
}

impl Default for SurvivalModeState {
    fn default() -> Self {
        Self {
            lives: [0; MAX_PLAYERS],
            alive: [false; MAX_PLAYERS],
            alive_count: 0,
            total_players: 0,
            eliminations: [0; MAX_PLAYERS],
            initialised: false,
            spawn_timer: 2.0,
        }
    }
}

/// Survival enemy spawn rate (seconds between spawns).
const SURVIVAL_SPAWN_RATE_MIN: f32 = 1.5;
const SURVIVAL_SPAWN_RATE_MAX: f32 = 3.0;
/// Max enemies on screen during survival.
const SURVIVAL_MAX_ENEMIES: usize = 8;

/// Spawns hazard enemies during Survival mode (C++ spawns Thwomps, Podoboos, BowserFire).
/// Since we have Goomba/Spiny/BuzzyBeetle types, we spawn a mix of dangerous enemies.
pub fn survival_spawn_enemies(
    time: Res<Time>,
    config: Res<GameModeConfig>,
    mut state: ResMut<SurvivalModeState>,
    mut commands: Commands,
    enemy_query: Query<Entity, With<Enemy>>,
) {
    if config.mode_type != GameModeType::Survival {
        return;
    }
    if !state.initialised {
        return;
    }

    let current_count = enemy_query.iter().count();
    if current_count >= SURVIVAL_MAX_ENEMIES {
        return;
    }

    state.spawn_timer -= time.delta_secs();
    if state.spawn_timer > 0.0 {
        return;
    }

    // Reset timer
    state.spawn_timer = rand::random::<f32>() * (SURVIVAL_SPAWN_RATE_MAX - SURVIVAL_SPAWN_RATE_MIN)
        + SURVIVAL_SPAWN_RATE_MIN;

    // Survival enemies are more dangerous: Spiny (unstompable), BuzzyBeetle, Goomba
    let weights: [(EnemyType, u32); 5] = [
        (EnemyType::Spiny, 4),
        (EnemyType::BuzzyBeetle, 3),
        (EnemyType::Goomba, 3),
        (EnemyType::Koopa, 2),
        (EnemyType::KoopaRed, 2),
    ];
    let total: u32 = weights.iter().map(|(_, w)| w).sum();
    let mut roll = rand::random::<u32>() % total;
    let mut selected = EnemyType::Goomba;
    for (etype, weight) in &weights {
        if roll < *weight {
            selected = *etype;
            break;
        }
        roll -= *weight;
    }

    // Spawn at random X, top of screen
    let x = (rand::random::<f32>() * SCREEN_WIDTH) - SCREEN_WIDTH / 2.0;
    let y = SCREEN_HEIGHT / 2.0 - TILE_SIZE;

    let enemy = Enemy::new(selected);
    spawn_enemy(&mut commands, enemy, x, y);
}

/// Initialise lives for all players at the start of a round.
pub fn survival_init(
    config: Res<GameModeConfig>,
    mut state: ResMut<SurvivalModeState>,
    players: Query<&crate::player::components::Player>,
) {
    if config.mode_type != GameModeType::Survival {
        return;
    }
    if state.initialised {
        return;
    }

    let mut count = 0u32;
    for player in players.iter() {
        if player.index < MAX_PLAYERS {
            state.lives[player.index] = config.lives as i32;
            state.alive[player.index] = true;
            count += 1;
        }
    }
    state.alive_count = count;
    state.total_players = count;
    state.initialised = true;
}

/// When a player is killed, decrement their lives. If they hit 0, they are eliminated.
pub fn handle_survival_death(
    config: Res<GameModeConfig>,
    mut state: ResMut<SurvivalModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut death_events: EventReader<PlayerDeathEvent>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::Survival {
        return;
    }

    for event in death_events.read() {
        let idx = event.player_index;
        if idx >= MAX_PLAYERS || !state.alive[idx] {
            continue;
        }

        state.lives[idx] -= 1;
        scoreboard.deaths[idx] += 1;

        if state.lives[idx] <= 0 {
            state.alive[idx] = false;
            state.alive_count = state.alive_count.saturating_sub(1);

            // Credit the killer with an elimination point
            if let DeathCause::Killed { by } = event.cause {
                if by < MAX_PLAYERS {
                    state.eliminations[by] += 1;
                    scoreboard.score[by] += 1;
                    scoreboard.objective_count[by] += 1;
                }
            }
        }

        // Check if only one (or zero) players remain
        if state.alive_count <= 1 && state.total_players > 1 {
            let winner = state
                .alive
                .iter()
                .enumerate()
                .find(|(_, &a)| a)
                .map(|(i, _)| i);

            round_over.send(RoundOverEvent {
                winning_player: winner,
                winning_team: None,
            });
            return;
        }
    }
}

/// Processes kill events for scoreboard tracking in Survival mode.
/// NOTE: Does NOT re-emit PlayerDeathEvent (that would create an infinite
/// loop with bridge_death_to_kill_events). Life tracking is handled by
/// handle_survival_death which reads the original PlayerDeathEvent directly.
pub fn handle_survival_kills(
    config: Res<GameModeConfig>,
    mut kill_events: EventReader<PlayerKillEvent>,
    mut scoreboard: ResMut<ScoreBoard>,
) {
    if config.mode_type != GameModeType::Survival {
        return;
    }

    for event in kill_events.read() {
        if event.killer >= MAX_PLAYERS {
            continue;
        }
        // Track killer stats on the scoreboard (lives handled by handle_survival_death)
        scoreboard.kills[event.killer] += 1;
    }
}
