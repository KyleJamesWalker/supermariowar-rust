//! The list of map files, the current selection, map filters, and the map summary cache.

use crate::game::Game;
use crate::common::file_list::SimpleDirectoryList;
use crate::common::file_io::throw_runtime_error;
use crate::common::global::*;
use crate::common::global_constants::NUM_AUTO_FILTERS;
use crate::common::linfunc::lowercase;
use crate::common::map::read_type_summary;
use crate::common::path::{convert_path, strip_creator_and_ext};
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::util::dir_iterator::FilesIterator;
use crate::common::version::GAME_VERSION;
use crate::globals::Aliased;
use std::io::{BufRead, Write};

/// One map file in the list.
pub struct MapListNode {
    /// Per filter (the `NUM_AUTO_FILTERS` automatic ones, then the user filter files), whether the map is in it.
    pub pfFilters: Vec<bool>,
    /// The map file's full path.
    pub filename: String,

    /// Position in the whole list, and in the filtered list.
    pub iIndex: i16,
    pub iFilteredIndex: i16,

    /// Passes the filters last applied.
    pub fInCurrentFilterSet: bool,
    /// Its automatic filters came from mapsummary.txt rather than from loading the map.
    pub fReadFromCache: bool,
    pub fValid: bool,
}

impl MapListNode {
    pub fn new(g: &mut Game, fullName: String) -> Self {
        let count = g.filterslist.count();
        MapListNode {
            pfFilters: vec![false; NUM_AUTO_FILTERS as usize + count],
            filename: fullName,
            iIndex: 0,
            iFilteredIndex: 0,
            fInCurrentFilterSet: true,
            fReadFromCache: false,
            fValid: true,
        }
    }
}

/// Map entries sorted by key (bytewise), equal keys in insertion order.
/// Positions are indices; `len()` is the past-the-end position.
#[derive(Default)]
pub struct MapMultimap {
    entries: Vec<(String, MapListNode)>,
}

impl MapMultimap {
    /// Inserts after any entries with an equal key; returns the new entry's position.
    pub fn emplace(&mut self, key: String, node: MapListNode) -> usize {
        let pos = self.entries.partition_point(|(k, _)| k.as_bytes() <= key.as_bytes());
        self.entries.insert(pos, (key, node));
        pos
    }

    /// The first entry with this key, or `len()`.
    pub fn find(&self, key: &str) -> usize {
        let pos = self.entries.partition_point(|(k, _)| k.as_bytes() < key.as_bytes());
        if pos < self.entries.len() && self.entries[pos].0 == key {
            pos
        } else {
            self.entries.len()
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn key(&self, it: usize) -> &String {
        &self.entries[it].0
    }
    pub fn node(&self, it: usize) -> &MapListNode {
        &self.entries[it].1
    }
    pub fn node_mut(&mut self, it: usize) -> &mut MapListNode {
        &mut self.entries[it].1
    }
}

fn add_maps_from(g: &mut Game, dirpath: &str, container: &mut MapMultimap) {
    add_maps_from_with(g, dirpath, |key, node| {
        container.emplace(key, node);
    });
}

/// Adds every .map file in `dirpath`, keyed by its display name.
fn add_maps_from_with(g: &mut Game, dirpath: &str, mut emplace: impl FnMut(String, MapListNode)) {
    let mut dir = FilesIterator::new(g, dirpath.to_string(), vec![".map".to_string()]);
    while let Some(path) = dir.next() {
        let node = MapListNode::new(g, path.to_string_lossy().into_owned());
        let fname = path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        emplace(strip_creator_and_ext(&fname), node);
    }
}

/// Which container `outercurrent` points into.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OuterIter {
    Maps(usize),
    WorldMaps(usize),
}

/// The maps the menus pick from, a current position in them, and the world-only maps.
pub struct MapList {
    /// The maps in maps/, plus tour, world and special maps for the world editor.
    pub maps: MapMultimap,
    /// Tour, world and special maps, which tour and world stages look up by name.
    pub worldmaps: MapMultimap,

