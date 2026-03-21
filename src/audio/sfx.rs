use bevy::prelude::*;

/// Named sound effects matching the original game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfxId {
    Jump,
    Stomp,
    Kick,
    Coin,
    PowerupAppear,
    PowerupCollect,
    Fireball,
    BlockBreak,
    BlockHit,
    PlayerDeath,
    Invincible,
    ExtraLife,
    ThrowBlock,
    Skid,
    Bump,
    GameOver,
    Winner,
    Countdown,
    Transform,
}

/// Resource holding preloaded sound effect handles.
#[derive(Resource, Default)]
pub struct SfxLibrary {
    pub sounds: Vec<(SfxId, Handle<AudioSource>)>,
}

impl SfxLibrary {
    pub fn get(&self, id: SfxId) -> Option<&Handle<AudioSource>> {
        self.sounds.iter().find(|(sid, _)| *sid == id).map(|(_, h)| h)
    }
}

/// Plays a one-shot sound effect.
pub fn play_sfx(
    commands: &mut Commands,
    library: &SfxLibrary,
    id: SfxId,
) {
    if let Some(handle) = library.get(id) {
        commands.spawn(AudioPlayer(handle.clone()));
    }
}
