use std::collections::HashMap;

use bevy::prelude::*;

/// Named sound effects matching the original game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfxId {
    // Player movement
    Jump,
    DoubleJump,
    Stomp,
    Kick,
    Skid,
    Bump,
    Spring,

    // Collectibles
    Coin,
    PowerupAppear,
    PowerupCollect,

    // Projectiles
    Fireball,
    IceBall,
    Hammer,
    Boomerang,
    Bomb,
    BombExplode,

    // Blocks
    BlockBreak,
    BlockHit,
    BlockBump,

    // Player state
    PlayerDeath,
    PlayerRespawn,
    Invincible,
    Star,
    ExtraLife,
    OneUp,

    // Interaction
    ThrowBlock,
    ShellKick,
    Transform,
    Warp,

    // Game flow
    GameStart,
    GameOver,
    Winner,
    Draw,
    Countdown321,
    CountdownGo,

    // Menu
    MenuSelect,
    MenuMove,
    MenuBack,

    // CTF / objectives
    FlagPickup,
    FlagCapture,
    FlagReturn,

    // Hazards
    Thwomp,
    Cannon,
    PiranhaChomp,
}

/// Event that systems can send to trigger a sound effect.
#[derive(Event, Debug, Clone)]
pub struct SfxEvent {
    pub id: SfxId,
    pub volume: Option<f32>,
}

impl SfxEvent {
    pub fn new(id: SfxId) -> Self {
        Self { id, volume: None }
    }

    pub fn with_volume(id: SfxId, volume: f32) -> Self {
        Self {
            id,
            volume: Some(volume),
        }
    }
}

/// Global SFX volume multiplier (0.0 to 1.0).
#[derive(Resource, Debug)]
pub struct SfxVolume {
    pub volume: f32,
    pub muted: bool,
}

impl Default for SfxVolume {
    fn default() -> Self {
        Self {
            volume: 1.0,
            muted: false,
        }
    }
}

impl SfxVolume {
    pub fn effective_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume
        }
    }
}

/// Resource holding preloaded sound effect handles, keyed by SfxId.
#[derive(Resource, Default)]
pub struct SfxLibrary {
    pub sounds: HashMap<SfxId, Handle<AudioSource>>,
}

impl SfxLibrary {
    pub fn get(&self, id: SfxId) -> Option<&Handle<AudioSource>> {
        self.sounds.get(&id)
    }

    pub fn insert(&mut self, id: SfxId, handle: Handle<AudioSource>) {
        self.sounds.insert(id, handle);
    }
}

