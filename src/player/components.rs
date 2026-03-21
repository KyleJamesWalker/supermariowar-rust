use bevy::prelude::*;

use crate::map::components::WarpDirection;

/// Marker component identifying a player entity.
#[derive(Component, Debug)]
pub struct Player {
    pub index: usize,
    pub team: u8,
    pub lives: i32,
    pub score: i32,
}

/// Current velocity of a player.
#[derive(Component, Debug, Default)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

/// Player state machine.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAction {
    #[default]
    Standing,
    Running,
    Jumping,
    Falling,
    Skidding,
    Dead,
    Spawning,
}

/// Direction the player is facing.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    #[default]
    Right,
    Left,
}

/// Tracks whether the player is on the ground.
#[derive(Component, Debug, Default)]
pub struct Grounded {
    pub on_ground: bool,
    pub on_ice: bool,
}

/// Invincibility timer (e.g., after being hit or respawning).
#[derive(Component, Debug)]
pub struct Invincible {
    pub timer: Timer,
}

/// Currently held powerup.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PowerupState {
    #[default]
    None,
    Fire,
    Ice,
    Hammer,
    Boomerang,
    Feather,
    Bomb,
    Leaf,
    Star,
    Jail,
}

/// Shield status.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ShieldState {
    #[default]
    None,
    Soft,
    Hard,
}

/// Player input state for the current frame.
#[derive(Component, Debug, Default)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub down: bool,
    pub turbo: bool,
    pub use_item: bool,
    pub jump_pressed: bool,
    pub pause_pressed: bool,
}

/// Tracks warp pipe entry/exit animation state.
#[derive(Component, Debug)]
pub struct WarpState {
    pub direction: WarpDirection,
    pub exit_direction: WarpDirection,
    pub progress: f32,
    pub warp_id: u32,
    pub dest_x: f32,
    pub dest_y: f32,
    pub entering: bool,
}

impl WarpState {
    pub fn new_entering(
        direction: WarpDirection,
        exit_direction: WarpDirection,
        warp_id: u32,
        dest_x: f32,
        dest_y: f32,
    ) -> Self {
        Self {
            direction,
            exit_direction,
            progress: 0.0,
            warp_id,
            dest_x,
            dest_y,
            entering: true,
        }
    }
}

/// Tracks per-player kill/death/game statistics.
#[derive(Component, Debug, Default)]
pub struct PlayerStats {
    pub kills: u32,
    pub deaths: u32,
    pub suicides: u32,
    pub stomps: u32,
}

/// Sub-state for cape attack (Feather powerup).
#[derive(Component, Debug)]
pub struct CapeState {
    pub active: bool,
    pub timer: Timer,
}

impl Default for CapeState {
    fn default() -> Self {
        Self {
            active: false,
            // C++ original: 12 active frames at ~62fps ≈ 0.19s
            timer: Timer::from_seconds(0.19, TimerMode::Once),
        }
    }
}

/// Sub-state for tail attack (Leaf powerup).
#[derive(Component, Debug)]
pub struct TailState {
    pub active: bool,
    pub timer: Timer,
}

impl Default for TailState {
    fn default() -> Self {
        Self {
            active: false,
            // C++ original: 12 active frames at ~62fps ≈ 0.19s
            timer: Timer::from_seconds(0.19, TimerMode::Once),
        }
    }
}

/// Tracks if the player is carrying an object (shell, throw block, etc.).
#[derive(Component, Debug, Default)]
pub struct CarriedObject {
    pub entity: Option<Entity>,
}

/// Applied when a player is frozen by ice or clock effect.
#[derive(Component, Debug)]
pub struct FrozenState {
    pub timer: Timer,
}

impl FrozenState {
    pub fn new(duration: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

/// Death animation state -- player bounces up then falls off screen.
#[derive(Component, Debug)]
pub struct BurnupTimer {
    pub timer: Timer,
}

impl Default for BurnupTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        }
    }
}

/// Brief invulnerability after spawning.
#[derive(Component, Debug)]
pub struct SpawnProtection {
    pub timer: Timer,
}

impl SpawnProtection {
    pub fn new(duration: f32) -> Self {
        Self {
            timer: Timer::from_seconds(duration, TimerMode::Once),
        }
    }
}

impl Default for SpawnProtection {
    fn default() -> Self {
        Self::new(3.0)
    }
}

/// Indicator for jail mode -- player has a key.
#[derive(Component, Debug, Default)]
pub struct JailKey {
    pub has_key: bool,
}

/// Which skin/sprite set to render for this player.
#[derive(Component, Debug)]
pub struct PlayerSkinIndex(pub usize);

impl Default for PlayerSkinIndex {
    fn default() -> Self {
        Self(0)
    }
}

/// How a player died.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCause {
    Killed { by: usize },
    Suicide,
    Environment,
    OutOfLives,
}

/// Event: a player has died.
#[derive(Event, Debug)]
pub struct PlayerDeathEvent {
    pub player_entity: Entity,
    pub player_index: usize,
    pub killer_index: Option<usize>,
    pub cause: DeathCause,
}

/// Event: a player has stomped another player.
#[derive(Event, Debug)]
pub struct PlayerStompEvent {
    pub stomper_entity: Entity,
    pub stomped_entity: Entity,
    pub stomper_index: usize,
    pub stomped_index: usize,
}

/// Event: a player has respawned.
#[derive(Event, Debug)]
pub struct PlayerRespawnEvent {
    pub player_entity: Entity,
    pub player_index: usize,
}

/// Event: pause requested.
#[derive(Event, Debug)]
pub struct PauseEvent {
    pub player_index: usize,
}

/// Stores the player's position from the previous frame for cross-frame collision checks.
/// C++ original tracks fOldX/fOldY used for reliable stomp detection.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct PreviousTransform {
    pub translation: Vec3,
}

/// Bundle for spawning a complete player entity.
#[derive(Bundle)]
pub struct PlayerBundle {
    pub player: Player,
    pub velocity: Velocity,
    pub action: PlayerAction,
    pub facing: Facing,
    pub grounded: Grounded,
    pub powerup: PowerupState,
    pub shield: ShieldState,
    pub input: PlayerInput,
    pub stats: PlayerStats,
    pub carried_object: CarriedObject,
    pub jail_key: JailKey,
    pub skin_index: PlayerSkinIndex,
    pub previous_transform: PreviousTransform,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

impl PlayerBundle {
    pub fn new(index: usize, team: u8, spawn_x: f32, spawn_y: f32) -> Self {
        let pos = Vec3::new(spawn_x, spawn_y, 10.0);
        Self {
            player: Player {
                index,
                team,
                lives: 5,
                score: 0,
            },
            velocity: Velocity::default(),
            action: PlayerAction::default(),
            facing: Facing::default(),
            grounded: Grounded::default(),
            powerup: PowerupState::None,
            shield: ShieldState::None,
            input: PlayerInput::default(),
            stats: PlayerStats::default(),
            carried_object: CarriedObject::default(),
            jail_key: JailKey::default(),
            skin_index: PlayerSkinIndex(index),
            previous_transform: PreviousTransform { translation: pos },
            transform: Transform::from_xyz(spawn_x, spawn_y, 10.0),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }
}
