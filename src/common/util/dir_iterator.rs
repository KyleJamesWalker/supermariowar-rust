//! Port of src/common/util/DirIterator.cpp

use crate::common::linfunc::lowercase;
use std::fs::{DirEntry, ReadDir};
use std::path::{Path, PathBuf};

fn lowercase_ext(entry: &DirEntry) -> String {
    let ext = entry.path().extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    lowercase(ext)
}

/// Same `readdir` order as libc++ `std::filesystem::directory_iterator`.
pub struct DirIterator {
    m_iter: Option<ReadDir>,
}

impl DirIterator {
    fn new(path: &Path) -> Self {
        DirIterator { m_iter: std::fs::read_dir(path).ok() }
    }

    fn next_accepted(&mut self, is_accepted: impl Fn(&DirEntry) -> bool) -> Option<PathBuf> {
        let iter = self.m_iter.as_mut()?;
        for entry in iter.by_ref() {
            let Ok(entry) = entry else { continue };
            if is_accepted(&entry) {
                return Some(entry.path());
            }
        }
        None
    }
}

pub struct FilesIterator {
    base: DirIterator,
    m_extensions: Vec<String>,
}

impl FilesIterator {
    pub fn new(path: impl AsRef<Path>, extensions: Vec<String>) -> Self {
        FilesIterator { base: DirIterator::new(path.as_ref()), m_extensions: extensions }
    }

    pub fn next(&mut self) -> Option<PathBuf> {
        let exts = &self.m_extensions;
        self.base.next_accepted(|entry| exts.is_empty() || exts.contains(&lowercase_ext(entry)))
    }
}

pub struct SubdirsIterator {
    base: DirIterator,
}

impl SubdirsIterator {
    pub fn new(path: impl AsRef<Path>) -> Self {
        SubdirsIterator { base: DirIterator::new(path.as_ref()) }
    }

    pub fn next(&mut self) -> Option<PathBuf> {
        self.base.next_accepted(|entry| entry.path().is_dir())
    }
}
