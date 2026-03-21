use bevy::prelude::*;

use crate::core::constants::*;
use crate::graphics::sprites::z_order;

// ---------------------------------------------------------------------------
// Screen shake
// ---------------------------------------------------------------------------

/// Resource controlling screen shake effect.
#[derive(Resource, Debug, Default)]
pub struct ScreenShake {
    pub intensity: f32,
    pub duration: f32,
    pub elapsed: f32,
}

impl ScreenShake {
    pub fn trigger(&mut self, intensity: f32, duration: f32) {
        self.intensity = intensity;
        self.duration = duration;
        self.elapsed = 0.0;
    }

    pub fn is_active(&self) -> bool {
        self.elapsed < self.duration && self.intensity > 0.0
    }
}

/// Applies screen shake to the camera. Properly resets camera position when done.
pub fn apply_screen_shake(
    time: Res<Time>,
    mut shake: ResMut<ScreenShake>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    if !shake.is_active() {
        // Reset camera to origin if shake just finished
        if shake.intensity > 0.0 && shake.elapsed >= shake.duration {
            for mut transform in camera_query.iter_mut() {
                transform.translation.x = 0.0;
                transform.translation.y = 0.0;
            }
            shake.intensity = 0.0;
        }
        return;
    }

    shake.elapsed += time.delta_secs();

    if shake.elapsed >= shake.duration {
        // Shake finished -- reset camera
        for mut transform in camera_query.iter_mut() {
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
        }
        shake.intensity = 0.0;
        return;
    }

    let progress = shake.elapsed / shake.duration;
    let current_intensity = shake.intensity * (1.0 - progress);

    for mut transform in camera_query.iter_mut() {
        let offset_x = (rand::random::<f32>() - 0.5) * 2.0 * current_intensity;
        let offset_y = (rand::random::<f32>() - 0.5) * 2.0 * current_intensity;
        transform.translation.x = offset_x;
        transform.translation.y = offset_y;
    }
}

// ---------------------------------------------------------------------------
// Screen fade
// ---------------------------------------------------------------------------

/// Resource controlling screen fade transitions.
#[derive(Resource, Debug)]
pub struct ScreenFade {
    pub alpha: f32,
    pub target_alpha: f32,
    pub speed: f32,
    /// Whether the fade is currently animating.
    pub active: bool,
}

impl Default for ScreenFade {
    fn default() -> Self {
        Self {
            alpha: 0.0,
            target_alpha: 0.0,
            speed: 2.0,
            active: false,
        }
    }
}

impl ScreenFade {
    /// Start fading in (from black to transparent).
    pub fn fade_in(&mut self, speed: f32) {
        self.target_alpha = 0.0;
        self.alpha = 1.0;
        self.speed = speed;
        self.active = true;
    }

    /// Start fading out (from transparent to black).
    pub fn fade_out(&mut self, speed: f32) {
        self.target_alpha = 1.0;
        self.alpha = 0.0;
        self.speed = speed;
        self.active = true;
    }

    /// Returns true if the fade has reached its target.
    pub fn is_complete(&self) -> bool {
        !self.active || (self.alpha - self.target_alpha).abs() < 0.01
    }
}

/// Marker component for the screen fade overlay entity.
#[derive(Component, Debug)]
pub struct FadeOverlay;

/// Animates the ScreenFade alpha over time and applies it to the fade overlay.
pub fn screen_fade_system(
    time: Res<Time>,
    mut fade: ResMut<ScreenFade>,
    mut overlay_query: Query<&mut Sprite, With<FadeOverlay>>,
) {
    if !fade.active {
        return;
    }

    let dt = time.delta_secs();
    let direction = if fade.target_alpha > fade.alpha {
        1.0
    } else {
        -1.0
    };

    fade.alpha += direction * fade.speed * dt;

    // Clamp and check for completion
    if direction > 0.0 && fade.alpha >= fade.target_alpha {
        fade.alpha = fade.target_alpha;
        fade.active = false;
    } else if direction < 0.0 && fade.alpha <= fade.target_alpha {
        fade.alpha = fade.target_alpha;
        fade.active = false;
    }

    // Update the overlay sprite's alpha
    for mut sprite in overlay_query.iter_mut() {
        sprite.color = Color::srgba(0.0, 0.0, 0.0, fade.alpha);
    }
}

