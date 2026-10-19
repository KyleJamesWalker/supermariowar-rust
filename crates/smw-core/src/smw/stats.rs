//! Gameplay stats (docs/design/GAMEPLAY_STATS.md): per-player counters for the running match, the after-match page,
//! and the lifetime totals saved in the settings directory. The match counters live in `game_values`, so a rollback
//! restores them with the rest of the match; nothing in the simulation reads them.

use crate::common::player_kill_styles::KillStyle;
use crate::game::Game;
use crate::smw::player::PlayerKeys;
use crate::smw::players::PlayerId;
use std::collections::BTreeMap;

pub const KILL_STYLES: usize = KillStyle::Phanto as usize + 1;
const FILE: &str = "stats.txt";
const FORMAT: &str = "smw-stats 1";

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlayerStats {
    pub kills: u16,
    pub deaths: u16,
    pub suicides: u16,
    pub kills_by: [u16; KILL_STYLES],
    pub deaths_by: [u16; KILL_STYLES],
    /// Kills of each player slot.
    pub victims: [u16; 4],
    pub powerups: u16,
    pub items_stored: u16,
    pub items_used: u16,
    pub boxes: u16,
    pub jumps: u32,
    pub air_ticks: u32,
    pub distance: f32,
    pub star_ticks: u32,
    pub streak: u16,
    pub best_streak: u16,
    pub alive_ticks: u32,
    pub best_alive_ticks: u32,
    pub air_chain: u16,
    pub triple_kills: u16,
}

impl PlayerStats {
    const ZERO: Self = PlayerStats {
        kills: 0,
        deaths: 0,
        suicides: 0,
        kills_by: [0; KILL_STYLES],
        deaths_by: [0; KILL_STYLES],
        victims: [0; 4],
        powerups: 0,
        items_stored: 0,
        items_used: 0,
        boxes: 0,
        jumps: 0,
        air_ticks: 0,
        distance: 0.0,
        star_ticks: 0,
        streak: 0,
        best_streak: 0,
        alive_ticks: 0,
        best_alive_ticks: 0,
        air_chain: 0,
        triple_kills: 0,
    };
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MatchStats {
    pub players: [PlayerStats; 4],
}

impl MatchStats {
    pub const fn new() -> Self {
        MatchStats { players: [PlayerStats::ZERO; 4] }
    }
}

impl Default for MatchStats {
    fn default() -> Self {
        Self::new()
    }
}

fn slot(g: &mut Game, id: PlayerId) -> Option<usize> {
    let gid = g.player(id).globalID;
    (0..4).contains(&gid).then_some(gid as usize)
}

fn with(g: &mut Game, id: PlayerId, f: impl FnOnce(&mut PlayerStats)) {
    if let Some(s) = slot(g, id) {
        f(&mut g.game_values.stats.players[s]);
    }
}

pub fn reset(g: &mut Game) {
    g.game_values.stats = MatchStats::new();
}

/// `killer` killed `killed`, another player. Three kills with no landing between them make a triple kill.
pub fn kill(g: &mut Game, killer: PlayerId, killed: PlayerId, style: KillStyle) {
    let victim = slot(g, killed);
    with(g, killer, |s| {
        s.kills += 1;
        s.kills_by[style as usize] += 1;
        if let Some(v) = victim {
            s.victims[v] += 1;
        }
        s.streak += 1;
        s.best_streak = s.best_streak.max(s.streak);
        s.air_chain += 1;
        if s.air_chain % 3 == 0 {
            s.triple_kills += 1;
        }
    });
    with(g, killed, |s| s.deaths_by[style as usize] += 1);
}

pub fn suicide(g: &mut Game, killed: PlayerId, style: KillStyle) {
    with(g, killed, |s| {
        s.suicides += 1;
        s.deaths_by[style as usize] += 1;
    });
}

pub fn death(g: &mut Game, killed: PlayerId) {
    with(g, killed, |s| {
        s.deaths += 1;
        s.streak = 0;
        s.alive_ticks = 0;
    });
}

pub fn jump(g: &mut Game, id: PlayerId) {
    with(g, id, |s| s.jumps += 1);
}

pub fn powerup(g: &mut Game, id: PlayerId) {
    with(g, id, |s| s.powerups += 1);
}

pub fn item_stored(g: &mut Game, id: PlayerId) {
    with(g, id, |s| s.items_stored += 1);
}

pub fn item_used(g: &mut Game, id: PlayerId) {
    with(g, id, |s| s.items_used += 1);
}

pub fn item_box(g: &mut Game, id: PlayerId) {
    with(g, id, |s| s.boxes += 1);
}

/// Once a tick, after the players move.
pub fn tick(g: &mut Game) {
    for i in 0..g.players.len() {
        let id = g.players[i];
        let p = g.player(id);
        let (ready, inair, velx, star) = (p.isready(), p.inair, p.velx, p.is_invincible());
        with(g, id, |s| {
            if !ready {
                return;
            }
            s.alive_ticks += 1;
            s.best_alive_ticks = s.best_alive_ticks.max(s.alive_ticks);
            s.distance += velx.abs();
            if inair {
                s.air_ticks += 1;
            } else {
                s.air_chain = 0;
            }
            if star {
                s.star_ticks += 1;
            }
        });
    }
}

fn kill_style_name(i: usize) -> &'static str {
    const NAMES: [&str; KILL_STYLES] = [
        "stomp", "star", "fireball", "bobomb", "bounce", "pow", "goomba", "bulletbill", "hammer", "shell", "throwblock",
        "cheepcheep", "koopa", "boomerang", "feather", "iceblast", "podobo", "bomb", "leaf", "pwings", "kuriboshoe",
        "poisonmushroom", "environment", "push", "buzzybeetle", "spiny", "phanto",
    ];
    NAMES[i]
}

