//! Not in the C++ (docs/PROGRESS.md, Netplay deviations): deaths, scores and the end of a net game.
//!
//! Kills, team removals and the score board are `net_random` events: the game host decides them and its
//! joiners replay them, so a joiner never kills a player or removes a team on its own. At the end of every
//! frame whose scores or game end changed, the host sends them all (`Ev::Scores`); a joiner changes its
//! scores only inside the host's events and takes its game over and winner only from the host.
//!
//! The harness dump records every death, every change of the team scores and the end of the game as `C`
//! records (docs/REPLAY.md), so net_game_compare.py can tell whether the clients agree on them.

use crate::common::game_values::game_values;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::smw::harness;
use crate::smw::main::{score, score_cnt};
use crate::smw::net::netplay;
use crate::smw::net_random::{self, Ev};
use crate::smw::player::{get_player_from_global_id, player_killed_player_now, PlayerDeathStyle, PlayerState};

static mut g_lastScores: Vec<i16> = Vec::new();
static mut g_lastGameOver: bool = false;
static mut g_lastSent: Vec<i32> = Vec::new();
static mut g_hostGameOver: bool = false;
static mut g_hostWinner: i16 = -1;
static mut g_applying: bool = false;
static mut g_killResult: PlayerKillType = PlayerKillType::None;
static mut g_removeResult: bool = false;

pub fn begin_game() {
    unsafe {
        g_lastScores.clear();
        g_lastGameOver = false;
        g_lastSent.clear();
        g_hostGameOver = false;
        g_hostWinner = -1;
    }
}

pub fn note_death(player: i16, style: i32, removed: bool) {
    unsafe {
        if netplay.active {
            harness::note_net(format!("death p={} style={} removed={}", player, style, removed as i32));
        }
    }
}

/// While true, score changes outside the game host's events are ignored.
pub fn score_locked() -> bool {
    unsafe { net_random::host_decides() && net_random::in_game() && !net_random::in_event() && !g_applying }
}

/// A joiner keeps the game host's game over and winner, whatever its own mode logic concluded.
pub fn frame_start() {
    unsafe {
        if net_random::host_decides() && !game_values.gamemode.is_null() {
            let mut gm = game_values.gamemode;
            gm.gameover = g_hostGameOver;
            gm.winningteam = g_hostWinner;
        }
    }
}

/// End of a gameplay frame: the host sends changed scores; every client but a deciding host's joiner records them.
pub fn sample() {
    unsafe {
        if !netplay.active || game_values.gamemode.is_null() {
            return;
        }
        if netplay.theHostIsMe && net_random::in_game() {
            let snapshot = snapshot();
            if snapshot != g_lastSent {
                g_lastSent = snapshot.clone();
                net_random::event(Ev::Scores, &snapshot);
            }
        }
        if !net_random::host_decides() {
            record();
        }
    }
}

fn snapshot() -> Vec<i32> {
    unsafe {
        let gm = game_values.gamemode;
        let mut v = vec![gm.gameover as i32, gm.winningteam as i32];
        for t in 0..score_cnt as usize {
            v.push(score[t].score as i32);
            v.extend(score[t].subscore.iter().map(|s| *s as i32));
        }
        v
    }
}

fn record() {
    unsafe {
        let scores: Vec<i16> = (0..score_cnt as usize).map(|t| score[t].score).collect();
        if scores != g_lastScores {
            let text: Vec<String> = scores.iter().map(|s| s.to_string()).collect();
            harness::note_net(format!("scores {}", text.join(",")));
            g_lastScores = scores;
        }
        let gm = game_values.gamemode;
        if gm.gameover && !g_lastGameOver {
            harness::note_net(format!("gameover winner={}", gm.winningteam));
        }
        g_lastGameOver = gm.gameover;
    }
}

/// `Ev::Scores`.
pub fn net_apply_scores(args: &[i32]) {
    unsafe {
        g_hostGameOver = args[0] != 0;
        g_hostWinner = args[1] as i16;
        if netplay.theHostIsMe {
            return;
        }
        g_applying = true;
        for (t, v) in args[2..].chunks(4).enumerate().take(score_cnt as usize) {
            score[t].set_score(v[0] as i16);
            for (i, s) in v[1..].iter().enumerate() {
                score[t].subscore[i] = *s as i16;
            }
        }
        g_applying = false;
        let mut gm = game_values.gamemode;
        gm.gameover = g_hostGameOver;
        gm.winningteam = g_hostWinner;
        record();
    }
}

