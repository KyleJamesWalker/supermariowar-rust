use std::collections::HashMap;

use bevy::prelude::*;
use rand::Rng;

use crate::core::game_state::GameplayState;

/// Categories for music tracks matching the original game structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MusicCategory {
    #[default]
    Menu,
    Gameplay,
    World,
    Bonus,
    GameOver,
}

/// Resource managing background music playback state.
#[derive(Resource, Debug)]
pub struct MusicPlayer {
    pub current_track: Option<Handle<AudioSource>>,
    pub current_entity: Option<Entity>,
    pub volume: f32,
    pub category: MusicCategory,
    pub muted: bool,
}

impl Default for MusicPlayer {
    fn default() -> Self {
        Self {
            current_track: None,
            current_entity: None,
            volume: 0.7,
            category: MusicCategory::Menu,
            muted: false,
        }
    }
}

impl MusicPlayer {
    pub fn effective_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume
        }
    }
}

/// Marker component for the music playback entity.
#[derive(Component, Debug)]
pub struct MusicEntity;

/// Resource mapping music categories and track names to asset paths.
#[derive(Resource, Debug, Default)]
pub struct MusicTrackMap {
    pub tracks: HashMap<MusicCategory, Vec<String>>,
}

impl MusicTrackMap {
    pub fn get_tracks(&self, category: MusicCategory) -> Option<&Vec<String>> {
        self.tracks.get(&category)
    }

    pub fn get_random_track(&self, category: MusicCategory) -> Option<&String> {
        self.tracks.get(&category).and_then(|tracks| {
            if tracks.is_empty() {
                None
            } else if tracks.len() == 1 {
                Some(&tracks[0])
            } else {
                let idx = rand::thread_rng().gen_range(0..tracks.len());
                Some(&tracks[idx])
            }
        })
    }
}

/// Startup system that registers known music asset paths.
pub fn load_music_tracks(mut track_map: ResMut<MusicTrackMap>) {
    // NOTE: menu.ogg is encoded with libVorbis I (1.1.2) which lewton cannot
    // decode.  tournamentmenu.ogg uses a newer encoder and works correctly.
    track_map.tracks.insert(
        MusicCategory::Menu,
        vec!["music/game/Standard/Menu/tournamentmenu.ogg".to_string()],
    );

    track_map.tracks.insert(
        MusicCategory::Gameplay,
        vec![
            "music/game/Standard/SMB_Overground.ogg".to_string(),
            "music/game/Standard/SMB_Athletic.ogg".to_string(),
            "music/game/Standard/SMB_Underground.ogg".to_string(),
            "music/game/Standard/SMB_Castle.ogg".to_string(),
            "music/game/Standard/SMB_Battle.ogg".to_string(),
        ],
    );

    track_map.tracks.insert(
        MusicCategory::World,
        vec![
            "music/world/Standard/GrassLand.ogg".to_string(),
            "music/world/Standard/DesertHill.ogg".to_string(),
            "music/world/Standard/SeaSide.ogg".to_string(),
            "music/world/Standard/IceLand.ogg".to_string(),
            "music/world/Standard/StarWorld.ogg".to_string(),
        ],
    );

    track_map.tracks.insert(
        MusicCategory::Bonus,
        vec!["music/world/Standard/BonusGame.ogg".to_string()],
    );

    track_map.tracks.insert(
        MusicCategory::GameOver,
        vec!["music/game/Standard/Special/tournamentover.ogg".to_string()],
    );

    info!(
        "[Audio] Registered music tracks for {} categories",
        track_map.tracks.len()
    );
}

/// Stops any currently playing music by despawning the music entity.
pub fn stop_music(commands: &mut Commands, music_player: &mut MusicPlayer) {
    if let Some(entity) = music_player.current_entity.take() {
        commands.entity(entity).despawn_recursive();
    }
    music_player.current_track = None;
}