/// Spawns the fade overlay entity (a full-screen black sprite).
pub fn spawn_fade_overlay(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Sprite {
                color: Color::srgba(0.0, 0.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(SCREEN_WIDTH, SCREEN_HEIGHT)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, z_order::FADE_OVERLAY),
            FadeOverlay,
        ))
        .id()
}

// ---------------------------------------------------------------------------
// Particle system
// ---------------------------------------------------------------------------

/// Component for a single particle.
#[derive(Component, Debug)]
pub struct Particle {
    pub velocity: Vec2,
    pub lifetime: f32,
    pub elapsed: f32,
    pub gravity: f32,
    pub fade_out: bool,
}

/// Component for a particle emitter that spawns particles.
#[derive(Component, Debug)]
pub struct ParticleEmitter {
    /// World position to spawn particles from.
    pub position: Vec2,
    /// Base direction of emission (radians).
    pub direction: f32,
    /// Spread angle (radians) around the base direction.
    pub spread: f32,
    /// Base speed of emitted particles.
    pub speed: f32,
    /// Speed variation (+/-).
    pub speed_variation: f32,
    /// Lifetime of each particle in seconds.
    pub particle_lifetime: f32,
    /// Number of particles to spawn.
    pub count: u32,
    /// Color of the particles.
    pub color: Color,
    /// Size of each particle (width, height).
    pub particle_size: Vec2,
    /// Whether this emitter has already fired.
    pub fired: bool,
    /// Whether particles are affected by gravity.
    pub gravity: f32,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            direction: -std::f32::consts::FRAC_PI_2, // upward
            spread: std::f32::consts::PI,
            speed: 100.0,
            speed_variation: 30.0,
            particle_lifetime: 0.8,
            count: 8,
            color: Color::WHITE,
            particle_size: Vec2::new(4.0, 4.0),
            fired: false,
            gravity: 200.0,
        }
    }
}

/// Predefined emitter for block break particles.
pub fn block_break_emitter(position: Vec2, color: Color) -> ParticleEmitter {
    ParticleEmitter {
        position,
        direction: -std::f32::consts::FRAC_PI_2, // upward
        spread: std::f32::consts::FRAC_PI_2,
        speed: 150.0,
        speed_variation: 50.0,
        particle_lifetime: 0.6,
        count: 12,
        color,
        particle_size: Vec2::new(6.0, 6.0),
        fired: false,
        gravity: 300.0,
    }
}

/// Predefined emitter for stomp star burst.
pub fn stomp_burst_emitter(position: Vec2) -> ParticleEmitter {
    ParticleEmitter {
        position,
        direction: 0.0,
        spread: std::f32::consts::TAU, // full circle
        speed: 80.0,
        speed_variation: 20.0,
        particle_lifetime: 0.4,
        count: 6,
        color: Color::srgb(1.0, 1.0, 0.5), // yellow-ish
        particle_size: Vec2::new(3.0, 3.0),
        fired: false,
        gravity: 0.0,
    }
}

/// Predefined emitter for powerup collect flash.
pub fn powerup_collect_emitter(position: Vec2) -> ParticleEmitter {
    ParticleEmitter {
        position,
        direction: -std::f32::consts::FRAC_PI_2,
        spread: std::f32::consts::TAU,
        speed: 60.0,
        speed_variation: 20.0,
        particle_lifetime: 0.3,
        count: 8,
        color: Color::srgb(1.0, 1.0, 1.0),
        particle_size: Vec2::new(4.0, 4.0),
        fired: false,
        gravity: 0.0,
    }
}

