//! Match checkpoints and segment replays (docs/guides/REPLAY.md, "Markers, checkpoints and clips").
//!
//! When a recorded match starts, `capture` saves everything the match reads from earlier frames: the
//! settings, the menu's choices, the RNG, the virtual mixer and the files the match loaded. A segment
//! replay loads the game data, `restore`s the checkpoint and enters gameplay where the menu would have,
//! so its dump from that frame on equals the full replay's.
#![deny(clippy::all)]

use crate::game::Game;
use crate::common::game_values::AppState;
use crate::common::gameplay_styles::StarStyle;
use crate::common::version::GAME_VERSION;
use crate::common::match_types::{Boss, MatchType, Minigame};
use crate::common::path::convert_path;
use crate::common::random_number_generator::RandomNumberGenerator;
use crate::common::sfx::{self, checkpoint as mixer, data_relative, sfxMusic, sfxSound};
use crate::common::world_tour_stop::{parse_tour_stop_line, write_tour_stop_line};
use crate::globals::*;
use crate::smw::game_state::GameState;
use crate::smw::gs_gameplay::{lookup_team_id, GameplayState};
use crate::smw::gs_menu::{enter_gameplay_tail, MenuState};
use std::path::Path;

/// The first bytes of every checkpoint, followed by its version byte.
const MAGIC: &[u8; 4] = b"SMWC";
/// Version of an offline checkpoint (`capture`).
const VERSION: u8 = 1;
/// Netplay v2's match start: the map file, then version 1's fields. Each peer keeps its own net state.
const SHARED_VERSION: u8 = 3;
/// Where a peer keeps the map of a shared checkpoint, in its settings directory.
const SHARED_MAP: &str = "net_v2.map";

/// One routine both saves and loads each value, so the two directions cannot drift apart.
pub struct Snap {
    /// Reading `buf` back rather than writing it.
    load: bool,
    buf: Vec<u8>,
    /// Read position in `buf`.
    pos: usize,
    /// A read ran past the end or met an impossible length, or the map in it did not load.
    damaged: bool,
    /// Loading another peer's match start, whose map name this machine must not trust.
    shared: bool,
    /// Why a shared match start was refused.
    refused: Option<String>,
}

/// A value a `Snap` saves and loads in place.
pub trait Field {
    fn io(&mut self, s: &mut Snap);
}

impl Snap {
    fn saver() -> Self {
        Snap { load: false, buf: Vec::new(), pos: 0, damaged: false, shared: false, refused: None }
    }

    fn loader(bytes: Vec<u8>) -> Self {
        Snap { load: true, buf: bytes, pos: 0, damaged: false, shared: false, refused: None }
    }

    /// Saves `v`, or overwrites it with the next value when loading.
    pub fn io<T: Field + ?Sized>(&mut self, v: &mut T) {
        v.io(self);
    }

    pub fn loading(&self) -> bool {
        self.load
    }

    /// The next `n` bytes; past the end it marks the snap damaged and returns zeros.
    fn take(&mut self, n: usize) -> &[u8] {
        if self.pos + n > self.buf.len() {
            self.damaged = true;
            self.pos = self.buf.len();
            return &[0; 8][..n.min(8)];
        }
        self.pos += n;
        &self.buf[self.pos - n..self.pos]
    }

    fn put(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }
}

/// Numbers as their little-endian bytes.
macro_rules! number_field {
    ($($t:ty),*) => {$(
        impl Field for $t {
            fn io(&mut self, s: &mut Snap) {
                if s.load {
                    let mut b = [0u8; std::mem::size_of::<$t>()];
                    let src = s.take(b.len());
                    b[..src.len()].copy_from_slice(src);
                    *self = <$t>::from_le_bytes(b);
                } else {
                    s.put(&self.to_le_bytes());
                }
            }
        }
    )*};
}
number_field!(u8, i16, u16, i32, u32, u64, f32);

macro_rules! enum_field {
    ($($t:ty),*) => {$(
        impl Field for $t {
            fn io(&mut self, s: &mut Snap) {
                let mut v = *self as u8;
                v.io(s);
                *self = <$t>::from_u8(v);
            }
        }
    )*};
}
enum_field!(AppState, MatchType, Minigame, Boss);

impl Field for bool {
    fn io(&mut self, s: &mut Snap) {
        let mut v = *self as u8;
        v.io(s);
        *self = v != 0;
    }
}

