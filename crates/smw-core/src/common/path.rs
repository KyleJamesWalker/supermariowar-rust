//! Path helpers: the settings and data directories, pack path resolution, and names derived from file names.

use crate::game::Game;
use std::path::Path;


/// The settings directory, where options and controls files live.
pub fn get_home_directory(g: &mut Game) -> String {
    let mut gp = g.ptr();
    crate::services::services(gp.get()).storage.settings_dir().to_string_lossy().into_owned()
}

/// The directory that holds `data`.
pub fn get_root_directory(g: &mut Game) -> String {
    let mut gp = g.ptr();
    crate::services::services(gp.get()).storage.root_dir().to_string_lossy().into_owned()
}

pub fn file_exists(path: &str) -> bool {
    std::fs::metadata(path).is_ok()
}

/// Defaults `SMW_Root_Data_Dir` to `./` when it is empty.
pub fn initialize_paths(g: &mut Game) {
    if !g.SMW_Root_Data_Dir.is_empty() {
        return;
    }
    g.SMW_Root_Data_Dir = "./".to_string();
    println!("Located data folder at: {}", g.SMW_Root_Data_Dir);
}


/// `source`, relative to the data directory, as a path under it.
pub fn convert_path(g: &mut Game, source: &str) -> String {
    if !g.are_paths_initialized {
        g.SMW_Root_Data_Dir = g.RootDataDirectory.clone();

        initialize_paths(g);

        let last_slash_pos = g.SMW_Root_Data_Dir.rfind('/');
        if last_slash_pos != Some(g.SMW_Root_Data_Dir.len().wrapping_sub(1)) {
            g.SMW_Root_Data_Dir.push('/');
        }

        g.are_paths_initialized = true;
    }
    g.SMW_Root_Data_Dir.clone() + source
}

/// A `gfx/packs/` or `sfx/packs/` path resolved in `packdir`, falling back to the Classic pack when it is missing there.
pub fn convert_path_pack(g: &mut Game, relpath: &str, packdir: &str) -> String {
    const prefixes: [&str; 2] = ["gfx/packs/", "sfx/packs/"];
    for prefix in prefixes {
        let Some(rel) = relpath.strip_prefix(prefix) else {
            continue;
        };

        let path = Path::new(packdir).join(rel);
        if path.exists() {
            return path.to_string_lossy().into_owned();
        }

        return convert_path(g, &(prefix.to_string() + "Classic/" + rel));
    }
    convert_path(g, relpath)
}

pub fn concat(a: &str, b: &str) -> String {
    a.to_string() + b
}

pub const fn dir_separator() -> char {
    '/'
}

/// The part of `path` after its last `/`.
pub fn get_filename_from_path(path: &str) -> String {
    match path.rfind(dir_separator()) {
        Some(pos) => path[pos + 1..].to_string(),
        None => path.to_string(),
    }
}

/// The file name without directory and extension; with `strip_author`, also without the `author_` prefix.
pub fn get_name_from_file_name(path: &str, strip_author: bool) -> String {
    const PATH_SEPARATOR: char = '/';

    let mut left_pos = 0usize;
    let mut right_pos = path.len();

    if let Some(sep_pos) = path.rfind(PATH_SEPARATOR) {
        left_pos = sep_pos + 1;
    }

    if strip_author {
        if let Some(off) = path[left_pos..].find('_') {
            left_pos = left_pos + off + 1;
        }
    }

    if let Some(dot_pos) = path[..right_pos].rfind('.') {
        if left_pos < dot_pos {
            right_pos = dot_pos;
        }
    }

    assert!(right_pos >= left_pos);
    path[left_pos..right_pos].to_string()
}

/// A file name without its `creator_` prefix and 4-character extension, first letter uppercased.
pub fn strip_creator_and_ext(filename: &str) -> String {
    let first_underscore = match filename.find('_') {
        None => 0,
        Some(p) => p + 1,
    };

    let without_prefix = &filename[first_underscore..];
    let mut without_prefix = substr_len(without_prefix, 0, without_prefix.len().wrapping_sub(4));

    if !without_prefix.is_empty() {
        let first = without_prefix.as_bytes()[0].to_ascii_uppercase() as char;
        without_prefix.replace_range(0..1, &first.to_string());
    }

    without_prefix
}

/// `path` from just after its first `_`, or else its last `/`, without a 4-character extension.
pub fn strip_path_and_extension(path: &str) -> String {
    let chop_here = match path.find('_') {
        None => match path.rfind(dir_separator()) {
            None => 0,
            Some(p) => p + 1,
        },
        Some(p) => p + 1,
    };

    let without_path = &path[chop_here..];
    substr_len(without_path, 0, without_path.len().wrapping_sub(4))
}

/// A substring of `n` bytes from `pos`; an `n` past the end takes the rest.
pub fn substr_len(s: &str, pos: usize, n: usize) -> String {
    let end = if n > s.len() - pos { s.len() } else { pos + n };
    s[pos..end].to_string()
}
