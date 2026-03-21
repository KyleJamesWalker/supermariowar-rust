use bevy::prelude::*;

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
pub struct Grounded(pub bool);

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
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}

impl PlayerBundle {
    pub fn new(index: usize, team: u8, spawn_x: f32, spawn_y: f32) -> Self {
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
            grounded: Grounded(false),
            powerup: PowerupState::None,
            shield: ShieldState::None,
            input: PlayerInput::default(),
            transform: Transform::from_xyz(spawn_x, spawn_y, 10.0),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            view_visibility: ViewVisibility::default(),
        }
    }
}
