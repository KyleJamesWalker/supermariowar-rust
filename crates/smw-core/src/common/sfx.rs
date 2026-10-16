//! Port of src/common/sfx.cpp

use crate::game::Game;
use crate::common::sfx_durations;
use crate::globals::Aliased;
use crate::services::services;
use smw_platform::{SoundId, TrackId};
use std::io::Write;
use std::path::Path;
use std::ptr::null_mut;


fn sdl_get_ticks(g: &mut Game) -> u32 {
    crate::services::ticks(g)
}


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

/// The sound subsystem's part of `Game`.
pub struct SfxState {
    pub fResumeMusic: bool,
    // Clock for the sfxSound::play() retrigger throttle; the replay harness swaps it (cpp-harness.patch).
    pub sfx_ticks: fn(&mut Game) -> u32,
    pub sfx_ignore_channel_failure: bool,
    // Not in upstream: a virtual mixer driven by sfx_ticks owns all game-visible sound state in every run, and the
    // audio service plays the same commands as output only (docs/REPLAY.md, "Sound"). Seeded runs log every
    // command to sfx_events.
    pub sfx_log_events: bool,
    pub sfx_events: Vec<String>,
    // Whether the audio service plays anything; headless replays only run the virtual mixer.
    pub sfx_audible: bool,
    /// The C++ links `extern void musicfinished()` from GSGameplay.cpp (an empty one in the editors).
    /// The port resolves that link at runtime: gs_gameplay assigns its `musicfinished` here.
    pub musicfinished: fn(),
    v_channels: [VirtualChannel; sfxSound::k_channels],
    v_music: VirtualMusic,
    next_id: u32,
    s_channels: [*mut sfxSound; sfxSound::k_channels],
}

impl SfxState {
    pub const fn new() -> Self {
        SfxState {
            fResumeMusic: true,
            sfx_ticks: sdl_get_ticks,
            sfx_ignore_channel_failure: false,
            sfx_log_events: false,
            sfx_events: Vec::new(),
            sfx_audible: false,
            musicfinished: || {},
            v_channels: [VirtualChannel { chunk: None, forever: false, end: 0 }; sfxSound::k_channels],
            v_music: VirtualMusic { music: None, forever: false, paused: false, end: 0, remaining: 0 },
            next_id: 0,
            s_channels: [null_mut(); sfxSound::k_channels],
        }
    }
}

fn sfx_state(g: &mut Game) -> &mut SfxState {
    &mut g.sfx
}

fn new_id(g: &mut Game) -> u32 {
    unsafe {
        sfx_state(g).next_id += 1;
        sfx_state(g).next_id
    }
}