/// Startup system that loads all sound effects from the asset directory.
pub fn load_sfx(mut sfx_library: ResMut<SfxLibrary>, asset_server: Res<AssetServer>) {
    let mappings: Vec<(SfxId, &str)> = vec![
        // Player movement
        (SfxId::Jump, "sfx/packs/Classic/jump.wav"),
        (SfxId::DoubleJump, "sfx/packs/Classic/springjump.wav"),
        (SfxId::Stomp, "sfx/packs/Classic/hit.wav"),
        (SfxId::Kick, "sfx/packs/Classic/kick.wav"),
        (SfxId::Skid, "sfx/packs/Classic/skid.wav"),
        (SfxId::Bump, "sfx/packs/Classic/bump.wav"),
        (SfxId::Spring, "sfx/packs/Classic/springjump.wav"),
        // Collectibles
        (SfxId::Coin, "sfx/packs/Classic/coin.wav"),
        (SfxId::PowerupAppear, "sfx/packs/Classic/storedpowerup.wav"),
        (SfxId::PowerupCollect, "sfx/packs/Classic/collectpowerup.wav"),
        // Projectiles
        (SfxId::Fireball, "sfx/packs/Classic/fireball.wav"),
        (SfxId::IceBall, "sfx/packs/Classic/flamecannon.wav"),
        (SfxId::Hammer, "sfx/packs/Classic/throw.wav"),
        (SfxId::Boomerang, "sfx/packs/Classic/boomerang.wav"),
        (SfxId::Bomb, "sfx/packs/Classic/bob-omb.wav"),
        (SfxId::BombExplode, "sfx/packs/Classic/bob-omb.wav"),
        // Blocks
        (SfxId::BlockBreak, "sfx/packs/Classic/breakblock.wav"),
        (SfxId::BlockHit, "sfx/packs/Classic/hit.wav"),
        (SfxId::BlockBump, "sfx/packs/Classic/bump.wav"),
        // Player state
        (SfxId::PlayerDeath, "sfx/packs/Classic/death.wav"),
        (SfxId::PlayerRespawn, "sfx/packs/Classic/enter-stage.wav"),
        (SfxId::Invincible, "sfx/packs/Classic/invincible.wav"),
        (SfxId::Star, "sfx/packs/Classic/starwarning.wav"),
        (SfxId::ExtraLife, "sfx/packs/Classic/1up.wav"),
        (SfxId::OneUp, "sfx/packs/Classic/1up.wav"),
        // Interaction
        (SfxId::ThrowBlock, "sfx/packs/Classic/throw.wav"),
        (SfxId::ShellKick, "sfx/packs/Classic/kick.wav"),
        (SfxId::Transform, "sfx/packs/Classic/transform.wav"),
        (SfxId::Warp, "sfx/packs/Classic/warp.wav"),
        // Game flow
        (SfxId::GameStart, "sfx/packs/Classic/race.wav"),
        (SfxId::GameOver, "sfx/packs/Classic/gameover.wav"),
        (SfxId::Winner, "sfx/packs/Classic/gameover.wav"),
        (SfxId::Draw, "sfx/packs/Classic/gameover.wav"),
        (SfxId::Countdown321, "sfx/packs/Classic/clock.wav"),
        (SfxId::CountdownGo, "sfx/packs/Classic/race.wav"),
        // Menu
        (SfxId::MenuSelect, "sfx/packs/Classic/switchpress.wav"),
        (SfxId::MenuMove, "sfx/packs/Classic/mapmove.wav"),
        (SfxId::MenuBack, "sfx/packs/Classic/bump.wav"),
        // CTF / objectives
        (SfxId::FlagPickup, "sfx/packs/Classic/pickup.wav"),
        (SfxId::FlagCapture, "sfx/packs/Classic/pickup.wav"),
        (SfxId::FlagReturn, "sfx/packs/Classic/pickup.wav"),
        // Hazards
        (SfxId::Thwomp, "sfx/packs/Classic/impact.wav"),
        (SfxId::Cannon, "sfx/packs/Classic/cannon.wav"),
        (SfxId::PiranhaChomp, "sfx/packs/Classic/mip.wav"),
    ];

    for (id, path) in mappings {
        let handle: Handle<AudioSource> = asset_server.load(path);
        sfx_library.insert(id, handle);
    }

    info!("[Audio] Loaded {} sound effect mappings", sfx_library.sounds.len());
}

/// Plays a one-shot sound effect at the global SFX volume.
pub fn play_sfx(commands: &mut Commands, library: &SfxLibrary, sfx_vol: &SfxVolume, id: SfxId) {
    let vol = sfx_vol.effective_volume();
    if vol <= 0.0 {
        return;
    }
    if let Some(handle) = library.get(id) {
        commands.spawn((
            AudioPlayer(handle.clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(vol)),
        ));
    }
}

/// Plays a one-shot sound effect with a custom volume multiplier.
pub fn play_sfx_with_volume(
    commands: &mut Commands,
    library: &SfxLibrary,
    id: SfxId,
    volume: f32,
) {
    if let Some(handle) = library.get(id) {
        commands.spawn((
            AudioPlayer(handle.clone()),
            PlaybackSettings::DESPAWN.with_volume(bevy::audio::Volume::new(volume)),
        ));
    }
}

/// System that listens for SfxEvent and plays the requested sound effect.
pub fn sfx_event_handler(
    mut commands: Commands,
    sfx_library: Res<SfxLibrary>,
    sfx_volume: Res<SfxVolume>,
    mut sfx_events: EventReader<SfxEvent>,
) {
    let global_vol = sfx_volume.effective_volume();
    if global_vol <= 0.0 {
        sfx_events.clear();
        return;
    }

    for event in sfx_events.read() {
        let vol = event.volume.unwrap_or(1.0) * global_vol;
        if vol > 0.0 {
            play_sfx_with_volume(&mut commands, &sfx_library, event.id, vol);
        }
    }
}