impl Field for usize {
    fn io(&mut self, s: &mut Snap) {
        let mut v = *self as u64;
        v.io(s);
        *self = v as usize;
    }
}

impl Field for String {
    fn io(&mut self, s: &mut Snap) {
        let mut bytes = self.as_bytes().to_vec();
        bytes.io(s);
        *self = String::from_utf8_lossy(&bytes).into_owned();
    }
}

impl<T: Field, const N: usize> Field for [T; N] {
    fn io(&mut self, s: &mut Snap) {
        for v in self.iter_mut() {
            v.io(s);
        }
    }
}

impl<T: Field + Default> Field for Vec<T> {
    fn io(&mut self, s: &mut Snap) {
        let mut n = self.len() as u32;
        n.io(s);
        if s.load {
            // Every element takes at least a byte, so a longer count means the checkpoint is damaged.
            if n as usize > s.buf.len() - s.pos {
                s.damaged = true;
                n = 0;
            }
            self.clear();
            self.resize_with(n as usize, T::default);
        }
        for v in self.iter_mut() {
            v.io(s);
        }
    }
}

impl<T: Field + Default> Field for Option<T> {
    fn io(&mut self, s: &mut Snap) {
        let mut some = self.is_some();
        some.io(s);
        if s.load {
            *self = some.then(T::default);
        }
        if let Some(v) = self {
            v.io(s);
        }
    }
}

impl<T: Field + Default> Field for std::collections::VecDeque<T> {
    fn io(&mut self, s: &mut Snap) {
        let mut v: Vec<T> = std::mem::take(self).into();
        v.io(s);
        *self = v.into();
    }
}

impl<A: Field, B: Field> Field for (A, B) {
    fn io(&mut self, s: &mut Snap) {
        self.0.io(s);
        self.1.io(s);
    }
}

impl<A: Field, B: Field, C: Field> Field for (A, B, C) {
    fn io(&mut self, s: &mut Snap) {
        self.0.io(s);
        self.1.io(s);
        self.2.io(s);
    }
}

impl<A: Field, B: Field, C: Field, D: Field> Field for (A, B, C, D) {
    fn io(&mut self, s: &mut Snap) {
        self.0.io(s);
        self.1.io(s);
        self.2.io(s);
        self.3.io(s);
    }
}

/// Starts a checkpoint's path in the settings directory, which differs between machines.
const HOME_PREFIX: &str = "~settings/";

/// A path for a checkpoint: one in the settings directory relative to it.
pub fn portable_path(g: &mut Game, path: &str) -> String {
    match path.strip_prefix(crate::common::path::get_home_directory(g).as_str()) {
        Some(rest) => format!("{}{}", HOME_PREFIX, rest),
        None => path.to_string(),
    }
}

/// The inverse of `portable_path`: a `~settings/` path in this machine's settings directory.
pub fn local_path(g: &mut Game, path: &str) -> String {
    match path.strip_prefix(HOME_PREFIX) {
        Some(rest) => crate::common::path::get_home_directory(g) + rest,
        None => path.to_string(),
    }
}

/// Remembers `path`, relative to the data directory, as the map the match is played on.
pub fn note_match_map(g: &mut Game, path: &str) {
    g.match_map = data_relative(path);
}

/// A data-relative name as a full path; empty and absolute names stay as they are.
fn data_path(g: &mut Game, name: &str) -> String {
    if name.is_empty() || name.starts_with('/') {
        name.to_string()
    } else {
        convert_path(g, name)
    }
}

/// A map name another peer's match start may use: data-relative or in the settings directory, never absolute or
/// climbing out with `..`.
fn shared_name_ok(name: &str) -> bool {
    let rest = name.strip_prefix(HOME_PREFIX).unwrap_or(name);
    !rest.starts_with(['/', '\\', '~']) && !rest.contains(':') && !rest.split(['/', '\\']).any(|part| part == "..")
}

/// Loads a sound or track a checkpoint names; one this machine lacks plays as silence of the same length.
fn load_audio<T: Default>(g: &mut Game, name: &str, load: fn(&mut Game, &Path) -> T) -> T {
    if name.is_empty() {
        return T::default();
    }
    let path = data_path(g, name);
    load(g, Path::new(&path))
}

