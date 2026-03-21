pub mod music;
pub mod sfx;

use bevy::prelude::*;

use crate::core::game_state::{GameState, GameplayState};

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        // Resources
        app.init_resource::<music::MusicPlayer>()
            .init_resource::<music::MusicTrackMap>()
            .init_resource::<sfx::SfxLibrary>()
            .init_resource::<sfx::SfxVolume>();

        // Events
        app.add_event::<sfx::SfxEvent>();

        // Startup systems: load assets
        app.add_systems(
            Startup,
            (sfx::load_sfx, music::load_music_tracks),
        );

        // Music state transitions: play appropriate music on state enter
        app.add_systems(OnEnter(GameState::Menu), music::menu_music_system);
        app.add_systems(OnEnter(GameState::Gameplay), music::gameplay_music_system);
        app.add_systems(OnEnter(GameState::GameOver), music::game_over_music_system);

        // Stop music when leaving gameplay (before game over music starts)
        app.add_systems(OnExit(GameState::Gameplay), music::stop_music_on_exit);
        // Stop music when leaving menu (before gameplay music starts)
        app.add_systems(OnExit(GameState::Menu), music::stop_music_on_exit);

        // Music volume control runs every frame to react to volume changes
        app.add_systems(Update, music::music_volume_control);

        // Pause/resume music based on gameplay sub-state
        app.add_systems(
            Update,
            music::pause_music_system.run_if(in_state(GameState::Gameplay)),
        );

        // SFX event handler runs every frame
        app.add_systems(Update, sfx::sfx_event_handler);

        // Gameplay SFX bridge systems: react to game events and emit SfxEvents
        app.add_systems(
            Update,
            (
                sfx::jump_sfx_system,
                sfx::stomp_sfx_system,
                sfx::death_sfx_system,
                sfx::block_sfx_system,
                sfx::powerup_sfx_system,
            )
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(GameplayState::Playing)),
        );

        // Countdown SFX (runs during the Countdown sub-state)
        app.add_systems(
            Update,
            sfx::countdown_sfx_system
                .run_if(in_state(GameState::Gameplay))
                .run_if(in_state(GameplayState::Countdown)),
        );

        // Menu SFX (runs during Menu state)
        app.add_systems(
            Update,
            sfx::menu_sfx_system.run_if(in_state(GameState::Menu)),
        );
    }
}
