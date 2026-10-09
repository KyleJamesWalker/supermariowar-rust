//! Port of src/common/sfx.cpp

use crate::common::sfx_durations;
use crate::globals::Aliased;
use crate::services::services;
use smw_platform::{SoundId, TrackId};
use std::io::Write;
use std::path::Path;
use std::ptr::null_mut;

pub static mut fResumeMusic: bool = true;
// Clock for the sfxSound::play() retrigger throttle; the replay harness swaps it (cpp-harness.patch).
pub static mut sfx_ticks: extern "C" fn() -> u32 = sdl_get_ticks;
pub static mut sfx_ignore_channel_failure: bool = false;
// Not in upstream: a virtual mixer driven by sfx_ticks owns all game-visible sound state in every run, and the
// audio service plays the same commands as output only (docs/REPLAY.md, "Sound"). Seeded runs log every
// command to sfx_events.
pub static mut sfx_log_events: bool = false;
pub static mut sfx_events: Vec<String> = Vec::new();
// Whether the audio service plays anything; headless replays only run the virtual mixer.
pub static mut sfx_audible: bool = false;

extern "C" fn sdl_get_ticks() -> u32 {
    crate::services::ticks()
}

/// The C++ links `extern void musicfinished()` from GSGameplay.cpp (an empty one in the editors).
/// The port resolves that link at runtime: gs_gameplay assigns its `musicfinished` here.
pub static mut musicfinished: fn() = || {};

#[derive(Clone, Copy)]
struct VirtualChannel {
    chunk: Option<SoundId>,
    forever: bool,
    end: u32,
}

struct VirtualMusic {
    music: Option<TrackId>,
    forever: bool,
    paused: bool,
    end: u32,
    remaining: u32,
}

static mut v_channels: [VirtualChannel; sfxSound::k_channels] =
    [VirtualChannel { chunk: None, forever: false, end: 0 }; sfxSound::k_channels];
static mut v_music: VirtualMusic = VirtualMusic { music: None, forever: false, paused: false, end: 0, remaining: 0 };
static mut next_id: u32 = 0;

fn new_id() -> u32 {
    unsafe {
        next_id += 1;
        next_id
    }
}

fn log_event(line: String) {
    unsafe {
        if sfx_log_events {
            sfx_events.push(line);
        }
    }
}

/// The data-relative part of a path ("sfx/packs/Classic/jump.wav"), identical for any data root and separator.
pub fn data_relative(path: &str) -> String {
    #[cfg(windows)]
    let path = &path.replace('\\', "/");
    let mut end = path.len();
    while let Some(pos) = path[..end].rfind("data/") {
        if pos == 0 || path.as_bytes()[pos - 1] == b'/' {
            return path[pos + 5..].to_string();
        }
        end = pos + 4;
    }
    path.to_string()
}

unsafe fn mix_play_channel(chunk: Option<SoundId>, duration_ms: u32, loops: i32) -> i32 {
    let Some(id) = chunk else {
        // Mix_PlayChannel rejects NULL chunks
        return -1;
    };

    for i in 0..sfxSound::k_channels {
        let ch = &mut v_channels[i];
        if ch.chunk.is_some() {
            continue;
        }
        ch.chunk = Some(id);
        ch.forever = loops < 0;
        ch.end = sfx_ticks().wrapping_add(duration_ms.wrapping_mul((loops + 1) as u32));
        if sfx_audible {
            services().audio.play(i as u8, id, loops);
        }
        return i as i32;
    }
    -1
}

unsafe fn mix_halt_channel(channel: i32) {
    if sfx_audible {
        services().audio.halt(if channel < 0 { None } else { Some(channel as u8) });
    }
    for i in 0..sfxSound::k_channels {
        if (channel < 0 || channel == i as i32) && v_channels[i].chunk.is_some() {
            v_channels[i].chunk = None;
            sfxSound::on_channel_finished(i as i32);
        }
    }
}

pub fn sfx_virtual_advance() {
    unsafe {
        let now = sfx_ticks();
        for i in 0..sfxSound::k_channels {
            let ch = &mut v_channels[i];
            if ch.chunk.is_some() && !ch.forever && now.wrapping_sub(ch.end) as i32 >= 0 {
                ch.chunk = None;
                log_event(format!("S done ch={}", i));
                sfxSound::on_channel_finished(i as i32);
            }
        }

        if v_music.music.is_some() && !v_music.forever && !v_music.paused && now.wrapping_sub(v_music.end) as i32 >= 0 {
            v_music.music = None;
            log_event("S musicdone".to_string());
            musicfinished();
        }
    }
}

/// `std::unique_ptr<Mix_Chunk, MixDeleter>`: the audio service's copy of a sound.
pub struct MixChunkPtr(Option<SoundId>);
/// `std::unique_ptr<Mix_Music, MixDeleter>`: the audio service's copy of a track.
pub struct MixMusicPtr(Option<TrackId>);

impl Drop for MixChunkPtr {
    fn drop(&mut self) {
        if let Some(id) = self.0 {
            unsafe {
                for ch in v_channels.iter_mut() {
                    if ch.chunk == Some(id) {
                        ch.chunk = None;
                    }
                }
            }
            services().audio.free_sound(id);
        }
    }
}