/// Lifetime totals: counters by name, bests by name, and (played, won) per mode and per map.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Lifetime {
    pub totals: BTreeMap<String, u64>,
    pub bests: BTreeMap<String, u64>,
    pub modes: BTreeMap<String, (u64, u64)>,
    pub maps: BTreeMap<String, (u64, u64)>,
}

impl Lifetime {
    pub fn parse(text: &str) -> Self {
        let mut life = Lifetime::default();
        let mut lines = text.lines();
        if lines.next() != Some(FORMAT) {
            return life;
        }
        for line in lines {
            let f: Vec<&str> = line.split('\t').collect();
            let num = |i: usize| f.get(i).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            match f.first().copied() {
                Some("total") if f.len() == 3 => {
                    life.totals.insert(f[1].to_string(), num(2));
                }
                Some("best") if f.len() == 3 => {
                    life.bests.insert(f[1].to_string(), num(2));
                }
                Some("mode") if f.len() == 4 => {
                    life.modes.insert(f[1].to_string(), (num(2), num(3)));
                }
                Some("map") if f.len() == 4 => {
                    life.maps.insert(f[1].to_string(), (num(2), num(3)));
                }
                _ => {}
            }
        }
        life
    }

    pub fn to_text(&self) -> String {
        let mut out = format!("{FORMAT}\n");
        for (k, v) in &self.totals {
            out += &format!("total\t{k}\t{v}\n");
        }
        for (k, v) in &self.bests {
            out += &format!("best\t{k}\t{v}\n");
        }
        for (k, (p, w)) in &self.modes {
            out += &format!("mode\t{k}\t{p}\t{w}\n");
        }
        for (k, (p, w)) in &self.maps {
            out += &format!("map\t{k}\t{p}\t{w}\n");
        }
        out
    }

    fn add(&mut self, key: &str, n: u64) {
        if n > 0 {
            *self.totals.entry(key.to_string()).or_default() += n;
        }
    }

    fn best(&mut self, key: &str, n: u64) {
        let b = self.bests.entry(key.to_string()).or_default();
        *b = (*b).max(n);
    }