/// The three ways a player gets killed: `player_killed_player`, `CPlayer::killed_player` and
/// `CPlayer::kill_player_map_hazard`. `None` means the caller kills as usual; a joiner whose game host decides
/// gets `NonKill` and waits for the host's kill.
pub fn kill(args: &[i32]) -> Option<PlayerKillType> {
    if !net_random::event(Ev::Kill, args) {
        return None;
    }
    unsafe { Some(if netplay.theHostIsMe { g_killResult } else { PlayerKillType::NonKill }) }
}

pub fn kill_result() -> PlayerKillType {
    unsafe { g_killResult }
}

pub fn kill_had_effect() -> bool {
    unsafe { g_killResult != PlayerKillType::None }
}

/// `Ev::Kill`: `[way, killer or player, killed, death style, kill style, force, kill carried item, credit]`.
pub fn net_kill(args: &[i32]) {
    unsafe {
        g_killResult = PlayerKillType::None;
        let first = get_player_from_global_id(args[1] as i16);
        let killed = get_player_from_global_id(args[2] as i16);
        let deathstyle = death_style(args[3]);
        let style = kill_style(args[4]);
        let (fForce, fKillCarriedItem) = (args[5] != 0, args[6] != 0);
        g_killResult = match args[0] {
            0 if !killed.is_null() => player_killed_player_now(args[1] as i16, killed, deathstyle, style, fForce, fKillCarriedItem),
            1 if !first.is_null() && !killed.is_null() => first.get().killed_player_now(killed, deathstyle, style, fForce, fKillCarriedItem),
            2 if !first.is_null() => first.get().kill_player_map_hazard_now(fForce, style, fKillCarriedItem, args[7] as i16),
            _ => PlayerKillType::None,
        };

        // A joiner whose copy of the victim was still spawning or protected dies as the host's did.
        let hostKilled = args.get(8).is_some_and(|&r| r == PlayerKillType::Normal as i32 || r == PlayerKillType::Removed as i32);
        let mut victim = if args[0] == 2 { first } else { killed };
        if hostKilled && !victim.is_null() && (victim.state == PlayerState::Ready || victim.state == PlayerState::Spawning) {
            victim.die(deathstyle, false, fKillCarriedItem);
        }
    }
}

pub fn kill_outcome(args: &[i32]) -> String {
    let mut p = get_player_from_global_id(args[2] as i16);
    if p.is_null() {
        p = get_player_from_global_id(args[1] as i16);
    }
    unsafe {
        let result = g_killResult as i32;
        if p.is_null() {
            format!("r{}", result)
        } else {
            format!("r{} p{} s{}", result, p.globalID, p.state as i32)
        }
    }
}

/// `remove_team`: `None` means the caller removes the team as usual.
pub fn remove_team_event(teamid: i16) -> Option<bool> {
    if !net_random::event(Ev::RemoveTeam, &[teamid as i32]) {
        return None;
    }
    unsafe {
        if netplay.theHostIsMe {
            Some(g_removeResult)
        } else {
            let removed = score[teamid as usize].order > -1;
            Some(game_values.flags.teamdeadcounter as i32 + if removed { 0 } else { 1 } == score_cnt as i32 - 1)
        }
    }
}

pub fn net_remove_team(teamid: i16) {
    unsafe {
        if (0..score_cnt).contains(&teamid) {
            g_removeResult = crate::smw::gamemodes::game_mode::remove_team_now(teamid);
        }
    }
}

fn death_style(v: i32) -> PlayerDeathStyle {
    match v {
        1 => PlayerDeathStyle::Squish,
        2 => PlayerDeathStyle::Shatter,
        _ => PlayerDeathStyle::Jump,
    }
}

fn kill_style(v: i32) -> KillStyle {
    if (0..=KillStyle::Phanto as i32).contains(&v) {
        unsafe { std::mem::transmute::<i32, KillStyle>(v) }
    } else {
        KillStyle::default()
    }
}
