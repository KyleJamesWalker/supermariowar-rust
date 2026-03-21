use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use super::traits::*;

/// Resource tracking state for Time Limit mode.
/// A countdown timer runs; most kills when time expires wins.
#[derive(Resource, Debug)]
pub struct TimeLimitModeState {
    pub kills: [u32; MAX_PLAYERS],
    pub timer_started: bool,
}

impl Default for TimeLimitModeState {
    fn default() -> Self {
        Self {
            kills: [0; MAX_PLAYERS],
            timer_started: false,
        }
    }
}

/// Initialises the GameTimer when time-limit mode starts playing.
pub fn timelimit_start_timer(
    config: Res<GameModeConfig>,
    mut state: ResMut<TimeLimitModeState>,
    mut game_timer: ResMut<GameTimer>,
) {
    if config.mode_type != GameModeType::TimeLimit {
        return;
    }

    if !state.timer_started {
        *game_timer = GameTimer::new(config.time_limit_secs as f32);
        state.timer_started = true;
    }
}

/// Ticks the game timer each frame and fires RoundOverEvent when it expires.
pub fn timelimit_tick(
    config: Res<GameModeConfig>,
    time: Res<Time>,
    mut game_timer: ResMut<GameTimer>,
    state: Res<TimeLimitModeState>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::TimeLimit {
        return;
    }

    if game_timer.expired {
        return;
    }

    game_timer.timer.tick(time.delta());

    if game_timer.timer.just_finished() {
        game_timer.expired = true;

        // Determine winner: player with most kills
        let mut best_player: Option<usize> = None;
        let mut best_kills: u32 = 0;
        let mut tied = false;

        for (i, &kills) in state.kills.iter().enumerate() {
            if kills > best_kills {
                best_kills = kills;
                best_player = Some(i);
                tied = false;
            } else if kills == best_kills && kills > 0 {
                tied = true;
            }
        }

        if tied && config.sudden_death {
            // In sudden death, no winner is declared yet -- the game continues
            // until the next kill. Reset the expired flag to keep the round going.
            game_timer.expired = false;
            // The sudden death period has no time limit; the next kill wins.
        } else {
            round_over.send(RoundOverEvent {
                winning_player: best_player,
                winning_team: None,
            });
        }
    }
}

/// Processes kills in time-limit mode.
pub fn handle_timelimit_kills(
    config: Res<GameModeConfig>,
    mut state: ResMut<TimeLimitModeState>,
    mut scoreboard: ResMut<ScoreBoard>,
    game_timer: Res<GameTimer>,
    mut kill_events: EventReader<PlayerKillEvent>,
    mut round_over: EventWriter<RoundOverEvent>,
) {
    if config.mode_type != GameModeType::TimeLimit {
        return;
    }

    for event in kill_events.read() {
        if event.killer >= MAX_PLAYERS {
            continue;
        }

        state.kills[event.killer] += 1;
        scoreboard.kills[event.killer] += 1;
        scoreboard.deaths[event.victim] += 1;
        scoreboard.score[event.killer] += 1;

        // In sudden death (timer expired but no winner), the next kill wins immediately
        if game_timer.expired {
            round_over.send(RoundOverEvent {
                winning_player: Some(event.killer),
                winning_team: None,
            });
            return;
        }
    }
}