    pub fn total(&self, key: &str) -> u64 {
        self.totals.get(key).copied().unwrap_or(0)
    }

    pub fn best_of(&self, key: &str) -> u64 {
        self.bests.get(key).copied().unwrap_or(0)
    }

    /// Adds one human player's match.
    pub fn add_match(&mut self, s: &PlayerStats, mode: &str, map: &str, won: bool) {
        self.add("matches", 1);
        self.add("wins", won as u64);
        self.add("kills", s.kills as u64);
        self.add("deaths", s.deaths as u64);
        self.add("suicides", s.suicides as u64);
        for i in 0..KILL_STYLES {
            self.add(&format!("kills.{}", kill_style_name(i)), s.kills_by[i] as u64);
            self.add(&format!("deaths.{}", kill_style_name(i)), s.deaths_by[i] as u64);
        }
        self.add("powerups", s.powerups as u64);
        self.add("items_stored", s.items_stored as u64);
        self.add("items_used", s.items_used as u64);
        self.add("item_boxes", s.boxes as u64);
        self.add("jumps", s.jumps as u64);
        self.add("air_ticks", s.air_ticks as u64);
        self.add("distance", s.distance as u64);
        self.add("star_ticks", s.star_ticks as u64);
        self.add("triple_kills", s.triple_kills as u64);
        self.best("kills_in_match", s.kills as u64);
        self.best("kill_streak", s.best_streak as u64);
        self.best("survival_ticks", s.best_alive_ticks as u64);
        self.best("triple_kills_in_match", s.triple_kills as u64);
        for (table, key) in [(&mut self.modes, mode), (&mut self.maps, map)] {
            let e = table.entry(key.to_string()).or_default();
            e.0 += 1;
            e.1 += won as u64;
        }
    }
}

fn path(g: &mut Game) -> std::path::PathBuf {
    crate::services::services(g).storage.settings_dir().join(FILE)
}

/// Whether this session's matches count toward the lifetime totals: a player's own session, not a replay or a
/// scripted or headless run.
pub fn counting(g: &mut Game) -> bool {
    !crate::smw::harness::replaying(g)
        && ["SMW_REPLAY", "SMW_DUMP", "SMW_LIVE_SCRIPT", "SMW_FRAMES", "SMW_SEED"].iter().all(|v| std::env::var_os(v).is_none())
}

pub fn load(g: &mut Game) -> Lifetime {
    std::fs::read_to_string(path(g)).map(|t| Lifetime::parse(&t)).unwrap_or_default()
}

fn save(g: &mut Game, life: &Lifetime) {
    if !counting(g) {
        return;
    }
    let p = path(g);
    if std::fs::write(&p, life.to_text()).is_ok() {
        crate::services::services(g).storage.persist();
    }
}

pub fn reset_lifetime(g: &mut Game) {
    save(g, &Lifetime::default());
}

/// Adds a finished match's human players on this device to the lifetime totals.
pub fn commit(g: &mut Game) {
    if !counting(g) {
        return;
    }
    let mode = g.mode().get_mode_name().to_string();
    let map = g.maplist.current_shortmapname().to_string();
    let winner = g.mode().winningteam;
    let mut life = load(g);
    let mut added = false;
    for i in 0..g.players.len() {
        let id = g.players[i];
        let p = g.player(id);
        if p.pPlayerAI.is_some() || matches!(p.playerKeys, PlayerKeys::Net(_)) {
            continue;
        }
        let team = p.teamID;
        let Some(s) = slot(g, id) else { continue };
        if !g.netplay.active && g.game_values.playercontrol[s] != 1 {
            continue;
        }
        let stats = g.game_values.stats.players[s];
        life.add_match(&stats, &mode, &map, winner == team);
        added = true;
    }
    if added {
        save(g, &life);
    }
}