/// Matches a checkpoint cannot start: a World's bonus house, which no replay covers yet.
pub fn unsupported_reason(g: &mut Game) -> Option<&'static str> {
    let mut gp = g.ptr();
    let g = gp;
    let stop = g.game_values.tourstops.get(g.game_values.tourstopcurrent);
    if g.game_values.matchtype == MatchType::World && stop.is_some_and(|s| s.iStageType == 1) {
        return Some("bonus_house");
    }
    None
}

/// The checkpoint at the current point of `MenuState::enter_gameplay`.
pub fn capture(g: &mut Game) -> Vec<u8> {
    let mut s = Snap::saver();
    s.put(MAGIC);
    s.put(&[VERSION]);
    walk(g, &mut s);
    s.buf
}

/// The match start every netplay v2 peer restores, so all their simulations begin alike: an offline checkpoint plus
/// the map file, which a peer may not have.
pub fn capture_shared(g: &mut Game) -> Result<Vec<u8>, String> {
    let mut s = Snap::saver();
    s.put(MAGIC);
    s.put(&[SHARED_VERSION]);
    let source = data_path(g, &g.match_map.clone());
    let mut map = std::fs::read(&source).map_err(|e| format!("cannot read the map {source}: {e}"))?;
    if map.is_empty() {
        return Err(format!("the map {source} is empty"));
    }
    s.io(&mut map);
    let mut frame = crate::smw::harness::frame(g);
    s.io(&mut frame);
    let shared = crate::common::path::get_home_directory(g) + SHARED_MAP;
    let played = std::mem::replace(&mut g.match_map, shared);
    walk(g, &mut s);
    g.match_map = played;
    Ok(s.buf)
}

/// Restores `capture_shared`'s bytes and starts the match, leaving this peer's netplay state as it is.
pub fn restore_shared(g: &mut Game, bytes: Vec<u8>) -> Result<(), String> {
    if bytes.len() < 5 || &bytes[..4] != MAGIC || bytes[4] != SHARED_VERSION {
        return Err("not a netplay v2 match start".to_string());
    }
    let mut s = Snap::loader(bytes);
    s.pos = 5;
    s.shared = true;
    let mut map: Vec<u8> = Vec::new();
    s.io(&mut map);
    let mut frame = 0u32;
    s.io(&mut frame);
    crate::smw::harness::set_frame(g, frame);
    let path = crate::common::path::get_home_directory(g) + SHARED_MAP;
    std::fs::write(&path, &map).map_err(|e| format!("cannot write {path}: {e}"))?;
    walk(g, &mut s);
    if let Some(why) = s.refused {
        return Err(why);
    }
    if s.damaged || s.pos != s.buf.len() {
        return Err("the match start is damaged".to_string());
    }
    enter_gameplay_tail(g);
    Ok(())
}

/// Restores a checkpoint after `load_game_data` and the menu's init, then starts the match.
pub fn restore(g: &mut Game, bytes: Vec<u8>) -> Result<(), String> {
    if bytes.len() < 5 || &bytes[..4] != MAGIC {
        return Err("not a checkpoint".to_string());
    }
    if bytes[4] != VERSION {
        return Err(format!("checkpoint version {} (this build reads {})", bytes[4], VERSION));
    }
    let mut s = Snap::loader(bytes);
    s.pos = 5;
    walk(g, &mut s);
    if s.damaged || s.pos != s.buf.len() {
        return Err("checkpoint is damaged".to_string());
    }
    enter_gameplay_tail(g);
    Ok(())
}

