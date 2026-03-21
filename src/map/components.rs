use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Tile collision types (maps to the C++ TileType enum)
// ---------------------------------------------------------------------------

/// Collision/behaviour type stored per map cell in the top-layer collision map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CollisionType {
    NonSolid = 0,
    Solid = 1,
    SolidOnTop = 2,
    Ice = 3,
    Death = 4,
    SuperDeath = 5,
    PlayerDeath = 6,
    Gap = 10,
}

impl Default for CollisionType {
    fn default() -> Self {
        Self::NonSolid
    }
}

impl CollisionType {
    pub fn from_raw(v: i32) -> Self {
        match v {
            1 => Self::Solid,
            2 => Self::SolidOnTop,
            3 => Self::Ice,
            4 => Self::Death,
            5 => Self::SuperDeath,
            6 => Self::PlayerDeath,
            10 => Self::Gap,
            _ => Self::NonSolid,
        }
    }
}

// ---------------------------------------------------------------------------
// Core tile types
// ---------------------------------------------------------------------------

/// The visual/behaviour category of a tile (kept for legacy compatibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Empty,
    Solid,
    SolidOnTop,
    Ice,
    Death,
    SuperDeath,
    NonSolid,
    Animated,
}

impl Default for TileType {
    fn default() -> Self {
        Self::Empty
    }
}

// ---------------------------------------------------------------------------
// Marker for the map entity
// ---------------------------------------------------------------------------

#[derive(Component, Debug)]
pub struct GameMap {
    pub name: String,
    pub width: u32,
    pub height: u32,
}

// ---------------------------------------------------------------------------
// Individual tile entity
// ---------------------------------------------------------------------------

#[derive(Component, Debug, Clone)]
pub struct Tile {
    pub grid_x: u32,
    pub grid_y: u32,
    pub layer: u32,
    pub tile_type: TileType,
    /// Tileset index (maps into TilesetManager::tilesets).  -1 / < 0 means
    /// "no tileset" (empty tile).
    pub tileset_index: i8,
    /// Column within the tileset image.
    pub tileset_col: i8,
    /// Row within the tileset image.
    pub tileset_row: i8,
    /// Collision type at this cell (only meaningful for the "top" layer).
    pub collision: CollisionType,
}

// ---------------------------------------------------------------------------
// Animated tile
// ---------------------------------------------------------------------------

#[derive(Component, Debug, Clone)]
pub struct AnimatedTile {
    pub frame: u32,
    pub num_frames: u32,
    pub timer: Timer,
}

// ---------------------------------------------------------------------------
// Warp / interaction zones
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpDirection {
    Up = 0,
    Right = 1,
    Down = 2,
    Left = 3,
    Undefined = 4,
}

