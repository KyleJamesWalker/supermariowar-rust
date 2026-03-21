use bevy::prelude::*;
use std::collections::HashMap;

use crate::player::components::{
    Facing, Invincible, Player, PlayerAction, PowerupState, SpawnProtection,
};

// ---------------------------------------------------------------------------
// Animation state enum
// ---------------------------------------------------------------------------

/// High-level animation states that map to specific frame sequences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationState {
    Idle,
    Walk,
    Run,
    Jump,
    Fall,
    Skid,
    Dead,
    Spawn,
    Attack,
    Warp,
}

impl AnimationState {
    /// Derive the animation state from a player's current action.
    pub fn from_player_action(action: &PlayerAction) -> Self {
        match action {
            PlayerAction::Standing => AnimationState::Idle,
            PlayerAction::Running => AnimationState::Run,
            PlayerAction::Jumping => AnimationState::Jump,
            PlayerAction::Falling => AnimationState::Fall,
            PlayerAction::Skidding => AnimationState::Skid,
            PlayerAction::Dead => AnimationState::Dead,
            PlayerAction::Spawning => AnimationState::Spawn,
        }
    }
}

// ---------------------------------------------------------------------------
// Animation frame data
// ---------------------------------------------------------------------------

/// Describes a sequence of atlas indices and timing for one animation.
#[derive(Debug, Clone)]
pub struct AnimationFrameData {
    /// Atlas indices for each frame in the animation.
    pub frames: Vec<usize>,
    /// Duration of each frame in seconds.
    pub frame_duration: f32,
    /// Whether the animation loops.
    pub looping: bool,
}

