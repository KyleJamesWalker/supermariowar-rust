//! Sound output through SDL2_mixer: one mixer channel per virtual channel the core chose, and one music track.

use sdl2::sys::mixer::*;
use sdl2::sys::{SDL_GetError, SDL_RWFromConstMem, AUDIO_S16};
use smw_platform::{AudioOut, SoundId, TrackId};
use std::collections::HashMap;
use std::ffi::CStr;

const CHANNELS: i32 = 16;

extern "C" {
    // SDL_mixer 2.6+, missing from sdl2-sys.
    fn Mix_MusicDuration(music: *mut Mix_Music) -> f64;
    // SDL_mixer 2.0.2+, missing from sdl2-sys.
    fn Mix_OpenAudioDevice(frequency: i32, format: u16, channels: i32, chunksize: i32, device: *const std::ffi::c_char, allowed_changes: i32) -> i32;
}

fn mix_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

#[derive(Default)]
pub struct Sdl2Audio {
    sounds: HashMap<SoundId, *mut Mix_Chunk>,
    /// SDL_mixer streams music from its source, so the bytes live as long as the track.
    tracks: HashMap<TrackId, (*mut Mix_Music, Vec<u8>)>,
}

impl Sdl2Audio {
    pub fn new() -> Self {
        Self::default()
    }

    fn bytes_per_second() -> u64 {
        let (mut frequency, mut format, mut channels) = (0, 0u16, 0);
        unsafe { Mix_QuerySpec(&mut frequency, &mut format, &mut channels) };
        frequency as u64 * channels as u64 * ((format & 0xFF) as u64 / 8)
    }
}

/// The real device with no format changes allowed; if that fails, the silent dummy driver rather than another spec.
unsafe fn open_exact() {
    if Mix_OpenAudioDevice(44100, AUDIO_S16 as u16, 2, 2048, std::ptr::null(), 0) == 0 {
        return;
    }
    eprintln!("[sfx] no usable audio device ({}); continuing silently", mix_error());
    sdl2::sys::SDL_QuitSubSystem(sdl2::sys::SDL_INIT_AUDIO);
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    sdl2::sys::SDL_InitSubSystem(sdl2::sys::SDL_INIT_AUDIO);
    Mix_OpenAudio(44100, AUDIO_S16 as u16, 2, 2048);
}

impl AudioOut for Sdl2Audio {
    fn open(&mut self, exact: bool) {
        unsafe {
            if exact {
                open_exact();
            } else {
                Mix_OpenAudio(44100, AUDIO_S16 as u16, 2, 2048);
            }
            Mix_AllocateChannels(CHANNELS);

            #[cfg(not(target_os = "emscripten"))]
            {
                let link_version = &*Mix_Linked_Version();
                println!("[sfx] SDL_Mixer {}.{}.{} initialized.", link_version.major, link_version.minor, link_version.patch);
                let (mut frequency, mut format, mut channels) = (0, 0u16, 0);
                let opened = Mix_QuerySpec(&mut frequency, &mut format, &mut channels);
                let driver = sdl2::sys::SDL_GetCurrentAudioDriver();
                let driver = if driver.is_null() { String::from("none") } else { CStr::from_ptr(driver).to_string_lossy().into_owned() };
                println!("[sfx] audio driver {}, {} Hz, format {:#06x}, {} channels (opened {})", driver, frequency, format, channels, opened);
            }
            // SDL_MIXER_VERSION of the emsdk 5.0.2 sdl2_mixer port (SDL_mixer-release-2.8.0).
            #[cfg(target_os = "emscripten")]
            println!("[sfx] SDL_Mixer 2.8.0 initialized.");
        }
    }

    fn close(&mut self) {
        unsafe { Mix_CloseAudio() };
    }

    fn is_open(&self) -> bool {
        let (mut frequency, mut format, mut channels) = (0, 0u16, 0);
        unsafe { Mix_QuerySpec(&mut frequency, &mut format, &mut channels) != 0 }
    }

    fn load_sound(&mut self, id: SoundId, bytes: &[u8]) -> Result<u32, String> {
        let chunk = unsafe { Mix_LoadWAV_RW(SDL_RWFromConstMem(bytes.as_ptr() as *const _, bytes.len() as i32), 1) };
        if chunk.is_null() {
            return Err(mix_error());
        }
        self.sounds.insert(id, chunk);
        let bytes_per_second = Self::bytes_per_second();
        Ok(if bytes_per_second != 0 { (unsafe { (*chunk).alen } as u64 * 1000 / bytes_per_second) as u32 } else { 0 })
    }

    fn load_track(&mut self, id: TrackId, bytes: &[u8]) -> Result<f64, String> {
        let bytes = bytes.to_vec();
        let music = unsafe { Mix_LoadMUS_RW(SDL_RWFromConstMem(bytes.as_ptr() as *const _, bytes.len() as i32), 1) };
        if music.is_null() {
            return Err(mix_error());
        }
        let duration = unsafe { Mix_MusicDuration(music) };
        self.tracks.insert(id, (music, bytes));
        Ok(duration)
    }

    fn free_sound(&mut self, id: SoundId) {
        if let Some(chunk) = self.sounds.remove(&id) {
            unsafe { Mix_FreeChunk(chunk) };
        }
    }

    fn free_track(&mut self, id: TrackId) {
        if let Some((music, _)) = self.tracks.remove(&id) {
            unsafe { Mix_FreeMusic(music) };
        }
    }

    fn play(&mut self, channel: u8, id: SoundId, loops: i32) {
        if let Some(&chunk) = self.sounds.get(&id) {
            unsafe { Mix_PlayChannelTimed(channel as i32, chunk, loops, -1) };
        }
    }

    fn halt(&mut self, channel: Option<u8>) {
        unsafe { Mix_HaltChannel(channel.map_or(-1, |c| c as i32)) };
    }

    fn play_track(&mut self, id: TrackId, once: bool) {
        if let Some(&(music, _)) = self.tracks.get(&id) {
            unsafe { Mix_PlayMusic(music, if once { 0 } else { -1 }) };
        }
    }

    fn stop_track(&mut self) {
        unsafe { Mix_HaltMusic() };
    }

    fn pause_track(&mut self, paused: bool) {
        unsafe {
            if paused {
                Mix_PauseMusic();
            } else {
                Mix_ResumeMusic();
            }
        }
    }

    fn set_sound_volume(&mut self, volume: i32) {
        unsafe { Mix_Volume(-1, volume) };
    }

    fn set_music_volume(&mut self, volume: i32) {
        unsafe { Mix_VolumeMusic(volume) };
    }
}
