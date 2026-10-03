//! Port of src/common/sfx.cpp

use sdl2::sys::mixer::*;
use sdl2::sys::{SDL_GetError, SDL_GetTicks, SDL_RWFromFile, AUDIO_S16};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::path::Path;
use std::ptr::null_mut;

pub static mut fResumeMusic: bool = true;
// Clock for the sfxSound::play() retrigger throttle; the replay harness swaps it (cpp-harness.patch).
pub static mut sfx_ticks: extern "C" fn() -> u32 = sdl_get_ticks;
pub static mut sfx_ignore_channel_failure: bool = false;

extern "C" fn sdl_get_ticks() -> u32 {
    unsafe { SDL_GetTicks() }
}

/// The C++ links `extern void musicfinished()` from GSGameplay.cpp (an empty one in the editors).
/// The port resolves that link at runtime: gs_gameplay assigns its `musicfinished` here.
pub static mut musicfinished: fn() = || {};

unsafe extern "C" fn musicfinished_trampoline() {
    musicfinished();
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
            unsafe { Mix_FreeChunk(self.0) };
        }
    }
}

impl Drop for MixMusicPtr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { Mix_FreeMusic(self.0) };
        }
    }
}

pub fn sfx_init() -> bool {
    unsafe {
        Mix_OpenAudio(44100, AUDIO_S16 as u16, 2, 2048);
        Mix_AllocateChannels(sfxSound::k_channels as i32);

        Mix_ChannelFinished(Some(sfxSound::on_channel_finished_c));
        Mix_HookMusicFinished(Some(musicfinished_trampoline));

        let link_version = &*Mix_Linked_Version();
        println!("[sfx] SDL_Mixer {}.{}.{} initialized.", link_version.major, link_version.minor, link_version.patch);
    }

    true
}

pub fn sfx_close() {
    unsafe { Mix_CloseAudio() };
}

pub fn sfx_stopallsounds() {
    unsafe { Mix_HaltChannel(-1) };
}

pub fn sfx_setmusicvolume(volume: i32) {
    unsafe { Mix_VolumeMusic(volume) };
}

pub fn sfx_setsoundvolume(volume: i32) {
    unsafe { Mix_Volume(-1, volume) };
}

pub fn sfx_can_play_audio() -> bool {
    let mut frequency = 0;
    let mut channels = 0;
    let mut format: u16 = 0;
    unsafe { Mix_QuerySpec(&mut frequency, &mut format, &mut channels) != 0 /* error */ }
}

pub struct sfxSound {
    m_sfx: MixChunkPtr,
    m_channels: u16,
    m_last_start_time: usize,
}

static mut s_channels: [*mut sfxSound; sfxSound::k_channels] = [null_mut(); sfxSound::k_channels];

impl Default for sfxSound {
    fn default() -> Self {
        sfxSound { m_sfx: MixChunkPtr(null_mut()), m_channels: 0, m_last_start_time: 0 }
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
        Ok(sfxSound { m_sfx: MixChunkPtr(chunk), m_channels: 0, m_last_start_time: 0 })
    }

    pub fn play(&mut self) -> bool {
        unsafe {
            let current_time: u32 = sfx_ticks();
            if (current_time as usize).wrapping_sub(self.m_last_start_time) < 40 {
                return false;
            }

            let channel = Mix_PlayChannelTimed(-1, self.m_sfx.0, 0, -1);
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
        let channel = unsafe { Mix_PlayChannelTimed(-1, self.m_sfx.0, loops, -1) };
        if channel < 0 {
            return;
        }

        self.m_channels |= 1 << channel;
    }

    pub fn stop(&mut self) {
        for i in 0..Self::k_channels {
            if self.m_channels & (1 << i) != 0 {
                unsafe { Mix_HaltChannel(i as i32) };
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
    m_paused: bool,
}

impl Default for sfxMusic {
    fn default() -> Self {
        sfxMusic { m_music: MixMusicPtr(null_mut()), m_paused: false }
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
        Ok(sfxMusic { m_music: MixMusicPtr(music), m_paused: false })
    }

    pub fn play(&mut self, fPlayonce: bool, fResume: bool) {
        unsafe {
            Mix_PlayMusic(self.m_music.0, if fPlayonce { 0 } else { -1 });
            fResumeMusic = fResume;
        }
    }

    pub fn stop(&mut self) {
        unsafe { Mix_HaltMusic() };
    }

    pub fn toggle_pause(&mut self) {
        unsafe {
            if self.m_paused {
                Mix_ResumeMusic();
            } else {
                Mix_PauseMusic();
            }
        }
        self.m_paused = !self.m_paused;
    }

    pub fn is_playing(&self) -> bool {
        unsafe { Mix_PlayingMusic() != 0 }
    }
}