    current: usize,
    savedcurrent: usize,
    outercurrent: OuterIter,

    /// Positions in `maps` by filtered index and by index, filled by `apply_filters`.
    mlnFilteredMaps: Vec<usize>,
    mlnMaps: Vec<usize>,

    iFilteredMapCount: usize,

    pub _alias: Aliased,
}

impl MapList {
    /// Lists the maps from the data directory; panics if there are none.
    pub fn new(g: &mut Game, fWorldEditor: bool) -> Self {
        let mut gp = g.ptr();
        let mut maps = MapMultimap::default();
        add_maps_from(g, &convert_path(gp.get(), "maps/"), &mut maps);

        if fWorldEditor {
            add_maps_from(g, &convert_path(gp.get(), "maps/tour/"), &mut maps);

            let worlds = convert_path(g, "worlds/");
            let mut worldeditormapdirs = SimpleDirectoryList::new(g, worlds);
            for _iDir in 0..worldeditormapdirs.count() {
                let szName = worldeditormapdirs.current_path().to_string_lossy().into_owned() + "/";
                add_maps_from(g, &szName, &mut maps);
                worldeditormapdirs.next();
            }

            add_maps_from(g, &convert_path(gp.get(), "maps/special/"), &mut maps);
        }

        if maps.is_empty() {
            println!("ERROR: Empty map directory!");
            throw_runtime_error("ERROR: Empty map directory!");
        }

        let mut iIndex: i16 = 0;
        for current in 0..maps.len() {
            maps.node_mut(current).iIndex = iIndex;
            iIndex += 1;
        }

        let n = maps.len();
        let mut worldmaps = MapMultimap::default();

        add_maps_from(g, &convert_path(gp.get(), "maps/tour/"), &mut worldmaps);

        let worlds = convert_path(g, "worlds/");
        let mut worldmapdirs = SimpleDirectoryList::new(g, worlds);
        for _iDir in 0..worldmapdirs.count() {
            let szName = worldmapdirs.current_path().to_string_lossy().into_owned() + "/";
            add_maps_from(g, &szName, &mut worldmaps);
            worldmapdirs.next();
        }

        add_maps_from(g, &convert_path(gp.get(), "maps/special/"), &mut worldmaps);

        MapList {
            maps,
            worldmaps,
            current: 0,
            savedcurrent: 0,
            outercurrent: OuterIter::Maps(0),
            mlnFilteredMaps: vec![n; n],
            mlnMaps: vec![n; n],
            iFilteredMapCount: n,
            _alias: Aliased::new(),
        }
    }

    /// Adds every world's maps to `maps`.
    pub fn add_world_maps(&mut self, g: &mut Game) {
        let worlds = convert_path(g, "worlds/");
        let mut worldmapdirs = SimpleDirectoryList::new(g, worlds);
        for _iDir in 0..worldmapdirs.count() {
            let szName = worldmapdirs.current_path().to_string_lossy().into_owned() + "/";
            add_maps_from_with(g, &szName, |key, node| self.emplace_map(key, node));
            worldmapdirs.next();
        }
    }

    /// Adds a map just saved to maps/ under `name`.
    pub fn add(&mut self, g: &mut Game, name: &str) {
        let fullName = convert_path(g, "maps/") + name;
        let node = MapListNode::new(g, fullName);
        self.emplace_map(strip_creator_and_ext(name), node);
    }

    /// Inserting into `maps` after construction moves every stored position at or after the insertion point,
    /// so each keeps naming the same map.
    fn emplace_map(&mut self, key: String, node: MapListNode) {
        let pos = self.maps.emplace(key, node);
        let reseat = |it: &mut usize| {
            if *it >= pos {
                *it += 1;
            }
        };
        reseat(&mut self.current);
        reseat(&mut self.savedcurrent);
        if let OuterIter::Maps(it) = &mut self.outercurrent {
            reseat(it);
        }
        self.mlnFilteredMaps.iter_mut().for_each(reseat);
        self.mlnMaps.iter_mut().for_each(reseat);
    }

