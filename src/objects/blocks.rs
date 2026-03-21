use bevy::prelude::*;

use crate::audio::sfx::{play_sfx, SfxId, SfxLibrary};
use crate::core::constants::*;
use crate::objects::components::*;
use crate::objects::powerups::PowerupType;
use crate::physics::collision::aabb_overlap;
use crate::physics::components::{Collider, CollisionLayer, Solid};
use crate::player::components::{
    CarriedObject, Facing, Grounded, Player, Velocity,
};

// ---------------------------------------------------------------------------
// Block components
// ---------------------------------------------------------------------------

/// Types of interactive blocks in the game.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Breakable,
    PowerupBlock,
    DonutBlock,
    FlipBlock,
    BounceBlock,
    NoteBlock,
    OnOffSwitch,
    OnOffBlock,
    WeaponBreakable,
    ViewBlock,
    ThrowBlock,
}

/// State of a block that can be hit/activated.
#[derive(Component, Debug)]
pub struct BlockState {
    pub hit: bool,
    pub hit_timer: f32,
    pub empty: bool,
    pub item_count: u32,
}

impl Default for BlockState {
    fn default() -> Self {
        Self {
            hit: false,
            hit_timer: 0.0,
            empty: false,
            item_count: 1,
        }
    }
}

/// Donut block that crumbles after a player stands on it.
#[derive(Component, Debug)]
pub struct DonutBlockState {
    pub weight_timer: f32,
    pub crumbling: bool,
    pub crumble_threshold: f32,
}

impl Default for DonutBlockState {
    fn default() -> Self {
        Self {
            weight_timer: 0.0,
            crumbling: false,
            crumble_threshold: 50.0,
        }
    }
}

/// Flip block animation state.
#[derive(Component, Debug)]
pub struct FlipBlockState {
    pub flipping: bool,
    pub flip_timer: f32,
    pub flip_duration: f32,
}

impl Default for FlipBlockState {
    fn default() -> Self {
        Self {
            flipping: false,
            flip_timer: 0.0,
            flip_duration: 3.0,
        }
    }
}

/// On/Off switch state shared across all on/off blocks.
#[derive(Resource, Debug, Default)]
pub struct OnOffSwitchState {
    pub active: bool,
}

/// Marker for the visual bump animation when a block is hit.
#[derive(Component, Debug)]
pub struct BlockBumpAnimation {
    pub original_y: f32,
    pub timer: f32,
    pub duration: f32,
}

// ---------------------------------------------------------------------------
// Block spawning
// ---------------------------------------------------------------------------