impl Drop for MixMusicPtr {
    fn drop(&mut self) {
        if let Some(id) = self.0 {
            unsafe {
                if v_music.music == Some(id) {
                    v_music.music = None;
                }
            }
            services().audio.free_track(id);
        }
    }
}

pub fn sfx_init() -> bool {
    unsafe { services().audio.open(sfx_audible) };
    true
}

pub fn sfx_close() {
    services().audio.close();
}

pub fn sfx_stopallsounds() {
    log_event("S haltall".to_string());
    unsafe { mix_halt_channel(-1) };
}

pub fn sfx_setmusicvolume(volume: i32) {
    log_event(format!("S musicvolume {}", volume));
    services().audio.set_music_volume(volume);
}

pub fn sfx_setsoundvolume(volume: i32) {
    log_event(format!("S soundvolume {}", volume));
    services().audio.set_sound_volume(volume);
}

#[cfg(target_os = "emscripten")]  // emscripten has sound capabilities
pub fn sfx_can_play_audio() -> bool {
    true
}

#[cfg(not(target_os = "emscripten"))]
pub fn sfx_can_play_audio() -> bool {
    services().audio.is_open()
}

pub struct sfxSound {
    m_sfx: MixChunkPtr,
    m_duration_ms: u32,
    m_name: String,
    m_channels: u16,
    m_last_start_time: usize,
    pub _alias: Aliased,
}

static mut s_channels: [*mut sfxSound; sfxSound::k_channels] = [null_mut(); sfxSound::k_channels];

impl Default for sfxSound {
    fn default() -> Self {
        sfxSound { _alias: Aliased::new(), m_sfx: MixChunkPtr(None), m_duration_ms: 0, m_name: String::new(), m_channels: 0, m_last_start_time: 0 }
    }
}

impl sfxSound {
    pub const k_channels: usize = 16;

    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `Err` where the C++ throws a `std::string`.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let path_str = path.to_string_lossy().into_owned();
        print!("loading {} ...", path_str);
        let _ = std::io::stdout().flush();

        let bytes = std::fs::read(path).map_err(|e| format!("Failed to load {}: {}", path_str, e))?;
        let id = SoundId(new_id());
        let decoded_ms = services().audio.load_sound(id, &bytes).map_err(|e| format!("Failed to load {}: {}", path_str, e))?;
        let name = data_relative(&path_str);

