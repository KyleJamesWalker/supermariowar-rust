//! Sound output through SDL3 audio streams: one stream per virtual channel the core chose and one for music, all
//! bound to one device, which mixes them. SDL3 decodes the WAV sounds; lewton decodes the Ogg Vorbis music.

use lewton::inside_ogg::OggStreamReader;
use sdl3_sys::everything::*;
use smw_platform::{AudioOut, SoundId, TrackId};
use std::collections::HashMap;
use std::ffi::{c_int, c_void, CStr};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

/// Voice streams; the core's virtual mixer has the same count.
const CHANNELS: usize = 16;
/// The device format, and every voice's PCM.
const SPEC: SDL_AudioSpec = SDL_AudioSpec { format: SDL_AUDIO_S16LE, channels: 2, freq: 44100 };
/// Of `SPEC`.
const BYTES_PER_SECOND: u64 = 44100 * 2 * 2;

fn sdl_error() -> String {
    unsafe { CStr::from_ptr(SDL_GetError()).to_string_lossy().into_owned() }
}

/// A channel's sound: PCM at `SPEC`, the read position in bytes, and how many more times it plays after this one
/// (-1 forever).
#[derive(Default)]
struct Voice {
    pcm: Option<Arc<Vec<u8>>>,
    pos: usize,
    loops: i32,
}

/// The playing track: its Ogg bytes, kept to restart a looping track, and the decoder.
#[derive(Default)]
struct MusicVoice {
    bytes: Option<Arc<[u8]>>,
    reader: Option<OggStreamReader<Cursor<Arc<[u8]>>>>,
    once: bool,
    paused: bool,
}

/// A Vorbis decoder over `bytes`, positioned at the start.
fn reader(bytes: &Arc<[u8]>) -> Result<OggStreamReader<Cursor<Arc<[u8]>>>, String> {
    OggStreamReader::new(Cursor::new(bytes.clone())).map_err(|e| e.to_string())
}

/// Takes the next `want` bytes of a voice, looping as asked; fewer when it ends.
fn take(voice: &mut Voice, mut want: usize, out: &mut Vec<u8>) {
    while want > 0 {
        let Some(pcm) = voice.pcm.clone() else { return };
        let n = want.min(pcm.len() - voice.pos);
        out.extend_from_slice(&pcm[voice.pos..voice.pos + n]);
        voice.pos += n;
        want -= n;
        if voice.pos >= pcm.len() {
            if voice.loops == 0 {
                voice.pcm = None;
            } else {
                if voice.loops > 0 {
                    voice.loops -= 1;
                }
                voice.pos = 0;
            }
        }
        if n == 0 && voice.pcm.is_some() {
            return;
        }
    }
}

/// The voice stream's get callback, on SDL's audio thread. Safety: `userdata` is the voice's `Mutex<Voice>`.
unsafe extern "C" fn feed_voice(userdata: *mut c_void, stream: *mut SDL_AudioStream, additional: c_int, _total: c_int) {
    let voice = &*(userdata as *const Mutex<Voice>);
    let Ok(mut voice) = voice.lock() else { return };
    let mut out = Vec::with_capacity(additional.max(0) as usize);
    take(&mut voice, additional.max(0) as usize, &mut out);
    if !out.is_empty() {
        SDL_PutAudioStreamData(stream, out.as_ptr() as *const c_void, out.len() as c_int);
    }
}

/// The music stream's get callback, on SDL's audio thread. Safety: `userdata` is the `Mutex<MusicVoice>`.
unsafe extern "C" fn feed_music(userdata: *mut c_void, stream: *mut SDL_AudioStream, additional: c_int, _total: c_int) {
    let music = &*(userdata as *const Mutex<MusicVoice>);
    let Ok(mut music) = music.lock() else { return };
    if music.paused {
        return;
    }
    let mut written = 0;
    while written < additional {
        let Some(r) = music.reader.as_mut() else { return };
        match r.read_dec_packet_itl() {
            Ok(Some(samples)) => {
                let bytes: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
                SDL_PutAudioStreamData(stream, bytes.as_ptr() as *const c_void, bytes.len() as c_int);
                written += bytes.len() as c_int;
            }
            _ => {
                music.reader = if music.once { None } else { music.bytes.as_ref().and_then(|b| reader(b).ok()) };
                if music.reader.is_none() {
                    return;
                }
            }
        }
    }
}

/// SDL3 audio streams fed from decoded sounds and tracks, by the core's ids.
pub struct Sdl3Audio {
    device: SDL_AudioDeviceID,
    /// One stream per channel; the boxes stay put because the callbacks hold pointers to them.
    voices: Vec<(*mut SDL_AudioStream, Box<Mutex<Voice>>)>,
    music: Option<(*mut SDL_AudioStream, Box<Mutex<MusicVoice>>)>,
    sounds: HashMap<SoundId, Arc<Vec<u8>>>,
    tracks: HashMap<TrackId, Arc<[u8]>>,
}

impl Sdl3Audio {
    pub fn new() -> Self {
        Sdl3Audio { device: SDL_AudioDeviceID(0), voices: Vec::new(), music: None, sounds: HashMap::new(), tracks: HashMap::new() }
    }

