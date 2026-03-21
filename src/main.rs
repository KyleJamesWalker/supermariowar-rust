#![allow(dead_code)]

use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};
use bevy::window::{PresentMode, WindowResolution};

mod ai;
mod audio;
mod core;
mod game_modes;
mod graphics;
mod map;
mod menu;
mod net;
mod objects;
mod physics;
mod player;

use core::constants::*;
use core::game_state::GameState;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Super Mario War".to_string(),
                        resolution: WindowResolution::new(
                            SCREEN_WIDTH,
                            SCREEN_HEIGHT,
                        ),
                        present_mode: PresentMode::AutoVsync,
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "data".to_string(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        // Remote debugging (BRP over HTTP on port 15702)
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        // Game plugins
        .add_plugins((
            core::CorePlugin,
            player::PlayerPlugin,
            physics::PhysicsPlugin,
            map::MapPlugin,
            objects::ObjectsPlugin,
            game_modes::GameModesPlugin,
            menu::MenuPlugin,
            graphics::GraphicsPlugin,
            audio::AudioPlugin,
            ai::AiPlugin,
            net::NetPlugin,
        ))
        // Startup systems
        .add_systems(Startup, setup_camera)
        // Splash screen transition (auto-advance to menu for now)
        .add_systems(OnEnter(GameState::Splash), splash_setup)
        .add_systems(
            Update,
            splash_advance.run_if(in_state(GameState::Splash)),
        )
        // Gameplay orchestration: reset round countdown when entering Gameplay
        .add_systems(OnEnter(GameState::Gameplay), reset_countdown_on_enter)
        // GameOver state: auto-return to menu after a delay
        .add_systems(OnEnter(GameState::GameOver), game_over_setup)
        .add_systems(
            Update,
            game_over_advance.run_if(in_state(GameState::GameOver)),
        )
        .run();
}

/// Spawns the 2D camera.
fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Resource for splash screen timing.
#[derive(Resource)]
struct SplashTimer(Timer);

/// Sets up the splash screen with a brief timer.
fn splash_setup(mut commands: Commands) {
    commands.insert_resource(SplashTimer(Timer::from_seconds(1.0, TimerMode::Once)));
}

/// Advances from splash screen to menu after the timer completes.
fn splash_advance(
    time: Res<Time>,
    mut timer: ResMut<SplashTimer>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        next_state.set(GameState::Menu);
    }
}

/// Resets the round countdown timer when entering Gameplay state so the
/// 3-2-1-GO! sequence starts fresh each time a game begins.
fn reset_countdown_on_enter(
    mut countdown: ResMut<game_modes::traits::RoundCountdown>,
    mut scoreboard: ResMut<game_modes::traits::ScoreBoard>,
    mut game_timer: ResMut<game_modes::traits::GameTimer>,
    mut round_over_timer: ResMut<game_modes::traits::RoundOverTimer>,
) {
    *countdown = game_modes::traits::RoundCountdown::default();
    *scoreboard = game_modes::traits::ScoreBoard::default();
    *game_timer = game_modes::traits::GameTimer::default();
    *round_over_timer = game_modes::traits::RoundOverTimer::default();
}

/// Resource for game-over screen timing.
#[derive(Resource)]
struct GameOverTimer(Timer);

/// Sets up a timer on the GameOver screen so it auto-returns to Menu.
fn game_over_setup(mut commands: Commands) {
    commands.insert_resource(GameOverTimer(Timer::from_seconds(5.0, TimerMode::Once)));
}

/// During GameOver, wait for timer or Enter press, then return to Menu.
fn game_over_advance(
    time: Res<Time>,
    mut timer: ResMut<GameOverTimer>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished()
        || keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space)
    {
        next_state.set(GameState::Menu);
    }
}
