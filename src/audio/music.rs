use bevy::prelude::*;

/// Resource managing background music state.
#[derive(Resource, Default)]
pub struct MusicPlayer {
    pub current_track: Option<Handle<AudioSource>>,
    pub volume: f32,
    pub category: MusicCategory,
}

/// Categories for music tracks matching the original game structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MusicCategory {
    #[default]
    Menu,
    Gameplay,
    World,
    Bonus,
    GameOver,
}

/// Marker component for the music playback entity.
#[derive(Component, Debug)]
pub struct MusicEntity;

/// Plays a music track by loading from the asset path.
pub fn play_music(
    commands: &mut Commands,
    asset_server: &AssetServer,
    music_player: &mut MusicPlayer,
    path: &str,
) {
    let handle: Handle<AudioSource> = asset_server.load(path);
    music_player.current_track = Some(handle.clone());

    commands.spawn((
        MusicEntity,
        AudioPlayer(handle),
    ));
}