pub fn clock(ticks: u64) -> String {
    let t = ticks / crate::common::global_constants::TICKS_PER_SECOND as u64;
    if t >= 3600 {
        format!("{}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60)
    } else {
        format!("{}:{:02}", t / 60, t % 60)
    }
}

/// The after-match page: per-player stats and who killed whom.
pub fn draw_page(g: &mut Game) {
    let mut gp = g.ptr();
    g.rm.sprite(g.rm.menu_shade).setalpha(255);
    gp.rm.sprite(gp.rm.menu_shade).draw(g, 0, 0);
    let large = &gp.rm.menu_font_large;
    let small = &gp.rm.menu_font_small;
    large.draw_centered(g, 320, 12, "Match Stats");
    let slots: Vec<usize> = (0..4).filter(|&s| g.game_values.playercontrol[s] > 0).collect();
    type Row = (&'static str, fn(&PlayerStats) -> String);
    let rows: [Row; 13] = [
        ("Kills", |s| s.kills.to_string()),
        ("Deaths", |s| s.deaths.to_string()),
        ("Suicides", |s| s.suicides.to_string()),
        ("Best streak", |s| s.best_streak.to_string()),
        ("Triple kills", |s| s.triple_kills.to_string()),
        ("Longest life", |s| clock(s.best_alive_ticks as u64)),
        ("Jumps", |s| s.jumps.to_string()),
        ("Air time", |s| clock(s.air_ticks as u64)),
        ("Distance", |s| (s.distance as u32 / 32).to_string()),
        ("Star time", |s| clock(s.star_ticks as u64)),
        ("Powerups", |s| s.powerups.to_string()),
        ("Items used", |s| format!("{}/{}", s.items_used, s.items_stored)),
        ("Item boxes", |s| s.boxes.to_string()),
    ];
    let col = |k: usize| 250 + k as i32 * 100;
    for (k, &s) in slots.iter().enumerate() {
        large.draw_centered(g, col(k), 44, &format!("P{}", s + 1));
    }
    let mut y = 74;
    for (name, value) in rows.iter() {
        small.draw(g, 24, y, name);
        for (k, &s) in slots.iter().enumerate() {
            let text = value(&g.game_values.stats.players[s]);
            small.draw_centered(g, col(k), y, &text);
        }
        y += 17;
    }
    small.draw(g, 24, y, "Mode score");
    for (k, &s) in slots.iter().enumerate() {
        let mut score = String::from("-");
        for i in 0..g.players.len() {
            let id = g.players[i];
            if g.player(id).globalID == s as i16 {
                score = g.score[g.player(id).score_slot].score.to_string();
            }
        }
        small.draw_centered(g, col(k), y, &score);
    }
    y += 26;
    small.draw(g, 24, y, "Kills of");
    for (k, &s) in slots.iter().enumerate() {
        small.draw_centered(g, col(k), y, &format!("P{}", s + 1));
    }
    y += 17;
    for &killer in slots.iter() {
        small.draw(g, 24, y, &format!("  by P{}", killer + 1));
        for (k, &victim) in slots.iter().enumerate() {
            let n = g.game_values.stats.players[killer].victims[victim];
            small.draw_centered(g, col(k), y, &n.to_string());
        }
        y += 17;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifetime_round_trips_and_keeps_bests() {
        let mut s = PlayerStats::ZERO;
        s.kills = 4;
        s.kills_by[KillStyle::Shell as usize] = 4;
        s.best_streak = 3;
        let mut life = Lifetime::default();
        life.add_match(&s, "Classic", "0smw", true);
        s.kills = 2;
        life.add_match(&s, "Classic", "Rubber Room", false);
        assert_eq!(life.total("kills"), 6);
        assert_eq!(life.total("kills.shell"), 8);
        assert_eq!(life.best_of("kills_in_match"), 4);
        assert_eq!(life.modes["Classic"], (2, 1));
        assert_eq!(life.maps["Rubber Room"], (1, 0));
        assert_eq!(Lifetime::parse(&life.to_text()), life);
    }

    #[test]
    fn an_unknown_format_reads_as_empty() {
        assert_eq!(Lifetime::parse("smw-stats 99\ntotal\tkills\t5\n"), Lifetime::default());
    }
}
