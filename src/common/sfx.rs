//! Port of src/common/sfx.cpp

use crate::globals::Aliased;
use sdl2::sys::mixer::*;
use sdl2::sys::{SDL_GetError, SDL_GetTicks, SDL_RWFromFile, SDL_version, AUDIO_S16};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::path::Path;
use std::ptr::null_mut;

pub static mut fResumeMusic: bool = true;
// Clock for the sfxSound::play() retrigger throttle; the replay harness swaps it (cpp-harness.patch).
pub static mut sfx_ticks: extern "C" fn() -> u32 = sdl_get_ticks;
pub static mut sfx_ignore_channel_failure: bool = false;
// Seeded replays replace SDL_mixer playback state with a virtual mixer driven by sfx_ticks,
// and log every sound command to sfx_events (docs/REPLAY.md, "Sound").
pub static mut sfx_virtual_mixer: bool = false;
pub static mut sfx_events: Vec<String> = Vec::new();
// Recorded sessions and --replay watching: the virtual mixer still owns all game-visible state, and
// SDL_mixer plays the same commands as output only (results ignored, no callbacks).
pub static mut sfx_audible: bool = false;

extern "C" fn sdl_get_ticks() -> u32 {
    unsafe { SDL_GetTicks() }
}

/// The C++ links `extern void musicfinished()` from GSGameplay.cpp (an empty one in the editors).
/// The port resolves that link at runtime: gs_gameplay assigns its `musicfinished` here.
pub static mut musicfinished: fn() = || {};

unsafe extern "C" fn musicfinished_trampoline() {
    musicfinished();
}

#[derive(Clone, Copy)]
struct VirtualChannel {
    chunk: *mut Mix_Chunk,
    forever: bool,
    end: u32,
}

struct VirtualMusic {
    music: *mut Mix_Music,
    forever: bool,
    paused: bool,
    end: u32,
    remaining: u32,
}

static mut v_channels: [VirtualChannel; sfxSound::k_channels] =
    [VirtualChannel { chunk: null_mut(), forever: false, end: 0 }; sfxSound::k_channels];
static mut v_music: VirtualMusic = VirtualMusic { music: null_mut(), forever: false, paused: false, end: 0, remaining: 0 };

fn log_event(line: String) {
    unsafe {
        if sfx_virtual_mixer {
            sfx_events.push(line);
        }
    }
}

/// The data-relative part of a path ("sfx/packs/Classic/jump.wav"), identical for any data root.
fn data_relative(path: &str) -> String {
    let mut end = path.len();
    while let Some(pos) = path[..end].rfind("data/") {
        if pos == 0 || path.as_bytes()[pos - 1] == b'/' {
            return path[pos + 5..].to_string();
        }
        end = pos + 4;
    }
    path.to_string()
}

unsafe fn chunk_duration_ms(chunk: *const Mix_Chunk) -> u32 {
    let mut frequency = 0;
    let mut channels = 0;
    let mut format: u16 = 0;
    Mix_QuerySpec(&mut frequency, &mut format, &mut channels);
    let bytesPerSecond = frequency as u64 * channels as u64 * ((format & 0xFF) as u64 / 8);
    if bytesPerSecond != 0 {
        ((*chunk).alen as u64 * 1000 / bytesPerSecond) as u32
    } else {
        0
    }
}

unsafe fn mix_play_channel(chunk: *mut Mix_Chunk, loops: i32) -> i32 {
    if !sfx_virtual_mixer {
        return Mix_PlayChannelTimed(-1, chunk, loops, -1);
    }

    if chunk.is_null() {
        // Mix_PlayChannel rejects NULL chunks
        return -1;
    }

    for i in 0..sfxSound::k_channels {
        let ch = &mut v_channels[i];
        if !ch.chunk.is_null() {
            continue;
        }
        ch.chunk = chunk;
        ch.forever = loops < 0;
        ch.end = sfx_ticks().wrapping_add(chunk_duration_ms(chunk).wrapping_mul((loops + 1) as u32));
        if sfx_audible {
            Mix_PlayChannelTimed(i as i32, chunk, loops, -1);
        }
        return i as i32;
    }
    -1
}

unsafe fn mix_halt_channel(channel: i32) {
    if !sfx_virtual_mixer {
        Mix_HaltChannel(channel);
        return;
    }

    if sfx_audible {
        Mix_HaltChannel(channel);
    }
    for i in 0..sfxSound::k_channels {
        if (channel < 0 || channel == i as i32) && !v_channels[i].chunk.is_null() {
            v_channels[i].chunk = null_mut();
            sfxSound::on_channel_finished(i as i32);
        }
    }
}

