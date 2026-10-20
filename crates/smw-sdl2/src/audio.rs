//! Sound output through SDL2_mixer: one mixer channel per virtual channel the core chose, and one music track.

use sdl2::sys::mixer::*;
use sdl2::sys::{SDL_GetError, SDL_RWFromConstMem, AUDIO_S16};
use smw_platform::{AudioOut, SoundId, TrackId};
use std::collections::HashMap;
use std::ffi::CStr;

/// Mixer channels; the core's virtual mixer has the same count.
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

/// SDL_mixer output, holding the chunks and tracks the core loaded by its ids.
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

    /// For the opened spec; 0 while the mixer is closed.
    fn bytes_per_second() -> u64 {
        let (mut frequency, mut format, mut channels) = (0, 0u16, 0);
        unsafe { Mix_QuerySpec(&mut frequency, &mut format, &mut channels) };
        frequency as u64 * channels as u64 * ((format & 0xFF) as u64 / 8)
    }
}

impl Drop for Sdl2Audio {
    fn drop(&mut self) {
        let tracks: Vec<TrackId> = self.tracks.keys().copied().collect();
        tracks.into_iter().for_each(|id| self.free_track(id));
        let sounds: Vec<SoundId> = self.sounds.keys().copied().collect();
        sounds.into_iter().for_each(|id| self.free_sound(id));
    }
}

/// The real device with no format changes allowed; if that fails, the silent dummy driver rather than another spec.
/// On failure, restarts SDL's audio subsystem on the dummy driver.
unsafe fn open_exact() {
    if Mix_OpenAudioDevice(44100, AUDIO_S16 as u16, 2, 2048, std::ptr::null(), 0) == 0 {
        return;
    }
    eprintln!("[sfx] no usable audio device ({}); continuing silently", mix_error());
    sdl2::sys::SDL_QuitSubSystem(sdl2::sys::SDL_INIT_AUDIO);
    sdl2::sys::SDL_SetHintWithPriority(
        sdl2::sys::SDL_HINT_AUDIODRIVER.as_ptr().cast(),
        c"dummy".as_ptr(),
        sdl2::sys::SDL_HintPriority::SDL_HINT_OVERRIDE,
    );
    sdl2::sys::SDL_InitSubSystem(sdl2::sys::SDL_INIT_AUDIO);
    Mix_OpenAudio(44100, AUDIO_S16 as u16, 2, 2048);
}

impl AudioOut for Sdl2Audio {
    fn open(&mut self, exact: bool) {
        // SAFETY: plain SDL_mixer calls; `Mix_Linked_Version` and the driver name are static.
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
        // SAFETY: SDL_mixer ignores a close without an open.
        unsafe { Mix_CloseAudio() };
    }

    fn is_open(&self) -> bool {
        // SAFETY: the out-pointers are live locals.
        let (mut frequency, mut format, mut channels) = (0, 0u16, 0);
        unsafe { Mix_QuerySpec(&mut frequency, &mut format, &mut channels) != 0 }
    }

    fn load_sound(&mut self, id: SoundId, bytes: &[u8]) -> Result<u32, String> {
        // SAFETY: the chunk is decoded before the call returns and frees the RWops, so `bytes` need not outlive it.
        let chunk = unsafe { Mix_LoadWAV_RW(SDL_RWFromConstMem(bytes.as_ptr() as *const _, bytes.len() as i32), 1) };
        if chunk.is_null() {
            return Err(mix_error());
        }
        if let Some(old) = self.sounds.insert(id, chunk) {
            // SAFETY: no longer in the map, so freed once; SDL_mixer halts the channels playing it first.
            unsafe { Mix_FreeChunk(old) };
        }
        let bytes_per_second = Self::bytes_per_second();
        // SAFETY: `chunk` was just checked non-null.
        Ok(if bytes_per_second != 0 { (unsafe { (*chunk).alen } as u64 * 1000 / bytes_per_second) as u32 } else { 0 })
    }

    fn load_track(&mut self, id: TrackId, bytes: &[u8]) -> Result<f64, String> {
        let bytes = bytes.to_vec();
        // SAFETY: SDL_mixer streams from `bytes`, whose heap buffer `tracks` keeps until the track is freed.
        let music = unsafe { Mix_LoadMUS_RW(SDL_RWFromConstMem(bytes.as_ptr() as *const _, bytes.len() as i32), 1) };
        if music.is_null() {
            return Err(mix_error());
        }
        // SAFETY: `music` was just checked non-null.
        let duration = unsafe { Mix_MusicDuration(music) };
        if let Some((old, _old_bytes)) = self.tracks.insert(id, (music, bytes)) {
            // SAFETY: freed once, which halts it if playing, before `_old_bytes` drops at the end of this block.
            unsafe { Mix_FreeMusic(old) };
        }
        Ok(duration)
    }

