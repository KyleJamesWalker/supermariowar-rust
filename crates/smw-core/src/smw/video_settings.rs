//! The shader preset and integer scaling settings: the presets on disk, `video.txt`, and applying them to the window.

use crate::common::path::get_home_directory;
use crate::game::Game;
use crate::services::services;
use std::path::{Path, PathBuf};

const FILE: &str = "video.txt";

/// A shader preset the Graphics menu offers.
#[derive(Clone, Debug, PartialEq)]
pub struct ShaderPreset {
    /// The file name without `.slangp`.
    pub name: String,
    /// `builtin/<name>`, or `bundled/<path>` or `user/<path>` relative to its folder; what `video.txt` stores.
    pub key: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct VideoSettings {
    /// The backend's built-in presets, then the bundled ones, then the player's, each group by name.
    pub presets: Vec<ShaderPreset>,
    /// The Graphics menu's choice: 0 is Off, `n` is `presets[n - 1]`.
    pub shader: i16,
    pub integer_scale: bool,
}

impl VideoSettings {
    pub const fn new() -> Self {
        VideoSettings { presets: Vec::new(), shader: 0, integer_scale: false }
    }

    /// The selected preset, if any.
    pub fn preset(&self) -> Option<&ShaderPreset> {
        (self.shader as usize).checked_sub(1).and_then(|i| self.presets.get(i))
    }
}

/// The bundled presets: `shaders/presets/` beside the data folder.
fn bundled_dir(g: &mut Game) -> PathBuf {
    let data = crate::common::path::convert_path(g, "");
    // The data folder's parent by name; a `..` would leave through a symlinked data folder.
    let data = Path::new(data.trim_end_matches('/'));
    data.parent().unwrap_or(Path::new(".")).join("shaders").join("presets")
}

/// The player's presets: `shaders/` in the settings folder.
fn user_dir(g: &mut Game) -> PathBuf {
    PathBuf::from(get_home_directory(g)).join("shaders")
}

/// Every `.slangp` under `dir`, four folders deep at most, sorted by name.
fn find_presets(dir: &Path, group: &str) -> Vec<ShaderPreset> {
    fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && depth < 4 {
                walk(&path, depth + 1, out);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("slangp")) {
                out.push(path);
            }
        }
    }
    let mut paths = Vec::new();
    walk(dir, 0, &mut paths);
    let mut presets: Vec<ShaderPreset> = paths
        .into_iter()
        .filter_map(|path| {
            let rel = path.strip_prefix(dir).ok()?.to_string_lossy().replace('\\', "/");
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some(ShaderPreset { name, key: format!("{group}/{rel}"), path })
        })
        .collect();
    presets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.key.cmp(&b.key)));
    presets
}

/// Finds the presets and reads `video.txt`, then picks the saved preset for the window `gfx_init` opens.
pub fn load(g: &mut Game) {
    if !services(g).video.supports_shaders() {
        return;
    }
    let mut presets: Vec<ShaderPreset> = services(g)
        .video
        .builtin_shaders()
        .into_iter()
        .map(|name| ShaderPreset { key: format!("builtin/{name}"), path: PathBuf::from(format!("builtin:{name}")), name })
        .collect();
    presets.extend(find_presets(&bundled_dir(g), "bundled"));
    presets.extend(find_presets(&user_dir(g), "user"));
    let text = std::fs::read_to_string(get_home_directory(g) + FILE).unwrap_or_default();
    let (shader, integer_scale) = parse(&text, &presets);
    g.video = VideoSettings { presets, shader, integer_scale };
    let path = g.video.preset().map(|p| p.path.clone());
    let _ = services(g).video.set_shader(path.as_deref());
    services(g).video.set_integer_scale(integer_scale);
}

/// The menu index and integer scaling `text` saved; an unknown preset reads as Off.
fn parse(text: &str, presets: &[ShaderPreset]) -> (i16, bool) {
    let (mut shader, mut integer_scale) = (0, false);
    for line in text.lines() {
        match line.split_once('=') {
            Some(("shader", key)) => {
                shader = presets.iter().position(|p| p.key == key.trim()).map_or(0, |i| i as i16 + 1);
            }
            Some(("integer_scale", v)) => integer_scale = v.trim() == "1",
            _ => {}
        }
    }
    (shader, integer_scale)
}

fn save(g: &mut Game) {
    let shader = g.video.preset().map_or(String::new(), |p| p.key.clone());
    let text = format!("shader={shader}\ninteger_scale={}\n", g.video.integer_scale as u8);
    if std::fs::write(get_home_directory(g) + FILE, text).is_ok() {
        services(g).storage.persist();
    }
}

/// Switches the window to the menu's preset and saves it. A preset that fails to load reads as Off.
pub fn apply_shader(g: &mut Game) {
    let path = g.video.preset().map(|p| p.path.clone());
    if let Err(e) = services(g).video.set_shader(path.as_deref()) {
        eprintln!("[gfx] Shader preset unavailable: {e}");
        g.video.shader = 0;
    }
    save(g);
}

pub fn apply_integer_scale(g: &mut Game) {
    let on = g.video.integer_scale;
    services(g).video.set_integer_scale(on);
    save(g);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(key: &str) -> ShaderPreset {
        ShaderPreset { name: key.to_string(), key: key.to_string(), path: PathBuf::from(key) }
    }

    #[test]
    fn saved_settings_pick_their_preset() {
        let presets = [preset("bundled/a.slangp"), preset("user/b/c.slangp")];
        assert_eq!(parse("shader=user/b/c.slangp\ninteger_scale=1\n", &presets), (2, true));
        assert_eq!(parse("shader=bundled/a.slangp\n", &presets), (1, false));
        assert_eq!(parse("shader=gone.slangp\ninteger_scale=0\n", &presets), (0, false));
        assert_eq!(parse("", &presets), (0, false));
    }

    #[test]
    fn presets_are_found_below_their_folder() {
        let dir = std::env::temp_dir().join(format!("smw-presets-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("crt/deep")).unwrap();
        for f in ["Zeta.slangp", "crt/alpha.slangp", "crt/deep/beta.SLANGP", "crt/notes.txt"] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        let found = find_presets(&dir, "user");
        std::fs::remove_dir_all(&dir).unwrap();
        let names: Vec<_> = found.iter().map(|p| (p.name.as_str(), p.key.as_str())).collect();
        assert_eq!(names, [("alpha", "user/crt/alpha.slangp"), ("beta", "user/crt/deep/beta.SLANGP"), ("Zeta", "user/Zeta.slangp")]);
    }
}