/// Spawns a block entity at the given grid position with the specified type.
pub fn spawn_block(
    commands: &mut Commands,
    block_type: BlockType,
    grid_x: u32,
    grid_y: u32,
) -> Entity {
    let world_x = grid_x as f32 * TILE_SIZE;
    let world_y = grid_y as f32 * TILE_SIZE;

    let mut entity_commands = commands.spawn((
        GameObject {
            object_type: ObjectType::Block,
            alive: true,
        },
        block_type,
        BlockState::default(),
        Collider::new(TILE_SIZE, TILE_SIZE),
        CollisionLayer::Block,
        Solid,
        Transform::from_xyz(world_x, world_y, crate::graphics::sprites::z_order::OBJECTS),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));

    match block_type {
        BlockType::DonutBlock => {
            entity_commands.insert(DonutBlockState::default());
        }
        BlockType::FlipBlock => {
            entity_commands.insert(FlipBlockState::default());
        }
        _ => {}
    }

    entity_commands.id()
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Handles block hit from below by a player.
/// Checks for player heads colliding with blocks from underneath and triggers
/// the appropriate block behavior.
pub fn block_hit_system(
    mut commands: Commands,
    sfx: Res<SfxLibrary>,
    mut on_off_state: ResMut<OnOffSwitchState>,
    mut block_query: Query<(
        Entity,
        &BlockType,
        &mut BlockState,
        &Collider,
        &Transform,
        &mut GameObject,
    )>,
    mut player_query: Query<(
        Entity,
        &Player,
        &mut Velocity,
        &Transform,
        &Collider,
        &Facing,
        &mut CarriedObject,
    )>,
    mut hit_events: EventWriter<BlockHitEvent>,
) {
    for (p_entity, player, mut p_vel, p_transform, p_collider, _facing, mut carried) in
        player_query.iter_mut()
    {
        // Only check for hits when player is moving upward (positive Y = up in Bevy's Y-up system)
        if p_vel.y <= 0.0 {
            continue; // Skip if player is NOT moving up
        }

        for (b_entity, block_type, mut block_state, b_collider, b_transform, mut game_obj) in
            block_query.iter_mut()
        {
            if !game_obj.alive || block_state.empty {
                continue;
            }

            // Check if the player's head overlaps the block from below
            let player_top = p_transform.translation.y + p_collider.offset_y;
            let block_bottom =
                b_transform.translation.y + b_collider.offset_y + b_collider.height;

            if !aabb_overlap(
                &p_transform.translation,
                p_collider,
                &b_transform.translation,
                b_collider,
            ) {
                continue;
            }

            // Player must be hitting from below: player top near block bottom
            let vertical_gap = (player_top - block_bottom).abs();
            if vertical_gap > p_vel.y.abs() + 4.0 {
                continue;
            }

            // Stop upward velocity
            p_vel.y = 1.0;

            hit_events.send(BlockHitEvent {
                block_entity: b_entity,
                player_entity: p_entity,
                player_index: player.index,
            });

            match block_type {
                BlockType::Breakable => {
                    play_sfx(&mut commands, &sfx, SfxId::BlockBreak);
                    game_obj.alive = false;
                    // Award points for breaking
                    commands.entity(b_entity).despawn_recursive();
                }
                BlockType::PowerupBlock => {
                    if !block_state.empty {
                        play_sfx(&mut commands, &sfx, SfxId::PowerupAppear);
                        block_state.item_count = block_state.item_count.saturating_sub(1);
                        if block_state.item_count == 0 {
                            block_state.empty = true;
                        }
                        block_state.hit = true;
                        block_state.hit_timer = 0.0;
                        commands.entity(b_entity).insert(BlockBumpAnimation {
                            original_y: b_transform.translation.y,
                            timer: 0.0,
                            duration: 0.15,
                        });

                        // Spawn a powerup above the block
                        let spawn_x = b_transform.translation.x;
                        let spawn_y = b_transform.translation.y - TILE_SIZE;
                        crate::objects::powerups::spawn_powerup(
                            &mut commands,
                            PowerupType::Mushroom,
                            spawn_x,
                            spawn_y,
                        );
                    }
                }
                BlockType::BounceBlock => {
                    play_sfx(&mut commands, &sfx, SfxId::BlockHit);
                    block_state.hit = true;
                    block_state.hit_timer = 0.0;
                    commands.entity(b_entity).insert(BlockBumpAnimation {
                        original_y: b_transform.translation.y,
                        timer: 0.0,
                        duration: 0.15,
                    });
                    // Entities standing on this block get bounced (handled in separate pass)
                }
                BlockType::NoteBlock => {
                    play_sfx(&mut commands, &sfx, SfxId::BlockHit);
                    block_state.hit = true;
                    block_state.hit_timer = 0.0;
                    commands.entity(b_entity).insert(BlockBumpAnimation {
                        original_y: b_transform.translation.y,
                        timer: 0.0,
                        duration: 0.15,
                    });
                    // Stronger bounce handled separately
                }
                BlockType::FlipBlock => {
                    play_sfx(&mut commands, &sfx, SfxId::BlockHit);
                    block_state.hit = true;
                    commands.entity(b_entity).insert(BlockBumpAnimation {
                        original_y: b_transform.translation.y,
                        timer: 0.0,
                        duration: 0.15,
                    });
                    // Flip animation starts -- FlipBlockState is handled in flip_block_system
                }
                BlockType::OnOffSwitch => {
                    play_sfx(&mut commands, &sfx, SfxId::BlockHit);
                    on_off_state.active = !on_off_state.active;
                    block_state.hit = true;
                    block_state.hit_timer = 0.0;
                    commands.entity(b_entity).insert(BlockBumpAnimation {
                        original_y: b_transform.translation.y,
                        timer: 0.0,
                        duration: 0.15,
                    });
                }
                BlockType::ThrowBlock => {
                    play_sfx(&mut commands, &sfx, SfxId::ThrowBlock);
                    // Player picks up the block as a carried object
                    if carried.entity.is_none() {
                        carried.entity = Some(b_entity);
                        // Remove Solid so it doesn't collide while carried
                        commands.entity(b_entity).remove::<Solid>();
                        game_obj.alive = false;
                    }
                }
                BlockType::DonutBlock => {
                    // Donut blocks aren't typically hit from below, but mark anyway
                    block_state.hit = true;
                    commands.entity(b_entity).insert(BlockBumpAnimation {
                        original_y: b_transform.translation.y,
                        timer: 0.0,
                        duration: 0.15,
                    });
                }
                BlockType::WeaponBreakable | BlockType::ViewBlock | BlockType::OnOffBlock => {
                    // These blocks respond to other interactions
                    play_sfx(&mut commands, &sfx, SfxId::Bump);
                }
            }
        }
    }
}

/// Bounce entities standing on top of BounceBlock / NoteBlock that were just hit.
pub fn block_bounce_system(
    mut block_query: Query<(&BlockType, &mut BlockState, &Collider, &Transform)>,
    mut player_query: Query<(&mut Velocity, &Transform, &Collider, &Grounded), With<Player>>,
) {
    for (block_type, mut block_state, b_collider, b_transform) in block_query.iter_mut() {
        if !block_state.hit {
            continue;
        }

        let bounce_vel = match block_type {
            BlockType::BounceBlock => VEL_BLOCK_BOUNCE,
            BlockType::NoteBlock => VEL_NOTE_BLOCK_BOUNCE * 2.5,
            _ => continue,
        };

        // Check for players standing on top of this block
        for (mut p_vel, p_transform, p_collider, grounded) in player_query.iter_mut() {
            if !grounded.on_ground {
                continue;
            }

            let player_bottom =
                p_transform.translation.y + p_collider.offset_y + p_collider.height;
            let block_top = b_transform.translation.y + b_collider.offset_y;

            // Player bottom must be near block top
            if (player_bottom - block_top).abs() > 4.0 {
                continue;
            }

            // Horizontal overlap check
            let p_left = p_transform.translation.x + p_collider.offset_x;
            let p_right = p_left + p_collider.width;
            let b_left = b_transform.translation.x + b_collider.offset_x;
            let b_right = b_left + b_collider.width;

            if p_right > b_left && p_left < b_right {
                p_vel.y = bounce_vel;
            }
        }

        // Reset hit flag after processing bounce
        block_state.hit = false;
        block_state.hit_timer = 0.0;
    }
}

/// Ticks donut block weight timers. When a player stands on a donut block, the
/// timer increases. When it exceeds the threshold, the block crumbles and is removed.
pub fn donut_block_timer_system(
    mut commands: Commands,
    time: Res<Time>,
    mut block_query: Query<(
        Entity,
        &mut DonutBlockState,
        &mut GameObject,
        &Collider,
        &Transform,
    )>,
    player_query: Query<(&Transform, &Collider, &Grounded), With<Player>>,
) {
    let dt = time.delta_secs();

    for (entity, mut donut, mut game_obj, b_collider, b_transform) in block_query.iter_mut() {
        if !game_obj.alive {
            continue;
        }

        let mut player_standing = false;

        for (p_transform, p_collider, grounded) in player_query.iter() {
            if !grounded.on_ground {
                continue;
            }

            let player_bottom =
                p_transform.translation.y + p_collider.offset_y + p_collider.height;
            let block_top = b_transform.translation.y + b_collider.offset_y;

            if (player_bottom - block_top).abs() > 4.0 {
                continue;
            }

            let p_left = p_transform.translation.x + p_collider.offset_x;
            let p_right = p_left + p_collider.width;
            let b_left = b_transform.translation.x + b_collider.offset_x;
            let b_right = b_left + b_collider.width;

            if p_right > b_left && p_left < b_right {
                player_standing = true;
                break;
            }
        }

        if player_standing {
            donut.weight_timer += dt * 60.0; // Frame-based timer scaled to delta
            if donut.weight_timer >= donut.crumble_threshold {
                donut.crumbling = true;
                game_obj.alive = false;
                commands.entity(entity).despawn_recursive();
            }
        } else {
            // Slowly recover when no player is standing on it
            donut.weight_timer = (donut.weight_timer - dt * 30.0).max(0.0);
        }
    }
}

/// Ticks flip block animation. While flipping, the block is passable. After the
/// animation completes, it returns to solid.
pub fn flip_block_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut FlipBlockState, &mut BlockState)>,
) {
    let dt = time.delta_secs();

    for (entity, mut flip, mut block_state) in query.iter_mut() {
        if !block_state.hit {
            continue;
        }

        if !flip.flipping {
            // Start the flip
            flip.flipping = true;
            flip.flip_timer = 0.0;
            // Remove Solid while flipping so entities can pass through
            commands.entity(entity).remove::<Solid>();
        }

        flip.flip_timer += dt;

        if flip.flip_timer >= flip.flip_duration {
            // Flip animation complete -- restore solidity
            flip.flipping = false;
            flip.flip_timer = 0.0;
            block_state.hit = false;
            commands.entity(entity).insert(Solid);
        }
    }
}