/// System that processes ParticleEmitters, spawning particles, and then despawning the emitter.
pub fn particle_spawn_system(
    mut commands: Commands,
    mut emitters: Query<(Entity, &mut ParticleEmitter)>,
) {
    for (entity, mut emitter) in emitters.iter_mut() {
        if emitter.fired {
            // Emitter has already spawned its particles; despawn it
            commands.entity(entity).despawn();
            continue;
        }

        emitter.fired = true;

        for _ in 0..emitter.count {
            // Randomize direction within spread
            let angle_offset = (rand::random::<f32>() - 0.5) * emitter.spread;
            let angle = emitter.direction + angle_offset;

            // Randomize speed
            let speed_offset = (rand::random::<f32>() - 0.5) * 2.0 * emitter.speed_variation;
            let speed = emitter.speed + speed_offset;

            let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

            commands.spawn((
                Sprite {
                    color: emitter.color,
                    custom_size: Some(emitter.particle_size),
                    ..default()
                },
                Transform::from_xyz(
                    emitter.position.x,
                    emitter.position.y,
                    z_order::PARTICLES,
                ),
                Particle {
                    velocity,
                    lifetime: emitter.particle_lifetime,
                    elapsed: 0.0,
                    gravity: emitter.gravity,
                    fade_out: true,
                },
            ));
        }
    }
}

/// System that updates particle positions, applies gravity, and despawns expired particles.
pub fn particle_update_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Particle, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform, mut sprite) in query.iter_mut() {
        particle.elapsed += dt;

        if particle.elapsed >= particle.lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        // Apply gravity to velocity
        particle.velocity.y -= particle.gravity * dt;

        // Update position
        transform.translation.x += particle.velocity.x * dt;
        transform.translation.y += particle.velocity.y * dt;

        // Fade out
        if particle.fade_out {
            let alpha = 1.0 - (particle.elapsed / particle.lifetime);
            sprite.color = sprite.color.with_alpha(alpha);
        }
    }
}

// ---------------------------------------------------------------------------
// Spawn effect
// ---------------------------------------------------------------------------

/// Component for a visual spawn effect (growing ring at spawn point).
#[derive(Component, Debug)]
pub struct SpawnEffect {
    /// Center position.
    pub position: Vec2,
    /// Total duration of the effect.
    pub duration: f32,
    /// Accumulated time.
    pub elapsed: f32,
    /// Maximum radius of the effect ring.
    pub max_radius: f32,
}

impl SpawnEffect {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            duration: 0.5,
            elapsed: 0.0,
            max_radius: 24.0,
        }
    }
}

/// System that animates spawn effects and despawns them when complete.
pub fn spawn_effect_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut SpawnEffect, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut effect, mut transform, mut sprite) in query.iter_mut() {
        effect.elapsed += dt;

        if effect.elapsed >= effect.duration {
            commands.entity(entity).despawn();
            continue;
        }

        let progress = effect.elapsed / effect.duration;
        let radius = effect.max_radius * progress;

        // Scale the sprite to represent the growing ring
        let size = radius * 2.0;
        sprite.custom_size = Some(Vec2::new(size, size));

        // Fade out
        let alpha = 1.0 - progress;
        sprite.color = Color::srgba(1.0, 1.0, 1.0, alpha * 0.6);

        // Keep it centered
        transform.translation.x = effect.position.x;
        transform.translation.y = effect.position.y;
    }
}

/// Helper to spawn a visual spawn effect entity at a position.
pub fn create_spawn_effect(commands: &mut Commands, position: Vec2) -> Entity {
    commands
        .spawn((
            Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                custom_size: Some(Vec2::new(2.0, 2.0)),
                ..default()
            },
            Transform::from_xyz(position.x, position.y, z_order::EFFECTS),
            SpawnEffect::new(position),
        ))
        .id()
}
