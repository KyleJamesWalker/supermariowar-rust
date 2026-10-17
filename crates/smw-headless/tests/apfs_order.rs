//! The headless backend lists data/ as tools/apfs_order.py orders it, and on macOS as APFS itself does.

use smw_platform::Storage;
use std::path::{Path, PathBuf};

fn dirs(dir: &Path, out: &mut Vec<PathBuf>) {
    out.push(dir.to_path_buf());
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        if entry.file_type().unwrap().is_dir() && entry.file_name() != ".git" {
            dirs(&entry.path(), out);
        }
    }
}

/// Keys from tools/apfs_order.py.
#[test]
fn matches_apfs_order_py() {
    for (name, key) in [("Classic", 0x4981_cc08), ("Retro", 0xb554_f006), ("tilesets", 0xe7c1_1409), ("Mario.png", 0xf002_5c0a), ("smw.tour.txt", 0xacb6_2c0d), ("0smw.bmp", 0xcc5d_d809)] {
        assert_eq!(smw_headless::apfs_key(name), key, "{}", name);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn matches_apfs() {
    let mut all = Vec::new();
    dirs(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"), &mut all);
    let storage = smw_headless::HeadlessStorage { settings_dir: PathBuf::new(), root_dir: PathBuf::new() };
    for dir in all {
        let fs: Vec<PathBuf> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).collect();
        assert_eq!(storage.list_dir(&dir), fs, "{}", dir.display());
    }
}
