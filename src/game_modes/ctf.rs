use bevy::prelude::*;

use crate::core::constants::{MAX_PLAYERS, TILE_SIZE, SCREEN_WIDTH, SCREEN_HEIGHT};
use crate::map::GameplayEntity;
use crate::player::components::Player;
use crate::physics::components::Collider;
use super::traits::*;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marker component for a flag entity.
#[derive(Component, Debug)]
pub struct Flag {
    pub team: u8,
    pub carried_by: Option<usize>,
    pub at_base: bool,
    pub base_position: Vec2,
}

/// Timer that counts how long a flag has been sitting on the ground (not at base).
/// When it expires the flag is automatically returned.
#[derive(Component, Debug)]
pub struct FlagAutoReturnTimer {
    pub timer: Timer,
}

/// Visual indicator attached to a player who is carrying a flag.
#[derive(Component, Debug)]
pub struct FlagCarrierIndicator {
    pub player_index: usize,
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Resource tracking capture-the-flag mode state.
#[derive(Resource, Debug)]
pub struct CtfModeState {
    pub captures: [u32; MAX_PLAYERS],
    pub flags_spawned: bool,
}

impl Default for CtfModeState {
    fn default() -> Self {
        Self {
            captures: [0; MAX_PLAYERS],
            flags_spawned: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Spawns flag entities at designated positions when the mode starts.
/// Uses hard-coded positions (left side / right side) as placeholder spawn points.
pub fn spawn_flags(
    config: Res<GameModeConfig>,
    mut state: ResMut<CtfModeState>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }
    if state.flags_spawned {
        return;
    }
    state.flags_spawned = true;

    let flag_positions: [(u8, Vec2); 2] = [
        (0, Vec2::new(TILE_SIZE * 2.0 - SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0 - (SCREEN_HEIGHT - TILE_SIZE * 3.0))),
        (1, Vec2::new(SCREEN_WIDTH - TILE_SIZE * 3.0 - SCREEN_WIDTH / 2.0, SCREEN_HEIGHT / 2.0 - (SCREEN_HEIGHT - TILE_SIZE * 3.0))),
    ];

    for (team, pos) in &flag_positions {
        commands.spawn((
            Flag {
                team: *team,
                carried_by: None,
                at_base: true,
                base_position: *pos,
            },
            GameplayEntity,
            Collider::new(TILE_SIZE, TILE_SIZE),
            Transform::from_xyz(pos.x, pos.y, 6.0),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));
    }
}

/// Handles flag pickup: a player walking over an enemy flag starts carrying it.
pub fn flag_pickup_system(
    config: Res<GameModeConfig>,
    players: Query<(&Player, &Transform)>,
    mut flags: Query<(&mut Flag, &Transform, Entity), Without<Player>>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }

    for (player, player_tf) in players.iter() {
        for (mut flag, flag_tf, _flag_entity) in flags.iter_mut() {
            // Can only pick up enemy flags
            if flag.team == player.team {
                continue;
            }
            // Already carried
            if flag.carried_by.is_some() {
                continue;
            }

            let dx = (player_tf.translation.x - flag_tf.translation.x).abs();
            let dy = (player_tf.translation.y - flag_tf.translation.y).abs();

            if dx < TILE_SIZE && dy < TILE_SIZE {
                flag.carried_by = Some(player.index);
                flag.at_base = false;

                // Spawn a visual indicator on the carrier
                commands.spawn((
                    FlagCarrierIndicator {
                        player_index: player.index,
                    },
                    GameplayEntity,
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ));
            }
        }
    }
}

/// Moves carried flags to follow their carrier.
pub fn flag_follow_carrier(
    config: Res<GameModeConfig>,
    players: Query<(&Player, &Transform)>,
    mut flags: Query<(&Flag, &mut Transform), Without<Player>>,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }

    for (flag, mut flag_tf) in flags.iter_mut() {
        if let Some(carrier_idx) = flag.carried_by {
            for (player, player_tf) in players.iter() {
                if player.index == carrier_idx {
                    flag_tf.translation.x = player_tf.translation.x;
                    flag_tf.translation.y = player_tf.translation.y + TILE_SIZE * 0.5;
                    break;
                }
            }
        }
    }
}

/// Detects flag capture: carrier brings enemy flag to their own base while
/// their own flag is still at base.
pub fn flag_capture_system(
    mut commands: Commands,
    config: Res<GameModeConfig>,
    mut state: ResMut<CtfModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    players: Query<(&Player, &Transform)>,
    mut flags: Query<(&mut Flag, &mut Transform), Without<Player>>,
    indicator_query: Query<(Entity, &FlagCarrierIndicator)>,
    mut objective_events: EventWriter<ObjectiveEvent>,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }

    // Collect flag state snapshot to avoid borrow issues
    let flag_snapshot: Vec<(u8, Option<usize>, bool, Vec2)> = flags
        .iter()
        .map(|(f, _)| (f.team, f.carried_by, f.at_base, f.base_position))
        .collect();

