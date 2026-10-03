//! Port of src/common/FileList.cpp

use crate::globals::Aliased;
use crate::common::file_io::cstr_bytes_to_string;
use crate::common::linfunc::tokenize;
use crate::common::path::{convert_path, strip_creator_and_ext, strip_path_and_extension};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::util::container_helpers;
use crate::common::util::dir_iterator::{FilesIterator, SubdirsIterator};
use crate::impl_base;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MAXCATEGORYTRACKS: usize = 64;

/// `std::getline` over a whole file; `None` when the file cannot be opened.
fn read_lines(path: &Path) -> Option<Vec<String>> {
    let bytes = std::fs::read(path).ok()?;
    let mut lines: Vec<String> = bytes.split(|&b| b == b'\n').map(cstr_bytes_to_string).collect();
    if bytes.last() == Some(&b'\n') || bytes.is_empty() {
        lines.pop();
    }
    Some(lines)
}

fn is_comment_line(line: &str) -> bool {
    match line.as_bytes().first() {
        None => true,
        Some(&c) => c == b'#' || c == b'\n' || c == b'\r' || c == b' ' || c == b'\t',
    }
}

fn filename_string(path: &Path) -> String {
    path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn update_music_with_overrides(musiclist: &mut MusicList, worldmusiclist: &mut WorldMusicList) {
    let mut mapmusicoverrides: Vec<MapMusicOverride> = Vec::new();
    let mut worldmusicoverrides: Vec<WorldMusicOverride> = Vec::new();

    #[derive(PartialEq)]
    enum Section {
        None,
        Maps,
        Worlds,
    }

    let Some(lines) = read_lines(Path::new(&convert_path("music/Overrides.txt"))) else {
        return;
    };

    let mut current_section = Section::None;
    for mut line in lines {
        if is_comment_line(&line) {
            continue;
        }

        if line.ends_with('\r') {
            line.pop();
        }

        if line.as_bytes()[0] == b'[' {
            line.make_ascii_lowercase();

            if line == "[maps]" {
                current_section = Section::Maps;
            } else if line == "[worlds]" {
                current_section = Section::Worlds;
            }

            continue;
        }

        if current_section == Section::None {
            continue;
        }

        let mut tokens = tokenize(&line, ',', usize::MAX);
        if tokens.is_empty() {
            continue;
        }

        if current_section == Section::Maps {
            let mut ovr = MapMusicOverride { mapname: tokens.remove(0).to_string(), songs: Vec::new() };

            for token in tokens {
                let path = PathBuf::from(convert_path(token));
                if path.exists() {
                    ovr.songs.push(path);
                }
            }
            if !ovr.songs.is_empty() {
                mapmusicoverrides.push(ovr);
            }
        } else if current_section == Section::Worlds {
            let worldname = tokens.remove(0).to_string();

            // C++ reads front() of an empty list here (UB) when the line has no comma.
            let path = PathBuf::from(convert_path(tokens.first().copied().unwrap_or("")));
            if path.exists() {
                worldmusicoverrides.push(WorldMusicOverride { worldname, song: path });
            }
        }
    }

    musiclist.update_entries_with_overrides(&mapmusicoverrides);
    worldmusiclist.update_entries_with_overrides(&worldmusicoverrides);
}

///////////// SimpleFileList ///////////////

#[derive(Clone, Debug)]
pub struct SimpleFileList {
    pub m_filelist: Vec<PathBuf>,
    pub m_index: usize,
    _alias: Aliased,
}

impl Default for SimpleFileList {
    fn default() -> Self {
        SimpleFileList { _alias: Aliased::new(), m_filelist: Vec::new(), m_index: usize::MAX }
    }
}

impl SimpleFileList {
    pub fn new(dirpath: impl AsRef<Path>, extension: &str, fAlphabetize: bool) -> Self {
        let dirpath = dirpath.as_ref();
        let mut s = SimpleFileList::default();

        let mut dir = FilesIterator::new(dirpath, vec![extension.to_string()]);
        while let Some(path) = dir.next() {
            s.m_filelist.push(path);
        }

        if s.m_filelist.is_empty() {
            println!("WARNING: The directory `{}` is empty", dirpath.display());
            s.m_index = usize::MAX;
            return s;
        }

        s.m_index = 0;

        if fAlphabetize {
            let mut names: Vec<String> = Vec::with_capacity(s.m_filelist.len());

            for filepath in &s.m_filelist {
                let mut name = strip_path_and_extension(&filename_string(filepath));
                name.make_ascii_lowercase();
                names.push(name);
            }

            let mut fDone = false;
            while !fDone {
                fDone = true;
                for i in 0..s.m_filelist.len() - 1 {
                    if names[i] > names[i + 1] {
                        fDone = false;
                        names.swap(i, i + 1);
                        s.m_filelist.swap(i, i + 1);
                    }
                }
            }
        }

        s
    }

    pub fn at(&self, index: usize) -> &PathBuf {
        &self.m_filelist[index]
    }
    pub fn count(&self) -> usize {
        self.m_filelist.len()
    }
    pub fn current_index(&self) -> usize {
        self.m_index
    }
    pub fn current_path(&self) -> &PathBuf {
        self.at(self.m_index)
    }

    pub fn set_current_index(&mut self, index: usize) {
        if index < self.m_filelist.len() {
            self.m_index = index;
        }
    }

    pub fn set_current_path(&mut self, name: &Path) {
        if let Some(pos) = self.m_filelist.iter().position(|p| p == name) {
            self.m_index = pos;
        }
    }

    pub fn next(&mut self) {
        if self.m_filelist.is_empty() {
            return;
        }

        self.m_index = self.m_index.wrapping_add(1);
        if self.m_index >= self.m_filelist.len() {
            self.m_index = 0;
        }
    }

    pub fn prev(&mut self) {
        if self.m_filelist.is_empty() {
            return;
        }

        self.m_index = self.m_index.wrapping_sub(1);
        if self.m_index >= self.m_filelist.len() {
            self.m_index = self.m_filelist.len() - 1;
        }
    }

    pub fn random(&mut self) {
        if !self.m_filelist.is_empty() {
            self.m_index = RANDOM_INT(self.m_filelist.len() as i32) as usize;
        }
    }

    pub fn add(&mut self, path: PathBuf) {
        self.m_filelist.push(path);
    }

    pub fn find(&mut self, name: &str) -> bool {
        if self.m_filelist.is_empty() {
            return false;
        }

        let start = self.m_index;
        loop {
            self.next();

            if filename_string(&self.m_filelist[self.m_index]).starts_with(name) {
                return true;
            }
            if self.m_index == start {
                break;
            }
        }

        false
    }
}

#[derive(Clone, Debug)]
pub struct SimpleDirectoryList {
    pub simple_file_list: SimpleFileList,
}
impl_base!(SimpleDirectoryList => simple_file_list: SimpleFileList);

impl SimpleDirectoryList {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let mut s = SimpleFileList::default();

        let mut dir = SubdirsIterator::new(path);
        while let Some(subdir) = dir.next() {
            s.m_filelist.push(subdir);
        }
        if s.m_filelist.is_empty() {
            println!("ERROR: Empty directory.  {}", path.display());
        }

        s.m_index = 0;
        SimpleDirectoryList { simple_file_list: s }
    }
}