        println!(" done");
        Ok(sfxSound {
            _alias: Aliased::new(),
            m_sfx: MixChunkPtr(Some(id)),
            m_duration_ms: sfx_durations::sound_ms(&name).unwrap_or(decoded_ms),
            m_name: name,
            m_channels: 0,
            m_last_start_time: 0,
        })
    }

    pub fn play(&mut self) -> bool {
        unsafe {
            let current_time: u32 = sfx_ticks();
            if (current_time as usize).wrapping_sub(self.m_last_start_time) < 40 {
                log_event(format!("S skip {}", self.m_name));
                return false;
            }

            let channel = mix_play_channel(self.m_sfx.0, self.m_duration_ms, 0);
            log_event(format!("S play {} ch={}", self.m_name, channel));
            if channel < 0 {
                if sfx_ignore_channel_failure {
                    self.m_last_start_time = current_time as usize;
                }
                return sfx_ignore_channel_failure;
            }

            self.m_last_start_time = current_time as usize;
            self.m_channels |= 1 << channel;
            s_channels[channel as usize] = self as *mut sfxSound;
        }
        true
    }

    pub fn play_loop(&mut self, loops: i32) {
        let channel = unsafe { mix_play_channel(self.m_sfx.0, self.m_duration_ms, loops) };
        log_event(format!("S loop {} loops={} ch={}", self.m_name, loops, channel));
        if channel < 0 {
            return;
        }

        self.m_channels |= 1 << channel;
    }

    pub fn stop(&mut self) {
        log_event(format!("S stop {}", self.m_name));
        for i in 0..Self::k_channels {
            if self.m_channels & (1 << i) != 0 {
                unsafe { mix_halt_channel(i as i32) };
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        self.m_channels != 0
    }

    pub fn on_channel_finished(channel: i32) {
        unsafe {
            let sfx = s_channels[channel as usize];
            if !sfx.is_null() {
                (*sfx).m_channels &= !(1 << channel);
            }
            s_channels[channel as usize] = null_mut();
        }
    }
}

pub struct sfxMusic {
    m_music: MixMusicPtr,
    /// `Mix_MusicDuration`: seconds, negative when unknown.
    m_duration: f64,
    m_name: String,
    m_paused: bool,
    pub _alias: Aliased,
}

impl Default for sfxMusic {
    fn default() -> Self {
        sfxMusic { _alias: Aliased::new(), m_music: MixMusicPtr(None), m_duration: -1.0, m_name: String::new(), m_paused: false }
    }
}

impl sfxMusic {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `Err` where the C++ throws a `std::string`.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let path_str = path.to_string_lossy().into_owned();
        print!("loading {} ...", path_str);
        let _ = std::io::stdout().flush();

        let bytes = std::fs::read(path).map_err(|e| format!("Failed to load {}: {}", path_str, e))?;
        let id = TrackId(new_id());
        let decoded = services().audio.load_track(id, &bytes).map_err(|e| format!("Failed to load {}: {}", path_str, e))?;
        let name = data_relative(&path_str);

        println!(" done");
        Ok(sfxMusic {
            _alias: Aliased::new(),
            m_music: MixMusicPtr(Some(id)),
            m_duration: sfx_durations::track_seconds(&name).unwrap_or(decoded),
            m_name: name,
            m_paused: false,
        })
    }

    pub fn play(&mut self, fPlayonce: bool, fResume: bool) {
        log_event(format!("S music {} once={} resume={}", self.m_name, fPlayonce as i32, fResume as i32));
        unsafe {
            let duration = self.m_duration;
            v_music.music = self.m_music.0;
            v_music.forever = !fPlayonce || duration <= 0.0;
            v_music.paused = false;
            v_music.end = sfx_ticks().wrapping_add((duration * 1000.0) as u32);
            if sfx_audible {
                if let Some(id) = self.m_music.0 {
                    services().audio.play_track(id, fPlayonce);
                }
            }
            fResumeMusic = fResume;
        }
    }

    pub fn stop(&mut self) {
        log_event(format!("S musicstop {}", self.m_name));
        unsafe {
            v_music.music = None;
            if sfx_audible {
                services().audio.stop_track();
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        log_event(format!("S musicpause {} paused={}", self.m_name, if self.m_paused { 0 } else { 1 }));
        unsafe {
            if sfx_audible {
                services().audio.pause_track(!self.m_paused);
            }
            if v_music.music.is_some() {
                if self.m_paused && v_music.paused {
                    v_music.end = sfx_ticks().wrapping_add(v_music.remaining);
                    v_music.paused = false;
                } else if !self.m_paused && !v_music.paused {
                    v_music.remaining = v_music.end.wrapping_sub(sfx_ticks());
                    v_music.paused = true;
                }
            }
        }
        self.m_paused = !self.m_paused;
    }

    pub fn is_playing(&self) -> bool {
        unsafe { v_music.music.is_some() }
    }
}

/// Not in the C++: the virtual mixer state a replay checkpoint saves and restores (smw/checkpoint.rs).
/// Chunks and tracks are identified by their owner, so the state survives a reload in another process.
pub mod checkpoint {
    use super::*;

    pub struct Channel {
        pub sound: *mut sfxSound,
        pub owner: *mut sfxSound,
        pub forever: bool,
        pub end: u32,
    }

    pub struct Music {
        pub track: *mut sfxMusic,
        pub forever: bool,
        pub paused: bool,
        pub end: u32,
        pub remaining: u32,
    }

    /// `sounds` and `tracks` are every loaded sound and track; a channel or track owned by none reads as null.
    pub fn save(sounds: &[*mut sfxSound], tracks: &[*mut sfxMusic]) -> (Vec<Channel>, Music, bool) {
        unsafe {
            let channels = (0..sfxSound::k_channels)
                .map(|i| Channel {
                    sound: sounds.iter().copied().find(|&s| v_channels[i].chunk.is_some() && (*s).m_sfx.0 == v_channels[i].chunk).unwrap_or(null_mut()),
                    owner: s_channels[i],
                    forever: v_channels[i].forever,
                    end: v_channels[i].end,
                })
                .collect();
            let music = Music {
                track: tracks.iter().copied().find(|&t| v_music.music.is_some() && (*t).m_music.0 == v_music.music).unwrap_or(null_mut()),
                forever: v_music.forever,
                paused: v_music.paused,
                end: v_music.end,
                remaining: v_music.remaining,
            };
            (channels, music, fResumeMusic)
        }
    }

    pub fn restore(channels: &[Channel], music: &Music, resume: bool) {
        unsafe {
            for (i, ch) in channels.iter().enumerate().take(sfxSound::k_channels) {
                v_channels[i] = VirtualChannel { chunk: if ch.sound.is_null() { None } else { (*ch.sound).m_sfx.0 }, forever: ch.forever, end: ch.end };
                s_channels[i] = ch.owner;
            }
            v_music = VirtualMusic {
                music: if music.track.is_null() { None } else { (*music.track).m_music.0 },
                forever: music.forever,
                paused: music.paused,
                end: music.end,
                remaining: music.remaining,
            };
            fResumeMusic = resume;
        }
    }

    impl sfxSound {
        pub fn name(&self) -> &str {
            &self.m_name
        }
        pub fn state(&self) -> (u16, u64) {
            (self.m_channels, self.m_last_start_time as u64)
        }
        pub fn set_state(&mut self, channels: u16, last_start: u64) {
            self.m_channels = channels;
            self.m_last_start_time = last_start as usize;
        }
    }

    impl sfxMusic {
        pub fn name(&self) -> &str {
            &self.m_name
        }
        pub fn paused(&self) -> bool {
            self.m_paused
        }
        pub fn set_paused(&mut self, paused: bool) {
            self.m_paused = paused;
        }
    }
}