fn log_event(g: &mut Game, line: String) {
    unsafe {
        if sfx_state(g).sfx_log_events {
            sfx_state(g).sfx_events.push(line);
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

unsafe fn mix_play_channel(g: &mut Game, chunk: Option<SoundId>, duration_ms: u32, loops: i32) -> i32 {
    let mut gp = g.ptr();
    let Some(id) = chunk else {
        // Mix_PlayChannel rejects NULL chunks
        return -1;
    };

    for i in 0..sfxSound::k_channels {
        let ch = &mut sfx_state(g).v_channels[i];
        if ch.chunk.is_some() {
            continue;
        }
        ch.chunk = Some(id);
        ch.forever = loops < 0;
        ch.end = (sfx_state(gp.get()).sfx_ticks)(gp.get()).wrapping_add(duration_ms.wrapping_mul((loops + 1) as u32));
        if sfx_state(gp.get()).sfx_audible {
            services(gp.get()).audio.play(i as u8, id, loops);
        }
        return i as i32;
    }
    -1
}

unsafe fn mix_halt_channel(g: &mut Game, channel: i32) {
    let mut gp = g.ptr();
    if sfx_state(g).sfx_audible {
        services(gp.get()).audio.halt(if channel < 0 { None } else { Some(channel as u8) });
    }
    for i in 0..sfxSound::k_channels {
        if (channel < 0 || channel == i as i32) && sfx_state(g).v_channels[i].chunk.is_some() {
            sfx_state(g).v_channels[i].chunk = None;
            sfxSound::on_channel_finished(g, i as i32);
        }
    }
}

pub fn sfx_virtual_advance(g: &mut Game) {
    let mut gp = g.ptr();
    unsafe {
        let now = (sfx_state(g).sfx_ticks)(gp.get());
        for i in 0..sfxSound::k_channels {
            let ch = &mut sfx_state(g).v_channels[i];
            if ch.chunk.is_some() && !ch.forever && now.wrapping_sub(ch.end) as i32 >= 0 {
                ch.chunk = None;
                log_event(g, format!("S done ch={}", i));
                sfxSound::on_channel_finished(g, i as i32);
            }
        }

        if sfx_state(g).v_music.music.is_some() && !sfx_state(g).v_music.forever && !sfx_state(g).v_music.paused && now.wrapping_sub(sfx_state(g).v_music.end) as i32 >= 0 {
            sfx_state(g).v_music.music = None;
            log_event(g, "S musicdone".to_string());
            (sfx_state(g).musicfinished)();
        }
    }
}

/// `std::unique_ptr<Mix_Chunk, MixDeleter>`: the audio service's copy of a sound.
pub struct MixChunkPtr(Option<SoundId>);
/// `std::unique_ptr<Mix_Music, MixDeleter>`: the audio service's copy of a track.
pub struct MixMusicPtr(Option<TrackId>);

impl Drop for MixChunkPtr {
    fn drop(&mut self) {
        let g = crate::game::game().get();
        let mut gp = g.ptr();
        if let Some(id) = self.0 {
            unsafe {
                for ch in sfx_state(gp.get()).v_channels.iter_mut() {
                    if ch.chunk == Some(id) {
                        ch.chunk = None;
                    }
                }
            }
            services(gp.get()).audio.free_sound(id);
        }
    }
}

impl Drop for MixMusicPtr {
    fn drop(&mut self) {
        let g = crate::game::game().get();
        let mut gp = g.ptr();
        if let Some(id) = self.0 {
            unsafe {
                let sfx = sfx_state(gp.get());
                if sfx.v_music.music == Some(id) {
                    sfx.v_music.music = None;
                }
            }
            services(gp.get()).audio.free_track(id);
        }
    }
}

pub fn sfx_init(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    unsafe { services(gp.get()).audio.open(sfx_state(g).sfx_audible) };
    true
}

pub fn sfx_close(g: &mut Game) {
    let mut gp = g.ptr();
    services(gp.get()).audio.close();
}

pub fn sfx_stopallsounds(g: &mut Game) {
    log_event(g, "S haltall".to_string());
    unsafe { mix_halt_channel(g, -1) };
}

pub fn sfx_setmusicvolume(g: &mut Game, volume: i32) {
    let mut gp = g.ptr();
    log_event(g, format!("S musicvolume {}", volume));
    services(gp.get()).audio.set_music_volume(volume);
}

pub fn sfx_setsoundvolume(g: &mut Game, volume: i32) {
    let mut gp = g.ptr();
    log_event(g, format!("S soundvolume {}", volume));
    services(gp.get()).audio.set_sound_volume(volume);
}

#[cfg(target_os = "emscripten")]  // emscripten has sound capabilities
pub fn sfx_can_play_audio(_g: &mut Game) -> bool {
    true
}

#[cfg(not(target_os = "emscripten"))]
pub fn sfx_can_play_audio(g: &mut Game) -> bool {
    let mut gp = g.ptr();
    services(gp.get()).audio.is_open()
}

pub struct sfxSound {
    m_sfx: MixChunkPtr,
    m_duration_ms: u32,
    m_name: String,
    m_channels: u16,
    m_last_start_time: usize,
    pub _alias: Aliased,
}


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
    pub fn from_file(g: &mut Game, path: &Path) -> Result<Self, String> {
        let mut gp = g.ptr();
        let path_str = path.to_string_lossy().into_owned();
        print!("loading {} ...", path_str);
        let _ = std::io::stdout().flush();

        let bytes = std::fs::read(path).map_err(|e| format!("Failed to load {}: {}", path_str, e))?;
        let id = SoundId(new_id(g));
        let decoded_ms = services(gp.get()).audio.load_sound(id, &bytes).map_err(|e| format!("Failed to load {}: {}", path_str, e))?;
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

    pub fn play(&mut self, g: &mut Game) -> bool {
        let mut gp = g.ptr();
        unsafe {
            let current_time: u32 = (sfx_state(g).sfx_ticks)(gp.get());
            if (current_time as usize).wrapping_sub(self.m_last_start_time) < 40 {
                log_event(g, format!("S skip {}", self.m_name));
                return false;
            }

            let channel = mix_play_channel(g, self.m_sfx.0, self.m_duration_ms, 0);
            log_event(g, format!("S play {} ch={}", self.m_name, channel));
            if channel < 0 {
                if sfx_state(g).sfx_ignore_channel_failure {
                    self.m_last_start_time = current_time as usize;
                }
                return sfx_state(g).sfx_ignore_channel_failure;
            }

            self.m_last_start_time = current_time as usize;
            self.m_channels |= 1 << channel;
            sfx_state(g).s_channels[channel as usize] = self as *mut sfxSound;
        }
        true
    }

    pub fn play_loop(&mut self, g: &mut Game, loops: i32) {
        let channel = unsafe { mix_play_channel(g, self.m_sfx.0, self.m_duration_ms, loops) };
        log_event(g, format!("S loop {} loops={} ch={}", self.m_name, loops, channel));
        if channel < 0 {
            return;
        }

        self.m_channels |= 1 << channel;
    }

    pub fn stop(&mut self, g: &mut Game) {
        log_event(g, format!("S stop {}", self.m_name));
        for i in 0..Self::k_channels {
            if self.m_channels & (1 << i) != 0 {
                unsafe { mix_halt_channel(g, i as i32) };
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        self.m_channels != 0
    }

    pub fn on_channel_finished(g: &mut Game, channel: i32) {
        unsafe {
            let sfx = sfx_state(g).s_channels[channel as usize];
            if !sfx.is_null() {
                (*sfx).m_channels &= !(1 << channel);
            }
            sfx_state(g).s_channels[channel as usize] = null_mut();
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
    pub fn from_file(g: &mut Game, path: &Path) -> Result<Self, String> {
        let mut gp = g.ptr();
        let path_str = path.to_string_lossy().into_owned();
        print!("loading {} ...", path_str);
        let _ = std::io::stdout().flush();

        let bytes = std::fs::read(path).map_err(|e| format!("Failed to load {}: {}", path_str, e))?;
        let id = TrackId(new_id(g));
        let decoded = services(gp.get()).audio.load_track(id, &bytes).map_err(|e| format!("Failed to load {}: {}", path_str, e))?;
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

    pub fn play(&mut self, g: &mut Game, fPlayonce: bool, fResume: bool) {
        let mut gp = g.ptr();
        log_event(g, format!("S music {} once={} resume={}", self.m_name, fPlayonce as i32, fResume as i32));
        unsafe {
            let duration = self.m_duration;
            sfx_state(g).v_music.music = self.m_music.0;
            sfx_state(g).v_music.forever = !fPlayonce || duration <= 0.0;
            sfx_state(g).v_music.paused = false;
            sfx_state(g).v_music.end = (sfx_state(g).sfx_ticks)(gp.get()).wrapping_add((duration * 1000.0) as u32);
            if sfx_state(g).sfx_audible {
                if let Some(id) = self.m_music.0 {
                    services(gp.get()).audio.play_track(id, fPlayonce);
                }
            }
            sfx_state(g).fResumeMusic = fResume;
        }
    }

    pub fn stop(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        log_event(g, format!("S musicstop {}", self.m_name));
        unsafe {
            sfx_state(g).v_music.music = None;
            if sfx_state(g).sfx_audible {
                services(gp.get()).audio.stop_track();
            }
        }
    }

    pub fn toggle_pause(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        log_event(g, format!("S musicpause {} paused={}", self.m_name, if self.m_paused { 0 } else { 1 }));
        unsafe {
            if sfx_state(g).sfx_audible {
                services(gp.get()).audio.pause_track(!self.m_paused);
            }
            if sfx_state(g).v_music.music.is_some() {
                if self.m_paused && sfx_state(g).v_music.paused {
                    sfx_state(g).v_music.end = (sfx_state(g).sfx_ticks)(gp.get()).wrapping_add(sfx_state(g).v_music.remaining);
                    sfx_state(g).v_music.paused = false;
                } else if !self.m_paused && !sfx_state(g).v_music.paused {
                    sfx_state(g).v_music.remaining = sfx_state(g).v_music.end.wrapping_sub((sfx_state(g).sfx_ticks)(gp.get()));
                    sfx_state(g).v_music.paused = true;
                }
            }
        }
        self.m_paused = !self.m_paused;
    }

    pub fn is_playing(&self, g: &mut Game) -> bool {
        unsafe { sfx_state(g).v_music.music.is_some() }
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
    pub fn save(g: &mut Game, sounds: &[*mut sfxSound], tracks: &[*mut sfxMusic]) -> (Vec<Channel>, Music, bool) {
        unsafe {
            let channels = (0..sfxSound::k_channels)
                .map(|i| Channel {
                    sound: sounds.iter().copied().find(|&s| sfx_state(g).v_channels[i].chunk.is_some() && (*s).m_sfx.0 == sfx_state(g).v_channels[i].chunk).unwrap_or(null_mut()),
                    owner: sfx_state(g).s_channels[i],
                    forever: sfx_state(g).v_channels[i].forever,
                    end: sfx_state(g).v_channels[i].end,
                })
                .collect();
            let music = Music {
                track: tracks.iter().copied().find(|&t| sfx_state(g).v_music.music.is_some() && (*t).m_music.0 == sfx_state(g).v_music.music).unwrap_or(null_mut()),
                forever: sfx_state(g).v_music.forever,
                paused: sfx_state(g).v_music.paused,
                end: sfx_state(g).v_music.end,
                remaining: sfx_state(g).v_music.remaining,
            };
            (channels, music, sfx_state(g).fResumeMusic)
        }
    }

    pub fn restore(g: &mut Game, channels: &[Channel], music: &Music, resume: bool) {
        unsafe {
            for (i, ch) in channels.iter().enumerate().take(sfxSound::k_channels) {
                sfx_state(g).v_channels[i] = VirtualChannel { chunk: if ch.sound.is_null() { None } else { (*ch.sound).m_sfx.0 }, forever: ch.forever, end: ch.end };
                sfx_state(g).s_channels[i] = ch.owner;
            }
            sfx_state(g).v_music = VirtualMusic {
                music: if music.track.is_null() { None } else { (*music.track).m_music.0 },
                forever: music.forever,
                paused: music.paused,
                end: music.end,
                remaining: music.remaining,
            };
            sfx_state(g).fResumeMusic = resume;
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