/// Saves or restores every part of the checkpoint, in the order a restore must apply them.
fn walk(g: &mut Game, s: &mut Snap) {
        let mut gp = g.ptr();
    unsafe {
        let gv = &mut *gp.get().game_values;
        // Settings, as options.bin and controls.sdl2.bin would hold them now.
        let (mut options, mut controls) = if s.load { (Vec::new(), Vec::new()) } else { gv.config_bytes(g) };
        s.io(&mut options);
        s.io(&mut controls);
        if s.load {
            gv.read_config_bytes(g, options, controls);
        }

        // Which input configuration each player reads, as `inputConfiguration` index * 2 + keyboard/joystick.
        let mut inputs = [0u8; 4];
        for (input, control) in inputs.iter_mut().zip(gv.playerInput.inputControls) {
            if let Some((set, kind)) = control {
                *input = set * 2 + kind;
            }
        }
        s.io(&mut inputs);
        if s.load {
            for (p, &input) in inputs.iter().enumerate() {
                let i = input as usize % 8;
                gv.cgame_config.playerInput.inputControls[p] = Some(((i / 2) as u8, (i % 2) as u8));
            }
        }
        s.io(gv.playerInput.joy_directions_mut());

        s.io(&mut gv.showfps);
        s.io(&mut gv.frameadvance);
        s.io(&mut gv.autokill);
        s.io(&mut gv.appstate);
        s.io(&mut gv.screenfade);
        s.io(&mut gv.screenfadespeed);
        s.io(&mut gv.matchtype);
        s.io(&mut gv.teamids);
        s.io(&mut gv.teamcounts);
        s.io(&mut gv.tournamentgames);
        s.io(&mut gv.tournamentwinner);
        s.io(&mut gv.tournamentcontrolteam);
        s.io(&mut gv.tournamentnextcontrol);
        s.io(&mut gv.selectedminigame);
        s.io(&mut gv.tourindex);
        s.io(&mut gv.tourstopcurrent);
        s.io(&mut gv.worldindex);
        s.io(&mut gv.storedpowerups);
        s.io(&mut gv.gamepowerups);
        s.io(&mut gv.powerupweights);
        s.io(&mut gv.worldpowerups);
        s.io(&mut gv.worldpowerupcount);
        s.io(&mut gv.worldpointsbonus);
        s.io(&mut gv.colorids);
        s.io(&mut gv.bulletbilltimer);
        s.io(&mut gv.bulletbillspawntimer);
        s.io(&mut gv.loadedannouncer);
        s.io(&mut gv.loadedmusic);
        s.io(&mut gv.cputurn);
        for t in gv.tournament_scores.iter_mut() {
            s.io(&mut t.wins);
            s.io(&mut t.type_);
            s.io(&mut t.total);
        }
        let mut settings = gv.gamemodesettings.to_raw_bytes();
        s.io(&mut settings);
        gv.gamemodesettings.from_raw_bytes(&settings);
        s.io(&mut gv.soundcapable);
        s.io(&mut gv.singleplayermode);
        s.io(&mut gv.worldskipscoreboard);
        s.io(&mut gv.windaffectsplayers);
        s.io(&mut gv.spinscreen);
        s.io(&mut gv.reversewalk);
        s.io(&mut gv.spotlights);
        s.io(&mut gv.unlocksecret1part1);
        s.io(&mut gv.unlocksecret1part2);
        s.io(&mut gv.unlocksecret2part1);
        s.io(&mut gv.unlocksecret2part2);
        s.io(&mut gv.unlocksecret3part1);
        s.io(&mut gv.unlocksecret3part2);
        s.io(&mut gv.unlocksecretunlocked);

        
        use crate::common::game_mode::*;
        s.io(&mut g.currentgamemode);
        let mut mode = g.mode().gamemode;
        s.io(&mut mode);
        let mut boss = g.bossgamemode.get_boss_type();
        s.io(&mut boss);
        s.io(&mut g.bonushousemode.goal);
        s.io(&mut g.pipegamemode.goal);
        s.io(&mut g.bossgamemode.goal);
        s.io(&mut g.boxesgamemode.goal);
        if s.load {
            g.bossgamemode.set_boss_type(boss);
            gv.gamemode = match mode {
                game_mode_bonus => Some(crate::common::game_mode::ModeId::BonusHouse),
                game_mode_pipe_minigame => Some(crate::common::game_mode::ModeId::Pipe),
                game_mode_boss_minigame => Some(crate::common::game_mode::ModeId::Boss),
                game_mode_boxes_minigame => Some(crate::common::game_mode::ModeId::Boxes),
                m => Some(crate::common::game_mode::ModeId::Standard((m as usize).min(GAMEMODE_LAST as usize - 1) as u8)),
            };
        }

        let world = gv.matchtype == MatchType::World;
        let mut stops: Vec<String> = gv.tourstops.iter().map(|ts| write_tour_stop_line(ts, world)).collect();
        s.io(&mut stops);
        if s.load {
            gv.tourstops.clear();
            for line in stops.iter() {
                gv.tourstops.push(Box::new(parse_tour_stop_line(g, line.as_bytes(), &GAME_VERSION, world)));
            }
        }

        for k in 0..4 {
            let mut sc = g.score[k].as_ptr();
            s.io(&mut sc.score);
            s.io(&mut sc.subscore);
            s.io(&mut sc.x);
            s.io(&mut sc.y);
            s.io(&mut sc.destx);
            s.io(&mut sc.desty);
            s.io(&mut sc.place);
            s.io(&mut sc.displayorder);
            s.io(&mut sc.order);
            s.io(&mut sc.fromx);
            s.io(&mut sc.fromy);
            s.io(&mut sc.iDigitRight);
            s.io(&mut sc.iDigitMiddle);
            s.io(&mut sc.iDigitLeft);
        }
        s.io(&mut g.score_cnt);
        GameplayState::instance(g).checkpoint(s);

        // The map and the music the menu chose. A net game's joiner plays the host's map from its settings directory.
        let mut map = portable_path(g, &gp.match_map);
        let mut map_index = g.maplist.get_current();
        s.io(&mut map);
        s.io(&mut map_index);
        if s.shared && !shared_name_ok(&map) {
            s.refused.get_or_insert(format!("the match start names the map {map}"));
        }
        map = local_path(g, &map);
        if s.load && s.refused.is_none() {
            g.maplist.set_current(map_index);
            let path = data_path(g, &map);
            if !gp.g_map.load_map(g, &path, crate::common::map::read_type_full) {
                s.damaged = true;
            }
            note_match_map(g, &map);
            crate::common::global::load_current_map_background(g);
        }
        let mut musiclist = g.musiclist;
        let (song, song_index) = musiclist.song_state_mut();
        let mut song_name = data_relative(&song.to_string_lossy());
        s.io(&mut song_name);
        s.io(song_index);
        if s.load {
            *song = data_path(g, &song_name).into();
            for k in 0..4 {
                if gv.playercontrol[k] > 0 {
                    let strip = match crate::smw::net::net_skin_path(g, k) {
                        Some(path) => gp.rm.load_full_skin_path(g, Path::new(&path), k as i16),
                        None => gp.get().rm.load_full_skin(g, gv.skinids[k], gv.colorids[k]),
                    };
                    gp.rm.set_strip(gp.rm.spr_player[k], strip);
                }
            }
        }

        let tracks: Vec<*mut sfxMusic> = g.backgroundmusic.iter_mut().map(|t| t as *mut sfxMusic).collect();
        for &t in tracks.iter() {
            let mut name = (*t).name().to_string();
            let mut paused = (*t).paused();
            s.io(&mut name);
            s.io(&mut paused);
            if s.load {
                if (*t).name() != name {
                    *t = load_audio(gp.get(), &name, sfxMusic::from_file_or_silence);
                }
                (*t).set_paused(paused);
            }
        }
        let sounds = g.rm.sounds();
        for &snd in sounds.iter() {
            let mut name = (*snd).name().to_string();
            let (mut channels, mut last_start) = (*snd).state(gp.get());
            s.io(&mut name);
            s.io(&mut channels);
            s.io(&mut last_start);
            if s.load {
                if (*snd).name() != name {
                    *snd = load_audio(gp.get(), &name, sfxSound::from_file_or_silence);
                }
                (*snd).set_state(gp.get(), channels, last_start);
            }
        }

        // The virtual mixer, with sounds and tracks as indices into the lists above.
        let index_of = |list: &[*mut sfxSound], p: *mut sfxSound| list.iter().position(|&x| x == p).map(|i| i as i32).unwrap_or(-1);
        let (channels, music, mut resume) = mixer::save(g, &sounds, &tracks);
        let mut slots: Vec<(i32, i32, bool, u32)> = channels.iter().map(|c| (index_of(&sounds, c.sound), index_of(&sounds, c.owner), c.forever, c.end)).collect();
        let mut track = tracks.iter().position(|&t| t == music.track).map(|i| i as i32).unwrap_or(-1);
        let (mut forever, mut paused, mut end, mut remaining) = (music.forever, music.paused, music.end, music.remaining);
        s.io(&mut slots);
        s.io(&mut track);
        s.io(&mut forever);
        s.io(&mut paused);
        s.io(&mut end);
        s.io(&mut remaining);
        s.io(&mut resume);
        if s.load {
            let at = |i: i32| if i >= 0 && (i as usize) < sounds.len() { sounds[i as usize] } else { std::ptr::null_mut() };
            let channels: Vec<mixer::Channel> = slots.iter().map(|&(sound, owner, forever, end)| mixer::Channel { sound: at(sound), owner: at(owner), forever, end }).collect();
            let track = if track >= 0 && (track as usize) < tracks.len() { tracks[track as usize] } else { std::ptr::null_mut() };
            mixer::restore(g, &channels, &mixer::Music { track, forever, paused, end, remaining }, resume);
        }

        let rng = crate::common::random_number_generator::RandomNumberGenerator::generator(gp.get());
        let (state, index) = rng.state_mut();
        s.io(state);
        s.io(index);
        let mut calls = RandomNumberGenerator::call_count(gp.get());
        let mut last = RandomNumberGenerator::last_value(gp.get());
        s.io(&mut calls);
        s.io(&mut last);
        if s.load {
            RandomNumberGenerator::set_counters(gp.get(), calls, last);
        }

        // Sound commands of this frame so far, which belong to its dump block.
        s.io(&mut g.sfx.sfx_events);
    }
}