impl Default for AnimationFrameData {
    fn default() -> Self {
        Self {
            frames: vec![0],
            frame_duration: 0.15,
            looping: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Entity type for animation bank lookup
// ---------------------------------------------------------------------------

/// Categorizes entities for looking up their animation data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimatedEntityType {
    Player,
    Enemy,
    Projectile,
    Block,
    Powerup,
}

// ---------------------------------------------------------------------------
// Animation bank resource
// ---------------------------------------------------------------------------

/// Resource mapping (entity_type, animation_state) -> frame data.
/// Provides a central registry for all animation definitions.
#[derive(Resource, Default)]
pub struct AnimationBank {
    pub entries: HashMap<(AnimatedEntityType, AnimationState), AnimationFrameData>,
}

impl AnimationBank {
    /// Look up frame data for a given entity type and animation state.
    pub fn get(
        &self,
        entity_type: AnimatedEntityType,
        state: AnimationState,
    ) -> Option<&AnimationFrameData> {
        self.entries.get(&(entity_type, state))
    }
}

/// Build the default animation bank with player animation definitions.
/// Player skin strips are 6 frames wide (indices 0-5):
///   0 = standing, 1-3 = walk/run cycle, 4 = jump, 5 = dead/spin
pub fn build_default_animation_bank() -> AnimationBank {
    let mut bank = AnimationBank::default();

    // Player animations (frame indices into the 6-frame skin strip)
    let player_anims = [
        (
            AnimationState::Idle,
            AnimationFrameData {
                frames: vec![0],
                frame_duration: 1.0,
                looping: false,
            },
        ),
        (
            AnimationState::Walk,
            AnimationFrameData {
                frames: vec![0, 1, 2, 3],
                frame_duration: 0.12,
                looping: true,
            },
        ),
        (
            AnimationState::Run,
            AnimationFrameData {
                frames: vec![0, 1, 2, 3],
                frame_duration: 0.08,
                looping: true,
            },
        ),
        (
            AnimationState::Jump,
            AnimationFrameData {
                frames: vec![4],
                frame_duration: 1.0,
                looping: false,
            },
        ),
        (
            AnimationState::Fall,
            AnimationFrameData {
                frames: vec![4],
                frame_duration: 1.0,
                looping: false,
            },
        ),
        (
            AnimationState::Skid,
            AnimationFrameData {
                frames: vec![3],
                frame_duration: 1.0,
                looping: false,
            },
        ),
        (
            AnimationState::Dead,
            AnimationFrameData {
                frames: vec![5],
                frame_duration: 0.1,
                looping: false,
            },
        ),
        (
            AnimationState::Spawn,
            AnimationFrameData {
                frames: vec![0],
                frame_duration: 1.0,
                looping: false,
            },
        ),
        (
            AnimationState::Attack,
            AnimationFrameData {
                frames: vec![0, 1],
                frame_duration: 0.1,
                looping: false,
            },
        ),
        (
            AnimationState::Warp,
            AnimationFrameData {
                frames: vec![0],
                frame_duration: 1.0,
                looping: false,
            },
        ),
    ];

    for (state, data) in player_anims {
        bank.entries
            .insert((AnimatedEntityType::Player, state), data);
    }

    bank
}

// ---------------------------------------------------------------------------
// AnimationController component
// ---------------------------------------------------------------------------

/// Drives frame-based sprite animation on an entity.
#[derive(Component, Debug)]
pub struct AnimationController {
    /// Atlas frame indices to cycle through.
    pub frames: Vec<usize>,
    /// Seconds per frame.
    pub frame_duration: f32,
    /// Current position in the frames array.
    pub current_frame: usize,
    /// Accumulated time since last frame advance.
    pub elapsed: f32,
    /// Whether the animation loops.
    pub looping: bool,
    /// Whether a non-looping animation has finished.
    pub finished: bool,
    /// The current high-level animation state (for change detection).
    pub current_state: Option<AnimationState>,
}

impl AnimationController {
    pub fn new(frames: Vec<usize>, frame_duration: f32, looping: bool) -> Self {
        Self {
            frames,
            frame_duration,
            current_frame: 0,
            elapsed: 0.0,
            looping,
            finished: false,
            current_state: None,
        }
    }

    /// Returns the atlas index for the current frame.
    pub fn current_sprite_index(&self) -> usize {
        self.frames.get(self.current_frame).copied().unwrap_or(0)
    }

    /// Switch to a new animation. Resets frame counter only if the state changed.
    pub fn set_animation(&mut self, state: AnimationState, data: &AnimationFrameData) {
        if self.current_state == Some(state) {
            return;
        }
        self.frames = data.frames.clone();
        self.frame_duration = data.frame_duration;
        self.looping = data.looping;
        self.current_frame = 0;
        self.elapsed = 0.0;
        self.finished = false;
        self.current_state = Some(state);
    }
}

// ---------------------------------------------------------------------------
// Blink component (for spawn protection visual)
// ---------------------------------------------------------------------------

/// Component that toggles visibility at a fixed rate (spawn protection blink).
#[derive(Component, Debug)]
pub struct BlinkEffect {
    /// Seconds per blink cycle (on + off).
    pub period: f32,
    /// Accumulated time.
    pub elapsed: f32,
}

impl Default for BlinkEffect {
    fn default() -> Self {
        Self {
            period: 0.12,
            elapsed: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// DeathAnimation component
// ---------------------------------------------------------------------------

/// Component driving the spinning/bouncing death animation.
#[derive(Component, Debug)]
pub struct DeathAnimation {
    /// Rotation speed in radians per second.
    pub spin_speed: f32,
    /// Accumulated time.
    pub elapsed: f32,
}

impl Default for DeathAnimation {
    fn default() -> Self {
        Self {
            spin_speed: 12.0,
            elapsed: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Advances all animation controllers each frame, updating the current frame index.
pub fn animate_sprites(
    time: Res<Time>,
    mut query: Query<(&mut AnimationController, Option<&mut Sprite>)>,
) {
    let dt = time.delta_secs();
    for (mut anim, sprite) in query.iter_mut() {
        if anim.finished {
            continue;
        }

        anim.elapsed += dt;
        if anim.elapsed >= anim.frame_duration {
            anim.elapsed -= anim.frame_duration;
            anim.current_frame += 1;

            if anim.current_frame >= anim.frames.len() {
                if anim.looping {
                    anim.current_frame = 0;
                } else {
                    anim.current_frame = anim.frames.len() - 1;
                    anim.finished = true;
                }
            }
        }

        // Update the TextureAtlas index on the Sprite component
        if let Some(mut sprite) = sprite {
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.index = anim.current_sprite_index();
            }
        }
    }
}

/// Reads PlayerAction and Facing, then sets the correct AnimationController frames
/// based on the AnimationBank.
pub fn animation_state_sync(
    animation_bank: Res<AnimationBank>,
    mut query: Query<
        (&PlayerAction, &mut AnimationController),
        (With<Player>, Changed<PlayerAction>),
    >,
) {
    for (action, mut anim_ctrl) in query.iter_mut() {
        let anim_state = AnimationState::from_player_action(action);

        if let Some(frame_data) =
            animation_bank.get(AnimatedEntityType::Player, anim_state)
        {
            anim_ctrl.set_animation(anim_state, frame_data);
        }
    }
}

/// Flips sprites horizontally based on the Facing component.
pub fn sprite_flip_system(
    mut query: Query<(&Facing, &mut Sprite), (With<Player>, Changed<Facing>)>,
) {
    for (facing, mut sprite) in query.iter_mut() {
        sprite.flip_x = *facing == Facing::Left;
    }
}

/// Toggles visibility for entities with BlinkEffect (spawn protection or invincibility blinking).
pub fn blink_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut BlinkEffect,
        &mut Visibility,
        Option<&SpawnProtection>,
        Option<&Invincible>,
        &PowerupState,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, mut blink, mut visibility, spawn_prot, invincible, powerup) in query.iter_mut() {
        // Blink continues while player has spawn protection OR non-star invincibility.
        // Star invincibility uses rainbow color cycling instead of blinking.
        let should_blink = spawn_prot.is_some()
            || (invincible.is_some() && *powerup != PowerupState::Star);
        if !should_blink {
            *visibility = Visibility::Inherited;
            commands.entity(entity).remove::<BlinkEffect>();
            continue;
        }

        blink.elapsed += dt;
        let cycle = blink.elapsed % blink.period;
        if cycle < blink.period * 0.5 {
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

/// Drives the spinning death animation: rotates the sprite and advances time.
pub fn death_animation_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DeathAnimation, &mut Transform, &PlayerAction)>,
) {
    let dt = time.delta_secs();
    for (entity, mut death_anim, mut transform, action) in query.iter_mut() {
        if *action != PlayerAction::Dead {
            // Player is no longer dead (respawned), remove the component and reset rotation
            transform.rotation = Quat::IDENTITY;
            commands.entity(entity).remove::<DeathAnimation>();
            continue;
        }

        death_anim.elapsed += dt;
        let angle = death_anim.spin_speed * death_anim.elapsed;
        transform.rotation = Quat::from_rotation_z(angle);
    }
}