    for (player, player_tf) in players.iter() {
        // Find the flag this player is carrying (should be an enemy flag)
        let carrying_enemy_flag = flag_snapshot
            .iter()
            .any(|(team, carrier, _, _)| *carrier == Some(player.index) && *team != player.team);

        if !carrying_enemy_flag {
            continue;
        }

        // Check if the player's own flag is at base
        let own_flag_at_base = flag_snapshot
            .iter()
            .any(|(team, _, at_base, _)| *team == player.team && *at_base);

        if !own_flag_at_base {
            continue;
        }

        // Check if player is near their own base
        let own_base_pos = flag_snapshot
            .iter()
            .find(|(team, _, _, _)| *team == player.team)
            .map(|(_, _, _, base)| *base)
            .unwrap_or_default();

        let dx = (player_tf.translation.x - own_base_pos.x).abs();
        let dy = (player_tf.translation.y - own_base_pos.y).abs();

        if dx < TILE_SIZE * 1.5 && dy < TILE_SIZE * 1.5 {
            // Capture! Return the enemy flag to its base.
            for (mut flag, mut flag_tf) in flags.iter_mut() {
                if flag.carried_by == Some(player.index) {
                    flag.carried_by = None;
                    flag.at_base = true;
                    flag_tf.translation.x = flag.base_position.x;
                    flag_tf.translation.y = flag.base_position.y;
                }
            }

            // Remove the carrier indicator from this player
            for (ind_entity, ind) in indicator_query.iter() {
                if ind.player_index == player.index {
                    commands.entity(ind_entity).despawn_recursive();
                }
            }

            if player.index < MAX_PLAYERS {
                state.captures[player.index] += 1;
                scoreboard.score[player.index] += 1;
                scoreboard.objective_count[player.index] += 1;
            }

            objective_events.send(ObjectiveEvent {
                player_index: player.index,
                objective_type: ObjectiveType::FlagCapture,
            });
        }
    }
}

/// When a flag carrier is killed, drop the flag at their location.
pub fn flag_drop_on_kill(
    config: Res<GameModeConfig>,
    mut kill_events: EventReader<PlayerKillEvent>,
    players: Query<(&Player, &Transform)>,
    mut flags: Query<(&mut Flag, &mut Transform, Entity), Without<Player>>,
    indicator_query: Query<(Entity, &FlagCarrierIndicator)>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }

    for event in kill_events.read() {
        // Find if the victim was carrying a flag
        let victim_pos = players
            .iter()
            .find(|(p, _)| p.index == event.victim)
            .map(|(_, tf)| tf.translation);

        for (mut flag, mut flag_tf, entity) in flags.iter_mut() {
            if flag.carried_by == Some(event.victim) {
                flag.carried_by = None;
                if let Some(pos) = victim_pos {
                    flag_tf.translation = pos;
                }

                // Start auto-return timer
                commands.entity(entity).insert(FlagAutoReturnTimer {
                    timer: Timer::from_seconds(
                        config.flag_auto_return_secs,
                        TimerMode::Once,
                    ),
                });
            }
        }

        // Remove carrier indicator for the victim
        for (entity, indicator) in indicator_query.iter() {
            if indicator.player_index == event.victim {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Automatically returns a dropped flag to its base after a timeout.
pub fn flag_auto_return_system(
    config: Res<GameModeConfig>,
    time: Res<Time>,
    mut flags: Query<(&mut Flag, &mut Transform, Entity, &mut FlagAutoReturnTimer)>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }

    for (mut flag, mut flag_tf, entity, mut auto_return) in flags.iter_mut() {
        // Only tick if flag is not carried and not at base
        if flag.carried_by.is_some() || flag.at_base {
            commands.entity(entity).remove::<FlagAutoReturnTimer>();
            continue;
        }

        auto_return.timer.tick(time.delta());
        if auto_return.timer.just_finished() {
            flag.at_base = true;
            flag_tf.translation.x = flag.base_position.x;
            flag_tf.translation.y = flag.base_position.y;
            commands.entity(entity).remove::<FlagAutoReturnTimer>();
        }
    }
}

/// Also handle legacy ScoreEvent for backward compat.
pub fn handle_ctf_score(
    config: Res<GameModeConfig>,
    mut state: ResMut<CtfModeState>,
    mut score_events: EventReader<ScoreEvent>,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }

    for event in score_events.read() {
        if let ScoreReason::FlagCapture = event.reason {
            if event.player_index < MAX_PLAYERS {
                state.captures[event.player_index] += 1;
            }
        }
    }
}

/// Checks if any team has captured enough flags.
pub fn check_ctf_win_condition(
    config: Res<GameModeConfig>,
    state: Res<CtfModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::CaptureTheFlag {
        return;
    }

    for (i, &captures) in state.captures.iter().enumerate() {
        if captures >= config.point_limit {
            round_over.send(RoundOverEvent {
                winning_player: Some(i),
                winning_team: None,
            });
            return;
        }
    }
}