macro_rules! file_list_subclass {
    ($name:ident, $base:ty, $field:ident, $ctor:expr) => {
        #[derive(Clone, Debug)]
        pub struct $name {
            pub $field: $base,
        }
        impl_base!($name => $field: $base);
        impl $name {
            pub fn new() -> Self {
                $name { $field: $ctor }
            }
        }
    };
}

file_list_subclass!(AnnouncerList, SimpleFileList, simple_file_list, SimpleFileList::new(convert_path("sfx/announcer/"), ".txt", false));
file_list_subclass!(GraphicsList, SimpleDirectoryList, simple_directory_list, SimpleDirectoryList::new(convert_path("gfx/packs/")));
file_list_subclass!(SoundsList, SimpleDirectoryList, simple_directory_list, SimpleDirectoryList::new(convert_path("sfx/packs/")));
file_list_subclass!(TourList, SimpleFileList, simple_file_list, SimpleFileList::new(convert_path("tours/"), ".txt", false));
file_list_subclass!(WorldList, SimpleFileList, simple_file_list, SimpleFileList::new(convert_path("worlds/"), ".txt", true));
file_list_subclass!(
    BackgroundList,
    SimpleFileList,
    simple_file_list,
    SimpleFileList::new(convert_path("gfx/packs/Classic/backgrounds/"), ".png", false)
);
file_list_subclass!(FiltersList, SimpleFileList, simple_file_list, SimpleFileList::new(convert_path("filters/"), ".txt", false));