impl WarpDirection {
    pub fn from_raw(v: i32) -> Self {
        match v {
            0 => Self::Up,
            1 => Self::Right,
            2 => Self::Down,
            3 => Self::Left,
            _ => Self::Undefined,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WarpZone {
    pub id: i32,
    pub x: u32,
    pub y: u32,
    pub direction: WarpDirection,
    pub connection: i32,
}

#[derive(Debug, Clone)]
pub struct WarpExit {
    pub direction: WarpDirection,
    pub connection: i32,
    pub id: i32,
    pub x: i32,
    pub y: i32,
    pub lock_x: i32,
    pub lock_y: i32,
    pub warp_x: i32,
    pub warp_y: i32,
    pub num_blocks: i32,
}

/// Component for an entity that acts as a warp entry/exit area.
#[derive(Component, Debug, Clone)]
pub struct InteractionZone {
    pub zone_type: InteractionZoneType,
    pub rect: IRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionZoneType {
    WarpEntry { direction: WarpDirection, connection: i32 },
    WarpExit { direction: WarpDirection, connection: i32 },
}

// ---------------------------------------------------------------------------
// Spawn areas
// ---------------------------------------------------------------------------

/// Type index for spawn areas – matches original NUMSPAWNAREATYPES (6):
/// 0 = All players, 1-4 = Team 1-4, 5 = Items
pub const NUM_SPAWN_AREA_TYPES: usize = 6;

#[derive(Debug, Clone)]
pub struct SpawnArea {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub size: i32,
}

// ---------------------------------------------------------------------------
// Draw areas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DrawArea {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

// ---------------------------------------------------------------------------
// Moving platforms
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformPathType {
    StraightSegment = 0,
    StraightContinuous = 1,
    Ellipse = 2,
}

#[derive(Debug, Clone)]
pub enum PlatformPath {
    Straight {
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        speed: f32,
    },
    Continuous {
        start_x: f32,
        start_y: f32,
        angle: f32,
        speed: f32,
    },
    Ellipse {
        radius_x: f32,
        radius_y: f32,
        center_x: f32,
        center_y: f32,
        start_angle: f32,
        speed: f32,
    },
}

#[derive(Debug, Clone)]
pub struct PlatformTile {
    pub tileset_id: i8,
    pub col: i8,
    pub row: i8,
    pub collision: CollisionType,
}

#[derive(Component, Debug, Clone)]
pub struct MovingPlatform {
    pub tile_width: i32,
    pub tile_height: i32,
    pub tiles: Vec<PlatformTile>,
    pub draw_layer: i32,
    pub path: PlatformPath,
    /// Current position in Bevy coordinates (centered, Y-up).
    pub current_x: f32,
    pub current_y: f32,
    /// Frame-to-frame velocity in Bevy coordinates (derived from position delta).
    pub vel_x: f32,
    pub vel_y: f32,
    /// Accumulated angle for ellipse paths (radians).
    pub angle: f32,
    /// Direction multiplier for straight-segment paths (+1 or -1).
    pub direction: f32,
}

// ---------------------------------------------------------------------------
// Map items  (carried springs, spikes, kuribo shoe, etc.)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MapItemSpawn {
    pub item_type: i32,
    pub tile_x: i32,
    pub tile_y: i32,
}

// ---------------------------------------------------------------------------
// Map hazards  (fireball strings, roto-discs, piranha plants, etc.)
// ---------------------------------------------------------------------------

pub const NUM_MAP_HAZARD_PARAMS: usize = 5;

#[derive(Debug, Clone)]
pub struct MapHazard {
    pub hazard_type: i32,
    pub tile_x: i32,
    pub tile_y: i32,
    pub iparam: [i32; NUM_MAP_HAZARD_PARAMS],
    pub dparam: [f32; NUM_MAP_HAZARD_PARAMS],
}

/// Component for a hazard spawner entity.
#[derive(Component, Debug, Clone)]
pub struct HazardSpawner {
    pub hazard: MapHazard,
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BackgroundLayer {
    pub file: String,
}

// ---------------------------------------------------------------------------
// Per-tile object block data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ObjectBlock {
    pub block_type: i8,
    pub hidden: bool,
    pub settings: [i8; 8],
}

// ---------------------------------------------------------------------------
// Per-tile warp data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct PerTileWarp {
    pub direction: i32,
    pub connection: i32,
    pub id: i32,
}

// ---------------------------------------------------------------------------
// Tileset tile reference (col/row within a named tileset)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct TilesetTileRef {
    /// Tileset id (-2 = none, >=0 = index)
    pub id: i8,
    pub col: i8,
    pub row: i8,
}

// ---------------------------------------------------------------------------
// Main map data resource
// ---------------------------------------------------------------------------

/// Resource holding everything parsed from a .map file.
#[derive(Resource, Debug, Default)]
pub struct MapData {
    /// Tile sheet references: [col][row][layer]  (column-major like C++ original)
    pub tiles: Vec<Vec<Vec<TilesetTileRef>>>,
    /// Object/block data: [col][row]
    pub object_data: Vec<Vec<ObjectBlock>>,
    /// Top-layer collision types: [col][row]
    pub collision_map: Vec<Vec<CollisionType>>,
    /// Per-tile warp placement data: [col][row]
    pub warp_data: Vec<Vec<PerTileWarp>>,
    /// No-spawn flags: [type][col][row]
    pub no_spawn: Vec<Vec<Vec<bool>>>,
    /// Spawn areas per type.
    pub spawn_areas: Vec<Vec<SpawnArea>>,
    /// Background file name.
    pub background_file: String,
    /// Switch states (4 on/off switches).
    pub switches: [i32; 4],
    /// Warp exits.
    pub warp_exits: Vec<WarpExit>,
    /// Draw areas (foreground optimisation rects).
    pub draw_areas: Vec<DrawArea>,
    /// Moving platforms.
    pub platforms: Vec<MovingPlatform>,
    /// Map items (carryable objects placed on map).
    pub map_items: Vec<MapItemSpawn>,
    /// Map hazards.
    pub map_hazards: Vec<MapHazard>,
    /// Eye candy layer flags.
    pub eyecandy: [i32; 3],
    /// Music category id.
    pub music_category: i32,
    /// Tileset names referenced by the map (index → name).
    pub tileset_names: Vec<(i16, String)>,
    /// Whether loading succeeded fully.
    pub loaded: bool,
}
