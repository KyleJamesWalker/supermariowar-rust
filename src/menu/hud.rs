use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;
use crate::core::game_state::GameplayState;
use crate::game_modes::survival::SurvivalModeState;
use crate::game_modes::traits::{
    CountdownPhase, GameModeConfig, GameModeType, GameTimer, RoundCountdown, RoundManager,
    ScoreBoard,
};
use crate::player::components::{Player, PowerupState};
use super::components::*;
use super::main_menu::TEAM_COLORS;

/// Spawns the in-game HUD when entering Gameplay state.
pub fn setup_hud(
    mut commands: Commands,
    config: Res<GameModeConfig>,
    round_manager: Res<RoundManager>,
) {
    commands
        .spawn((
            HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                ..default()
            },
            GlobalZIndex(50),
        ))
        .with_children(|parent| {
            // Top bar: scores, timer, round info
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(50.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    ..default()
                })
                .with_children(|top_bar| {
                    // Player scores section
                    top_bar
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(20.0),
                            ..default()
                        })
                        .with_children(|scores_row| {
                            for i in 0..MAX_PLAYERS {
                                let team_color = TEAM_COLORS[i % TEAM_COLORS.len()];
                                scores_row.spawn((
                                    ScoreDisplay { player_index: i },
                                    Text::new(format!("P{}: 0", i + 1)),
                                    TextFont {
                                        font_size: 18.0,
                                        ..default()
                                    },
                                    TextColor(team_color),
                                ));
                            }
                        });

                    // Timer (for time-based modes)
                    if config.mode_type == GameModeType::TimeLimit {
                        top_bar.spawn((
                            TimerDisplay,
                            Text::new("00:00"),
                            TextFont {
                                font_size: 22.0,
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                    }

                    // Round indicator
                    top_bar.spawn((
                        RoundDisplay,
                        Text::new(format!(
                            "Round {} of {}",
                            round_manager.current_round, round_manager.total_rounds
                        )),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                    ));
                });

            // Second row: lives and powerups
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    ..default()
                })
                .with_children(|second_row| {
                    // Lives displays
                    second_row
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(20.0),
                            ..default()
                        })
                        .with_children(|lives_row| {
                            for i in 0..MAX_PLAYERS {
                                let team_color = TEAM_COLORS[i % TEAM_COLORS.len()];
                                lives_row.spawn((
                                    LivesDisplay { player_index: i },
                                    Text::new(""),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(team_color),
                                ));
                            }
                        });

                    // Powerup displays
                    second_row
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(20.0),
                            ..default()
                        })
                        .with_children(|powerup_row| {
                            for i in 0..MAX_PLAYERS {
                                powerup_row.spawn((
                                    PowerupDisplay { player_index: i },
                                    Text::new(""),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgba(0.9, 0.9, 0.9, 0.8)),
                                ));
                            }
                        });
                });

            // Center: countdown display (large, centered)
            parent.spawn((
                CountdownDisplay,
                Text::new(""),
                TextFont {
                    font_size: 80.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(40.0),
                    margin: UiRect::left(Val::Px(-40.0)),
                    ..default()
                },
            ));
        });
}

/// Updates score displays in the HUD.
pub fn update_hud_scores(
    scoreboard: Res<ScoreBoard>,
    player_query: Query<&Player>,
    mut score_query: Query<(&ScoreDisplay, &mut Text, &mut TextColor)>,
) {
    // Determine which players are active
    let mut active_players = [false; MAX_PLAYERS];
    let mut player_teams = [0u8; MAX_PLAYERS];
    for player in player_query.iter() {
        if player.index < MAX_PLAYERS {
            active_players[player.index] = true;
            player_teams[player.index] = player.team;
        }
    }

    for (display, mut text, mut color) in score_query.iter_mut() {
        let i = display.player_index;
        if active_players[i] {
            let team_idx = player_teams[i] as usize % TEAM_COLORS.len();
            *text = Text::new(format!("P{}: {}", i + 1, scoreboard.score[i]));
            *color = TextColor(TEAM_COLORS[team_idx]);
        } else {
            *text = Text::new("");
        }
    }
}