/// Stops current music and starts a new track with looping playback.
pub fn play_music(
    commands: &mut Commands,
    asset_server: &AssetServer,
    music_player: &mut MusicPlayer,
    path: &str,
    category: MusicCategory,
) {
    // Stop current track first
    stop_music(commands, music_player);

    let handle: Handle<AudioSource> = asset_server.load(path);
    music_player.current_track = Some(handle.clone());
    music_player.category = category;

    let volume = music_player.effective_volume();

    let entity = commands
        .spawn((
            MusicEntity,
            AudioPlayer(handle),
            PlaybackSettings::LOOP.with_volume(bevy::audio::Volume::new(volume)),
        ))
        .id();

    music_player.current_entity = Some(entity);
}

/// Plays a one-shot music track (no looping), used for game over jingles etc.
pub fn play_music_oneshot(
    commands: &mut Commands,
    asset_server: &AssetServer,
    music_player: &mut MusicPlayer,
    path: &str,
    category: MusicCategory,
) {
    stop_music(commands, music_player);

    let handle: Handle<AudioSource> = asset_server.load(path);
    music_player.current_track = Some(handle.clone());
    music_player.category = category;

    let volume = music_player.effective_volume();

    let entity = commands
        .spawn((
            MusicEntity,
            AudioPlayer(handle),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(volume)),
        ))
        .id();

    music_player.current_entity = Some(entity);
}

/// Adjusts music playback volume when the MusicPlayer resource changes.
pub fn music_volume_control(
    music_player: Res<MusicPlayer>,
    music_query: Query<&AudioSink, With<MusicEntity>>,
) {
    if !music_player.is_changed() {
        return;
    }

    let target_volume = music_player.effective_volume();

    for sink in music_query.iter() {
        sink.set_volume(target_volume);
    }
}

/// Plays menu music when entering the Menu state.
pub fn menu_music_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut music_player: ResMut<MusicPlayer>,
    track_map: Res<MusicTrackMap>,
) {
    if music_player.category == MusicCategory::Menu && music_player.current_entity.is_some() {
        return;
    }

    if let Some(path) = track_map.get_random_track(MusicCategory::Menu) {
        let path = path.clone();
        play_music(
            &mut commands,
            &asset_server,
            &mut music_player,
            &path,
            MusicCategory::Menu,
        );
        info!("[Audio] Playing menu music: {}", path);
    }
}

/// Plays a random gameplay track when entering the Gameplay state.
pub fn gameplay_music_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut music_player: ResMut<MusicPlayer>,
    track_map: Res<MusicTrackMap>,
) {
    if music_player.category == MusicCategory::Gameplay && music_player.current_entity.is_some() {
        return;
    }

    if let Some(path) = track_map.get_random_track(MusicCategory::Gameplay) {
        let path = path.clone();
        play_music(
            &mut commands,
            &asset_server,
            &mut music_player,
            &path,
            MusicCategory::Gameplay,
        );
        info!("[Audio] Playing gameplay music: {}", path);
    }
}

/// Plays game over music when entering the GameOver state.
pub fn game_over_music_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut music_player: ResMut<MusicPlayer>,
    track_map: Res<MusicTrackMap>,
) {
    if let Some(path) = track_map.get_random_track(MusicCategory::GameOver) {
        let path = path.clone();
        play_music_oneshot(
            &mut commands,
            &asset_server,
            &mut music_player,
            &path,
            MusicCategory::GameOver,
        );
        info!("[Audio] Playing game over music: {}", path);
    }
}

/// Pauses music when the game enters the Paused gameplay sub-state.
pub fn pause_music_system(
    gameplay_state: Option<Res<State<GameplayState>>>,
    music_query: Query<&AudioSink, With<MusicEntity>>,
) {
    let Some(state) = gameplay_state else {
        return;
    };

    if !state.is_changed() {
        return;
    }

    match state.get() {
        GameplayState::Paused => {
            for sink in music_query.iter() {
                sink.pause();
            }
        }
        GameplayState::Playing | GameplayState::Countdown => {
            for sink in music_query.iter() {
                if sink.is_paused() {
                    sink.play();
                }
            }
        }
        _ => {}
    }
}

/// Stops music when leaving the Gameplay state (cleanup).
pub fn stop_music_on_exit(
    mut commands: Commands,
    mut music_player: ResMut<MusicPlayer>,
) {
    stop_music(&mut commands, &mut music_player);
}
