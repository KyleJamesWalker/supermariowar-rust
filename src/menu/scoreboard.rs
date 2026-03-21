use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use crate::core::game_state::{GameState, MatchResult};
use crate::game_modes::traits::{RoundManager, ScoreBoard};
use crate::player::components::Player;
use super::components::*;
use super::main_menu::TEAM_COLORS;

/// Spawns the end-of-round scoreboard overlay.
pub fn setup_round_scoreboard(
    mut commands: Commands,
    scoreboard: Res<ScoreBoard>,
    round_manager: Res<RoundManager>,
    match_result: Res<MatchResult>,
    player_query: Query<&Player>,
) {
    spawn_scoreboard(
        &mut commands,
        &scoreboard,
        &round_manager,
        &match_result,
        &player_query,
        false,
    );
    commands.insert_resource(ScoreboardTimer::default());
}

/// Spawns the end-of-match final scoreboard overlay.
pub fn setup_match_scoreboard(
    mut commands: Commands,
    scoreboard: Res<ScoreBoard>,
    round_manager: Res<RoundManager>,
    match_result: Res<MatchResult>,
    player_query: Query<&Player>,
) {
    spawn_scoreboard(
        &mut commands,
        &scoreboard,
        &round_manager,
        &match_result,
        &player_query,
        true,
    );
    commands.insert_resource(ScoreboardTimer::default());
}

fn spawn_scoreboard(
    commands: &mut Commands,
    scoreboard: &ScoreBoard,
    round_manager: &RoundManager,
    match_result: &MatchResult,
    player_query: &Query<&Player>,
    is_match_over: bool,
) {
    // Collect active player info
    let mut active_players = Vec::new();
    for player in player_query.iter() {
        if player.index < MAX_PLAYERS {
            active_players.push((player.index, player.team));
        }
    }
    // If no player entities exist, show all 4 slots with default teams
    if active_players.is_empty() {
        for i in 0..MAX_PLAYERS {
            active_players.push((i, i as u8));
        }
    }
    active_players.sort_by_key(|(idx, _)| *idx);

    let title = if is_match_over {
        "MATCH OVER"
    } else {
        "ROUND OVER"
    };

    let prompt_text = if is_match_over {
        "Press Enter to return to menu..."
    } else {
        "Next round starting soon..."
    };

    // Determine winner text
    let winner_text = if let Some(player_idx) = match_result.winning_player {
        format!("Player {} Wins!", player_idx + 1)
    } else if let Some(team_idx) = match_result.winning_team {
        format!("Team {} Wins!", team_idx + 1)
    } else {
        "Draw!".to_string()
    };

    commands
        .spawn((
            ScoreboardRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(8.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
            GlobalZIndex(90),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new(title),
                TextFont {
                    font_size: 44.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(8.0)),
                    ..default()
                },
            ));

            // Winner announcement
            parent.spawn((
                Text::new(&winner_text),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                Node {
                    margin: UiRect::bottom(Val::Px(16.0)),
                    ..default()
                },
            ));

            // Round info
            if !is_match_over {
                parent.spawn((
                    Text::new(format!(
                        "Round {} of {}",
                        round_manager.current_round, round_manager.total_rounds
                    )),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                    Node {
                        margin: UiRect::bottom(Val::Px(12.0)),
                        ..default()
                    },
                ));
            }

            // Column headers
            parent
                .spawn(Node {
                    width: Val::Px(500.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                })
                .with_children(|header| {
                    let headers = ["Player", "Score", "Kills", "Deaths", "Rounds"];
                    let widths = [100.0, 80.0, 80.0, 80.0, 80.0];
                    for (h, w) in headers.iter().zip(widths.iter()) {
                        header.spawn((
                            Text::new(*h),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.8, 0.8, 0.8, 0.7)),
                            Node {
                                width: Val::Px(*w),
                                ..default()
                            },
                        ));
                    }
                });

            // Separator line (thin colored bar)
            parent.spawn((
                Node {
                    width: Val::Px(500.0),
                    height: Val::Px(2.0),
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.5, 0.5, 0.5, 0.5)),
            ));

            // Player rows
            for &(player_idx, team) in &active_players {
                let is_winner = match_result.winning_player == Some(player_idx);
                let team_color = TEAM_COLORS[team as usize % TEAM_COLORS.len()];
                let row_color = if is_winner {
                    Color::srgba(1.0, 1.0, 0.0, 1.0)
                } else {
                    team_color
                };

                parent
                    .spawn((
                        ScoreboardEntry {
                            player_index: player_idx,
                        },
                        Node {
                            width: Val::Px(500.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(6.0)),
                            ..default()
                        },
                        if is_winner {
                            BackgroundColor(Color::srgba(0.3, 0.3, 0.0, 0.3))
                        } else {
                            BackgroundColor(Color::NONE)
                        },
                    ))
                    .with_children(|row| {
                        let values = [
                            format!("P{}", player_idx + 1),
                            format!("{}", scoreboard.score[player_idx]),
                            format!("{}", scoreboard.kills[player_idx]),
                            format!("{}", scoreboard.deaths[player_idx]),
                            format!("{}", round_manager.round_wins[player_idx]),
                        ];
                        let widths = [100.0, 80.0, 80.0, 80.0, 80.0];

                        for (val, w) in values.iter().zip(widths.iter()) {
                            row.spawn((
                                Text::new(val.as_str()),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(row_color),
                                Node {
                                    width: Val::Px(*w),
                                    ..default()
                                },
                            ));
                        }
                    });
            }

            // Prompt text
            parent.spawn((
                ScoreboardPrompt,
                Text::new(prompt_text),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.8)),
                Node {
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
            ));
        });
}

/// During RoundOver, allows pressing Enter to advance, or auto-advances via timer.
pub fn scoreboard_round_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut timer: ResMut<ScoreboardTimer>,
) {
    timer.timer.tick(time.delta());

    // Press Enter or wait for timer to auto-advance.
    // The actual round transition is handled by game_modes::round_over_timer_system.
    // We just allow a manual skip by forcing the timer to finish.
    if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
        if !timer.timer.finished() {
            let remaining = timer.timer.remaining();
            timer.timer.tick(remaining);
        }
    }
}

/// During MatchOver, pressing Enter returns to the menu.
pub fn scoreboard_match_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut timer: ResMut<ScoreboardTimer>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    timer.timer.tick(time.delta());

    if keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
        || timer.timer.just_finished()
    {
        next_game_state.set(GameState::Menu);
    }
}

/// Despawns the scoreboard.
pub fn cleanup_scoreboard(
    mut commands: Commands,
    query: Query<Entity, With<ScoreboardRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
