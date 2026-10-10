//! Not in the C++: match checkpoints and segment replays (docs/REPLAY.md, "Markers, checkpoints and clips").
//!
//! When a recorded match starts, `capture` saves everything the match reads from earlier frames: the
//! settings, the menu's choices, the RNG, the virtual mixer and the files the match loaded. A segment
//! replay loads the game data, `restore`s the checkpoint and enters gameplay where the menu would have,
//! so its dump from that frame on equals the full replay's.

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
use crate::smw::gs_gameplay::{eyecandy, lookup_team_id, GameplayState};
use crate::smw::gs_menu::{enter_gameplay_tail, MenuState};
use std::path::Path;

const MAGIC: &[u8; 4] = b"SMWC";
const VERSION: u8 = 1;
/// An online match's checkpoint: the net state (smw/net.rs) comes first, then version 1's fields.
const ONLINE_VERSION: u8 = 2;

/// One routine both saves and loads each value, so the two directions cannot drift apart.
pub struct Snap {
    load: bool,
    buf: Vec<u8>,
    pos: usize,
    damaged: bool,
}

pub trait Field {
    fn io(&mut self, s: &mut Snap);
}

impl Snap {
    fn saver() -> Self {
        Snap { load: false, buf: Vec::new(), pos: 0, damaged: false }
    }

    fn loader(bytes: Vec<u8>) -> Self {
        Snap { load: true, buf: bytes, pos: 0, damaged: false }
    }

    pub fn io<T: Field + ?Sized>(&mut self, v: &mut T) {
        v.io(self);
    }

    pub fn loading(&self) -> bool {
        self.load
    }

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
pub fn portable_path(path: &str) -> String {
    match path.strip_prefix(crate::common::path::get_home_directory().as_str()) {
        Some(rest) => format!("{}{}", HOME_PREFIX, rest),
        None => path.to_string(),
    }
}

pub fn local_path(path: &str) -> String {
    match path.strip_prefix(HOME_PREFIX) {
        Some(rest) => crate::common::path::get_home_directory() + rest,
        None => path.to_string(),
    }
}

/// The file a match loaded, relative to the data directory so a checkpoint works with any data root.
static mut match_map: String = String::new();

pub fn note_match_map(path: &str) {
    unsafe { match_map = data_relative(path) };
}

fn data_path(name: &str) -> String {
    if name.is_empty() || name.starts_with('/') {
        name.to_string()
    } else {
        convert_path(name)
    }
}

/// Matches a checkpoint cannot start: a World's bonus house, which no replay covers yet.
pub fn unsupported_reason() -> Option<&'static str> {
    unsafe {
        let stop = game_values.tourstops.get(game_values.tourstopcurrent);
        if game_values.matchtype == MatchType::World && stop.is_some_and(|s| s.iStageType == 1) {
            return Some("bonus_house");
        }
    }
    None
}

/// The checkpoint at the current point of `MenuState::enter_gameplay`.
pub fn capture() -> Vec<u8> {
    let online = unsafe { crate::smw::net::netplay.active };
    let mut s = Snap::saver();
    s.put(MAGIC);
    s.put(&[if online { ONLINE_VERSION } else { VERSION }]);
    walk(&mut s, online);
    s.buf
}

/// Restores a checkpoint after `load_game_data` and the menu's init, then starts the match.
pub fn restore(bytes: Vec<u8>) -> Result<(), String> {
    if bytes.len() < 5 || &bytes[..4] != MAGIC {
        return Err("not a checkpoint".to_string());
    }
    if bytes[4] != VERSION && bytes[4] != ONLINE_VERSION {
        return Err(format!("checkpoint version {} (this build reads {} and {})", bytes[4], VERSION, ONLINE_VERSION));
    }
    let online = bytes[4] == ONLINE_VERSION;
    let mut s = Snap::loader(bytes);
    s.pos = 5;
    walk(&mut s, online);
    if s.damaged || s.pos != s.buf.len() {
        return Err("checkpoint is damaged".to_string());
    }
    enter_gameplay_tail();
    Ok(())
}