pub fn sfx_virtual_advance() {
    unsafe {
        if !sfx_virtual_mixer {
            return;
        }

        let now = sfx_ticks();
        for i in 0..sfxSound::k_channels {
            let ch = &mut v_channels[i];
            if !ch.chunk.is_null() && !ch.forever && now.wrapping_sub(ch.end) as i32 >= 0 {
                ch.chunk = null_mut();
                log_event(format!("S done ch={}", i));
                sfxSound::on_channel_finished(i as i32);
            }
        }

        if !v_music.music.is_null() && !v_music.forever && !v_music.paused && now.wrapping_sub(v_music.end) as i32 >= 0 {
            v_music.music = null_mut();
            log_event("S musicdone".to_string());
            musicfinished();
        }
    }
}

extern "C" {
    // SDL_mixer 2.6+, missing from sdl2-sys 0.37.
    fn Mix_MusicDuration(music: *mut Mix_Music) -> f64;
}

fn mix_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

/// `std::unique_ptr<Mix_Chunk, MixDeleter>`
pub struct MixChunkPtr(*mut Mix_Chunk);
/// `std::unique_ptr<Mix_Music, MixDeleter>`
pub struct MixMusicPtr(*mut Mix_Music);

impl Drop for MixChunkPtr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                for ch in v_channels.iter_mut() {
                    if ch.chunk == self.0 {
                        ch.chunk = null_mut();
                    }
                }
                Mix_FreeChunk(self.0)
            };
        }
    }
}

impl Drop for MixMusicPtr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                if v_music.music == self.0 {
                    v_music.music = null_mut();
                }
                Mix_FreeMusic(self.0)
            };
        }
    }
}

extern "C" {
    // SDL_mixer 2.0.2+, missing from sdl2-sys 0.37.
    fn Mix_OpenAudioDevice(frequency: i32, format: u16, channels: i32, chunksize: i32, device: *const std::ffi::c_char, allowed_changes: i32) -> i32;
}

/// Audible sessions must report the same mixer spec as the headless dummy driver, since the virtual
/// mixer derives sound lengths from it: open the real device with no format changes allowed, and if
/// that fails, fall back to the silent dummy driver rather than to a different spec.
unsafe fn open_audible_audio() {
    if Mix_OpenAudioDevice(44100, AUDIO_S16 as u16, 2, 2048, std::ptr::null(), 0) == 0 {
        return;
    }
    eprintln!("[sfx] no usable audio device ({}); continuing silently", mix_error());
    sdl2::sys::SDL_QuitSubSystem(sdl2::sys::SDL_INIT_AUDIO);
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    sdl2::sys::SDL_InitSubSystem(sdl2::sys::SDL_INIT_AUDIO);
    Mix_OpenAudio(44100, AUDIO_S16 as u16, 2, 2048);
}

pub fn sfx_init() -> bool {
    unsafe {
        if sfx_audible {
            open_audible_audio();
        } else {
            Mix_OpenAudio(44100, AUDIO_S16 as u16, 2, 2048);
        }
        Mix_AllocateChannels(sfxSound::k_channels as i32);

        // With the virtual mixer the game's channel and music state never come from SDL_mixer's threads.
        if !sfx_virtual_mixer {
            Mix_ChannelFinished(Some(sfxSound::on_channel_finished_c));
            Mix_HookMusicFinished(Some(musicfinished_trampoline));
        }

        #[cfg(not(target_os = "emscripten"))]
        {
            let link_version = &*Mix_Linked_Version();
            println!("[sfx] SDL_Mixer {}.{}.{} initialized.", link_version.major, link_version.minor, link_version.patch);
        }
        #[cfg(target_os = "emscripten")]
        {
            // SDL_MIXER_VERSION of the emsdk 5.0.2 sdl2_mixer port (SDL_mixer-release-2.8.0).
            let ver_compiled = SDL_version { major: 2, minor: 8, patch: 0 };
            println!("[sfx] SDL_Mixer {}.{}.{} initialized.", ver_compiled.major, ver_compiled.minor, ver_compiled.patch);
        }
    }

    true
}

pub fn sfx_close() {
    unsafe { Mix_CloseAudio() };
}

pub fn sfx_stopallsounds() {
    log_event("S haltall".to_string());
    unsafe { mix_halt_channel(-1) };
}

pub fn sfx_setmusicvolume(volume: i32) {
    log_event(format!("S musicvolume {}", volume));
    unsafe { Mix_VolumeMusic(volume) };
}

pub fn sfx_setsoundvolume(volume: i32) {
    log_event(format!("S soundvolume {}", volume));
    unsafe { Mix_Volume(-1, volume) };
}

#[cfg(target_os = "emscripten")]  // emscripten has sound capabilities
pub fn sfx_can_play_audio() -> bool {
    true
}

#[cfg(not(target_os = "emscripten"))]
pub fn sfx_can_play_audio() -> bool {
    let mut frequency = 0;
    let mut channels = 0;
    let mut format: u16 = 0;
    unsafe { Mix_QuerySpec(&mut frequency, &mut format, &mut channels) != 0 /* error */ }
}