///////////// SkinList ///////////////

#[derive(Clone, Debug)]
pub struct SkinListNode {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct SkinList {
    m_skins: Vec<SkinListNode>,
    _alias: Aliased,
}

impl SkinList {
    pub fn new() -> Self {
        let mut m_skins = Vec::new();
        let mut dir = FilesIterator::new(convert_path("gfx/skins/"), vec![".bmp".to_string(), ".png".to_string()]);
        while let Some(path) = dir.next() {
            m_skins.push(SkinListNode { name: strip_creator_and_ext(&filename_string(&path)), path });
        }
        container_helpers::sort(&mut m_skins, |lhs: &SkinListNode, rhs: &SkinListNode| lhs.name < rhs.name);
        SkinList { _alias: Aliased::new(), m_skins }
    }

    pub fn count(&self) -> usize {
        self.m_skins.len()
    }
    pub fn at(&self, idx: usize) -> &SkinListNode {
        &self.m_skins[idx]
    }
}

///////////// Music ///////////////

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum MusicCategory {
    Land,
    Underground,
    Underwater,
    Castle,
    Platforms,
    Ghost,
    Bonus,
    Battle,
    Desert,
    Clouds,
    Snow,
    COUNT,
}

impl MusicCategory {
    pub const ALL: [MusicCategory; 11] = [
        MusicCategory::Land,
        MusicCategory::Underground,
        MusicCategory::Underwater,
        MusicCategory::Castle,
        MusicCategory::Platforms,
        MusicCategory::Ghost,
        MusicCategory::Bonus,
        MusicCategory::Battle,
        MusicCategory::Desert,
        MusicCategory::Clouds,
        MusicCategory::Snow,
    ];
}

pub fn music_category_to_string(category: MusicCategory) -> &'static str {
    match category {
        MusicCategory::Land => "Land",
        MusicCategory::Underground => "Underground",
        MusicCategory::Underwater => "Underwater",
        MusicCategory::Castle => "Castle",
        MusicCategory::Platforms => "Platforms",
        MusicCategory::Ghost => "Ghost",
        MusicCategory::Bonus => "Bonus",
        MusicCategory::Battle => "Battle",
        MusicCategory::Desert => "Desert",
        MusicCategory::Clouds => "Clouds",
        MusicCategory::Snow => "Snow",
        MusicCategory::COUNT => "COUNT",
    }
}

#[derive(Clone, Debug, Default)]
pub struct MapMusicOverride {
    pub mapname: String,
    pub songs: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct MusicPack {
    m_name: String,
    m_currentMusic: usize,
    m_all_songs: Vec<PathBuf>,
    m_category_songs: HashMap<MusicCategory, Vec<usize>>,
    m_map_overrides: HashMap<String, Vec<usize>>,
    m_background_overrides: HashMap<String, Vec<usize>>,
}

impl MusicPack {
    pub fn new(name: String) -> Self {
        MusicPack {
            m_name: name,
            m_currentMusic: 0,
            m_all_songs: Vec::new(),
            m_category_songs: HashMap::new(),
            m_map_overrides: HashMap::new(),
            m_background_overrides: HashMap::new(),
        }
    }