    /// Moves to the next map whose path contains `name`, searching from the one after the current; false if none.
    pub fn find(&mut self, name: &str) -> bool {
        let mut fFound = false;

        let oldCurrent = self.current;
        loop {
            self.next(false);

            if self.maps.node(self.current).filename.contains(name) {
                fFound = true;
            }
            if !(self.current != oldCurrent && !fFound) {
                break;
            }
        }

        fFound
    }

    /// Selects the map whose name is `name` (ignoring ASCII case); with `fWorld`, the world-only maps come first.
    pub fn findexact(&mut self, name: &str, fWorld: bool) -> bool {
        let szLookForName = lowercase(name.to_string());

        let mut fFound = false;

        if fWorld {
            let mut iterateAll = 0usize;
            let lim = self.worldmaps.len();

            while iterateAll != lim && !fFound {
                let szCurrentName = lowercase(self.worldmaps.key(iterateAll).clone());

                if szCurrentName == szLookForName {
                    fFound = true;
                    self.outercurrent = OuterIter::WorldMaps(iterateAll);
                }

                iterateAll += 1;
            }

            if fFound {
                return true;
            }
        }

        let oldCurrent = self.current;

        fFound = false;
        loop {
            self.next(false);

            let szCurrentName = lowercase(self.maps.key(self.current).clone());

            if szCurrentName == szLookForName {
                fFound = true;
            }
            if !(self.current != oldCurrent && !fFound) {
                break;
            }
        }

        fFound
    }

    /// Moves to the next filtered map if the current one is filtered out; true when it moved.
    pub fn find_filtered_map(&mut self) -> bool {
        if self.maps.node(self.current).fInCurrentFilterSet {
            return false;
        }

        self.next(true);
        true
    }

    /// Moves to the next filtered map whose name starts with ASCII letter `letter`, any case.
    pub fn startswith_char(&mut self, mut letter: u8) -> bool {
        if letter.is_ascii_lowercase() {
            letter -= 32;
        }

        let oldCurrent = self.current;
        loop {
            self.next(true);

            if self.current_shortmapname().as_bytes().first().copied().unwrap_or(0) == letter {
                return true;
            }
            if self.current == oldCurrent {
                break;
            }
        }

        false
    }

    /// Moves to the next filtered map whose name starts with `match`, ignoring ASCII case.
    pub fn startswith(&mut self, r#match: &str) -> bool {
        let oldCurrent = self.current;
        loop {
            self.next(true);

            let szMapName = self.current_shortmapname().as_bytes().to_vec();
            let m = r#match.as_bytes();
            if m.len() <= szMapName.len() {
                let matches = (0..m.len()).all(|i| szMapName[i].to_ascii_lowercase() == m[i].to_ascii_lowercase());
                if matches {
                    return true;
                }
            }
            if self.current == oldCurrent {
                break;
            }
        }

        false
    }

    fn outer_node(&self) -> (&String, &MapListNode) {
        match self.outercurrent {
            OuterIter::Maps(i) => (self.maps.key(i), self.maps.node(i)),
            OuterIter::WorldMaps(i) => (self.worldmaps.key(i), self.worldmaps.node(i)),
        }
    }

    /// The selected map's full path.
    pub fn current_filename(&self) -> &str {
        &self.outer_node().1.filename
    }

    /// The selected map's display name.
    pub fn current_shortmapname(&self) -> &str {
        self.outer_node().0
    }

    /// Steps to the previous map, wrapping; with `fUseFilters`, the previous one in the filtered set.
    pub fn prev(&mut self, fUseFilters: bool) {
        if fUseFilters {
            let oldCurrent = self.current;

            loop {
                self.prev(false);

                if self.maps.node(self.current).fInCurrentFilterSet {
                    return;
                }
                if self.current == oldCurrent {
                    break;
                }
            }
        } else {
            if self.current == 0 {
                self.current = self.maps.len();
            }

            self.current -= 1;
            self.outercurrent = OuterIter::Maps(self.current);
        }
    }