pub struct sfxSound {
    m_sfx: MixChunkPtr,
    m_name: String,
    m_channels: u16,
    m_last_start_time: usize,
    pub _alias: Aliased,
}

static mut s_channels: [*mut sfxSound; sfxSound::k_channels] = [null_mut(); sfxSound::k_channels];

impl Default for sfxSound {
    fn default() -> Self {
        sfxSound { _alias: Aliased::new(), m_sfx: MixChunkPtr(null_mut()), m_name: String::new(), m_channels: 0, m_last_start_time: 0 }
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

        let cpath = CString::new(path_str.as_bytes()).unwrap();
        let chunk = unsafe { Mix_LoadWAV_RW(SDL_RWFromFile(cpath.as_ptr(), b"rb\0".as_ptr() as *const _), 1) };
        if chunk.is_null() {
            return Err(format!("Failed to load {}: {}", path_str, mix_error()));
        }

        println!(" done");
        Ok(sfxSound { _alias: Aliased::new(), m_sfx: MixChunkPtr(chunk), m_name: data_relative(&path_str), m_channels: 0, m_last_start_time: 0 })
    }

    pub fn play(&mut self) -> bool {
        unsafe {
            let current_time: u32 = sfx_ticks();
            if (current_time as usize).wrapping_sub(self.m_last_start_time) < 40 {
                log_event(format!("S skip {}", self.m_name));
                return false;
            }

            let channel = mix_play_channel(self.m_sfx.0, 0);
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
        let channel = unsafe { mix_play_channel(self.m_sfx.0, loops) };
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

    unsafe extern "C" fn on_channel_finished_c(channel: i32) {
        Self::on_channel_finished(channel);
    }
}

pub struct sfxMusic {
    m_music: MixMusicPtr,
    m_name: String,
    m_paused: bool,
    pub _alias: Aliased,
}

impl Default for sfxMusic {
    fn default() -> Self {
        sfxMusic { _alias: Aliased::new(), m_music: MixMusicPtr(null_mut()), m_name: String::new(), m_paused: false }
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

        let cpath = CString::new(path_str.as_bytes()).unwrap();
        let music = unsafe { Mix_LoadMUS(cpath.as_ptr()) };
        if music.is_null() {
            return Err(format!("Failed to load {}: {}", path_str, mix_error()));
        }

        println!(" done");
        Ok(sfxMusic { _alias: Aliased::new(), m_music: MixMusicPtr(music), m_name: data_relative(&path_str), m_paused: false })
    }

    pub fn play(&mut self, fPlayonce: bool, fResume: bool) {
        log_event(format!("S music {} once={} resume={}", self.m_name, fPlayonce as i32, fResume as i32));
        unsafe {
            if sfx_virtual_mixer {
                let duration = Mix_MusicDuration(self.m_music.0);
                v_music.music = self.m_music.0;
                v_music.forever = !fPlayonce || duration <= 0.0;
                v_music.paused = false;
                v_music.end = sfx_ticks().wrapping_add((duration * 1000.0) as u32);
                if sfx_audible {
                    Mix_PlayMusic(self.m_music.0, if fPlayonce { 0 } else { -1 });
                }
            } else {
                Mix_PlayMusic(self.m_music.0, if fPlayonce { 0 } else { -1 });
            }
            fResumeMusic = fResume;
        }
    }

    pub fn stop(&mut self) {
        log_event(format!("S musicstop {}", self.m_name));
        unsafe {
            if sfx_virtual_mixer {
                v_music.music = null_mut();
                if sfx_audible {
                    Mix_HaltMusic();
                }
            } else {
                Mix_HaltMusic();
            }
        }
    }

    pub fn toggle_pause(&mut self) {
        log_event(format!("S musicpause {} paused={}", self.m_name, if self.m_paused { 0 } else { 1 }));
        unsafe {
            if sfx_virtual_mixer {
                if sfx_audible {
                    if self.m_paused {
                        Mix_ResumeMusic();
                    } else {
                        Mix_PauseMusic();
                    }
                }
                if !v_music.music.is_null() {
                    if self.m_paused && v_music.paused {
                        v_music.end = sfx_ticks().wrapping_add(v_music.remaining);
                        v_music.paused = false;
                    } else if !self.m_paused && !v_music.paused {
                        v_music.remaining = v_music.end.wrapping_sub(sfx_ticks());
                        v_music.paused = true;
                    }
                }
            } else if self.m_paused {
                Mix_ResumeMusic();
            } else {
                Mix_PauseMusic();
            }
        }
        self.m_paused = !self.m_paused;
    }

    pub fn is_playing(&self) -> bool {
        unsafe {
            if sfx_virtual_mixer {
                return !v_music.music.is_null();
            }
            Mix_PlayingMusic() != 0
        }
    }
}
