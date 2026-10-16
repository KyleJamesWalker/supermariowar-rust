//! Not in upstream: the length SDL2_mixer gives every sound and track in data/, which the virtual mixer
//! (sfx.rs) turns into channel and music end times. The table is the input for making the virtual mixer
//! independent of the backend's decoder (docs/ARCHITECTURE_V2.md, Audio).
//!
//! Regenerate with `cargo run -p smw-sdl2 --example sfx_durations > crates/smw-core/src/common/sfx_durations.txt`.

use std::collections::HashMap;
use std::sync::OnceLock;

pub const TABLE: &str = include_str!("sfx_durations.txt");

struct Table {
    sounds: HashMap<&'static str, u32>,
    tracks: HashMap<&'static str, f64>,
}

fn table() -> &'static Table {
    static TABLE_PARSED: OnceLock<Table> = OnceLock::new();
    TABLE_PARSED.get_or_init(|| {
        let mut t = Table { sounds: HashMap::new(), tracks: HashMap::new() };
        for line in TABLE.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split(' ').collect();
            match f[0] {
                "chunk" => {
                    t.sounds.insert(f[1], f[3].parse().unwrap());
                }
                "music" => {
                    t.tracks.insert(f[1], f[2].parse().unwrap());
                }
                _ => panic!("sfx_durations.txt: {}", line),
            }
        }
        t
    })
}

/// A sound's length in milliseconds at 44100 Hz, S16, stereo, by its data-relative path.
pub fn sound_ms(name: &str) -> Option<u32> {
    table().sounds.get(name).copied()
}

/// A track's `Mix_MusicDuration` in seconds, by its data-relative path.
pub fn track_seconds(name: &str) -> Option<f64> {
    table().tracks.get(name).copied()
}
