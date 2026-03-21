use bevy::prelude::*;

use crate::core::constants::{MAX_PLAYERS, SCREEN_WIDTH, TILE_SIZE};
use crate::player::components::Player;
use super::traits::*;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Marker component attached to the player who is currently "it".
#[derive(Component, Debug)]
pub struct TaggedIt;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Resource tracking Tag mode state.
/// In C++ original: the "it" player's score counts DOWN from the goal.
/// When a player's score reaches 0, they are eliminated.
/// Last player standing wins.
#[derive(Resource, Debug)]
pub struct TagModeState {
    /// Index of the player who is currently "it" (None before first tag).
    pub it_player: Option<usize>,
    /// Timer that decrements the "it" player's score each tick.
    pub score_tick_timer: Timer,
    pub initialised: bool,
}

impl Default for TagModeState {
    fn default() -> Self {
        Self {
            it_player: None,
            score_tick_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            initialised: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Pick a random initial "it" player at the start of the round.
pub fn tag_init(
    config: Res<GameModeConfig>,
    mut state: ResMut<TagModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    players: Query<(Entity, &Player)>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::Tag {
        return;
    }
    if state.initialised {
        return;
    }

    let player_list: Vec<(Entity, usize)> = players
        .iter()
        .map(|(e, p)| (e, p.index))
        .collect();

    if player_list.is_empty() {
        return;
    }

    state.initialised = true;

    // C++ behavior: all scores start at goal value and count down
    for &(_entity, idx) in &player_list {
        if idx < MAX_PLAYERS {
            scoreboard.score[idx] = config.point_limit as i32;
        }
    }

    // Pick a random player to be "it"
    let pick = rand::random::<usize>() % player_list.len();
    let (entity, idx) = player_list[pick];
    state.it_player = Some(idx);
    commands.entity(entity).insert(TaggedIt);
}

/// Awards points per second to the "it" player.
pub fn tag_score_tick(
    config: Res<GameModeConfig>,
    time: Res<Time>,
    mut state: ResMut<TagModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
) {
    if config.mode_type != GameModeType::Tag {
        return;
    }

    state.score_tick_timer.tick(time.delta());

    if !state.score_tick_timer.just_finished() {
        return;
    }

    if let Some(it) = state.it_player {
        if it < MAX_PLAYERS {
            scoreboard.score[it] -= 1;
        }
    }
}

/// Detects proximity between the "it" player and others to transfer the tag.
pub fn tag_transfer_system(
    config: Res<GameModeConfig>,
    mut state: ResMut<TagModeState>,
    players: Query<(Entity, &Player, &Transform)>,
    tagged_query: Query<Entity, With<TaggedIt>>,
    mut commands: Commands,
    mut objective_events: EventWriter<ObjectiveEvent>,
) {
    if config.mode_type != GameModeType::Tag {
        return;
    }

    let it_index = match state.it_player {
        Some(idx) => idx,
        None => return,
    };

    // Find position of the "it" player
    let it_pos = players
        .iter()
        .find(|(_, p, _)| p.index == it_index)
        .map(|(_, _, tf)| tf.translation);

    let it_pos = match it_pos {
        Some(p) => p,
        None => return,
    };

    // Check proximity to other players
    for (entity, player, tf) in players.iter() {
        if player.index == it_index {
            continue;
        }

        let raw_dx = (tf.translation.x - it_pos.x).abs();
        let dx = raw_dx.min(SCREEN_WIDTH - raw_dx); // Screen-wrap aware distance
        let dy = (tf.translation.y - it_pos.y).abs();

        if dx < TILE_SIZE * 1.2 && dy < TILE_SIZE * 1.2 {
            // Transfer tag
            // Remove TaggedIt from old "it" player
            for old_entity in tagged_query.iter() {
                commands.entity(old_entity).remove::<TaggedIt>();
            }

            // Add TaggedIt to new player
            commands.entity(entity).insert(TaggedIt);
            state.it_player = Some(player.index);

            objective_events.send(ObjectiveEvent {
                player_index: player.index,
                objective_type: ObjectiveType::TagTransfer,
            });

            return;
        }
    }
}

/// If the "it" player is killed, transfer the tag to the killer.
/// Also applies a -5 point penalty to the killed player (original C++ behavior).
pub fn tag_transfer_on_kill(
    config: Res<GameModeConfig>,
    mut state: ResMut<TagModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    mut kill_events: EventReader<PlayerKillEvent>,
    players: Query<(Entity, &Player)>,
    tagged_query: Query<Entity, With<TaggedIt>>,
    mut commands: Commands,
) {
    if config.mode_type != GameModeType::Tag {
        return;
    }

    for event in kill_events.read() {
        // Apply kill penalty: victim loses 5 points (original C++ Tag.cpp lines 106-145)
        if event.victim < MAX_PLAYERS {
            scoreboard.score[event.victim] = (scoreboard.score[event.victim] - 5).max(0);
            scoreboard.deaths[event.victim] += 1;
            scoreboard.kills[event.killer] += 1;
        }

        let it_index = match state.it_player {
            Some(idx) => idx,
            None => continue,
        };

        if event.victim != it_index {
            continue;
        }

        // Remove TaggedIt from old player
        for old_entity in tagged_query.iter() {
            commands.entity(old_entity).remove::<TaggedIt>();
        }

        // Give tag to the killer
        if let Some((entity, _)) = players.iter().find(|(_, p)| p.index == event.killer) {
            commands.entity(entity).insert(TaggedIt);
            state.it_player = Some(event.killer);
        }
    }
}

/// Checks if the "it" player's score has reached 0 (eliminated).
/// Last player standing wins.
pub fn check_tag_win_condition(
    config: Res<GameModeConfig>,
    scoreboard: Res<ScoreBoard>,
    state: Res<TagModeState>,
    players: Query<&Player>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::Tag {
        return;
    }

    // Only check after initialization
    if !state.initialised {
        return;
    }

    // Count players still alive (score > 0)
    let mut alive_players: Vec<usize> = Vec::new();
    for player in players.iter() {
        if player.index < MAX_PLAYERS && scoreboard.score[player.index] > 0 {
            alive_players.push(player.index);
        }
    }

    // If only one player remains, they win (send event once — handle_round_over consumes first)
    if alive_players.len() == 1 {
        round_over.send(RoundOverEvent {
            winning_player: Some(alive_players[0]),
            winning_team: None,
        });
    }
    // Also end if no one is alive (draw)
    if alive_players.is_empty() {
        round_over.send(RoundOverEvent {
            winning_player: None,
            winning_team: None,
        });
    }
}
