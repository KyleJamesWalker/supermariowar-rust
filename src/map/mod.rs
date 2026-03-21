pub mod components;
pub mod loader;
pub mod tileset;

use bevy::prelude::*;
use std::path::PathBuf;

use crate::ai::components::{AiControlled, AiIntent, AiMemory, AiPathfinding, AiPerception};
use crate::core::constants::*;
use crate::core::game_state::GameState;
use crate::menu::components::{ControlType, CpuDifficulty, GameSetupConfig};
use crate::player::components::PlayerBundle;
use crate::physics::components::Collider;
use components::*;
use tileset::*;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapData>()
            .init_resource::<TilesetManager>()
            .init_resource::<TileAnimationTimer>()
            .add_systems(Startup, load_tilesets)
            .add_systems(OnEnter(GameState::Gameplay), setup_gameplay)
            .add_systems(OnExit(GameState::Gameplay), cleanup_gameplay)
            .add_systems(
                Update,
                (
                    update_animated_tiles,
                    update_moving_platforms,
                )
                    .run_if(in_state(GameState::Gameplay)),
            );
    }
}

// ---------------------------------------------------------------------------
// Marker components for cleanup
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct GameplayEntity;

// ---------------------------------------------------------------------------
// Animation timer resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct TileAnimationTimer {
    timer: Timer,
    frame: u32,
}

impl Default for TileAnimationTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.2, TimerMode::Repeating),
            frame: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Gameplay setup
// ---------------------------------------------------------------------------

fn setup_gameplay(
    mut commands: Commands,
    mut map_data: ResMut<MapData>,
    tileset_mgr: Res<TilesetManager>,
    game_setup: Res<GameSetupConfig>,
) {
    // Pick a map file -- for now default to 0smw.map
    let map_path = PathBuf::from("data/maps/0smw.map");
    *map_data = loader::load_map_from_file(&map_path);

    // Build the tileset translation (map-file tileset ids -> loaded indices)
    let translation = build_tileset_translation(&map_data, &tileset_mgr);

    // Spawn tile entities for each non-empty tile
    spawn_tile_entities(&mut commands, &map_data, &tileset_mgr, &translation);

    // Spawn moving platform entities
    spawn_platforms(&mut commands, &map_data, &tileset_mgr, &translation);

    // Spawn map hazards (thwomps, piranha plants, etc.)
    spawn_map_hazards(&mut commands, &map_data);

    // Spawn interactive blocks from object_data
    spawn_blocks_from_object_data(&mut commands, &map_data);

    // Spawn map items (coins, powerup boxes, etc.)
    spawn_map_items(&mut commands, &map_data);

    // Spawn players based on GameSetupConfig (respects menu player selection)
    spawn_players_from_config(&mut commands, &map_data, &game_setup);

    info!("Gameplay map setup complete (loaded={})", map_data.loaded);
}

// ---------------------------------------------------------------------------
// Tileset translation
// ---------------------------------------------------------------------------

/// Maps a map-file tileset-id (i8) → index into TilesetManager.tilesets.
/// Indices not found fall back to Classic.
fn build_tileset_translation(
    md: &MapData,
    mgr: &TilesetManager,
) -> Vec<(i16, usize)> {
    let mut out = Vec::new();
    for (file_id, name) in &md.tileset_names {
        let idx = resolve_tileset_index(mgr, name);
        out.push((*file_id, idx));
    }
    out
}

fn translate_tileset_id(translation: &[(i16, usize)], file_id: i8) -> Option<usize> {
    translation.iter().find(|(fid, _)| *fid == file_id as i16).map(|(_, idx)| *idx)
}

// ---------------------------------------------------------------------------
// Tile entity spawning
// ---------------------------------------------------------------------------