    /// Stops channel `i` and drops the audio its stream has queued.
    fn halt_voice(&self, i: usize) {
        if let Some((stream, voice)) = self.voices.get(i) {
            if let Ok(mut v) = voice.lock() {
                v.pcm = None;
            }
            // SAFETY: streams in `voices` are live until `close`.
            unsafe { SDL_ClearAudioStream(*stream) };
        }
    }
}

impl Default for Sdl3Audio {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Sdl3Audio {
    fn drop(&mut self) {
        self.close();
    }
}

/// The default playback device at `SPEC`.
/// With `exact`, a failed open restarts SDL's audio subsystem on the dummy driver and tries again; 0 on failure.
unsafe fn open_device(exact: bool) -> SDL_AudioDeviceID {
    SDL_InitSubSystem(SDL_INIT_AUDIO);
    let device = SDL_OpenAudioDevice(SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &SPEC);
    if device.0 != 0 || !exact {
        return device;
    }
    eprintln!("[sfx] no usable audio device ({}); continuing silently", sdl_error());
    SDL_QuitSubSystem(SDL_INIT_AUDIO);
    SDL_SetHint(SDL_HINT_AUDIO_DRIVER, c"dummy".as_ptr());
    SDL_InitSubSystem(SDL_INIT_AUDIO);
    SDL_OpenAudioDevice(SDL_AUDIO_DEVICE_DEFAULT_PLAYBACK, &SPEC)
}

impl AudioOut for Sdl3Audio {
    fn open(&mut self, exact: bool) {
        // SAFETY: each callback's userdata is a boxed mutex that outlives its stream, which `close` destroys first.
        unsafe {
            self.device = open_device(exact);
            if self.device.0 == 0 {
                eprintln!("[sfx] SDL_OpenAudioDevice failed: {}", sdl_error());
                return;
            }
            for _ in 0..CHANNELS {
                let stream = SDL_CreateAudioStream(&SPEC, &SPEC);
                let voice = Box::new(Mutex::new(Voice::default()));
                SDL_SetAudioStreamGetCallback(stream, Some(feed_voice), &*voice as *const Mutex<Voice> as *mut c_void);
                SDL_BindAudioStream(self.device, stream);
                self.voices.push((stream, voice));
            }
            let stream = SDL_CreateAudioStream(&SPEC, &SPEC);
            let music = Box::new(Mutex::new(MusicVoice::default()));
            SDL_SetAudioStreamGetCallback(stream, Some(feed_music), &*music as *const Mutex<MusicVoice> as *mut c_void);
            SDL_BindAudioStream(self.device, stream);
            self.music = Some((stream, music));
            SDL_ResumeAudioDevice(self.device);

            let mut spec = SDL_AudioSpec::default();
            SDL_GetAudioDeviceFormat(self.device, &mut spec, std::ptr::null_mut());
            let driver = SDL_GetCurrentAudioDriver();
            let driver = if driver.is_null() { String::from("none") } else { CStr::from_ptr(driver).to_string_lossy().into_owned() };
            println!("[sfx] SDL3 audio streams initialized.");
            println!("[sfx] audio driver {}, {} Hz, format {:#06x}, {} channels", driver, spec.freq, spec.format.0, spec.channels);
        }
    }

    fn close(&mut self) {
        // SAFETY: each stream is destroyed once, before the box its callback reads is dropped.
        unsafe {
            for (stream, _) in self.voices.drain(..) {
                SDL_DestroyAudioStream(stream);
            }
            if let Some((stream, _)) = self.music.take() {
                SDL_DestroyAudioStream(stream);
            }
            if self.device.0 != 0 {
                SDL_CloseAudioDevice(self.device);
                self.device = SDL_AudioDeviceID(0);
            }
        }
    }

    fn is_open(&self) -> bool {
        self.device.0 != 0
    }

    fn load_sound(&mut self, id: SoundId, bytes: &[u8]) -> Result<u32, String> {
        let pcm = decode_wav(bytes)?;
        let ms = (pcm.len() as u64 * 1000 / BYTES_PER_SECOND) as u32;
        self.sounds.insert(id, Arc::new(pcm));
        Ok(ms)
    }

    /// The length is unknown (negative); the core's duration table lists every track in data/.
    fn load_track(&mut self, id: TrackId, bytes: &[u8]) -> Result<f64, String> {
        let bytes: Arc<[u8]> = Arc::from(bytes);
        reader(&bytes)?;
        self.tracks.insert(id, bytes);
        Ok(-1.0)
    }

    fn free_sound(&mut self, id: SoundId) {
        self.sounds.remove(&id);
    }

    fn free_track(&mut self, id: TrackId) {
        self.tracks.remove(&id);
    }

    fn play(&mut self, channel: u8, id: SoundId, loops: i32) {
        let Some(pcm) = self.sounds.get(&id).cloned() else { return };
        self.halt_voice(channel as usize);
        if let Some((_, voice)) = self.voices.get(channel as usize) {
            if let Ok(mut v) = voice.lock() {
                *v = Voice { pcm: Some(pcm), pos: 0, loops };
            }
        }
    }

