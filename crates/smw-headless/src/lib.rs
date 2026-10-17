//! The headless backend (docs/ARCHITECTURE_V2.md): smw-platform's traits in memory, with no SDL. It answers the
//! game's queries as SDL2 does on its dummy drivers, so replays dump and draw as under `tools/run_rust.sh`.

use smw_platform::{AudioOut, Clock, Events, Frame, Input, InputEvent, Overlay, PadChange, PadFamily, PadInfo, PadSlot, Pads, Services, SoundId, Storage, TrackId, Video, SCREEN_H, SCREEN_W};
use std::cell::Cell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

mod keys;
pub use keys::HeadlessKeyboard;

/// The last presented frame, 640x480 ARGB8888.
pub struct HeadlessVideo {
    pub pixels: Vec<u32>,
}

impl HeadlessVideo {
    pub fn new() -> Self {
        HeadlessVideo { pixels: vec![0; SCREEN_W * SCREEN_H] }
    }
}

impl Default for HeadlessVideo {
    fn default() -> Self {
        Self::new()
    }
}

impl Video for HeadlessVideo {
    fn open(&mut self, _fullscreen: bool) {}

    fn present(&mut self, frame: Frame<'_>, _overlay: Option<&Overlay>) {
        self.pixels.copy_from_slice(frame.pixels);
    }

    fn output_size(&self) -> (i32, i32) {
        (SCREEN_W as i32, SCREEN_H as i32)
    }

    fn display_dpi(&self) -> Option<f32> {
        None
    }

    fn set_fullscreen(&mut self, _on: bool) {}

    fn set_title(&mut self, _title: &str) {}

    fn show_error(&mut self, message: &str) {
        eprintln!("[headless] {}", message);
    }

    fn show_cursor(&mut self, _on: bool) {}
}

/// No device. The core's virtual mixer keeps the timing; a sound's length comes from its WAV header when the core's
/// duration table does not list it.
#[derive(Default)]
pub struct HeadlessAudio {
    open: bool,
}

impl HeadlessAudio {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Milliseconds of a PCM WAV's `data` chunk, or 0 for anything else.
fn wav_ms(bytes: &[u8]) -> u32 {
    let u32_at = |i: usize| bytes.get(i..i + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as u64);
    if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return 0;
    }
    let (mut at, mut bytes_per_second) = (12, 0);
    while at + 8 <= bytes.len() {
        let size = u32_at(at + 4);
        match &bytes[at..at + 4] {
            b"fmt " => bytes_per_second = u32_at(at + 16),
            b"data" if bytes_per_second > 0 => return (size * 1000 / bytes_per_second) as u32,
            _ => {}
        }
        at += 8 + size as usize + (size as usize & 1);
    }
    0
}

impl AudioOut for HeadlessAudio {
    fn open(&mut self, _exact: bool) {
        self.open = true;
    }

    fn close(&mut self) {
        self.open = false;
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn load_sound(&mut self, _id: SoundId, bytes: &[u8]) -> Result<u32, String> {
        Ok(wav_ms(bytes))
    }

    fn load_track(&mut self, _id: TrackId, _bytes: &[u8]) -> Result<f64, String> {
        Ok(-1.0)
    }

    fn free_sound(&mut self, _id: SoundId) {}

    fn free_track(&mut self, _id: TrackId) {}

    fn play(&mut self, _channel: u8, _id: SoundId, _loops: i32) {}

    fn halt(&mut self, _channel: Option<u8>) {}

    fn play_track(&mut self, _id: TrackId, _once: bool) {}

    fn stop_track(&mut self) {}

    fn pause_track(&mut self, _paused: bool) {}

    fn set_sound_volume(&mut self, _volume: i32) {}

    fn set_music_volume(&mut self, _volume: i32) {}
}

/// Time passes only when the game sleeps.
pub struct HeadlessClock {
    now: Cell<u64>,
}

impl HeadlessClock {
    pub fn new() -> Self {
        HeadlessClock { now: Cell::new(0) }
    }
}

impl Default for HeadlessClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for HeadlessClock {
    fn now_ms(&self) -> u64 {
        self.now.get()
    }