/// Updates all OnOff blocks when the global switch state changes. Blocks that match
/// the current state become solid; others become passable.
pub fn on_off_block_system(
    mut commands: Commands,
    on_off_state: Res<OnOffSwitchState>,
    mut query: Query<(Entity, &BlockType, &mut GameObject)>,
) {
    if !on_off_state.is_changed() {
        return;
    }

    for (entity, block_type, mut game_obj) in query.iter_mut() {
        if *block_type != BlockType::OnOffBlock {
            continue;
        }

        if on_off_state.active {
            // Blocks become solid when switch is active
            commands.entity(entity).insert(Solid);
            game_obj.alive = true;
        } else {
            // Blocks become passable when switch is inactive
            commands.entity(entity).remove::<Solid>();
            game_obj.alive = false;
        }
    }
}

/// Animates the bump offset for blocks that were hit, providing visual feedback.
pub fn block_bump_animation_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut BlockBumpAnimation, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (entity, mut bump, mut transform) in query.iter_mut() {
        bump.timer += dt;
        let progress = bump.timer / bump.duration;

        if progress >= 1.0 {
            // Animation complete -- snap back to original position
            transform.translation.y = bump.original_y;
            commands.entity(entity).remove::<BlockBumpAnimation>();
        } else {
            // Sine-based bump: move up then back down
            let offset = (progress * std::f32::consts::PI).sin() * -8.0;
            transform.translation.y = bump.original_y + offset;
        }
    }
}