/// Saves or restores every part of the checkpoint, in the order a restore must apply them.
fn walk(s: &mut Snap, online: bool) {
    unsafe {
        let gv = &mut *game_values;
        if online {
            crate::smw::net::checkpoint(s);
        }

        // Settings, as options.bin and controls.sdl2.bin would hold them now.
        let (mut options, mut controls) = if s.load { (Vec::new(), Vec::new()) } else { gv.config_bytes() };
        s.io(&mut options);
        s.io(&mut controls);
        if s.load {
            gv.read_config_bytes(options, controls);
        }

        // Which input configuration each player reads, as `inputConfiguration` index * 2 + keyboard/joystick.
        let mut inputs = [0u8; 4];
        for p in 0..4 {
            let current = gv.playerInput.inputControls[p].as_ptr();
            for i in 0..8 {
                if std::ptr::eq(current, &gv.inputConfiguration[i / 2][i % 2]) {
                    inputs[p] = i as u8;
                }
            }
        }
        s.io(&mut inputs);
        if s.load {
            for p in 0..4 {
                let i = inputs[p] as usize % 8;
                gv.cgame_config.playerInput.inputControls[p] = Ptr::from_mut(&mut gv.cgame_config.inputConfiguration[i / 2][i % 2]);
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

        use crate::smw::main::{bonushousemode, bossgamemode, boxesgamemode, currentgamemode, gamemodes, pipegamemode, score, score_cnt};
        use crate::common::game_mode::*;
        s.io(&mut currentgamemode);
        let mut mode = gv.gamemode.gamemode;
        s.io(&mut mode);
        let mut boss = bossgamemode.get_boss_type();
        s.io(&mut boss);
        s.io(&mut bonushousemode.goal);
        s.io(&mut pipegamemode.goal);
        s.io(&mut bossgamemode.goal);
        s.io(&mut boxesgamemode.goal);
        if s.load {
            bossgamemode.set_boss_type(boss);
            gv.gamemode = match mode {
                game_mode_bonus => Ptr::from_raw(bonushousemode.as_ptr() as *mut dyn CGameModeTrait),
                game_mode_pipe_minigame => Ptr::from_raw(pipegamemode.as_ptr() as *mut dyn CGameModeTrait),
                game_mode_boss_minigame => Ptr::from_raw(bossgamemode.as_ptr() as *mut dyn CGameModeTrait),
                game_mode_boxes_minigame => Ptr::from_raw(boxesgamemode.as_ptr() as *mut dyn CGameModeTrait),
                m => gamemodes[(m as usize).min(GAMEMODE_LAST as usize - 1)],
            };
        }

        let world = gv.matchtype == MatchType::World;
        let mut stops: Vec<String> = gv.tourstops.iter().map(|ts| write_tour_stop_line(ts, world)).collect();
        s.io(&mut stops);
        if s.load {
            gv.tourstops.clear();
            for line in stops.iter() {
                gv.tourstops.push(Ptr::new_box(parse_tour_stop_line(line.as_bytes(), &GAME_VERSION, world)));
            }
        }

        for k in 0..4 {
            let mut sc = score[k];
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
        s.io(&mut score_cnt);
        GameplayState::instance().checkpoint(s);

        // The map and the music the menu chose. A net game's joiner plays the host's map from its settings directory.
        let mut map = portable_path(&match_map);
        let mut map_index = maplist.get_current();
        s.io(&mut map);
        s.io(&mut map_index);
        map = local_path(&map);
        if s.load {
            maplist.set_current(map_index);
            g_map.load_map(&data_path(&map), crate::common::map::read_type_full);
            note_match_map(&map);
            crate::common::global::load_current_map_background();
        }
        let (song, song_index) = musiclist.song_state_mut();
        let mut song_name = data_relative(&song.to_string_lossy());
        s.io(&mut song_name);
        s.io(song_index);
        if s.load {
            *song = data_path(&song_name).into();
            for k in 0..4 {
                if gv.playercontrol[k] > 0 {
                    rm.spr_player[k] = match crate::smw::net::net_skin_path(k) {
                        Some(path) => rm.load_full_skin_path(Path::new(&path), k as i16),
                        None => rm.load_full_skin(gv.skinids[k], gv.colorids[k]),
                    };
                }
            }
        }

        let tracks: Vec<*mut sfxMusic> = rm.backgroundmusic.iter_mut().map(|t| t as *mut sfxMusic).collect();
        for &t in tracks.iter() {
            let mut name = (*t).name().to_string();
            let mut paused = (*t).paused();
            s.io(&mut name);
            s.io(&mut paused);
            if s.load {
                if (*t).name() != name {
                    *t = if name.is_empty() { sfxMusic::new() } else { sfxMusic::from_file(Path::new(&data_path(&name))).unwrap_or_else(|e| std::panic::panic_any(e)) };
                }
                (*t).set_paused(paused);
            }
        }
        let sounds = rm.sounds();
        for &snd in sounds.iter() {
            let mut name = (*snd).name().to_string();
            let (mut channels, mut last_start) = (*snd).state();
            s.io(&mut name);
            s.io(&mut channels);
            s.io(&mut last_start);
            if s.load {
                if (*snd).name() != name {
                    *snd = if name.is_empty() { sfxSound::new() } else { sfxSound::from_file(Path::new(&data_path(&name))).unwrap_or_else(|e| std::panic::panic_any(e)) };
                }
                (*snd).set_state(channels, last_start);
            }
        }

        // The virtual mixer, with sounds and tracks as indices into the lists above.
        let index_of = |list: &[*mut sfxSound], p: *mut sfxSound| list.iter().position(|&x| x == p).map(|i| i as i32).unwrap_or(-1);
        let (channels, music, mut resume) = mixer::save(&sounds, &tracks);
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
            mixer::restore(&channels, &mixer::Music { track, forever, paused, end, remaining }, resume);
        }

        let rng = crate::common::random_number_generator::RandomNumberGenerator::generator();
        let (state, index) = rng.state_mut();
        s.io(state);
        s.io(index);
        let mut calls = RandomNumberGenerator::call_count();
        let mut last = RandomNumberGenerator::last_value();
        s.io(&mut calls);
        s.io(&mut last);
        if s.load {
            RandomNumberGenerator::set_counters(calls, last);
        }

        // Sound commands of this frame so far, which belong to its dump block.
        s.io(&mut sfx::sfx_events);
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
pub fn start_fields() -> String {
    unsafe {
        let gv = &mut *game_values;
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
        let gm = gv.gamemode;
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
        let map = match_map.clone();
        let short = Path::new(&map).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let short = short.split_once('_').map(|(_, rest)| rest.to_string()).unwrap_or(short);
        f.push(kv("map", &short));
        f.push(kv("file", &portable_path(&map)));
        match gv.matchtype {
            MatchType::Tournament => {
                let played: i16 = gv.tournament_scores.iter().map(|t| t.wins).sum();
                f.push(kv("game", &format!("{}", played + 1)));
                f.push(kv("wins_needed", &gv.tournamentgames.to_string()));
            }
            MatchType::Tour | MatchType::World => {
                let name = if gv.matchtype == MatchType::World { crate::smw::world::g_worldmap.get_world_name().to_string() } else {
                    tourlist.current_path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
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
                let input = gv.playerInput.inputControls[p];
                if input.iDevice >= 0 {
                    format!("pad{}", input.iDevice)
                } else {
                    let set = (0..4).find(|&i| std::ptr::eq(input.as_ptr(), &gv.inputConfiguration[i][0])).unwrap_or(p);
                    format!("keys{}", set + 1)
                }
            };
            let skin = skinlist.at(gv.skinids[p].max(0) as usize).name.clone();
            f.push(kv(&format!("p{}", p + 1), &format!("{},team{},{}", who, lookup_team_id(p as i16) + 1, skin)));
        }
        f.join(" ")
    }
}

/// The result fields of a match's later markers.
pub fn result_fields() -> String {
    unsafe {
        let gm = game_values.gamemode;
        let scores: Vec<String> = (0..crate::smw::main::score_cnt.max(0) as usize).map(|k| crate::smw::main::score[k].score.to_string()).collect();
        let mut f = vec![kv("scores", &scores.join(","))];
        if gm.gameover {
            f.push(kv("winner", &if gm.winningteam >= 0 { format!("team{}", gm.winningteam + 1) } else { "tie".to_string() }));
        }
        f.join(" ")
    }
}

/// A segment replay's first frame: what the splash and the menu set up, then the checkpoint.
pub struct SegmentState {
    pub checkpoint: Vec<u8>,
    pub _alias: Aliased,
}

impl GameState for SegmentState {
    fn update(&mut self) {
        unsafe {
            crate::smw::gs_splash_screen::load_game_data();
            eyecandy[2].clean();
            game_values.playerInput.reset_keys();
            game_values.appstate = AppState::Menu;
            MenuState::instance().init();
            GameplayState::instance().init();
        }
        if let Err(e) = restore(std::mem::take(&mut self.checkpoint)) {
            eprintln!("[harness] {}", e);
            std::process::exit(2);
        }
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
    fn marker_values_with_spaces_are_quoted() {
        assert_eq!(kv("map", "Wacky Woods"), "map=\"Wacky Woods\"");
        assert_eq!(kv("map", "2skyfight"), "map=2skyfight");
        assert_eq!(slug("Capture The Flag"), "capture_the_flag");
    }
}