    /// Steps to the next map, wrapping; with `fUseFilters`, the next one in the filtered set.
    pub fn next(&mut self, fUseFilters: bool) {
        if fUseFilters {
            let oldCurrent = self.current;

            loop {
                self.next(false);

                if self.maps.node(self.current).fInCurrentFilterSet {
                    return;
                }
                if self.current == oldCurrent {
                    break;
                }
            }
        } else {
            self.current += 1;

            if self.current == self.maps.len() {
                self.current = 0;
            }

            self.outercurrent = OuterIter::Maps(self.current);
        }
    }

    /// Steps forward a random number of maps; with filters, does nothing if fewer than two pass.
    pub fn random(&mut self, g: &mut Game, fUseFilters: bool) {
        let mut gp = g.ptr();
        let iShuffle: i32;
        if fUseFilters {
            if self.iFilteredMapCount < 2 {
                return;
            }

            iShuffle = RANDOM_INT(gp.get(), (self.iFilteredMapCount - 1) as i32);
        } else {
            iShuffle = RANDOM_INT(gp.get(), self.maps.len().wrapping_sub(1) as i32);
        }

        for _i in 0..=iShuffle {
            self.next(fUseFilters);
        }
    }

    /// A random map's full path from the whole list; leaves the selection alone.
    pub fn random_filename(&self, g: &mut Game) -> String {
        let mut gp = g.ptr();
        let iRand: i16 = RANDOM_INT(gp.get(), self.maps.len() as i32) as i16;

        let rnd = iRand.max(0) as usize;

        self.maps.node(rnd).filename.clone()
    }

    pub fn set_valid(&mut self, fValid: bool) {
        self.maps.node_mut(self.current).fValid = fValid;
    }

    pub fn is_valid(&self) -> bool {
        self.maps.node(self.current).fValid
    }

    pub fn is_empty(&self) -> bool {
        self.maps.is_empty()
    }

    pub fn filtered_count(&self) -> usize {
        self.iFilteredMapCount
    }

    pub fn count(&self) -> usize {
        self.maps.len()
    }

    pub fn get_current(&self) -> usize {
        self.current
    }

    pub fn set_current(&mut self, itr: usize) {
        self.current = itr;
        self.outercurrent = OuterIter::Maps(itr);
    }

    /// The map node at a position from `get_current` or `get_iterator_at`.
    pub fn node_at(&self, itr: usize) -> &MapListNode {
        self.maps.node(itr)
    }

    pub fn node_at_mut(&mut self, itr: usize) -> &mut MapListNode {
        self.maps.node_mut(itr)
    }

    /// The display name at a position from `get_current` or `get_iterator_at`.
    pub fn key_at(&self, itr: usize) -> &str {
        self.maps.key(itr)
    }

    /// The past-the-end position.
    pub fn end(&self) -> usize {
        self.maps.len()
    }

    /// Writes each user filter file, if any filter changed.
    pub fn write_filters(&mut self, g: &mut Game) {
        if g.game_values.fNeedWriteFilters {
            g.game_values.fNeedWriteFilters = false;

            for iFilter in 0..g.filterslist.count() {
                let path = g.filterslist.at(iFilter).clone();
                let Ok(mut fp) = std::fs::File::create(&path) else {
                    continue;
                };

                let mut out = String::new();
                out += "#Version\n";
                out += &format!("{}.{}.{}.{}\n\n", GAME_VERSION.major, GAME_VERSION.minor, GAME_VERSION.patch, GAME_VERSION.build);

                out += "#Icon\n";
                out += &format!("{}\n\n", g.game_values.piFilterIcons[iFilter + NUM_AUTO_FILTERS as usize]);

                out += "#Maps\n";

                for itr in 0..self.maps.len() {
                    if self.maps.node(itr).pfFilters[iFilter + NUM_AUTO_FILTERS as usize] {
                        out += &format!("{}\n", self.maps.key(itr));
                    }
                }

                let _ = fp.write_all(out.as_bytes());
                drop(fp);

                #[cfg(target_os = "macos")]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o774));
                }
            }
        }
    }