/// `key=value`, quoted when the value has a space.
fn kv(key: &str, value: &str) -> String {
    let value = value.replace('"', "'");
    if value.is_empty() || value.contains(' ') {
        format!("{}=\"{}\"", key, value)
    } else {
        format!("{}={}", key, value)
    }
}

fn slug(text: &str) -> String {
    text.trim().to_lowercase().replace(' ', "_")
}

/// The fields of a match's start marker: what was played, where, by whom.
pub fn start_fields(g: &mut Game) -> String {
        let mut gp = g.ptr();
    let gv = &mut *gp.game_values;
    let mut f: Vec<String> = Vec::new();
    f.push(kv("type", match gv.matchtype {
        MatchType::SingleGame => "single",
        MatchType::Tournament => "tournament",
        MatchType::Tour => "tour",
        MatchType::MiniGame => "minigame",
        MatchType::World => "world",
        MatchType::QuickGame => "quick",
        MatchType::NetGame => "online",
    }));
    let gm = g.mode();
    f.push(kv("mode", &slug(gm.get_mode_name())));
    if gm.gamemode == crate::common::game_mode::game_mode_star {
        f.push(kv("style", match gv.gamemodesettings.star.shine {
            StarStyle::Ztar => "ztar",
            StarStyle::Shine => "shine",
            StarStyle::Multi => "multi",
            _ => "random",
        }));
    }
    f.push(kv("goal", &gm.goal.to_string()));
    let map = g.match_map.clone();
    let short = Path::new(&map).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let short = short.split_once('_').map(|(_, rest)| rest.to_string()).unwrap_or(short);
    f.push(kv("map", &short));
    f.push(kv("file", &portable_path(g, &map)));
    match gv.matchtype {
        MatchType::Tournament => {
            let played: i16 = gv.tournament_scores.iter().map(|t| t.wins).sum();
            f.push(kv("game", &format!("{}", played + 1)));
            f.push(kv("wins_needed", &gv.tournamentgames.to_string()));
        }
        MatchType::Tour | MatchType::World => {
            let name = if gv.matchtype == MatchType::World { g.g_worldmap.get_world_name().to_string() } else {
                g.tourlist.current_path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
            };
            f.push(kv(if gv.matchtype == MatchType::World { "world" } else { "tour" }, &name));
            f.push(kv("stop", &format!("{}/{}", gv.tourstopcurrent + 1, gv.tourstops.len())));
        }
        _ => {}
    }
    let difficulty = ["very_easy", "easy", "moderate", "hard", "very_hard"];
    f.push(kv("players", &gv.playercontrol.iter().filter(|&&c| c > 0).count().to_string()));
    for p in 0..4usize {
        let control = gv.playercontrol[p];
        if control <= 0 {
            continue;
        }
        let who = if control == 2 {
            format!("cpu-{}", difficulty.get(gv.cpudifficulty as usize).unwrap_or(&"?"))
        } else {
            let slot = gv.playerInput.inputControls[p];
            let input = g.input_control(slot);
            if input.iDevice >= 0 {
                format!("pad{}", input.iDevice)
            } else {
                let set = match slot {
                    Some((set, 0)) => set as usize,
                    _ => p,
                };
                format!("keys{}", set + 1)
            }
        };
        let skin = g.skinlist.at(gv.skinids[p].max(0) as usize).name.clone();
        f.push(kv(&format!("p{}", p + 1), &format!("{},team{},{}", who, lookup_team_id(g, p as i16) + 1, skin)));
    }
    f.join(" ")
}

