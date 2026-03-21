// Screen dimensions (original SMW resolution)
pub const SCREEN_WIDTH: f32 = 640.0;
pub const SCREEN_HEIGHT: f32 = 480.0;
pub const SCREEN_WIDTH_U32: u32 = 640;
pub const SCREEN_HEIGHT_U32: u32 = 480;

// Tile dimensions
pub const TILE_SIZE: f32 = 32.0;
pub const TILE_SIZE_U32: u32 = 32;
pub const PREVIEW_TILE_SIZE: f32 = 16.0;
pub const THUMB_TILE_SIZE: f32 = 8.0;

// Map dimensions (in tiles)
pub const MAP_WIDTH: u32 = 20;
pub const MAP_HEIGHT: u32 = 15;
pub const MAP_LAYERS: u32 = 4;

// Player dimensions
pub const PLAYER_WIDTH: f32 = 22.0;
pub const PLAYER_HEIGHT: f32 = 25.0;
pub const HALF_PLAYER_WIDTH: f32 = 11.0;
pub const HALF_PLAYER_HEIGHT: f32 = 12.0;
pub const PLAYER_HEIGHT_OFFSET: f32 = 4.0;
pub const PLAYER_WIDTH_OFFSET: f32 = 5.0;

// Physics constants
pub const GRAVITY: f32 = 0.40;
pub const MAX_FALL_SPEED: f32 = 20.0;
pub const MAX_SIDE_FALL_SPEED: f32 = 10.0;

// Movement speeds
pub const VEL_MOVING: f32 = 4.0;
pub const VEL_SLOW_MOVING: f32 = 2.2;
pub const VEL_TURBO_MOVING: f32 = 5.5;
pub const VEL_MOVING_ADD: f32 = 0.5;
pub const VEL_MOVING_FRICTION: f32 = 0.2;
pub const VEL_ICE_FRICTION: f32 = 0.06;
pub const VEL_AIR_FRICTION: f32 = 0.06;
pub const MAX_RUN_SPEED: f32 = 5.5;

// Jump velocities
pub const JUMP_VELOCITY: f32 = 9.0;
pub const VEL_SLOW_JUMP: f32 = 7.0;
pub const VEL_TURBO_JUMP: f32 = 10.2;
pub const VEL_SUPER_JUMP: f32 = 13.0;
pub const VEL_STOP_JUMP: f32 = 5.0;

// Combat / interaction velocities
pub const VEL_PUSHBACK: f32 = 5.0;
pub const VEL_ENEMY_BOUNCE: f32 = 7.0;
pub const VEL_POWERUP_BOUNCE: f32 = 8.1;
pub const FIREBALL_BOUNCE: f32 = 5.0;
pub const HAMMER_THROW: f32 = 8.5;
pub const BOUNCE_STRENGTH: f32 = 0.5;

// Block interaction
pub const VEL_MAX_BREAK_BLOCK: f32 = 3.0;
pub const VEL_NOTE_BLOCK_BOUNCE: f32 = 3.0;
pub const VEL_NOTE_BLOCK_REPEL: f32 = 5.0;
pub const VEL_BLOCK_BOUNCE: f32 = 3.0;
pub const VEL_BLOCK_REPEL: f32 = 3.0;

// Game rules
pub const MAX_PLAYERS: usize = 4;
pub const FRAMES_PER_SECOND: u32 = 62; // Original runs at ~62.5 FPS (16ms frame time)
pub const FRAME_TIME_MS: u32 = 16;

// Map limits
pub const MAX_WARPS: usize = 32;
pub const MAX_MAP_ITEMS: usize = 32;
pub const MAX_MAP_HAZARDS: usize = 30;
pub const MAX_SPAWN_AREAS: usize = 128;
pub const MAX_DRAW_AREAS: usize = 128;

// Tileset
pub const TILESET_WIDTH: u32 = 32;
pub const TILESET_HEIGHT: u32 = 30;

// Powerups
pub const NUM_POWERUPS: usize = 26;

// Eye candy / cosmetics
pub const MAX_EYE_CANDY: usize = 192;
pub const CORPSE_STAY_FRAMES: usize = 200;

// Scoring
pub const MAX_AWARDS: usize = 10;
pub const MIN_AWARDS_NEEDED: usize = 3;
pub const MAX_TOURNAMENT_GAMES: usize = 10;
