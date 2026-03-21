use bevy::prelude::*;
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
                .set(ImagePlugin::default_nearest()),
        )
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