/// The result fields of a match's later markers.
pub fn result_fields(g: &mut Game) -> String {
    let mut gp = g.ptr();
    let gm = g.mode();
    let scores: Vec<String> = (0..gp.score_cnt.max(0) as usize).map(|k| g.score[k].score.to_string()).collect();
    let mut f = vec![kv("scores", &scores.join(","))];
    if gm.gameover {
        f.push(kv("winner", &if gm.winningteam >= 0 { format!("team{}", gm.winningteam + 1) } else { "tie".to_string() }));
    }
    f.join(" ")
}

/// A segment replay's first frame: what the splash and the menu set up, then the checkpoint.
pub struct SegmentState {
    /// The checkpoint the first frame restores.
    pub checkpoint: Vec<u8>,
    pub _alias: Aliased,
}

impl GameState for SegmentState {
    /// Exits the process with status 2 when the checkpoint does not restore.
    fn update(&mut self, g: &mut Game) {
        crate::smw::gs_splash_screen::load_game_data(g);
        g.eyecandy[2].clean();
        g.game_values.playerInput.reset_keys();
        g.game_values.appstate = AppState::Menu;
        MenuState::instance(g).init(g);
        GameplayState::instance(g).init(g);
        if let Err(e) = restore(g, std::mem::take(&mut self.checkpoint)) {
            crate::smw::harness::fail(e.to_string());
        }
        crate::smw::net_v2::segment_started(g);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_round_trip() {
        let mut a: (i16, u32, f32, bool) = (-3, 0xDEADBEEF, -0.5, true);
        let mut v: Vec<String> = vec!["sfx/packs/Classic/jump.wav".into(), String::new()];
        let mut arr = [[1i16, 2, 3], [4, 5, 6]];
        let mut mode = MatchType::Tour;
        let mut s = Snap::saver();
        s.io(&mut a);
        s.io(&mut v);
        s.io(&mut arr);
        s.io(&mut mode);

        let (mut b, mut w, mut arr2, mut mode2) = ((0i16, 0u32, 0f32, false), Vec::<String>::new(), [[0i16; 3]; 2], MatchType::SingleGame);
        let mut l = Snap::loader(s.buf);
        l.io(&mut b);
        l.io(&mut w);
        l.io(&mut arr2);
        l.io(&mut mode2);
        assert!(!l.damaged && l.pos == l.buf.len());
        assert_eq!((b.0, b.1, b.2, b.3), (-3, 0xDEADBEEF, -0.5, true));
        assert_eq!(w, v);
        assert_eq!(arr2, arr);
        assert_eq!(mode2, MatchType::Tour);
    }

    #[test]
    fn damaged_lengths_do_not_allocate() {
        let mut v: Vec<String> = Vec::new();
        let mut l = Snap::loader(vec![0xFF, 0xFF, 0xFF, 0xFF, 1]);
        l.io(&mut v);
        assert!(l.damaged && v.is_empty());
    }

    #[test]
    fn shared_names_stay_in_data_and_settings() {
        assert!(shared_name_ok("maps/2skyfight.map"));
        assert!(shared_name_ok("~settings/net_v2.map"));
        assert!(!shared_name_ok("/etc/passwd"));
        assert!(!shared_name_ok("~settings//etc/passwd"));
        assert!(!shared_name_ok("~settings/../secret.map"));
        assert!(!shared_name_ok("maps/../../outside.map"));
        assert!(!shared_name_ok("maps\\..\\outside.map"));
        assert!(!shared_name_ok("C:\\maps\\x.map"));
        assert!(!shared_name_ok("\\\\server\\share\\x.map"));
        assert!(!shared_name_ok("~other/x.map"));
    }

    #[test]
    fn marker_values_with_spaces_are_quoted() {
        assert_eq!(kv("map", "Wacky Woods"), "map=\"Wacky Woods\"");
        assert_eq!(kv("map", "2skyfight"), "map=2skyfight");
        assert_eq!(slug("Capture The Flag"), "capture_the_flag");
    }
}