    pub fn load(musicdirectory: &Path) -> Option<MusicPack> {
        let mut this = MusicPack::new(filename_string(musicdirectory));

        let file_path = musicdirectory.join("Music.txt");
        let Some(lines) = read_lines(&file_path) else {
            println!("Error: Could not open: {}", file_path.display());
            return None;
        };

        #[derive(Clone, Copy)]
        enum Section {
            Header,
            Category(MusicCategory),
            MapSpecific,
            BackgroundSpecific,
        }

        let mut current_section = Section::Header;

        for mut line in lines {
            if is_comment_line(&line) {
                continue;
            }

            if line.ends_with('\r') {
                line.pop();
            }

            if line.as_bytes()[0] == b'[' {
                line.make_ascii_lowercase();
                current_section = match line.as_str() {
                    "[land]" => Section::Category(MusicCategory::Land),
                    "[underground]" => Section::Category(MusicCategory::Underground),
                    "[underwater]" => Section::Category(MusicCategory::Underwater),
                    "[castle]" => Section::Category(MusicCategory::Castle),
                    "[platforms]" => Section::Category(MusicCategory::Platforms),
                    "[ghost]" => Section::Category(MusicCategory::Ghost),
                    "[bonus]" => Section::Category(MusicCategory::Bonus),
                    "[battle]" => Section::Category(MusicCategory::Battle),
                    "[desert]" => Section::Category(MusicCategory::Desert),
                    "[clouds]" => Section::Category(MusicCategory::Clouds),
                    "[snow]" => Section::Category(MusicCategory::Snow),
                    "[maps]" => Section::MapSpecific,
                    "[backgrounds]" => Section::BackgroundSpecific,
                    _ => current_section,
                };
                continue;
            }

            match current_section {
                Section::Header => {
                    let path = musicdirectory.join(&line);
                    if path.exists() {
                        this.m_all_songs.push(path);
                    }
                }
                Section::Category(category) => {
                    if this.m_category_songs.entry(category).or_default().len() >= MAXCATEGORYTRACKS {
                        continue;
                    }

                    let path = musicdirectory.join(&line);
                    if !path.exists() {
                        continue;
                    }

                    this.m_all_songs.push(path);
                    let idx = this.m_all_songs.len() - 1;
                    this.m_category_songs.entry(category).or_default().push(idx);
                }
                Section::MapSpecific | Section::BackgroundSpecific => {
                    let mut tokens = tokenize(&line, ',', usize::MAX);
                    if tokens.len() < 2 {
                        continue;
                    }

                    let mapname = tokens.remove(0).to_string();

                    let target = if matches!(current_section, Section::MapSpecific) {
                        &mut this.m_map_overrides
                    } else {
                        &mut this.m_background_overrides
                    };
                    let target_index_list = target.entry(mapname).or_default();

                    for filename in tokens {
                        let path = musicdirectory.join(filename);
                        if path.exists() {
                            this.m_all_songs.push(path);
                            target_index_list.push(this.m_all_songs.len() - 1);
                        }
                    }
                }
            }
        }

        for category in MusicCategory::ALL {
            let category_dir = musicdirectory.join(music_category_to_string(category));
            if !category_dir.is_dir() {
                continue;
            }

            let mut iter = FilesIterator::new(&category_dir, vec![".ogg".to_string()]);
            while let Some(path) = iter.next() {
                if this.m_category_songs.entry(category).or_default().len() < MAXCATEGORYTRACKS {
                    this.m_all_songs.push(path);
                    let idx = this.m_all_songs.len() - 1;
                    this.m_category_songs.entry(category).or_default().push(idx);
                }
            }
        }

        if this.m_all_songs.is_empty() {
            println!("Error: No songs found in: {}", musicdirectory.display());
            return None;
        }

        let get_fallback_category = |category: MusicCategory| -> Option<MusicCategory> {
            match category {
                MusicCategory::Platforms => Some(MusicCategory::Land),
                MusicCategory::Ghost => Some(MusicCategory::Underground),
                MusicCategory::Bonus => Some(MusicCategory::Underwater),
                MusicCategory::Battle => Some(MusicCategory::Castle),
                MusicCategory::Desert => Some(MusicCategory::Land),
                MusicCategory::Clouds => Some(MusicCategory::Land),
                MusicCategory::Snow => Some(MusicCategory::Land),
                _ => None,
            }
        };
        for category in MusicCategory::ALL {
            if this.m_category_songs.entry(category).or_default().is_empty() {
                if let Some(fallback) = get_fallback_category(category) {
                    let songs = this.m_category_songs.entry(fallback).or_default().clone();
                    this.m_category_songs.insert(category, songs);
                } else {
                    println!("Error: Missing track definition for music category: {}", music_category_to_string(category));
                    return None;
                }
            }
        }

        Some(this)
    }