// ---------------------------------------------------------------------------
// Game event -> SFX bridge systems
// ---------------------------------------------------------------------------

/// Plays the Jump sound when a player enters the Jumping action.
pub fn jump_sfx_system(
    query: Query<&crate::player::components::PlayerAction, Changed<crate::player::components::PlayerAction>>,
    mut sfx_events: EventWriter<SfxEvent>,
) {
    for action in query.iter() {
        if *action == crate::player::components::PlayerAction::Jumping {
            sfx_events.send(SfxEvent::new(SfxId::Jump));
        }
    }
}

/// Plays the Stomp sound when a player stomps another.
pub fn stomp_sfx_system(
    mut stomp_events: EventReader<crate::player::components::PlayerStompEvent>,
    mut sfx_events: EventWriter<SfxEvent>,
) {
    for _event in stomp_events.read() {
        sfx_events.send(SfxEvent::new(SfxId::Stomp));
    }
}

/// Plays the PlayerDeath sound when a player dies.
pub fn death_sfx_system(
    mut death_events: EventReader<crate::player::components::PlayerDeathEvent>,
    mut sfx_events: EventWriter<SfxEvent>,
) {
    for _event in death_events.read() {
        sfx_events.send(SfxEvent::new(SfxId::PlayerDeath));
    }
}

/// Plays BlockHit or BlockBreak sounds when block hit events fire.
pub fn block_sfx_system(
    mut hit_events: EventReader<crate::objects::components::BlockHitEvent>,
    mut sfx_events: EventWriter<SfxEvent>,
) {
    for _event in hit_events.read() {
        sfx_events.send(SfxEvent::new(SfxId::BlockHit));
    }
}

/// Plays PowerupCollect sound on powerup collection.
pub fn powerup_sfx_system(
    mut collect_events: EventReader<crate::objects::components::PowerupCollectEvent>,
    mut sfx_events: EventWriter<SfxEvent>,
) {
    for _event in collect_events.read() {
        sfx_events.send(SfxEvent::new(SfxId::PowerupCollect));
    }
}

/// Plays countdown sounds (3, 2, 1, GO!) during the Countdown gameplay state.
pub fn countdown_sfx_system(
    countdown: Res<crate::game_modes::traits::RoundCountdown>,
    mut sfx_events: EventWriter<SfxEvent>,
) {
    if !countdown.is_changed() {
        return;
    }

    match countdown.phase {
        crate::game_modes::traits::CountdownPhase::Three
        | crate::game_modes::traits::CountdownPhase::Two
        | crate::game_modes::traits::CountdownPhase::One => {
            sfx_events.send(SfxEvent::new(SfxId::Countdown321));
        }
        crate::game_modes::traits::CountdownPhase::Go => {
            sfx_events.send(SfxEvent::new(SfxId::CountdownGo));
            sfx_events.send(SfxEvent::new(SfxId::GameStart));
        }
        crate::game_modes::traits::CountdownPhase::Done => {}
    }
}

/// Plays menu navigation sounds when the menu selection changes.
pub fn menu_sfx_system(
    selection: Option<Res<crate::menu::components::MenuSelection>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sfx_events: EventWriter<SfxEvent>,
) {
    let Some(_selection) = selection else {
        return;
    };

    let moved = keyboard.just_pressed(KeyCode::ArrowUp)
        || keyboard.just_pressed(KeyCode::ArrowDown)
        || keyboard.just_pressed(KeyCode::KeyW)
        || keyboard.just_pressed(KeyCode::KeyS);

    let selected = keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::Space);

    let back = keyboard.just_pressed(KeyCode::Escape);

    if moved {
        sfx_events.send(SfxEvent::new(SfxId::MenuMove));
    }
    if selected {
        sfx_events.send(SfxEvent::new(SfxId::MenuSelect));
    }
    if back {
        sfx_events.send(SfxEvent::new(SfxId::MenuBack));
    }
}