    fn free_sound(&mut self, id: SoundId) {
        if let Some(chunk) = self.sounds.remove(&id) {
            // SAFETY: removed from the map, so each chunk is freed once.
            unsafe { Mix_FreeChunk(chunk) };
        }
    }

    fn free_track(&mut self, id: TrackId) {
        if let Some((music, _)) = self.tracks.remove(&id) {
            // SAFETY: removed from the map, so each track is freed once; SDL_mixer halts it first if it is playing.
            unsafe { Mix_FreeMusic(music) };
        }
    }

    fn play(&mut self, channel: u8, id: SoundId, loops: i32) {
        if let Some(&chunk) = self.sounds.get(&id) {
            // SAFETY: `chunk` is live while it is in `sounds`.
            unsafe { Mix_PlayChannelTimed(channel as i32, chunk, loops, -1) };
        }
    }

    fn halt(&mut self, channel: Option<u8>) {
        // SAFETY: SDL_mixer checks the channel number.
        unsafe { Mix_HaltChannel(channel.map_or(-1, |c| c as i32)) };
    }

    fn play_track(&mut self, id: TrackId, once: bool) {
        if let Some(&(music, _)) = self.tracks.get(&id) {
            // SAFETY: `music` is live while it is in `tracks`.
            unsafe { Mix_PlayMusic(music, if once { 0 } else { -1 }) };
        }
    }

    fn stop_track(&mut self) {
        // SAFETY: the remaining calls take no pointers.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A mono 16-bit PCM WAV of `samples` silent samples at 44100 Hz.
    fn wav(samples: u32) -> Vec<u8> {
        let data = samples * 2;
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36 + data).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes());
        w.extend_from_slice(&44100u32.to_le_bytes());
        w.extend_from_slice(&88200u32.to_le_bytes());
        w.extend_from_slice(&2u16.to_le_bytes());
        w.extend_from_slice(&16u16.to_le_bytes());
        w.extend_from_slice(b"data");
        w.extend_from_slice(&data.to_le_bytes());
        w.resize(w.len() + data as usize, 0);
        w
    }

    /// One test, since both open SDL's process-wide audio.
    #[test]
    fn falls_back_to_dummy_and_reloading_an_id_replaces_what_plays() {
        // SAFETY: the hint name and value are static C strings.
        unsafe {
            sdl2::sys::SDL_SetHintWithPriority(sdl2::sys::SDL_HINT_AUDIODRIVER.as_ptr().cast(), c"no-such-driver".as_ptr(), sdl2::sys::SDL_HintPriority::SDL_HINT_OVERRIDE);
        }
        let mut audio = Sdl2Audio::new();
        audio.open(true);
        assert!(audio.is_open(), "{}", mix_error());
        // SAFETY: SDL returns a static name while audio is initialized.
        assert_eq!(unsafe { CStr::from_ptr(sdl2::sys::SDL_GetCurrentAudioDriver()) }, c"dummy");
        let (sound, track) = (SoundId(1), TrackId(1));

        audio.load_sound(sound, &wav(44100)).unwrap();
        audio.play(0, sound, -1);
        audio.load_sound(sound, &wav(22050)).unwrap();
        assert_eq!(audio.sounds.len(), 1);
        audio.play(0, sound, 0);
        // SAFETY: SDL_mixer checks the channel number.
        assert_eq!(unsafe { Mix_Playing(0) }, 1);

        audio.load_track(track, &wav(44100)).unwrap();
        audio.play_track(track, false);
        let duration = audio.load_track(track, &wav(22050)).unwrap();
        assert!((duration - 0.5).abs() < 0.01, "{duration}");
        assert_eq!(audio.tracks.len(), 1);
        audio.play_track(track, false);
        // SAFETY: takes no pointers.
        assert_eq!(unsafe { Mix_PlayingMusic() }, 1);

        drop(audio);
        // SAFETY: takes no pointers.
        unsafe { Mix_CloseAudio() };
    }
}