fn spawn_tile_entities(
    commands: &mut Commands,
    md: &MapData,
    mgr: &TilesetManager,
    translation: &[(i16, usize)],
) {
    let w = md.tiles.len();
    if w == 0 {
        return;
    }
    let h = md.tiles[0].len();

    for col in 0..w {
        for row in 0..h {
            let layers = &md.tiles[col][row];
            for (layer_idx, tref) in layers.iter().enumerate() {
                // Skip truly empty tiles: -2 = TILESETNONE, <= -3 = TILESETUNKNOWN
                if tref.id <= -2 {
                    continue;
                }

                // Determine collision for this cell
                let collision = if col < md.collision_map.len()
                    && row < md.collision_map[col].len()
                {
                    md.collision_map[col][row]
                } else {
                    CollisionType::NonSolid
                };

                let px = tile_to_bevy_x(col as u32);
                let py = tile_to_bevy_y(row as u32);
                let pz = layer_idx as f32 * 0.1;

                if tref.id == -1 {
                    // TILESETANIMATED: spawn with the first loaded tileset
                    // and attach an AnimatedTile component
                    let resolved_idx = mgr.classic_index.unwrap_or(0);
                    let Some(loaded) = mgr.get(resolved_idx) else {
                        continue;
                    };

                    let atlas_index = (tref.row as u32) * loaded.definition.columns + (tref.col as u32);

                    commands.spawn((
                        Sprite {
                            image: loaded.texture.clone(),
                            texture_atlas: Some(TextureAtlas {
                                layout: loaded.atlas_layout.clone(),
                                index: atlas_index as usize,
                            }),
                            ..default()
                        },
                        Transform::from_xyz(px, py, pz),
                        Tile {
                            grid_x: col as u32,
                            grid_y: row as u32,
                            layer: layer_idx as u32,
                            tile_type: TileType::Animated,
                            tileset_index: tref.id,
                            tileset_col: tref.col,
                            tileset_row: tref.row,
                            collision,
                        },
                        AnimatedTile {
                            frame: 0,
                            num_frames: 4,
                        },
                        GameplayEntity,
                    ));
                } else {
                    // Normal tile (id >= 0)
                    let resolved_idx = translate_tileset_id(translation, tref.id)
                        .or(mgr.classic_index)
                        .unwrap_or(0);

                    let Some(loaded) = mgr.get(resolved_idx) else {
                        continue;
                    };

                    let atlas_index = (tref.row as u32) * loaded.definition.columns + (tref.col as u32);

                    commands.spawn((
                        Sprite {
                            image: loaded.texture.clone(),
                            texture_atlas: Some(TextureAtlas {
                                layout: loaded.atlas_layout.clone(),
                                index: atlas_index as usize,
                            }),
                            ..default()
                        },
                        Transform::from_xyz(px, py, pz),
                        Tile {
                            grid_x: col as u32,
                            grid_y: row as u32,
                            layer: layer_idx as u32,
                            tile_type: tile_type_from_collision(collision),
                            tileset_index: tref.id,
                            tileset_col: tref.col,
                            tileset_row: tref.row,
                            collision,
                        },
                        GameplayEntity,
                    ));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Platform entity spawning
// ---------------------------------------------------------------------------

/// Convert SMW pixel X (0..SCREEN_WIDTH, left-origin) to Bevy X (centered).
fn smw_to_bevy_x(smw_x: f32) -> f32 {
    smw_x - SCREEN_WIDTH / 2.0
}

/// Convert SMW pixel Y (0..SCREEN_HEIGHT, top-origin, Y-down) to Bevy Y (centered, Y-up).
fn smw_to_bevy_y(smw_y: f32) -> f32 {
    SCREEN_HEIGHT / 2.0 - smw_y
}

fn spawn_platforms(
    commands: &mut Commands,
    md: &MapData,
    mgr: &TilesetManager,
    translation: &[(i16, usize)],
) {
    for platform in &md.platforms {
        let base_z = platform.draw_layer as f32 * 0.1 + 0.5;

        // Pre-collect child tile data so we can spawn them as children
        let mut child_tiles = Vec::new();
        for cx in 0..platform.tile_width {
            for ry in 0..platform.tile_height {
                let idx = (cx * platform.tile_height + ry) as usize;
                if idx >= platform.tiles.len() {
                    continue;
                }
                let pt = &platform.tiles[idx];
                if pt.tileset_id < 0 {
                    continue;
                }

                let resolved = translate_tileset_id(translation, pt.tileset_id)
                    .or(mgr.classic_index)
                    .unwrap_or(0);

                let Some(loaded) = mgr.get(resolved) else {
                    continue;
                };

                let atlas_index = (pt.row as u32) * loaded.definition.columns + (pt.col as u32);

                let local_x = cx as f32 * TILE_SIZE;
                let local_y = -(ry as f32 * TILE_SIZE);

                child_tiles.push((loaded.texture.clone(), loaded.atlas_layout.clone(), atlas_index, local_x, local_y));
            }
        }

        // Compute collider size from tile dimensions
        let collider_w = platform.tile_width as f32 * TILE_SIZE;
        let collider_h = platform.tile_height as f32 * TILE_SIZE;

        // Convert platform data from SMW pixel space to Bevy coordinates
        let mut bevy_platform = platform.clone();

        // Convert current position to Bevy space
        bevy_platform.current_x = smw_to_bevy_x(platform.current_x);
        bevy_platform.current_y = smw_to_bevy_y(platform.current_y);

        // Convert path coordinates to Bevy space
        bevy_platform.path = match &platform.path {
            PlatformPath::Straight { start_x, start_y, end_x, end_y, speed } => {
                PlatformPath::Straight {
                    start_x: smw_to_bevy_x(*start_x),
                    start_y: smw_to_bevy_y(*start_y),
                    end_x: smw_to_bevy_x(*end_x),
                    end_y: smw_to_bevy_y(*end_y),
                    speed: *speed,
                }
            }
            PlatformPath::Continuous { start_x, start_y, angle, speed } => {
                PlatformPath::Continuous {
                    start_x: smw_to_bevy_x(*start_x),
                    start_y: smw_to_bevy_y(*start_y),
                    angle: *angle,
                    speed: *speed,
                }
            }
            PlatformPath::Ellipse { radius_x, radius_y, center_x, center_y, start_angle, speed } => {
                PlatformPath::Ellipse {
                    radius_x: *radius_x,
                    radius_y: *radius_y,
                    center_x: smw_to_bevy_x(*center_x),
                    center_y: smw_to_bevy_y(*center_y),
                    start_angle: *start_angle,
                    speed: *speed,
                }
            }
        };

        let bx = bevy_platform.current_x;
        let by = bevy_platform.current_y;

        // Spawn a parent entity for the platform with a Collider
        let platform_entity = commands
            .spawn((
                Transform::from_xyz(bx, by, base_z),
                Visibility::default(),
                bevy_platform,
                Collider::new(collider_w, collider_h),
                crate::physics::components::CollisionLayer::Platform,
                GameplayEntity,
            ))
            .id();

        // Spawn child tile sprites parented to the platform entity
        commands.entity(platform_entity).with_children(|parent| {
            for (texture, atlas_layout, atlas_index, local_x, local_y) in child_tiles {
                parent.spawn((
                    Sprite {
                        image: texture,
                        texture_atlas: Some(TextureAtlas {
                            layout: atlas_layout,
                            index: atlas_index as usize,
                        }),
                        ..default()
                    },
                    Transform::from_xyz(local_x, local_y, 0.01),
                ));
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Player spawning
// ---------------------------------------------------------------------------

fn spawn_players_from_config(commands: &mut Commands, md: &MapData, config: &GameSetupConfig) {
    // Count active (non-None) player slots
    let active_slots: Vec<(usize, &crate::menu::components::PlayerSlotConfig)> = config
        .player_slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| slot.control_type != ControlType::None)
        .collect();

    let num_players = active_slots.len().max(2);
    let spawn_positions = pick_spawn_positions(md, num_players);

    if spawn_positions.is_empty() {
        warn!("No spawn positions available! Using center of map as fallback.");
    }

    for (spawn_idx, (player_index, slot)) in active_slots.iter().enumerate() {
        let (sx, sy) = if spawn_positions.is_empty() {
            (10, 7) // Center of 20x15 tile grid
        } else {
            spawn_positions[spawn_idx % spawn_positions.len()]
        };
        let bx = tile_to_bevy_x(sx as u32);
        let by = tile_to_bevy_y(sy as u32);

        let mut bundle = PlayerBundle::new(*player_index, slot.team, bx, by);
        bundle.player.lives = config.lives as i32;
        bundle.action = crate::player::components::PlayerAction::Spawning;
        bundle.skin_index = crate::player::components::PlayerSkinIndex(slot.skin_index);

        let entity_id = commands.spawn((
            bundle,
            Collider::new(PLAYER_WIDTH, PLAYER_HEIGHT),
            crate::physics::components::CollisionLayer::Player,
            crate::physics::components::GravityAffected::default(),
            crate::player::components::SpawnProtection::default(),
            GameplayEntity,
        )).id();

        // Add AI components for CPU-controlled players
        if slot.control_type == ControlType::Cpu {
            let ai_difficulty = match slot.cpu_difficulty {
                CpuDifficulty::Easy => crate::ai::components::AiDifficulty::Easy,
                CpuDifficulty::Medium => crate::ai::components::AiDifficulty::Medium,
                CpuDifficulty::Hard => crate::ai::components::AiDifficulty::Hard,
            };
            commands.entity(entity_id).insert((
                AiControlled::new(ai_difficulty),
                AiIntent::default(),
                AiPerception::default(),
                AiMemory::default(),
                AiPathfinding::default(),
            ));
        }

        info!(
            "Spawned player {} (team {}, {:?}) at tile ({}, {}) -> bevy ({:.0}, {:.0})",
            player_index, slot.team, slot.control_type, sx, sy, bx, by
        );
    }
}

fn spawn_players(commands: &mut Commands, md: &MapData) {
    let spawn_positions = pick_spawn_positions(md, 2);

    for (i, (sx, sy)) in spawn_positions.into_iter().enumerate() {
        let bx = tile_to_bevy_x(sx as u32);
        let by = tile_to_bevy_y(sy as u32);

        let mut bundle = PlayerBundle::new(i, i as u8, bx, by);
        bundle.action = crate::player::components::PlayerAction::Spawning;

        commands.spawn((
            bundle,
            Collider::new(PLAYER_WIDTH, PLAYER_HEIGHT),
            crate::physics::components::CollisionLayer::Player,
            crate::physics::components::GravityAffected::default(),
            crate::player::components::SpawnProtection::default(),
            GameplayEntity,
        ));

        info!("Spawned player {} at tile ({}, {}) -> bevy ({:.0}, {:.0})", i, sx, sy, bx, by);
    }
}

/// Pick spawn positions for `count` players from the map's spawn areas.
pub fn pick_spawn_positions(md: &MapData, count: usize) -> Vec<(i32, i32)> {
    let mut positions = Vec::new();

    // Default positions if we can't find valid spawn areas
    let defaults = [
        (3, 11),
        (16, 11),
        (3, 5),
        (16, 5),
    ];

    // Try to use the "all players" spawn areas (index 0)
    if let Some(areas) = md.spawn_areas.first() {
        for area in areas.iter() {
            if positions.len() >= count {
                break;
            }
            // Place player roughly in the center of the spawn area
            let cx = area.left + area.width / 2;
            let cy = area.top + area.height / 2;
            positions.push((cx, cy));
        }
    }

    // Fill remaining with defaults
    while positions.len() < count {
        let idx = positions.len();
        let d = defaults[idx % defaults.len()];
        positions.push(d);
    }

    positions
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

fn cleanup_gameplay(
    mut commands: Commands,
    query: Query<Entity, With<GameplayEntity>>,
) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}

// ---------------------------------------------------------------------------
// Animated tile update
// ---------------------------------------------------------------------------

fn update_animated_tiles(
    time: Res<Time>,
    mut anim_timer: ResMut<TileAnimationTimer>,
    mut query: Query<(&AnimatedTile, &mut Sprite)>,
) {
    if anim_timer.timer.tick(time.delta()).just_finished() {
        anim_timer.frame = anim_timer.frame.wrapping_add(1);

        for (anim, mut sprite) in &mut query {
            let frame = anim_timer.frame % anim.num_frames;
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = (atlas.index / anim.num_frames as usize) * anim.num_frames as usize
                    + frame as usize;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Moving platform update
// ---------------------------------------------------------------------------

fn update_moving_platforms(
    time: Res<Time>,
    mut query: Query<(&mut MovingPlatform, &mut Transform)>,
) {
    let dt = time.delta_secs();
    let half_w = SCREEN_WIDTH / 2.0;
    let half_h = SCREEN_HEIGHT / 2.0;

    for (mut platform, mut xf) in &mut query {
        // Save old position to derive velocity later
        let old_x = platform.current_x;
        let old_y = platform.current_y;

        // Clone the path to avoid borrow-checker conflict when mutating
        // platform fields while reading path parameters.
        let path = platform.path.clone();

        match &path {
            PlatformPath::Straight { start_x, start_y, end_x, end_y, speed } => {
                // All coordinates are already in Bevy space (converted at spawn).
                let dx = end_x - start_x;
                let dy = end_y - start_y;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 {
                    let nx = dx / len;
                    let ny = dy / len;

                    // Move along the direction, scaled by the direction flag
                    platform.current_x += nx * speed * platform.direction * dt * 60.0;
                    platform.current_y += ny * speed * platform.direction * dt * 60.0;

                    // Compute progress along the segment (0.0 = start, 1.0 = end)
                    let progress = if dx.abs() > dy.abs() {
                        (platform.current_x - start_x) / dx
                    } else {
                        (platform.current_y - start_y) / dy
                    };

                    // Bounce at endpoints: reverse direction and clamp
                    if progress >= 1.0 {
                        platform.direction = -1.0;
                        platform.current_x = *end_x;
                        platform.current_y = *end_y;
                    } else if progress <= 0.0 {
                        platform.direction = 1.0;
                        platform.current_x = *start_x;
                        platform.current_y = *start_y;
                    }
                }
            }
            PlatformPath::Continuous { angle, speed, .. } => {
                // Angle is in SMW space (Y-down). In Bevy Y-up, negate the Y component.
                platform.current_x += angle.cos() * speed * dt * 60.0;
                platform.current_y -= angle.sin() * speed * dt * 60.0;

                // Wrap around screen (Bevy coords: -half_w..+half_w, -half_h..+half_h)
                if platform.current_x > half_w {
                    platform.current_x -= SCREEN_WIDTH;
                } else if platform.current_x < -half_w {
                    platform.current_x += SCREEN_WIDTH;
                }
                if platform.current_y > half_h {
                    platform.current_y -= SCREEN_HEIGHT;
                } else if platform.current_y < -half_h {
                    platform.current_y += SCREEN_HEIGHT;
                }
            }
            PlatformPath::Ellipse { radius_x, radius_y, center_x, center_y, start_angle, speed } => {
                // Accumulate angle in the dedicated field
                platform.angle += speed * dt * 60.0;
                let a = start_angle + platform.angle;
                // center_x/center_y are in Bevy space (converted at spawn).
                // radius_y is negated because SMW Y-down ellipse motion maps to Bevy Y-up.
                platform.current_x = center_x + radius_x * a.cos();
                platform.current_y = center_y - radius_y * a.sin();
            }
        }

        // Derive velocity from position delta (Bevy-space units per frame)
        platform.vel_x = platform.current_x - old_x;
        platform.vel_y = platform.current_y - old_y;

        // Write the Bevy-space position directly to the transform
        xf.translation.x = platform.current_x;
        xf.translation.y = platform.current_y;
    }
}

// ---------------------------------------------------------------------------
// Map hazard spawning
// ---------------------------------------------------------------------------

fn spawn_map_hazards(commands: &mut Commands, map_data: &MapData) {
    use crate::objects::hazards::{Hazard, HazardType, spawn_hazard};

    for mh in &map_data.map_hazards {
        let hazard_type = match mh.hazard_type {
            0 => HazardType::FireBar,
            1 => HazardType::Thwomp,
            2 => HazardType::PiranhaPlant,
            3 => HazardType::BulletCannon,
            4 => HazardType::Fireball,
            5 => HazardType::Spikes,
            _ => continue, // Unknown hazard type, skip
        };

        // Hazard ix/iy are half-tile indices; multiply by 16 to get SMW pixel coords
        let bx = smw_to_bevy_x(mh.tile_x as f32 * (TILE_SIZE / 2.0));
        let by = smw_to_bevy_y(mh.tile_y as f32 * (TILE_SIZE / 2.0));

        let hazard = match hazard_type {
            HazardType::Thwomp => Hazard::thwomp(),
            HazardType::BulletCannon => Hazard::bullet_cannon(),
            HazardType::PiranhaPlant => Hazard::piranha_plant(),
            HazardType::FireBar => Hazard::fire_bar(),
            HazardType::Spikes => Hazard::spikes(),
            HazardType::Fireball => Hazard::fire_bar(), // Use fire_bar timing for fireballs
        };

        let entity = spawn_hazard(commands, hazard, bx, by);
        commands.entity(entity).insert(GameplayEntity);
    }

    if !map_data.map_hazards.is_empty() {
        info!("Spawned {} map hazards", map_data.map_hazards.len());
    }
}

// ---------------------------------------------------------------------------
// Map item spawning (springs, spikes, kuribo shoes, etc.)
// ---------------------------------------------------------------------------

/// Convert raw block_type (i8) from map data to a BlockType enum variant.
/// Returns None for empty/unrecognized block types.
fn raw_block_to_type(raw: i8) -> Option<crate::objects::blocks::BlockType> {
    use crate::objects::blocks::BlockType;
    match raw {
        1 => Some(BlockType::Breakable),         // MapBlock_BrickYellow
        2 => Some(BlockType::PowerupBlock),       // MapBlock_Powerup
        3 => Some(BlockType::DonutBlock),         // MapBlock_Donut
        4 => Some(BlockType::FlipBlock),          // MapBlock_Flip
        5 => Some(BlockType::BounceBlock),        // MapBlock_Bounce
        6..=9 => Some(BlockType::NoteBlock),      // MapBlock_NoteGray/Red/Blue + spare
        10 => Some(BlockType::ThrowBlock),        // MapBlock_BrickBlue
        11..=14 => Some(BlockType::OnOffSwitch),  // MapBlock_SwitchToggle (Red/Green/Yellow/Blue)
        15..=18 => Some(BlockType::OnOffBlock),   // MapBlock_SwitchBlock (Red/Green/Yellow/Blue)
        19 => Some(BlockType::ViewBlock),         // MapBlock_View
        20 => Some(BlockType::ThrowBlock),        // MapBlock_BrickRed
        21..=23 => Some(BlockType::NoteBlock),    // Note variants
        24 => Some(BlockType::ThrowBlock),        // MapBlock_BrickGray
        25..=34 => Some(BlockType::WeaponBreakable), // MapBlock_WpnBreak variants
        _ => None,
    }
}

/// Spawns block entities from the map's object_data array.
fn spawn_blocks_from_object_data(commands: &mut Commands, map_data: &MapData) {
    let mut count = 0u32;
    for (col, column) in map_data.object_data.iter().enumerate() {
        for (row, obj_block) in column.iter().enumerate() {
            if obj_block.block_type <= 0 {
                continue; // No block at this tile
            }
            if let Some(block_type) = raw_block_to_type(obj_block.block_type) {
                let entity = crate::objects::blocks::spawn_block(
                    commands,
                    block_type,
                    col as u32,
                    row as u32,
                    obj_block.settings,
                );
                commands.entity(entity).insert(GameplayEntity);

                // Hidden blocks start invisible
                if obj_block.hidden {
                    commands.entity(entity).insert(Visibility::Hidden);
                }

                count += 1;
            }
        }
    }
    if count > 0 {
        info!("Spawned {} blocks from object_data", count);
    }
}

// ---------------------------------------------------------------------------

fn spawn_map_items(commands: &mut Commands, map_data: &MapData) {
    use crate::objects::components::GameObject;
    use crate::physics::components::Collider;

    for item in &map_data.map_items {
        // Item ix/iy are tile indices; use tile_to_bevy for proper centering
        let bx = tile_to_bevy_x(item.tile_x as u32);
        let by = tile_to_bevy_y(item.tile_y as u32);

        // Item types: 0=GreenSpring, 1=Spike, 2=KuriboShoe, 3=GoldenSpring, 4=StickyShoe, 5=ThrowBox
        commands.spawn((
            Transform::from_xyz(bx, by, crate::graphics::sprites::z_order::OBJECTS),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            Collider::new(TILE_SIZE, TILE_SIZE),
            GameObject {
                object_type: crate::objects::components::ObjectType::Block,
                alive: true,
            },
            GameplayEntity,
        ));
    }

    if !map_data.map_items.is_empty() {
        info!("Spawned {} map items (placeholder entities)", map_data.map_items.len());
    }
}

// ---------------------------------------------------------------------------
// Coordinate helpers
// ---------------------------------------------------------------------------

/// Convert tile grid column to Bevy X (centred on screen).
pub fn tile_to_bevy_x(col: u32) -> f32 {
    (col as f32 * TILE_SIZE) - SCREEN_WIDTH / 2.0 + TILE_SIZE / 2.0
}

/// Convert tile grid row to Bevy Y (Y-up, centred on screen).
pub fn tile_to_bevy_y(row: u32) -> f32 {
    (SCREEN_HEIGHT / 2.0) - (row as f32 * TILE_SIZE) - TILE_SIZE / 2.0
}

/// Map a CollisionType to the legacy TileType enum.
fn tile_type_from_collision(c: CollisionType) -> TileType {
    if c.is_death() {
        TileType::Death
    } else if c == CollisionType::Ice || c == CollisionType::IceOnTop {
        TileType::Ice
    } else if c.is_solid() {
        TileType::Solid
    } else if c.is_solid_on_top() {
        TileType::SolidOnTop
    } else {
        TileType::NonSolid
    }
}