/// Updates lives displays in the HUD.
/// Reads from SurvivalModeState (the authoritative source) rather than
/// Player.lives which is only set at spawn time and never decremented.
pub fn update_hud_lives(
    config: Res<GameModeConfig>,
    survival_state: Res<SurvivalModeState>,
    player_query: Query<&Player>,
    mut lives_query: Query<(&LivesDisplay, &mut Text)>,
) {
    let show_lives = config.mode_type == GameModeType::Survival;

    let mut active = [false; MAX_PLAYERS];
    for player in player_query.iter() {
        if player.index < MAX_PLAYERS {
            active[player.index] = true;
        }
    }

    for (display, mut text) in lives_query.iter_mut() {
        let i = display.player_index;
        if show_lives && active[i] && survival_state.alive[i] {
            let lives = survival_state.lives[i].max(0);
            let hearts: String = (0..lives).map(|_| "♥").collect();
            *text = Text::new(format!("P{}: {}", i + 1, hearts));
        } else {
            *text = Text::new("");
        }
    }
}

/// Updates the timer display for time-based modes.
pub fn update_hud_timer(
    game_timer: Res<GameTimer>,
    mut timer_query: Query<(&TimerDisplay, &mut Text, &mut TextColor)>,
) {
    let remaining = game_timer.remaining_secs().max(0.0);
    let mins = (remaining / 60.0) as u32;
    let secs = (remaining % 60.0) as u32;

    for (_display, mut text, mut color) in timer_query.iter_mut() {
        *text = Text::new(format!("{:02}:{:02}", mins, secs));

        // Flash red when time is low
        if remaining < 10.0 {
            *color = TextColor(Color::srgb(1.0, 0.2, 0.2));
        } else if remaining < 30.0 {
            *color = TextColor(Color::srgb(1.0, 0.8, 0.0));
        } else {
            *color = TextColor(Color::WHITE);
        }
    }
}

/// Updates powerup indicators in the HUD.
pub fn update_hud_powerups(
    player_query: Query<(&Player, &PowerupState)>,
    mut powerup_query: Query<(&PowerupDisplay, &mut Text)>,
) {
    let mut powerup_text = [""; MAX_PLAYERS];
    for (player, powerup) in player_query.iter() {
        if player.index < MAX_PLAYERS {
            powerup_text[player.index] = match powerup {
                PowerupState::None => "",
                PowerupState::Fire => "[Fire]",
                PowerupState::Ice => "[Ice]",
                PowerupState::Hammer => "[Hammer]",
                PowerupState::Boomerang => "[Boomerang]",
                PowerupState::Feather => "[Feather]",
                PowerupState::Bomb => "[Bomb]",
                PowerupState::Leaf => "[Leaf]",
                PowerupState::Star => "[Star]",
                PowerupState::Jail => "[Jail]",
            };
        }
    }

    for (display, mut text) in powerup_query.iter_mut() {
        *text = Text::new(powerup_text[display.player_index]);
    }
}

/// Updates the round indicator text.
pub fn update_hud_round(
    round_manager: Res<RoundManager>,
    mut round_query: Query<(&RoundDisplay, &mut Text)>,
) {
    if !round_manager.is_changed() {
        return;
    }

    for (_display, mut text) in round_query.iter_mut() {
        *text = Text::new(format!(
            "Round {} of {}",
            round_manager.current_round, round_manager.total_rounds
        ));
    }
}

/// Updates the countdown display (3, 2, 1, GO!).
pub fn update_hud_countdown(
    gameplay_state: Res<State<GameplayState>>,
    countdown: Res<RoundCountdown>,
    mut countdown_query: Query<(&CountdownDisplay, &mut Text)>,
) {
    for (_display, mut text) in countdown_query.iter_mut() {
        if *gameplay_state.get() == GameplayState::Countdown {
            let countdown_text = match countdown.phase {
                CountdownPhase::Three => "3",
                CountdownPhase::Two => "2",
                CountdownPhase::One => "1",
                CountdownPhase::Go => "GO!",
                CountdownPhase::Done => "",
            };
            *text = Text::new(countdown_text);
        } else {
            *text = Text::new("");
        }
    }
}

/// Despawns the HUD when leaving Gameplay state.
pub fn cleanup_hud(
    mut commands: Commands,
    query: Query<Entity, With<HudRoot>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
