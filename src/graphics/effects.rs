use bevy::prelude::*;

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

/// Applies screen shake to the camera.
pub fn apply_screen_shake(
    time: Res<Time>,
    mut shake: ResMut<ScreenShake>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
) {
    if !shake.is_active() {
        return;
    }

    shake.elapsed += time.delta_secs();
    let progress = shake.elapsed / shake.duration;
    let current_intensity = shake.intensity * (1.0 - progress);

    for mut transform in camera_query.iter_mut() {
        let offset_x = (rand::random::<f32>() - 0.5) * 2.0 * current_intensity;
        let offset_y = (rand::random::<f32>() - 0.5) * 2.0 * current_intensity;
        transform.translation.x = offset_x;
        transform.translation.y = offset_y;
    }
}

/// Resource controlling screen fade transitions.
#[derive(Resource, Debug)]
pub struct ScreenFade {
    pub alpha: f32,
    pub target_alpha: f32,
    pub speed: f32,
}

impl Default for ScreenFade {
    fn default() -> Self {
        Self {
            alpha: 0.0,
            target_alpha: 0.0,
            speed: 2.0,
        }
    }
}

impl ScreenFade {
    pub fn fade_in(&mut self, speed: f32) {
        self.target_alpha = 0.0;
        self.alpha = 1.0;
        self.speed = speed;
    }

    pub fn fade_out(&mut self, speed: f32) {
        self.target_alpha = 1.0;
        self.alpha = 0.0;
        self.speed = speed;
    }
}