    fn halt(&mut self, channel: Option<u8>) {
        match channel {
            Some(c) => self.halt_voice(c as usize),
            None => (0..self.voices.len()).for_each(|i| self.halt_voice(i)),
        }
    }

    fn play_track(&mut self, id: TrackId, once: bool) {
        let Some(bytes) = self.tracks.get(&id).cloned() else { return };
        let Some((stream, music)) = &self.music else { return };
        let Ok(r) = reader(&bytes) else { return };
        let src = SDL_AudioSpec { format: SDL_AUDIO_S16LE, channels: r.ident_hdr.audio_channels as c_int, freq: r.ident_hdr.audio_sample_rate as c_int };
        if let Ok(mut m) = music.lock() {
            // SAFETY: the music stream is live; holding the lock keeps the callback out meanwhile.
            unsafe {
                SDL_ClearAudioStream(*stream);
                SDL_SetAudioStreamFormat(*stream, &src, std::ptr::null());
            }
            *m = MusicVoice { bytes: Some(bytes), reader: Some(r), once, paused: false };
        }
    }

    fn stop_track(&mut self) {
        if let Some((stream, music)) = &self.music {
            if let Ok(mut m) = music.lock() {
                *m = MusicVoice::default();
            }
            // SAFETY: the music stream is live until `close`.
            unsafe { SDL_ClearAudioStream(*stream) };
        }
    }

    fn pause_track(&mut self, paused: bool) {
        if let Some((_, music)) = &self.music {
            if let Ok(mut m) = music.lock() {
                m.paused = paused;
            }
        }
    }

    /// 0 to 128 on every channel.
    fn set_sound_volume(&mut self, volume: i32) {
        for (stream, _) in &self.voices {
            // SAFETY: streams in `voices` are live until `close`.
            unsafe { SDL_SetAudioStreamGain(*stream, volume.clamp(0, 128) as f32 / 128.0) };
        }
    }

    fn set_music_volume(&mut self, volume: i32) {
        if let Some((stream, _)) = &self.music {
            // SAFETY: the music stream is live until `close`.
            unsafe { SDL_SetAudioStreamGain(*stream, volume.clamp(0, 128) as f32 / 128.0) };
        }
    }
}

/// A WAV file as PCM at 44100 Hz, S16, stereo.
pub fn decode_wav(bytes: &[u8]) -> Result<Vec<u8>, String> {
    // SAFETY: `SDL_LoadWAV_IO` reads `bytes` and closes the stream before returning; both SDL buffers are freed once.
    unsafe {
        let io = SDL_IOFromConstMem(bytes.as_ptr() as *const c_void, bytes.len());
        let mut spec = SDL_AudioSpec::default();
        let (mut buf, mut len) = (std::ptr::null_mut(), 0u32);
        if !SDL_LoadWAV_IO(io, true, &mut spec, &mut buf, &mut len) {
            return Err(sdl_error());
        }
        let (mut out, mut out_len) = (std::ptr::null_mut(), 0);
        let converted = SDL_ConvertAudioSamples(&spec, buf, len as c_int, &SPEC, &mut out, &mut out_len);
        SDL_free(buf as *mut c_void);
        if !converted {
            return Err(sdl_error());
        }
        let pcm = std::slice::from_raw_parts(out, out_len.max(0) as usize).to_vec();
        SDL_free(out as *mut c_void);
        Ok(pcm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voice(len: usize, loops: i32) -> Voice {
        Voice { pcm: Some(Arc::new((0..len as u8).collect())), pos: 0, loops }
    }

    #[test]
    fn a_sound_plays_once_and_ends() {
        let mut v = voice(6, 0);
        let mut out = Vec::new();
        take(&mut v, 4, &mut out);
        take(&mut v, 4, &mut out);
        assert_eq!(out, [0, 1, 2, 3, 4, 5]);
        assert!(v.pcm.is_none());
    }

    #[test]
    fn loops_count_extra_plays() {
        let mut v = voice(3, 2);
        let mut out = Vec::new();
        take(&mut v, 100, &mut out);
        assert_eq!(out, [0, 1, 2, 0, 1, 2, 0, 1, 2]);
        let mut forever = voice(2, -1);
        let mut out = Vec::new();
        take(&mut forever, 7, &mut out);
        assert_eq!(out, [0, 1, 0, 1, 0, 1, 0]);
        assert!(forever.pcm.is_some());
    }

    #[test]
    fn music_decodes() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/music/game/Standard/Menu/menu.ogg");
        let bytes: Arc<[u8]> = Arc::from(std::fs::read(path).unwrap());
        let mut r = reader(&bytes).unwrap();
        assert_eq!((r.ident_hdr.audio_channels, r.ident_hdr.audio_sample_rate), (2, 44100));
        let samples: usize = (0..20).filter_map(|_| r.read_dec_packet_itl().ok().flatten()).map(|p| p.len()).sum();
        assert!(samples > 0);
    }
}
