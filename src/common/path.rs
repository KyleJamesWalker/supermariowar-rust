//! Port of src/common/path.cpp

use crate::common::global::RootDataDirectory;
use std::path::Path;

pub static mut SMW_Root_Data_Dir: String = String::new();

#[cfg(not(windows))]
pub fn get_home_directory() -> String {
    let mut result = String::from("/Library/Preferences/.smw/");
    if let Ok(folder) = std::env::var("HOME") {
        result = folder + &result;
    }
    result
}

/// `SHGetFolderPathA(CSIDL_PROFILE)` is the profile directory that `USERPROFILE` names.
#[cfg(windows)]
pub fn get_home_directory() -> String {
    let mut result = String::from(".smw/");
    if let Ok(folder) = std::env::var("USERPROFILE") {
        result = folder + "/" + &result;
    }
    result
}

pub fn get_root_directory() -> String {
    #[cfg(not(windows))]
    if let Ok(p) = sdl2::filesystem::base_path() {
        return p;
    }
    "./".to_string()
}

pub fn file_exists(path: &str) -> bool {
    std::fs::metadata(path).is_ok()
}

pub fn initialize_paths() {
    unsafe {
        if !SMW_Root_Data_Dir.is_empty() {
            return;
        }
        SMW_Root_Data_Dir = "./".to_string();
        println!("Located data folder at: {}", SMW_Root_Data_Dir);
    }
}

static mut are_paths_initialized: bool = false;

pub fn convert_path(source: &str) -> String {
    unsafe {
        if !are_paths_initialized {
            SMW_Root_Data_Dir = RootDataDirectory.clone();

            initialize_paths();

            let last_slash_pos = SMW_Root_Data_Dir.rfind('/');
            if last_slash_pos != Some(SMW_Root_Data_Dir.len().wrapping_sub(1)) {
                SMW_Root_Data_Dir.push('/');
            }

            are_paths_initialized = true;
        }
        SMW_Root_Data_Dir.clone() + source
    }
}

pub fn convert_path_pack(relpath: &str, packdir: &str) -> String {
    const prefixes: [&str; 2] = ["gfx/packs/", "sfx/packs/"];
    for prefix in prefixes {
        let Some(rel) = relpath.strip_prefix(prefix) else {
            continue;
        };

        let path = Path::new(packdir).join(rel);
        if path.exists() {
            return path.to_string_lossy().into_owned();
        }

        return convert_path(&(prefix.to_string() + "Classic/" + rel));
    }
    convert_path(relpath)
}

pub fn concat(a: &str, b: &str) -> String {
    a.to_string() + b
}

pub const fn dir_separator() -> char {
    '/'
}

pub fn get_filename_from_path(path: &str) -> String {
    match path.rfind(dir_separator()) {
        Some(pos) => path[pos + 1..].to_string(),
        None => path.to_string(),
    }
}

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

/// `std::string::substr(pos, n)`: `n` is clamped to the remaining length (so `npos`-like values take the rest).
pub fn substr_len(s: &str, pos: usize, n: usize) -> String {
    let end = if n > s.len() - pos { s.len() } else { pos + n };
    s[pos..end].to_string()
}