    pub fn name(&self) -> &str {
        &self.m_name
    }

    pub fn music(&self, idx: usize) -> &PathBuf {
        let idx = idx.min(self.m_all_songs.len() - 1);
        &self.m_all_songs[idx]
    }

    pub fn random_music(&mut self, musicCategory: MusicCategory, mapName: &str, background: &str) -> PathBuf {
        if let Some(list) = self.m_map_overrides.get(mapName) {
            if !list.is_empty() {
                self.m_currentMusic = RANDOM_INT(list.len() as i32) as usize;
                return self.m_all_songs[list[self.m_currentMusic]].clone();
            }
        }

        if let Some(list) = self.m_background_overrides.get(background) {
            if !list.is_empty() {
                self.m_currentMusic = RANDOM_INT(list.len() as i32) as usize;
                return self.m_all_songs[list[self.m_currentMusic]].clone();
            }
        }

        let list = self.m_category_songs.entry(musicCategory).or_default();
        if !list.is_empty() {
            self.m_currentMusic = RANDOM_INT(list.len() as i32) as usize;
            return self.m_all_songs[list[self.m_currentMusic]].clone();
        }

        self.m_all_songs[4].clone()
    }

    pub fn next_music(&mut self, musicCategory: MusicCategory, mapName: &str, background: &str) -> PathBuf {
        if let Some(list) = self.m_map_overrides.get(mapName) {
            if !list.is_empty() {
                self.m_currentMusic += 1;
                if self.m_currentMusic >= list.len() {
                    self.m_currentMusic = 0;
                }
                return self.m_all_songs[list[self.m_currentMusic]].clone();
            }
        }
        if let Some(list) = self.m_background_overrides.get(background) {
            if !list.is_empty() {
                self.m_currentMusic += 1;
                if self.m_currentMusic >= list.len() {
                    self.m_currentMusic = 0;
                }
                return self.m_all_songs[list[self.m_currentMusic]].clone();
            }
        }
        let list = self.m_category_songs.entry(musicCategory).or_default();
        if !list.is_empty() {
            self.m_currentMusic += 1;
            if self.m_currentMusic >= list.len() {
                self.m_currentMusic = 0;
            }
            return self.m_all_songs[list[self.m_currentMusic]].clone();
        }

        self.m_all_songs[4].clone()
    }

