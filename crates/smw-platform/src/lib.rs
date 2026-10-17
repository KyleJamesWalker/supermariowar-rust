//! The boundary between the game and a platform backend (docs/ARCHITECTURE_V2.md, Trait boundaries and
//! Normalized input). Nothing implements or calls these yet; phases 2 and 3 move the SDL calls behind them.

use std::path::PathBuf;

pub const SCREEN_W: usize = 640;
pub const SCREEN_H: usize = 480;

/// One finished frame: 640x480 ARGB8888, the same bytes `SMW_SHOT_STREAM` writes.
pub struct Frame<'a> {
    pub pixels: &'a [u32],
}

/// A straight-alpha image for the overlay: R, G, B, A bytes per pixel, row by row.
pub struct OverlayImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

/// One image drawn in window pixels, with its alpha and color modulation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OverlayQuad {
    pub image: u16,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub alpha: u8,
    pub color: [u8; 3],
}

/// What the core draws over the game's picture, outside its 640x480 viewport: the Android touch controls. The images
/// change only with `generation`, so a backend uploads them once per generation.
pub struct Overlay {
    pub generation: u64,
    pub images: std::sync::Arc<[OverlayImage]>,
    pub quads: Vec<OverlayQuad>,
}

pub trait Video {
    /// Creates the window. A backend that cannot panics with its error message, as the C++ throws.
    fn open(&mut self, fullscreen: bool);
    fn present(&mut self, frame: Frame<'_>, overlay: Option<&Overlay>);
    /// The window's size in pixels, which the overlay is laid out in.
    fn output_size(&self) -> (i32, i32);
    /// The display's diagonal DPI, if the platform reports one.
    fn display_dpi(&self) -> Option<f32>;
    fn set_fullscreen(&mut self, on: bool);
    fn set_title(&mut self, title: &str);
    fn show_error(&mut self, message: &str);
    /// Shows or hides the mouse cursor over the window.
    fn show_cursor(&mut self, on: bool);
}

pub trait Events {
    /// The next queued input, or `None` when the queue is empty.
    fn poll(&mut self) -> Option<Input>;
    /// Blocks until an input arrives.
    fn wait(&mut self) -> Input;
    /// Queues an input as if the platform delivered it.
    fn push(&mut self, input: &Input);
    /// Moves the platform's pending events into the queue.
    fn pump(&mut self);
}

/// Keyboard and mouse queries, in SDL2's key and scancode values under the current keyboard layout.
pub trait Keyboard {
    /// `SDL_GetScancodeFromKey`.
    fn scancode_from_key(&self, key: i32) -> i32;
    /// `SDL_GetKeyFromName`: 0 for an unknown name.
    fn key_from_name(&self, name: &str) -> i32;
    /// `SDL_GetKeyName`: empty for a key without one.
    fn key_name(&self, key: i32) -> String;
    /// `SDL_GetKeyboardState`: one byte per scancode, nonzero while held.
    fn state(&self) -> &[u8];
    /// `SDL_GetMouseState`'s button mask.
    fn mouse_buttons(&self) -> u32;
    /// Whether a window has keyboard focus.
    fn has_focus(&self) -> bool;
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
    pub events: Box<dyn Events>,
    pub keyboard: Box<dyn Keyboard>,
    pub pads: Box<dyn Pads>,
}

/// A pad the backend opened.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PadInfo {
    /// How its input events name it.
    pub pad: PadSlot,
    pub name: String,
    pub guid: String,
    /// A game controller whose events the backend translates (`SMW_PAD_TRANSLATE`).
    pub controller: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PadChange {
    /// A pad that was unplugged; the backend has closed it.
    Removed(PadSlot),
    /// A pad that was plugged in, by the backend's device index, for `Pads::open`.
    Connected(i32),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PadFamily {
    Generic,
    Xbox,
    PlayStation,
    Nintendo,
}

/// Game pads. The core keeps the device table (which device number a pad has) and who plays with which pad.
pub trait Pads {
    /// Starts pad input and, with `open`, opens every connected pad: entry i is backend device i, `None` when it
    /// fails to open. `translate` turns on mapping translation (`SMW_PAD_TRANSLATE`), which the backend applies
    /// before it hands out `Input`.
    fn open_all(&mut self, open: bool, translate: bool) -> Vec<Option<PadInfo>>;
    /// In a live session, the pads removed and then the ones connected since the last call.
    fn hotplug(&mut self) -> Vec<PadChange>;
    /// Opens a `PadChange::Connected` device as device number `index`.
    fn open(&mut self, device: i32, index: usize) -> Option<PadInfo>;
    /// The open pad's name, or with `pad` `None` the name of backend device `index`.
    fn name(&self, index: usize, pad: Option<PadSlot>) -> Option<String>;
    /// The family of backend device `index`.
    fn family(&self, index: usize) -> PadFamily;
    /// Sets the LED of the pad with player index `player` (SDL's numbering), where it has one.
    fn set_led(&mut self, player: i32, rgb: [u8; 3]);
    /// `SMW_DEBUG_INPUT`: logs an input as the game reads it.
    fn log_input(&mut self, input: &Input);
}

/// A pad as SDL's events name it (`which`): the device index for added events, the instance ID otherwise. The game
/// compares it with the device index a binding names, as the C++ does, and replays push `<dev>` here.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PadSlot(pub i32);

/// An SDL2 keycode value, as `controls.sdl2.bin`, `options.bin` and replays store keys.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Keycode(pub i32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TouchPhase {
    Down,
    Motion,
    Up,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MouseEvent {
    /// `buttons` is SDL's button mask.
    Motion { x: i32, y: i32, xrel: i32, yrel: i32, buttons: u32 },
    Button { button: u8, down: bool, clicks: u8, x: i32, y: i32 },
    Wheel { x: i32, y: i32 },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeviceChange {
    Added,
    Removed,
    Remapped,
}

/// One input event, as the backend delivers it. `timestamp` is the backend's milliseconds; the game never reads it,
/// but the replay harness keeps it so a pushed event reads back unchanged.
#[derive(Clone, PartialEq, Debug)]
pub struct Input {
    pub timestamp: u32,
    pub event: InputEvent,
}

#[derive(Clone, PartialEq, Debug)]
pub enum InputEvent {
    /// `scancode` and `mods` are SDL2's values; `window` is the window ID.
    Key { key: Keycode, scancode: i32, mods: u16, down: bool, repeat: bool, window: u32 },
    PadAxis { pad: PadSlot, axis: u8, value: i16 },
    PadButton { pad: PadSlot, button: u8, down: bool },
    PadHat { pad: PadSlot, hat: u8, value: u8 },
    /// A joystick plugged in, unplugged or (gamepads only) remapped.
    Pad { pad: PadSlot, change: DeviceChange },
    /// The SDL game controller API's view of a pad, which pad.rs translates; buttons and axes in SDL's numbering.
    GamepadAxis { pad: PadSlot, axis: u8, value: i16 },
    GamepadButton { pad: PadSlot, button: u8, down: bool },
    Gamepad { pad: PadSlot, change: DeviceChange },
    /// Normalized to the window, 0.0 to 1.0.
    Touch { touch: i64, finger: i64, phase: TouchPhase, x: f32, y: f32, dx: f32, dy: f32, pressure: f32 },
    Text(String),
    /// The window the event belongs to, then the event.
    Mouse { window: u32, which: u32, event: MouseEvent },
    /// A menu tap or click, in the game's 640x480 coordinates.
    Tap { x: i16, y: i16 },
    Quit,
    /// Any other backend event, by the backend's type number. Nothing reads its contents.
    Other(u32),
}

impl InputEvent {
    /// The point a menu should treat as tapped: a `Tap`, or a left-button release inside the picture.
    pub fn tap(&self) -> Option<(i16, i16)> {
        match *self {
            InputEvent::Tap { x, y } => Some((x, y)),
            InputEvent::Mouse { event: MouseEvent::Button { button: 1, down: false, x, y, .. }, .. }
                if (0..SCREEN_W as i32).contains(&x) && (0..SCREEN_H as i32).contains(&y) =>
            {
                Some((x as i16, y as i16))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Null;

    impl Video for Null {
        fn open(&mut self, _: bool) {}
        fn present(&mut self, frame: Frame<'_>, _: Option<&Overlay>) {
            assert_eq!(frame.pixels.len(), SCREEN_W * SCREEN_H);
        }
        fn output_size(&self) -> (i32, i32) {
            (SCREEN_W as i32, SCREEN_H as i32)
        }
        fn display_dpi(&self) -> Option<f32> {
            None
        }
        fn set_fullscreen(&mut self, _: bool) {}
        fn set_title(&mut self, _: &str) {}
        fn show_error(&mut self, _: &str) {}
        fn show_cursor(&mut self, _: bool) {}
    }

    impl Events for Null {
        fn poll(&mut self) -> Option<Input> {
            None
        }
        fn wait(&mut self) -> Input {
            Input { timestamp: 0, event: InputEvent::Quit }
        }
        fn push(&mut self, _: &Input) {}
        fn pump(&mut self) {}
    }

    impl Keyboard for Null {
        fn scancode_from_key(&self, _: i32) -> i32 {
            0
        }
        fn key_from_name(&self, _: &str) -> i32 {
            0
        }
        fn key_name(&self, _: i32) -> String {
            String::new()
        }
        fn state(&self) -> &[u8] {
            &[]
        }
        fn mouse_buttons(&self) -> u32 {
            0
        }
        fn has_focus(&self) -> bool {
            false
        }
    }

    impl Pads for Null {
        fn open_all(&mut self, _: bool, _: bool) -> Vec<Option<PadInfo>> {
            Vec::new()
        }
        fn hotplug(&mut self) -> Vec<PadChange> {
            Vec::new()
        }
        fn open(&mut self, _: i32, _: usize) -> Option<PadInfo> {
            None
        }
        fn name(&self, _: usize, _: Option<PadSlot>) -> Option<String> {
            None
        }
        fn family(&self, _: usize) -> PadFamily {
            PadFamily::Generic
        }
        fn set_led(&mut self, _: i32, _: [u8; 3]) {}
        fn log_input(&mut self, _: &Input) {}
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
        let mut services = Services { clock: Box::new(Null), storage: Box::new(Null), video: Box::new(Null), audio: Box::new(Log(log.clone())), events: Box::new(Null), keyboard: Box::new(Null), pads: Box::new(Null) };
        assert_eq!(services.clock.now_ms(), 1000);
        services.storage.persist();
        assert_eq!(services.storage.root_dir(), PathBuf::from("root"));

        let pixels = vec![0u32; SCREEN_W * SCREEN_H];
        services.video.open(false);
        services.video.present(Frame { pixels: &pixels }, None);
        assert_eq!(services.audio.load_sound(SoundId(3), &[0; 4]), Ok(250));
        services.audio.play(0, SoundId(3), -1);
        assert_eq!(*log.borrow(), ["load 3 4", "play 3 ch=0 loops=-1"]);
    }
}
