//! SDL2's key names and default keymap without SDL, from the table `smw_sdl2::keymap::measure` writes.

use smw_platform::Keyboard;
use std::sync::OnceLock;

const TABLE: &str = include_str!("sdl2_keys.txt");
/// `SDL_NUM_SCANCODES`.
const NUM_SCANCODES: usize = 512;
/// Set in a keycode that names a key by its scancode (`SDLK_SCANCODE_MASK`).
const SCANCODE_MASK: i32 = 1 << 30;

/// The table by scancode: its name (empty when unnamed) and its key (0 when none).
struct Keymap {
    names: Vec<&'static str>,
    keys: Vec<i32>,
}

/// Parsed from `TABLE` on first use.
fn keymap() -> &'static Keymap {
    static KEYMAP: OnceLock<Keymap> = OnceLock::new();
    KEYMAP.get_or_init(|| {
        let mut map = Keymap { names: vec![""; NUM_SCANCODES], keys: vec![0; NUM_SCANCODES] };
        for line in TABLE.lines().filter(|l| !l.starts_with('#')) {
            let mut fields = line.splitn(3, ' ');
            let scancode: usize = fields.next().and_then(|f| f.parse().ok()).expect("sdl2_keys.txt: scancode");
            map.keys[scancode] = fields.next().and_then(|f| f.parse().ok()).expect("sdl2_keys.txt: keycode");
            map.names[scancode] = fields.next().unwrap_or("");
        }
        map
    })
}

/// `SDL_GetScancodeName`: empty for an unnamed or out-of-range scancode.
fn scancode_name(scancode: i32) -> &'static str {
    usize::try_from(scancode).ok().and_then(|s| keymap().names.get(s)).copied().unwrap_or("")
}

/// `SDL_GetScancodeFromName`: 0 for an unknown name.
fn scancode_from_name(name: &str) -> usize {
    keymap().names.iter().position(|n| !n.is_empty() && n.eq_ignore_ascii_case(name)).unwrap_or(0)
}

/// The keyboard of SDL2 on the dummy video driver: the default keymap, no keys held, no window focus.
pub struct HeadlessKeyboard {
    state: [u8; NUM_SCANCODES],
}

impl HeadlessKeyboard {
    pub fn new() -> Self {
        HeadlessKeyboard { state: [0; NUM_SCANCODES] }
    }
}

impl Default for HeadlessKeyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Keyboard for HeadlessKeyboard {
    fn scancode_from_key(&self, key: i32) -> i32 {
        keymap().keys.iter().position(|&k| k == key).unwrap_or(0) as i32
    }

    fn key_from_name(&self, name: &str) -> i32 {
        let mut chars = name.chars();
        match (chars.next(), chars.next()) {
            (None, _) => 0,
            (Some(c), None) if c.is_ascii_uppercase() => c.to_ascii_lowercase() as i32,
            (Some(c), None) => c as i32,
            (Some(c), Some(_)) if !c.is_ascii() => 0,
            _ => keymap().keys[scancode_from_name(name)],
        }
    }

    fn key_name(&self, key: i32) -> String {
        if key & SCANCODE_MASK != 0 {
            return scancode_name(key & !SCANCODE_MASK).to_string();
        }
        match key {
            // Control characters are named after their keys' scancodes; other characters after themselves, uppercased.
            13 => scancode_name(40).to_string(),
            27 => scancode_name(41).to_string(),
            8 => scancode_name(42).to_string(),
            9 => scancode_name(43).to_string(),
            32 => scancode_name(44).to_string(),
            127 => scancode_name(76).to_string(),
            0 => String::new(),
            _ => char::from_u32(key as u32).map_or_else(String::new, |c| c.to_ascii_uppercase().to_string()),
        }
    }

    fn state(&self) -> &[u8] {
        &self.state
    }

    fn mouse_buttons(&self) -> u32 {
        0
    }

    fn has_focus(&self) -> bool {
        false
    }
}