    pub fn update_with_overrides(&mut self, overrides: &[MapMusicOverride]) {
        for ovr in overrides {
            let songlist = self.m_map_overrides.entry(ovr.mapname.clone()).or_default();
            for path in &ovr.songs {
                self.m_all_songs.push(path.clone());
                songlist.push(self.m_all_songs.len() - 1);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct MusicList {
    m_currentMusic: PathBuf,
    m_entries: Vec<MusicPack>,
    m_currentIndex: usize,
    _alias: Aliased,
}

impl MusicList {
    pub fn new() -> Self {
        let mut m_entries = Vec::new();
        let mut dir = SubdirsIterator::new(convert_path("music/game/"));
        while let Some(path) = dir.next() {
            if let Some(pack) = MusicPack::load(&path) {
                m_entries.push(pack);
            }
        }

        if m_entries.is_empty() {
            std::panic::panic_any("Could not load any map music!");
        }

        MusicList { _alias: Aliased::new(), m_currentMusic: PathBuf::new(), m_entries, m_currentIndex: 0 }
    }

    pub fn music(&self, musicID: usize) -> &PathBuf {
        self.m_entries[self.m_currentIndex].music(musicID)
    }
    pub fn current_music(&self) -> &PathBuf {
        &self.m_currentMusic
    }

    pub fn set_random_music(&mut self, category: MusicCategory, mapName: &str, background: &str) {
        self.m_currentMusic = self.m_entries[self.m_currentIndex].random_music(category, mapName, background);
    }

    pub fn set_next_music(&mut self, category: MusicCategory, mapName: &str, background: &str) {
        self.m_currentMusic = self.m_entries[self.m_currentIndex].next_music(category, mapName, background);
    }

    pub fn current_index(&self) -> usize {
        self.m_currentIndex
    }
    pub fn set_current(&mut self, index: usize) {
        self.m_currentIndex = if index < self.m_entries.len() { index } else { 0 };
    }
    pub fn current_name(&self) -> &str {
        self.m_entries[self.m_currentIndex].name()
    }

    pub fn next(&mut self) {
        self.m_currentIndex = (self.m_currentIndex + 1) % self.m_entries.len();
    }

    pub fn prev(&mut self) {
        self.m_currentIndex = (if self.m_currentIndex == 0 { self.m_entries.len() } else { self.m_currentIndex }) - 1;
    }

    pub fn random(&mut self) {
        self.m_currentIndex = RANDOM_INT(self.m_entries.len() as i32) as usize;
    }

    pub fn update_entries_with_overrides(&mut self, overrides: &[MapMusicOverride]) {
        for entry in &mut self.m_entries {
            entry.update_with_overrides(overrides);
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum WorldMusicCategory {
    Grass,
    Desert,
    Water,
    Giant,
    Sky,
    Ice,
    Pipe,
    Dark,
    Space,
    Bonus,
    Sleep,
    COUNT,
}

impl WorldMusicCategory {
    /// `++category`: wraps past the last category back to `Grass`.
    pub fn pre_increment(&mut self) -> WorldMusicCategory {
        let value = (*self as u8 + 1) % WorldMusicCategory::COUNT as u8;
        *self = unsafe { std::mem::transmute::<u8, WorldMusicCategory>(value) };
        *self
    }

    /// `category++`
    pub fn post_increment(&mut self) -> WorldMusicCategory {
        let temp = *self;
        self.pre_increment();
        temp
    }
}

pub fn world_music_category_to_string(category: WorldMusicCategory) -> &'static str {
    match category {
        WorldMusicCategory::Grass => "Grass",
        WorldMusicCategory::Desert => "Desert",
        WorldMusicCategory::Water => "Water",
        WorldMusicCategory::Giant => "Giant",
        WorldMusicCategory::Sky => "Sky",
        WorldMusicCategory::Ice => "Ice",
        WorldMusicCategory::Pipe => "Pipe",
        WorldMusicCategory::Dark => "Dark",
        WorldMusicCategory::Space => "Space",
        WorldMusicCategory::Bonus => "Bonus",
        WorldMusicCategory::Sleep => "Sleep",
        WorldMusicCategory::COUNT => "COUNT",
    }
}

#[derive(Clone, Debug, Default)]
pub struct WorldMusicOverride {
    pub worldname: String,
    pub song: PathBuf,
}

#[derive(Clone, Debug)]
pub struct WorldMusicPack {
    m_name: String,
    m_world_overrides: HashMap<String, PathBuf>,
    m_category_song: HashMap<WorldMusicCategory, PathBuf>,
}

impl WorldMusicPack {
    pub fn new(name: String) -> Self {
        WorldMusicPack { m_name: name, m_world_overrides: HashMap::new(), m_category_song: HashMap::new() }
    }

    pub fn load(musicdirectory: &Path) -> Option<WorldMusicPack> {
        let mut this = WorldMusicPack::new(filename_string(musicdirectory));

        let file_path = musicdirectory.join("Music.txt");
        let Some(lines) = read_lines(&file_path) else {
            println!("Error: Could not open: {}", file_path.display());
            return None;
        };

        #[derive(Clone, Copy)]
        enum Section {
            Header,
            Category(WorldMusicCategory),
            WorldSpecific,
        }

        let mut current_section = Section::Header;

        for mut line in lines {
            if is_comment_line(&line) {
                continue;
            }

            if line.ends_with('\r') {
                line.pop();
            }

            if line.as_bytes()[0] == b'[' {
                line.make_ascii_lowercase();
                current_section = match line.as_str() {
                    "[grass]" => Section::Category(WorldMusicCategory::Grass),
                    "[desert]" => Section::Category(WorldMusicCategory::Desert),
                    "[water]" => Section::Category(WorldMusicCategory::Water),
                    "[giant]" => Section::Category(WorldMusicCategory::Giant),
                    "[sky]" => Section::Category(WorldMusicCategory::Sky),
                    "[ice]" => Section::Category(WorldMusicCategory::Ice),
                    "[pipe]" => Section::Category(WorldMusicCategory::Pipe),
                    "[dark]" => Section::Category(WorldMusicCategory::Dark),
                    "[space]" => Section::Category(WorldMusicCategory::Space),
                    "[bonus]" => Section::Category(WorldMusicCategory::Bonus),
                    "[sleep]" => Section::Category(WorldMusicCategory::Sleep),
                    "[worlds]" => Section::WorldSpecific,
                    _ => current_section,
                };
                continue;
            }

            match current_section {
                Section::Header => {}
                Section::Category(category) => {
                    let path = musicdirectory.join(&line);
                    if path.exists() {
                        this.m_category_song.insert(category, path);
                    }
                }
                Section::WorldSpecific => {
                    let tokens = tokenize(&line, ',', usize::MAX);
                    if tokens.len() < 2 {
                        continue;
                    }

                    let worldname = tokens[0].to_string();
                    let path = musicdirectory.join(tokens[1]);
                    if !path.exists() {
                        continue;
                    }

                    this.m_world_overrides.insert(worldname, path);
                }
            }
        }

        Some(this)
    }

    pub fn music(&self, category: WorldMusicCategory, worldName: &str) -> &PathBuf {
        if let Some(p) = self.m_world_overrides.get(worldName) {
            return p;
        }

        if let Some(p) = self.m_category_song.get(&category) {
            return p;
        }
        &self.m_category_song[&WorldMusicCategory::Grass]
    }

    pub fn name(&self) -> &str {
        &self.m_name
    }

    pub fn update_with_overrides(&mut self, overrides: &[WorldMusicOverride]) {
        for ovr in overrides {
            self.m_world_overrides.insert(ovr.worldname.clone(), ovr.song.clone());
        }
    }
}

#[derive(Clone, Debug)]
pub struct WorldMusicList {
    m_entries: Vec<WorldMusicPack>,
    m_currentIndex: usize,
    _alias: Aliased,
}

impl WorldMusicList {
    pub fn new() -> Self {
        let mut m_entries = Vec::new();
        let mut dir = SubdirsIterator::new(convert_path("music/world/"));
        while let Some(path) = dir.next() {
            if let Some(pack) = WorldMusicPack::load(&path) {
                m_entries.push(pack);
            }
        }
        if m_entries.is_empty() {
            std::panic::panic_any("Could not load any world music!");
        }
        WorldMusicList { _alias: Aliased::new(), m_entries, m_currentIndex: 0 }
    }

    pub fn current_index(&self) -> usize {
        self.m_currentIndex
    }
    pub fn set_current(&mut self, index: usize) {
        self.m_currentIndex = if index < self.m_entries.len() { index } else { 0 };
    }
    pub fn current_name(&self) -> &str {
        self.m_entries[self.m_currentIndex].name()
    }
    pub fn current_music(&self, category: WorldMusicCategory, worldName: &str) -> &PathBuf {
        self.m_entries[self.m_currentIndex].music(category, worldName)
    }
    pub fn count(&self) -> usize {
        self.m_entries.len()
    }

    pub fn next(&mut self) {
        self.m_currentIndex = (self.m_currentIndex + 1) % self.m_entries.len();
    }

    pub fn prev(&mut self) {
        self.m_currentIndex = (if self.m_currentIndex == 0 { self.m_entries.len() } else { self.m_currentIndex }) - 1;
    }

    pub fn random(&mut self) {
        self.m_currentIndex = RANDOM_INT(self.m_entries.len() as i32) as usize;
    }

    pub fn update_entries_with_overrides(&mut self, overrides: &[WorldMusicOverride]) {
        for entry in &mut self.m_entries {
            entry.update_with_overrides(overrides);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::global::RootDataDirectory;

    fn set_data_root() {
        unsafe {
            RootDataDirectory = concat!(env!("CARGO_MANIFEST_DIR"), "/data").to_string();
        }
    }

    #[test]
    fn lists_load_from_data_dir() {
        set_data_root();

        let gfx = GraphicsList::new();
        assert!(gfx.count() >= 1);
        assert!(gfx.m_filelist.iter().any(|p| p.ends_with("Classic")));

        let sounds = SoundsList::new();
        assert!(sounds.count() >= 1);

        // data/sfx/announcer/ has only subdirectories when None.txt is missing from the checkout.
        let ann = AnnouncerList::new();
        assert_eq!(ann.current_index(), if ann.count() == 0 { usize::MAX } else { 0 });

        let tours = TourList::new();
        assert!(tours.count() >= 1);

        let worlds = WorldList::new();
        assert!(worlds.count() >= 1);
        let names: Vec<String> =
            worlds.m_filelist.iter().map(|p| strip_path_and_extension(&filename_string(p)).to_ascii_lowercase()).collect();
        assert!(names.windows(2).all(|w| w[0] <= w[1]), "worlds not alphabetized: {names:?}");

        let filters = FiltersList::new();
        assert!(filters.count() >= 1);

        let skins = SkinList::new();
        assert!(skins.count() >= 1);
        assert!((1..skins.count()).all(|i| skins.at(i - 1).name <= skins.at(i).name));

        let mut music = MusicList::new();
        assert!(!music.current_name().is_empty());
        let mut worldmusic = WorldMusicList::new();
        assert!(worldmusic.count() >= 1);
        update_music_with_overrides(&mut music, &mut worldmusic);
        assert!(music.music(0).exists());
        assert!(worldmusic.current_music(WorldMusicCategory::Grass, "nonexistent").exists());
    }

    #[test]
    fn simple_file_list_navigation() {
        let mut l = SimpleFileList::default();
        assert!(!l.find("x"));
        l.add(PathBuf::from("a/foo.txt"));
        l.add(PathBuf::from("a/bar.txt"));
        l.m_index = 0;
        l.prev();
        assert_eq!(l.current_index(), 1);
        l.next();
        assert_eq!(l.current_index(), 0);
        assert!(l.find("bar"));
        assert_eq!(l.current_index(), 1);
        assert!(l.find("bar"));
        assert!(!l.find("zzz"));
        l.set_current_path(Path::new("a/foo.txt"));
        assert_eq!(l.current_index(), 0);
    }
}
