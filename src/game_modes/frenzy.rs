use bevy::prelude::*;

use crate::core::constants::{MAX_PLAYERS, TILE_SIZE, SCREEN_WIDTH, SCREEN_HEIGHT};
use crate::objects::powerups::{spawn_powerup, PowerupType};
use super::traits::*;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Resource tracking Frenzy mode state.
/// Random powerups continuously spawn on the map.
/// Kills from powerup usage score points.
#[derive(Resource, Debug)]
pub struct FrenzyModeState {
    pub kills: [u32; MAX_PLAYERS],
    /// Timer controlling powerup spawn rate.
    pub spawn_timer: Timer,
    /// Whether "chaos mode" is active (double spawn rate).
    pub chaos_mode: bool,
}

impl Default for FrenzyModeState {
    fn default() -> Self {
        Self {
            kills: [0; MAX_PLAYERS],
            spawn_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
            chaos_mode: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Picks a random combat-oriented powerup type for frenzy spawning.
fn random_frenzy_powerup() -> PowerupType {
    let options = [
        PowerupType::Fire,
        PowerupType::Hammer,
        PowerupType::Boomerang,
        PowerupType::Bomb,
        PowerupType::Star,
        PowerupType::BulletBill,
        PowerupType::Podoboo,
        PowerupType::BobOmb,
        PowerupType::IceWand,
        PowerupType::Feather,
        PowerupType::Leaf,
    ];
    let idx = rand::random::<usize>() % options.len();
    options[idx]
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Continuously spawns random powerups at configurable intervals.
pub fn frenzy_spawn_powerups(
    config: Res<GameModeConfig>,
    mut state: ResMut<FrenzyModeState>,
    time: Res<Time>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::Frenzy {
        return;
    }

    // Adjust spawn rate based on config and chaos mode
    let rate = if state.chaos_mode {
        config.frenzy_rate * 0.5
    } else {
        config.frenzy_rate
    };

    // Dynamically update the timer duration if it doesn't match
    let target_secs = rate.max(0.1);
    if (state.spawn_timer.duration().as_secs_f32() - target_secs).abs() > 0.01 {
        state.spawn_timer = Timer::from_seconds(target_secs, TimerMode::Repeating);
    }

    state.spawn_timer.tick(time.delta());

    if state.spawn_timer.just_finished() {
        let x = rand::random::<f32>() * (SCREEN_WIDTH - TILE_SIZE * 2.0) - SCREEN_WIDTH / 2.0 + TILE_SIZE;
        let y = SCREEN_HEIGHT / 2.0 - rand::random::<f32>() * (SCREEN_HEIGHT - TILE_SIZE * 2.0) - TILE_SIZE;

        let powerup_type = random_frenzy_powerup();

        spawn_powerup(&mut commands, powerup_type, x, y);

        // In chaos mode, spawn a second powerup per tick
        if state.chaos_mode {
            let x2 = rand::random::<f32>() * (SCREEN_WIDTH - TILE_SIZE * 2.0) - SCREEN_WIDTH / 2.0 + TILE_SIZE;
            let y2 = SCREEN_HEIGHT / 2.0 - rand::random::<f32>() * (SCREEN_HEIGHT - TILE_SIZE * 2.0) - TILE_SIZE;
            let powerup_type2 = random_frenzy_powerup();

            spawn_powerup(&mut commands, powerup_type2, x2, y2);
        }
    }
}

/// Processes kills in frenzy mode.
pub fn handle_frenzy_kills(
    config: Res<GameModeConfig>,
    mut state: ResMut<FrenzyModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut kill_events: EventReader<PlayerKillEvent>,
) {
    if config.mode_type != GameModeType::Frenzy {
        return;
    }

    for event in kill_events.read() {
        if event.killer >= MAX_PLAYERS || event.victim >= MAX_PLAYERS {
            continue;
        }

        state.kills[event.killer] += 1;
        scoreboard.kills[event.killer] += 1;
        scoreboard.deaths[event.victim] += 1;
        scoreboard.score[event.killer] += 1;
    }
}

/// Activates chaos mode after a certain score threshold is reached by any player.
pub fn frenzy_chaos_check(
    config: Res<GameModeConfig>,
    mut state: ResMut<FrenzyModeState>,
) {
    if config.mode_type != GameModeType::Frenzy {
        return;
    }

    if state.chaos_mode {
        return;
    }

    // Activate chaos mode when any player reaches 60% of the kill limit
    let threshold = (config.kill_limit as f32 * 0.6) as u32;
    for &kills in state.kills.iter() {
        if kills >= threshold {
            state.chaos_mode = true;
            return;
        }
    }
}

/// Checks if any player has reached the kill limit.
pub fn check_frenzy_win_condition(
    config: Res<GameModeConfig>,
    state: Res<FrenzyModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::Frenzy {
        return;
    }

    for (i, &kills) in state.kills.iter().enumerate() {
        if kills >= config.kill_limit {
            round_over.send(RoundOverEvent {
                winning_player: Some(i),
                winning_team: None,
            });
            return;
        }
    }
}
