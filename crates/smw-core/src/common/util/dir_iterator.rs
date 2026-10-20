//! Iterators over a directory's files or subdirectories, through the platform storage service.

use crate::common::linfunc::lowercase;
use crate::game::Game;
use std::path::{Path, PathBuf};

fn lowercase_ext(path: &Path) -> String {
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    lowercase(ext)
}

/// A directory's entries in `Storage::list_dir` order, the order a case-insensitive APFS volume lists them.
pub struct DirIterator {
    m_entries: std::vec::IntoIter<PathBuf>,
}

impl DirIterator {
    fn new(g: &mut Game, path: &Path) -> Self {
        DirIterator { m_entries: crate::services::services(g).storage.list_dir(path).into_iter() }
    }

    fn next_accepted(&mut self, is_accepted: impl Fn(&Path) -> bool) -> Option<PathBuf> {
        self.m_entries.by_ref().find(|path| is_accepted(path))
    }
}

/// The files in a directory whose lowercased extension (with its dot) is in `extensions`, or every entry when empty.
pub struct FilesIterator {
    base: DirIterator,
    m_extensions: Vec<String>,
}

impl FilesIterator {
    pub fn new(g: &mut Game, path: impl AsRef<Path>, extensions: Vec<String>) -> Self {
        FilesIterator { base: DirIterator::new(g, path.as_ref()), m_extensions: extensions }
    }

    pub fn next(&mut self) -> Option<PathBuf> {
        let exts = &self.m_extensions;
        self.base.next_accepted(|path| exts.is_empty() || exts.contains(&lowercase_ext(path)))
    }
}

/// The subdirectories of a directory.
pub struct SubdirsIterator {
    base: DirIterator,
}

impl SubdirsIterator {
    pub fn new(g: &mut Game, path: impl AsRef<Path>) -> Self {
        SubdirsIterator { base: DirIterator::new(g, path.as_ref()) }
    }

    pub fn next(&mut self) -> Option<PathBuf> {
        self.base.next_accepted(|path| path.is_dir())
    }
}