    /// At least 1 ms, so a wait that polls the clock ends.
    fn sleep_ms(&self, ms: u32) {
        self.now.set(self.now.get() + ms.max(1) as u64);
    }
}

pub struct HeadlessStorage {
    pub settings_dir: PathBuf,
    pub root_dir: PathBuf,
}

impl Storage for HeadlessStorage {
    fn settings_dir(&self) -> PathBuf {
        self.settings_dir.clone()
    }

    fn root_dir(&self) -> PathBuf {
        self.root_dir.clone()
    }

    /// In the order a case-insensitive APFS volume lists them, as the goldens were made; the same on every OS.
    fn list_dir(&self, dir: &Path) -> Vec<PathBuf> {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir).map(|e| e.filter_map(|e| e.ok()).map(|e| e.path()).collect()).unwrap_or_default();
        entries.sort_by_cached_key(|p| apfs_key(&p.file_name().unwrap_or_default().to_string_lossy()));
        entries
    }
}

const fn crc32c_table() -> [u32; 256] {
    let mut table = [0; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut bit = 0;
        while bit < 8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0x82F6_3B78 } else { c >> 1 };
            bit += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

/// `j_drec_hashed_key_t.name_len_and_hash` (tools/apfs_order.py). Names outside ASCII skip Unicode normalization.
pub fn apfs_key(name: &str) -> u32 {
    const CRC32C: [u32; 256] = crc32c_table();
    let mut crc = 0xFFFF_FFFFu32;
    for c in name.chars().flat_map(char::to_lowercase) {
        for b in (c as u32).to_le_bytes() {
            crc = CRC32C[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
        }
    }
    ((crc & 0x3F_FFFF) << 10) | ((name.len() as u32 + 1) & 0x3FF)
}

/// An in-memory queue: what the game pushes, it polls back.
#[derive(Default)]
pub struct HeadlessEvents {
    queue: VecDeque<Input>,
}

impl HeadlessEvents {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Events for HeadlessEvents {
    fn poll(&mut self) -> Option<Input> {
        self.queue.pop_front()
    }

    /// Nothing else delivers input, so an empty queue quits.
    fn wait(&mut self) -> Input {
        self.queue.pop_front().unwrap_or(Input { timestamp: 0, event: InputEvent::Quit })
    }

    fn push(&mut self, input: &Input) {
        self.queue.push_back(input.clone());
    }

    fn pump(&mut self) {}
}

/// No pads connected.
pub struct HeadlessPads;

impl Pads for HeadlessPads {
    fn open_all(&mut self, _open: bool, _translate: bool) -> Vec<Option<PadInfo>> {
        Vec::new()
    }

    fn hotplug(&mut self) -> Vec<PadChange> {
        Vec::new()
    }

    fn open(&mut self, _device: i32, _index: usize) -> Option<PadInfo> {
        None
    }

    fn name(&self, _index: usize, _pad: Option<PadSlot>) -> Option<String> {
        None
    }

    fn family(&self, _index: usize) -> PadFamily {
        PadFamily::Generic
    }

    fn set_led(&mut self, _player: i32, _rgb: [u8; 3]) {}

    fn log_input(&mut self, _input: &Input) {}
}

/// Headless services with the settings in `settings_dir` and `root_dir` as the executable's directory. Both end in a
/// separator, as SDL's paths do, since the game appends file names to them.
pub fn services(settings_dir: PathBuf, root_dir: PathBuf) -> Services {
    Services {
        clock: Box::new(HeadlessClock::new()),
        storage: Box::new(HeadlessStorage { settings_dir, root_dir }),
        video: Box::new(HeadlessVideo::new()),
        audio: Box::new(HeadlessAudio::new()),
        events: Box::new(HeadlessEvents::new()),
        keyboard: Box::new(HeadlessKeyboard::new()),
        pads: Box::new(HeadlessPads),
    }
}
