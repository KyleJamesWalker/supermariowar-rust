use bevy::prelude::*;

use crate::core::constants::{MAX_PLAYERS, SCREEN_WIDTH, SCREEN_HEIGHT, TILE_SIZE};
use crate::map::GameplayEntity;
use crate::objects::components::Collectible;
use crate::physics::components::Collider;
use super::traits::*;

/// Marker component for a coin entity spawned on the map.
#[derive(Component, Debug)]
pub struct CoinEntity {
    pub value: u32,
}

impl Default for CoinEntity {
    fn default() -> Self {
        Self { value: 1 }
    }
}

/// Resource tracking coin collection mode state.
#[derive(Resource, Debug)]
pub struct CoinModeState {
    pub coins: [u32; MAX_PLAYERS],
    pub spawn_timer: Timer,
}

impl Default for CoinModeState {
    fn default() -> Self {
        Self {
            coins: [0; MAX_PLAYERS],
            spawn_timer: Timer::from_seconds(3.0, TimerMode::Repeating),
        }
    }
}

/// Periodically spawns coin entities on the map at random positions.
pub fn coin_spawn_system(
    config: Res<GameModeConfig>,
    mut state: ResMut<CoinModeState>,
    time: Res<Time>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::CoinCollection {
        return;
    }

    state.spawn_timer.tick(time.delta());
    if state.spawn_timer.just_finished() {
        // Pick a random position using Bevy centered coordinates
        let x = rand::random::<f32>() * (SCREEN_WIDTH - TILE_SIZE * 2.0) - SCREEN_WIDTH / 2.0 + TILE_SIZE;
        let y = SCREEN_HEIGHT / 2.0 - rand::random::<f32>() * (SCREEN_HEIGHT - TILE_SIZE * 4.0) - TILE_SIZE;

        commands.spawn((
            CoinEntity::default(),
            GameplayEntity,
            Collectible,
            Collider::new(TILE_SIZE * 0.5, TILE_SIZE * 0.5),
            Transform::from_xyz(x, y, 5.0),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));
    }
}

/// Processes coin collection via ItemCollectEvent.
pub fn handle_coin_collect(
    config: Res<GameModeConfig>,
    mut state: ResMut<CoinModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut item_events: EventReader<ItemCollectEvent>,
) {
    if config.mode_type != GameModeType::CoinCollection {
        return;
    }

    for event in item_events.read() {
        if let CollectibleItemType::Coin = event.item_type {
            if event.player_index < MAX_PLAYERS {
                state.coins[event.player_index] += 1;
                scoreboard.score[event.player_index] += 1;
            }
        }
    }
}

/// When a player is killed, scatter coins nearby for other players to pick up.
pub fn coins_drop_on_kill(
    config: Res<GameModeConfig>,
    mut state: ResMut<CoinModeState>,
    mut kill_events: EventReader<PlayerKillEvent>,
    players: Query<(&crate::player::components::Player, &Transform)>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::CoinCollection {
        return;
    }

    for event in kill_events.read() {
        if event.victim >= MAX_PLAYERS {
            continue;
        }

        let victim_coins = state.coins[event.victim];
        let drop_count = victim_coins.min(config.coins_dropped_on_kill);

        if drop_count == 0 {
            continue;
        }

        // Reduce victim's coins
        state.coins[event.victim] = state.coins[event.victim].saturating_sub(drop_count);

        // Find victim position by matching on player index
        let victim_pos = players
            .iter()
            .find(|(p, _)| p.index == event.victim)
            .map(|(_, t)| t.translation)
            .unwrap_or(Vec3::new(0.0, 0.0, 5.0));

        // Spawn scattered coins around the victim using Bevy centered coordinates
        for i in 0..drop_count {
            let angle = (i as f32 / drop_count as f32) * std::f32::consts::TAU;
            let offset_x = angle.cos() * TILE_SIZE;
            let offset_y = angle.sin() * TILE_SIZE;

            commands.spawn((
                CoinEntity::default(),
                GameplayEntity,
                Collectible,
                Collider::new(TILE_SIZE * 0.5, TILE_SIZE * 0.5),
                Transform::from_xyz(
                    (victim_pos.x + offset_x).clamp(-SCREEN_WIDTH / 2.0, SCREEN_WIDTH / 2.0),
                    (victim_pos.y + offset_y).clamp(-SCREEN_HEIGHT / 2.0, SCREEN_HEIGHT / 2.0),
                    5.0,
                ),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ));
        }
    }
}

/// Also handle legacy ScoreEvent for backward compat.
pub fn handle_coin_score(
    config: Res<GameModeConfig>,
    mut state: ResMut<CoinModeState>,
    mut score_events: EventReader<ScoreEvent>,
) {
    if config.mode_type != GameModeType::CoinCollection {
        return;
    }

    for event in score_events.read() {
        if let ScoreReason::CoinCollect = event.reason {
            if event.player_index < MAX_PLAYERS {
                state.coins[event.player_index] += event.points.max(0) as u32;
            }
        }
    }
}

/// Checks if any player has collected enough coins.
pub fn check_coin_win_condition(
    config: Res<GameModeConfig>,
    state: Res<CoinModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::CoinCollection {
        return;
    }

    for (i, &coins) in state.coins.iter().enumerate() {
        if coins >= config.point_limit {
            round_over.send(RoundOverEvent {
                winning_player: Some(i),
                winning_team: None,
            });
            return;
        }
    }
}
