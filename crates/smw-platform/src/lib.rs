//! The boundary between the game and a platform backend (docs/ARCHITECTURE_V2.md, Trait boundaries and
//! Normalized input). Nothing implements or calls these yet; phases 2 and 3 move the SDL calls behind them.

use std::path::PathBuf;

pub const SCREEN_W: usize = 640;
pub const SCREEN_H: usize = 480;

/// One finished frame: 640x480 ARGB8888, the same bytes `SMW_SHOT_STREAM` writes.
pub struct Frame<'a> {
    pub pixels: &'a [u32],
}

pub trait Video {
    /// Creates the window. A backend that cannot panics with its error message, as the C++ throws.
    fn open(&mut self, fullscreen: bool);
    fn present(&mut self, frame: Frame<'_>);
    fn set_fullscreen(&mut self, on: bool);
    fn set_title(&mut self, title: &str);
    fn show_error(&mut self, message: &str);
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SoundId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TrackId(pub u32);

/// Output only: the core's virtual mixer chooses channels and decides `isPlaying()`, finished channels and
/// finished music in every run; a backend never calls back into the game. The two load methods return a length,
/// which the core uses only for files its duration table (`sfx_durations.txt`) does not list.
pub trait AudioOut {
    /// Opens the device. With `exact`, it must take 44100 Hz, S16, stereo unchanged, or fall back to a silent
    /// device of that spec, so lengths match the headless runs.
    fn open(&mut self, exact: bool);
    fn close(&mut self);
    fn is_open(&self) -> bool;
    /// Decodes a sound. Returns its length in milliseconds at the opened spec, or the decoder's error.
    fn load_sound(&mut self, id: SoundId, bytes: &[u8]) -> Result<u32, String>;
    /// Decodes a track. Returns its length in seconds (negative when unknown), or the decoder's error.
    fn load_track(&mut self, id: TrackId, bytes: &[u8]) -> Result<f64, String>;
    fn free_sound(&mut self, id: SoundId);
    fn free_track(&mut self, id: TrackId);
    /// Plays on the channel the core's virtual mixer chose.
    fn play(&mut self, channel: u8, id: SoundId, loops: i32);
    /// `None` halts every channel.
    fn halt(&mut self, channel: Option<u8>);
    fn play_track(&mut self, id: TrackId, once: bool);
    fn stop_track(&mut self);
    fn pause_track(&mut self, paused: bool);
    /// 0 to 128, as SDL_mixer.
    fn set_sound_volume(&mut self, volume: i32);
    fn set_music_volume(&mut self, volume: i32);
}

pub trait Clock {
    /// Wall-clock milliseconds. Only the frame driver, the FPS overlay and netplay message timestamps read it.
    fn now_ms(&self) -> u64;
    fn sleep_ms(&self, ms: u32);
}

pub trait Storage {
    /// `~/Library/Preferences/.smw` on macOS, as the C++ game.
    fn settings_dir(&self) -> PathBuf;
    /// `GetRootDirectory()`: the executable's directory, where the game looks for `data/` unless `--datadir` says otherwise.
    fn root_dir(&self) -> PathBuf;
    /// Flushes written settings where that needs a step (IDBFS on the web).
    fn persist(&mut self) {}
}

/// What the game reaches through its context.
pub struct Services {
    pub clock: Box<dyn Clock>,
    pub storage: Box<dyn Storage>,
    pub video: Box<dyn Video>,
    pub audio: Box<dyn AudioOut>,
}

/// The open index of a pad: a replay's `<dev>`, SDL's `which`, and the device a binding names.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PadSlot(pub u8);

/// An SDL2 keycode value, as `controls.sdl2.bin`, `options.bin` and replays store keys.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Keycode(pub i32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TouchPhase {
    Down,
    Motion,
    Up,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum MouseEvent {
    Motion { x: i32, y: i32, buttons: u32 },
    Button { button: u8, down: bool, x: i32, y: i32 },
    Wheel { x: i32, y: i32 },
}

#[derive(Clone, PartialEq, Debug)]
pub enum InputEvent {
    Key { key: Keycode, down: bool },
    PadAxis { pad: PadSlot, axis: u8, value: i16 },
    PadButton { pad: PadSlot, button: u8, down: bool },
    PadHat { pad: PadSlot, hat: u8, value: u8 },
    PadAdded { pad: PadSlot, name: String, guid: [u8; 16] },
    PadRemoved { pad: PadSlot },
    /// Normalized to the window, 0.0 to 1.0.
    Touch { finger: u64, phase: TouchPhase, x: f32, y: f32 },
    Text(String),
    /// The editors only.
    Mouse(MouseEvent),
    Quit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Null;

    impl Video for Null {
        fn open(&mut self, _: bool) {}
        fn present(&mut self, frame: Frame<'_>) {
            assert_eq!(frame.pixels.len(), SCREEN_W * SCREEN_H);
        }
        fn set_fullscreen(&mut self, _: bool) {}
        fn set_title(&mut self, _: &str) {}
        fn show_error(&mut self, _: &str) {}
    }

    struct Log(Rc<RefCell<Vec<String>>>);

    impl AudioOut for Log {
        fn open(&mut self, _: bool) {}
        fn close(&mut self) {}
        fn is_open(&self) -> bool {
            true
        }
        fn load_sound(&mut self, id: SoundId, bytes: &[u8]) -> Result<u32, String> {
            self.0.borrow_mut().push(format!("load {} {}", id.0, bytes.len()));
            Ok(250)
        }
        fn load_track(&mut self, _: TrackId, _: &[u8]) -> Result<f64, String> {
            Ok(-1.0)
        }
        fn free_sound(&mut self, _: SoundId) {}
        fn free_track(&mut self, _: TrackId) {}
        fn play(&mut self, channel: u8, id: SoundId, loops: i32) {
            self.0.borrow_mut().push(format!("play {} ch={} loops={}", id.0, channel, loops));
        }
        fn halt(&mut self, _: Option<u8>) {}
        fn play_track(&mut self, _: TrackId, _: bool) {}
        fn stop_track(&mut self) {}
        fn pause_track(&mut self, _: bool) {}
        fn set_sound_volume(&mut self, _: i32) {}
        fn set_music_volume(&mut self, _: i32) {}
    }

    impl Clock for Null {
        fn now_ms(&self) -> u64 {
            1000
        }
        fn sleep_ms(&self, _: u32) {}
    }

    impl Storage for Null {
        fn settings_dir(&self) -> PathBuf {
            PathBuf::from("settings")
        }
        fn root_dir(&self) -> PathBuf {
            PathBuf::from("root")
        }
    }

    #[test]
    fn services_hold_trait_objects() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut services = Services { clock: Box::new(Null), storage: Box::new(Null), video: Box::new(Null), audio: Box::new(Log(log.clone())) };
        assert_eq!(services.clock.now_ms(), 1000);
        services.storage.persist();
        assert_eq!(services.storage.root_dir(), PathBuf::from("root"));

        let pixels = vec![0u32; SCREEN_W * SCREEN_H];
        services.video.open(false);
        services.video.present(Frame { pixels: &pixels });
        assert_eq!(services.audio.load_sound(SoundId(3), &[0; 4]), Ok(250));
        services.audio.play(0, SoundId(3), -1);
        assert_eq!(*log.borrow(), ["load 3 4", "play 3 ch=0 loops=-1"]);
    }
}