    /// Loads each map's automatic filters (from the cache, else from the map) and the user filter files.
    pub fn read_filters(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.current = 0;

        if let Ok(f) = std::fs::File::open(convert_path(g, "maps/cache/mapsummary.txt")) {
            for line in fgets_lines(f) {
                let mut toks = strtok(&line, b",\n");
                let Some(pszMapName) = toks.next() else { continue };
                let it = self.maps.find(&pszMapName);

                if it != self.maps.len() {
                    let mut fErrorReading = false;
                    let mut values = [false; NUM_AUTO_FILTERS as usize];
                    let mut nread = 0;
                    for iFilter in 0..NUM_AUTO_FILTERS as usize {
                        match toks.next() {
                            Some(psz) => {
                                values[iFilter] = psz != "0";
                                nread = iFilter + 1;
                            }
                            None => {
                                fErrorReading = true;
                                break;
                            }
                        }
                    }

                    let node = self.maps.node_mut(it);
                    node.pfFilters[..nread].copy_from_slice(&values[..nread]);

                    if !fErrorReading {
                        node.fReadFromCache = true;
                    }
                }
            }
        }

        while self.current != self.maps.len() {
            if !self.maps.node(self.current).fReadFromCache {
                let filename = self.maps.node(self.current).filename.clone();
                gp.g_map.load_map(g, &filename, read_type_summary);
                let af = g.g_map.fAutoFilter;
                let mln = self.maps.node_mut(self.current);
                mln.pfFilters[..af.len()].copy_from_slice(&af);
            }

            self.current += 1;
        }

        self.current = 0;
        for iFilter in 0..g.filterslist.count() {
            let Ok(f) = std::fs::File::open(g.filterslist.at(iFilter)) else {
                continue;
            };

            let mut iReadState: i16 = 0;
            for buffer in fgets_lines(f) {
                let b0 = buffer.as_bytes().first().copied().unwrap_or(0);
                if b0 == b'#' || b0 == b'\n' || b0 == b'\r' || b0 == b' ' || b0 == b'\t' {
                    continue;
                }

                if iReadState == 0 {
                    iReadState = 1;
                    continue;
                } else if iReadState == 1 {
                    g.game_values.piFilterIcons[iFilter + NUM_AUTO_FILTERS as usize] = atoi(&buffer) as i16;
                    iReadState = 2;
                    continue;
                } else {
                    let pszMap = strtok(&buffer, b"\r\n").next().unwrap_or_default();

                    if self.findexact(&pszMap, false) {
                        self.maps.node_mut(self.current).pfFilters[iFilter + NUM_AUTO_FILTERS as usize] = true;
                    }
                }
            }
        }

        self.current = 0;
        self.outercurrent = OuterIter::Maps(self.current);
    }

    /// Recomputes every map's automatic filters by loading it.
    pub fn reload_map_auto_filters(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        for itr in 0..self.maps.len() {
            let filename = self.maps.node(itr).filename.clone();
            gp.g_map.load_map(g, &filename, read_type_summary);
            let af = g.g_map.fAutoFilter;
            self.maps.node_mut(itr).pfFilters[..af.len()].copy_from_slice(&af);
        }
    }

