use bevy::prelude::*;

use crate::core::constants::{MAX_PLAYERS, TILE_SIZE, SCREEN_WIDTH, SCREEN_HEIGHT};
use crate::map::GameplayEntity;
use crate::player::components::Player;
use crate::physics::components::Collider;
use super::traits::*;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// A control zone on the map that can be claimed by standing in it.
#[derive(Component, Debug)]
pub struct DominationZone {
    /// Which player currently owns the zone (None = unclaimed).
    pub owner: Option<usize>,
    /// Which team currently owns the zone.
    pub owner_team: Option<u8>,
    /// Timer tracking how long the current claimant has stood in the zone.
    pub claim_timer: Timer,
    /// How long a player must stay in the zone to claim it.
    pub claim_duration: f32,
    /// Zone bounding area (world coordinates).
    pub center: Vec2,
    pub radius: f32,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Resource tracking domination mode state.
#[derive(Resource, Debug)]
pub struct DominationModeState {
    /// Points accumulated per player from holding zones.
    pub points: [u32; MAX_PLAYERS],
    /// Per-team points.
    pub team_points: [u32; MAX_PLAYERS],
    /// Timer driving per-second point ticks.
    pub score_tick_timer: Timer,
    pub zones_spawned: bool,
}

impl Default for DominationModeState {
    fn default() -> Self {
        Self {
            points: [0; MAX_PLAYERS],
            team_points: [0; MAX_PLAYERS],
            score_tick_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            zones_spawned: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns a set of control zones on the map.
pub fn spawn_domination_zones(
    config: Res<GameModeConfig>,
    mut state: ResMut<DominationModeState>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::Domination {
        return;
    }
    if state.zones_spawned {
        return;
    }
    state.zones_spawned = true;

    // Place 3 zones spread across the map
    let zone_positions = [
        Vec2::new(SCREEN_WIDTH * 0.2 - SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0 - (SCREEN_HEIGHT - TILE_SIZE * 3.0)),
        Vec2::new(SCREEN_WIDTH * 0.5 - SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0 - (SCREEN_HEIGHT - TILE_SIZE * 5.0)),
        Vec2::new(SCREEN_WIDTH * 0.8 - SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0 - (SCREEN_HEIGHT - TILE_SIZE * 3.0)),
    ];

    let zone_radius = TILE_SIZE * 2.0;

    for pos in &zone_positions {
        commands.spawn((
            DominationZone {
                owner: None,
                owner_team: None,
                claim_timer: Timer::from_seconds(1.5, TimerMode::Once),
                claim_duration: 1.5,
                center: *pos,
                radius: zone_radius,
            },
            GameplayEntity,
            Collider::new(zone_radius * 2.0, zone_radius * 2.0),
            Transform::from_xyz(pos.x, pos.y, 2.0),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));
    }
}

/// Detects players standing inside zones and progresses claim timers.
pub fn domination_claim_system(
    config: Res<GameModeConfig>,
    time: Res<Time>,
    players: Query<(&Player, &Transform)>,
    mut zones: Query<&mut DominationZone>,
    mut objective_events: EventWriter<ObjectiveEvent>,
) {
    if config.mode_type != GameModeType::Domination {
        return;
    }

    for mut zone in zones.iter_mut() {
        // Find all players inside this zone
        let mut players_in_zone: Vec<(usize, u8)> = Vec::new();
        for (player, tf) in players.iter() {
            let dx = (tf.translation.x - zone.center.x).abs();
            let dy = (tf.translation.y - zone.center.y).abs();
            if dx < zone.radius && dy < zone.radius {
                players_in_zone.push((player.index, player.team));
            }
        }

        if players_in_zone.is_empty() {
            // No one in zone -- reset claim progress (but keep current owner)
            zone.claim_timer.reset();
            continue;
        }

        // If multiple players from different teams are in the zone, it's contested
        let first_team = players_in_zone[0].1;
        let all_same_team = players_in_zone.iter().all(|(_, t)| *t == first_team);

        if !all_same_team {
            // Contested -- reset timer, no progress
            zone.claim_timer.reset();
            continue;
        }

        let claiming_player = players_in_zone[0].0;
        let claiming_team = first_team;

        // If this team already owns the zone, nothing to do
        if zone.owner_team == Some(claiming_team) {
            zone.claim_timer.reset();
            continue;
        }

        // Progress the claim timer
        zone.claim_timer.tick(time.delta());

        if zone.claim_timer.just_finished() {
            zone.owner = Some(claiming_player);
            zone.owner_team = Some(claiming_team);
            zone.claim_timer.reset();

            objective_events.send(ObjectiveEvent {
                player_index: claiming_player,
                objective_type: ObjectiveType::ZoneCaptured,
            });
        }
    }
}

/// Awards points per second to players/teams that control zones.
pub fn domination_score_tick(
    config: Res<GameModeConfig>,
    time: Res<Time>,
    mut state: ResMut<DominationModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    zones: Query<&DominationZone>,
) {
    if config.mode_type != GameModeType::Domination {
        return;
    }

    state.score_tick_timer.tick(time.delta());

    if !state.score_tick_timer.just_finished() {
        return;
    }

    for zone in zones.iter() {
        if let Some(owner) = zone.owner {
            if owner < MAX_PLAYERS {
                state.points[owner] += 1;
                scoreboard.score[owner] += 1;
            }
        }
        if let Some(team) = zone.owner_team {
            let team_idx = team as usize;
            if team_idx < MAX_PLAYERS {
                state.team_points[team_idx] += 1;
            }
        }
    }
}

/// Checks if any player or team has reached the point limit.
pub fn check_domination_win_condition(
    config: Res<GameModeConfig>,
    state: Res<DominationModeState>,
    player_query: Query<&Player>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::Domination {
        return;
    }

    // Check team points first (domination supports team play)
    for (team_idx, &points) in state.team_points.iter().enumerate() {
        if points >= config.point_limit {
            // Find the highest-scoring player on the winning team
            let best_player = player_query
                .iter()
                .filter(|p| p.team as usize == team_idx)
                .max_by_key(|p| state.points.get(p.index).copied().unwrap_or(0))
                .map(|p| p.index);

            round_over.send(RoundOverEvent {
                winning_player: best_player,
                winning_team: Some(team_idx as u8),
            });
            return;
        }
    }

    // Also check individual points (for free-for-all)
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
