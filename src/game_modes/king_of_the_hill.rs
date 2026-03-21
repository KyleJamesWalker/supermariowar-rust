use bevy::prelude::*;

use crate::core::constants::{MAX_PLAYERS, TILE_SIZE, SCREEN_WIDTH, SCREEN_HEIGHT};
use crate::player::components::Player;
use crate::physics::components::Collider;
use super::traits::*;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// The single "hill" zone. Only one player at a time can score from it.
#[derive(Component, Debug)]
pub struct HillZone {
    pub center: Vec2,
    pub radius: f32,
    /// Player currently occupying the hill exclusively (None if contested or empty).
    pub occupant: Option<usize>,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Resource tracking King of the Hill mode state.
#[derive(Resource, Debug)]
pub struct KothModeState {
    pub points: [u32; MAX_PLAYERS],
    /// Timer that awards a point each time it fires.
    pub score_tick_timer: Timer,
    /// Timer that periodically relocates the hill.
    pub zone_change_timer: Timer,
    pub hill_spawned: bool,
}

impl Default for KothModeState {
    fn default() -> Self {
        Self {
            points: [0; MAX_PLAYERS],
            score_tick_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            zone_change_timer: Timer::from_seconds(30.0, TimerMode::Repeating),
            hill_spawned: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns the initial hill zone.
pub fn spawn_hill(
    config: Res<GameModeConfig>,
    mut state: ResMut<KothModeState>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::KingOfTheHill {
        return;
    }
    if state.hill_spawned {
        return;
    }
    state.hill_spawned = true;
    state.zone_change_timer = Timer::from_seconds(
        config.zone_change_interval_secs,
        TimerMode::Repeating,
    );

    let center = Vec2::new(SCREEN_WIDTH * 0.5, SCREEN_HEIGHT - TILE_SIZE * 4.0);
    let radius = TILE_SIZE * 2.5;

    commands.spawn((
        HillZone {
            center,
            radius,
            occupant: None,
        },
        Collider::new(radius * 2.0, radius * 2.0),
        Transform::from_xyz(center.x, center.y, 2.0),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));
}

/// Detects which player (if any) exclusively occupies the hill.
pub fn hill_occupancy_system(
    config: Res<GameModeConfig>,
    players: Query<(&Player, &Transform)>,
    mut hills: Query<&mut HillZone>,
) {
    if config.mode_type != GameModeType::KingOfTheHill {
        return;
    }

    for mut hill in hills.iter_mut() {
        let mut players_inside: Vec<usize> = Vec::new();
        for (player, tf) in players.iter() {
            let dx = (tf.translation.x - hill.center.x).abs();
            let dy = (tf.translation.y - hill.center.y).abs();
            if dx < hill.radius && dy < hill.radius {
                players_inside.push(player.index);
            }
        }

        hill.occupant = if players_inside.len() == 1 {
            Some(players_inside[0])
        } else {
            // Empty or contested -- no one scores
            None
        };
    }
}

/// Awards points per second to the exclusive hill occupant.
pub fn hill_score_tick(
    config: Res<GameModeConfig>,
    time: Res<Time>,
    mut state: ResMut<KothModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    hills: Query<&HillZone>,
) {
    if config.mode_type != GameModeType::KingOfTheHill {
        return;
    }

    state.score_tick_timer.tick(time.delta());

    if !state.score_tick_timer.just_finished() {
        return;
    }

    for hill in hills.iter() {
        if let Some(occupant) = hill.occupant {
            if occupant < MAX_PLAYERS {
                state.points[occupant] += 1;
                scoreboard.score[occupant] += 1;
            }
        }
    }
}

/// Periodically relocates the hill to a random position on the map.
pub fn hill_relocation_system(
    config: Res<GameModeConfig>,
    time: Res<Time>,
    mut state: ResMut<KothModeState>,
    mut hills: Query<(&mut HillZone, &mut Transform)>,
) {
    if config.mode_type != GameModeType::KingOfTheHill {
        return;
    }

    state.zone_change_timer.tick(time.delta());

    if !state.zone_change_timer.just_finished() {
        return;
    }

    for (mut hill, mut tf) in hills.iter_mut() {
        let new_x = (rand::random::<f32>() * (SCREEN_WIDTH - TILE_SIZE * 6.0)) + TILE_SIZE * 3.0;
        let new_y = (rand::random::<f32>() * (SCREEN_HEIGHT - TILE_SIZE * 8.0)) + TILE_SIZE * 3.0;

        hill.center = Vec2::new(new_x, new_y);
        hill.occupant = None;
        tf.translation.x = new_x;
        tf.translation.y = new_y;
    }
}

/// Checks if any player has reached the point limit.
pub fn check_koth_win_condition(
    config: Res<GameModeConfig>,
    state: Res<KothModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::KingOfTheHill {
        return;
    }

    for (i, &points) in state.points.iter().enumerate() {
        if points >= config.point_limit {
            round_over.send(RoundOverEvent {
                winning_player: Some(i),
                winning_team: None,
            });
            return;
        }
    }
}
