use bevy::prelude::*;

/// Drives frame-based sprite animation.
#[derive(Component, Debug)]
pub struct AnimationController {
    pub frames: Vec<usize>,
    pub frame_duration: f32,
    pub current_frame: usize,
    pub elapsed: f32,
    pub looping: bool,
    pub finished: bool,
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
        }
    }

    pub fn current_sprite_index(&self) -> usize {
        self.frames.get(self.current_frame).copied().unwrap_or(0)
    }
}

/// Advances all animation controllers each frame.
pub fn animate_sprites(
    time: Res<Time>,
    mut query: Query<&mut AnimationController>,
) {
    let dt = time.delta_secs();
    for mut anim in query.iter_mut() {
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
    }
}