    /// Writes every map's automatic filters to maps/cache/mapsummary.txt.
    pub fn write_map_summary_cache(&mut self, g: &mut Game) {
        let path = convert_path(g, "maps/cache/mapsummary.txt");
        let Ok(mut fp) = std::fs::File::create(&path) else {
            return;
        };

        let mut out = String::new();
        for itr in 0..self.maps.len() {
            out += self.maps.key(itr);

            for iFilter in 0..NUM_AUTO_FILTERS as usize {
                out += if self.maps.node(itr).pfFilters[iFilter] { ",1" } else { ",0" };
            }

            out += "\n";
        }
        let _ = fp.write_all(out.as_bytes());
        drop(fp);

        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o774));
        }
    }

    /// Marks the maps that are in every filter turned on in `pfFilters`, and moves off a map that is not.
    pub fn apply_filters(&mut self, g: &mut Game, pfFilters: &[bool]) {
        self.iFilteredMapCount = 0;
        let mut iTotalCount: i16 = 0;
        for itr in 0..self.maps.len() {
            let mut fMatched = true;
            for iFilter in 0..pfFilters.len() {
                if pfFilters[iFilter] && !self.maps.node(itr).pfFilters[iFilter] {
                    fMatched = false;
                    break;
                }
            }

            self.maps.node_mut(itr).fInCurrentFilterSet = fMatched;

            if fMatched {
                self.maps.node_mut(itr).iFilteredIndex = self.iFilteredMapCount as i16;
                self.mlnFilteredMaps[self.iFilteredMapCount] = itr;
                self.iFilteredMapCount += 1;
            }

            self.mlnMaps[iTotalCount as usize] = itr;
            iTotalCount += 1;
        }

        g.game_values.fFiltersOn = pfFilters.contains(&true);
        self.find_filtered_map();
    }

    pub fn map_in_filtered_set(&self) -> bool {
        self.maps.node(self.current).fInCurrentFilterSet
    }

    /// The position of map `iIndex` in the filtered or whole list, or `end()`.
    pub fn get_iterator_at(&self, iIndex: u16, fUseFilters: bool) -> usize {
        if fUseFilters {
            if iIndex as usize >= self.iFilteredMapCount {
                return self.maps.len();
            }

            self.mlnFilteredMaps[iIndex as usize]
        } else {
            if iIndex as usize >= self.maps.len() {
                return self.maps.len();
            }

            self.mlnMaps[iIndex as usize]
        }
    }

    pub fn get_filter(&self, iFilter: usize) -> bool {
        self.maps.node(self.current).pfFilters[iFilter]
    }

    pub fn toggle_filter(&mut self, iFilter: usize) {
        let f = &mut self.maps.node_mut(self.current).pfFilters[iFilter];
        *f = !*f;
    }

    /// Remembers the current position for `resume_current`.
    pub fn save_current(&mut self) {
        self.savedcurrent = self.current;
    }

    pub fn resume_current(&mut self) {
        self.current = self.savedcurrent;
    }
}

/// Splits a file the way repeated reads of a 256-byte line buffer do: chunks end after `'\n'` or 255 bytes.
pub fn fgets_lines(f: std::fs::File) -> Vec<String> {
    let mut reader = std::io::BufReader::new(f);
    let mut bytes = Vec::new();
    let _ = std::io::Read::read_to_end(&mut reader, &mut bytes);

    let mut out = Vec::new();
    let mut start = 0;
    while start < bytes.len() {
        let mut end = start;
        while end < bytes.len() && end - start < 255 {
            end += 1;
            if bytes[end - 1] == b'\n' {
                break;
            }
        }
        out.push(crate::common::file_io::cstr_bytes_to_string(&bytes[start..end]));
        start = end;
    }
    let _ = reader.fill_buf();
    out
}

/// Non-empty tokens of `s` separated by any of `delims`.
pub fn strtok<'a>(s: &'a str, delims: &'a [u8]) -> impl Iterator<Item = String> + 'a {
    s.split(move |c: char| c.is_ascii() && delims.contains(&(c as u8))).filter(|t| !t.is_empty()).map(|t| t.to_string())
}

/// C `atoi`: optional whitespace and sign, then digits; 0 when there are none. Overflow wraps.
pub fn atoi(s: &str) -> i32 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || (b'\t'..=b'\r').contains(&b[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut v: i32 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((b[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}
