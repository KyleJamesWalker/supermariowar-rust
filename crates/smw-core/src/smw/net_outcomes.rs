//! Not in the C++ (docs/PROGRESS.md, Netplay deviations): deaths, scores and the end of a net game.
//!
//! Kills, team removals and the score board are `net_random` events: the game host decides them and its
//! joiners replay them, so a joiner never kills a player or removes a team on its own. At the end of every
//! frame whose scores or game end changed, the host sends them all (`Ev::Scores`); a joiner changes its
//! scores only inside the host's events and takes its game over and winner only from the host.
//!
//! The harness dump records every death, every change of the team scores and the end of the game as `C`
//! records (docs/REPLAY.md), so net_game_compare.py can tell whether the clients agree on them.

use crate::game::Game;
use crate::game::game;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::smw::harness;
use crate::smw::net_random::{self, Ev};
use crate::smw::player::{get_player_from_global_id, player_killed_player_now, PlayerDeathStyle, PlayerState};

/// The netplay outcome sync's part of `Game`.
pub struct NetOutcomesState {
    g_lastScores: Vec<i16>,
    g_lastGameOver: bool,
    g_lastSent: Vec<i32>,
    g_hostGameOver: bool,
    g_hostWinner: i16,
    g_applying: bool,
    g_killResult: PlayerKillType,
    g_removeResult: bool,
}

impl NetOutcomesState {
    pub const fn new() -> Self {
        NetOutcomesState {
            g_lastScores: Vec::new(),
            g_lastGameOver: false,
            g_lastSent: Vec::new(),
            g_hostGameOver: false,
            g_hostWinner: -1,
            g_applying: false,
            g_killResult: PlayerKillType::None,
            g_removeResult: false,
        }
    }
}

fn outcome_state(g: &mut Game) -> &mut NetOutcomesState {
    &mut g.net_outcomes
}

pub fn begin_game(g: &mut Game) {
    unsafe {
        outcome_state(g).g_lastScores.clear();
        outcome_state(g).g_lastGameOver = false;
        outcome_state(g).g_lastSent.clear();
        outcome_state(g).g_hostGameOver = false;
        outcome_state(g).g_hostWinner = -1;
    }
}

pub fn note_death(g: &mut Game, player: i16, style: i32, removed: bool) {
    unsafe {
        if g.netplay.active {
            harness::note_net(g, format!("death p={} style={} removed={}", player, style, removed as i32));
        }
    }
}

/// While true, score changes outside the game host's events are ignored.
pub fn score_locked(g: &mut Game) -> bool {
    unsafe { net_random::host_decides(g) && net_random::in_game(g) && !net_random::in_event(g) && !outcome_state(g).g_applying }
}

/// A joiner keeps the game host's game over and winner, whatever its own mode logic concluded.
pub fn frame_start(g: &mut Game) {
    unsafe {
        if net_random::host_decides(g) && !g.game_values.gamemode.is_null() {
            let mut gm = g.game_values.gamemode;
            gm.gameover = outcome_state(g).g_hostGameOver;
            gm.winningteam = outcome_state(g).g_hostWinner;
        }
    }
}

/// End of a gameplay frame: the host sends changed scores; every client but a deciding host's joiner records them.
pub fn sample(g: &mut Game) {
    unsafe {
        if !g.netplay.active || g.game_values.gamemode.is_null() {
            return;
        }
        if g.netplay.theHostIsMe && net_random::in_game(g) {
            let snapshot = snapshot(g);
            if snapshot != outcome_state(g).g_lastSent {
                outcome_state(g).g_lastSent = snapshot.clone();
                net_random::event(g, Ev::Scores, &snapshot);
            }
        }
        if !net_random::host_decides(g) {
            record(g);
        }
    }
}

fn snapshot(g: &mut Game) -> Vec<i32> {
    unsafe {
        let gm = g.game_values.gamemode;
        let mut v = vec![gm.gameover as i32, gm.winningteam as i32];
        for t in 0..game().score_cnt as usize {
            v.push(g.score[t].score as i32);
            v.extend(g.score[t].subscore.iter().map(|s| *s as i32));
        }
        v
    }
}

fn record(g: &mut Game) {
    unsafe {
        let scores: Vec<i16> = (0..game().score_cnt as usize).map(|t| g.score[t].score).collect();
        if scores != outcome_state(g).g_lastScores {
            let text: Vec<String> = scores.iter().map(|s| s.to_string()).collect();
            harness::note_net(g, format!("scores {}", text.join(",")));
            outcome_state(g).g_lastScores = scores;
        }
        let gm = g.game_values.gamemode;
        if gm.gameover && !outcome_state(g).g_lastGameOver {
            harness::note_net(g, format!("gameover winner={}", gm.winningteam));
        }
        outcome_state(g).g_lastGameOver = gm.gameover;
    }
}

/// `Ev::Scores`.
pub fn net_apply_scores(g: &mut Game, args: &[i32]) {
        let mut gp = g.ptr();
    unsafe {
        outcome_state(g).g_hostGameOver = args[0] != 0;
        outcome_state(g).g_hostWinner = args[1] as i16;
        if g.netplay.theHostIsMe {
            return;
        }
        outcome_state(g).g_applying = true;
        for (t, v) in args[2..].chunks(4).enumerate().take(g.score_cnt as usize) {
            gp.score[t].set_score(g, v[0] as i16);
            for (i, s) in v[1..].iter().enumerate() {
                g.score[t].subscore[i] = *s as i16;
            }
        }
        outcome_state(g).g_applying = false;
        let mut gm = g.game_values.gamemode;
        gm.gameover = outcome_state(g).g_hostGameOver;
        gm.winningteam = outcome_state(g).g_hostWinner;
        record(g);
    }
}

/// The three ways a player gets killed: `player_killed_player`, `CPlayer::killed_player` and
/// `CPlayer::kill_player_map_hazard`. `None` means the caller kills as usual; a joiner whose game host decides
/// gets `NonKill` and waits for the host's kill.
pub fn kill(g: &mut Game, args: &[i32]) -> Option<PlayerKillType> {
    if !net_random::event(g, Ev::Kill, args) {
        return None;
    }
    unsafe { Some(if g.netplay.theHostIsMe { outcome_state(g).g_killResult } else { PlayerKillType::NonKill }) }
}

pub fn kill_result(g: &mut Game) -> PlayerKillType {
    unsafe { outcome_state(g).g_killResult }
}

pub fn kill_had_effect(g: &mut Game) -> bool {
    unsafe { outcome_state(g).g_killResult != PlayerKillType::None }
}

/// `Ev::Kill`: `[way, killer or player, killed, death style, kill style, force, kill carried item, credit]`.
pub fn net_kill(g: &mut Game, args: &[i32]) {
    unsafe {
        outcome_state(g).g_killResult = PlayerKillType::None;
        let first = get_player_from_global_id(g, args[1] as i16);
        let killed = get_player_from_global_id(g, args[2] as i16);
        let deathstyle = death_style(args[3]);
        let style = kill_style(args[4]);
        let (fForce, fKillCarriedItem) = (args[5] != 0, args[6] != 0);
        outcome_state(g).g_killResult = match args[0] {
            0 if !killed.is_null() => player_killed_player_now(g, args[1] as i16, killed, deathstyle, style, fForce, fKillCarriedItem),
            1 if !first.is_null() && !killed.is_null() => first.get().killed_player_now(g, killed, deathstyle, style, fForce, fKillCarriedItem),
            2 if !first.is_null() => first.get().kill_player_map_hazard_now(g, fForce, style, fKillCarriedItem, args[7] as i16),
            _ => PlayerKillType::None,
        };

        // A joiner whose copy of the victim was still spawning or protected dies as the host's did.
        let hostKilled = args.get(8).is_some_and(|&r| r == PlayerKillType::Normal as i32 || r == PlayerKillType::Removed as i32);
        let mut victim = if args[0] == 2 { first } else { killed };
        if hostKilled && !victim.is_null() && (victim.state == PlayerState::Ready || victim.state == PlayerState::Spawning) {
            victim.die(g, deathstyle, false, fKillCarriedItem);
        }
    }
}

pub fn kill_outcome(g: &mut Game, args: &[i32]) -> String {
    let mut p = get_player_from_global_id(g, args[2] as i16);
    if p.is_null() {
        p = get_player_from_global_id(g, args[1] as i16);
    }
    unsafe {
        let result = outcome_state(g).g_killResult as i32;
        if p.is_null() {
            format!("r{}", result)
        } else {
            format!("r{} p{} s{}", result, p.globalID, p.state as i32)
        }
    }
}

/// `remove_team`: `None` means the caller removes the team as usual.
pub fn remove_team_event(g: &mut Game, teamid: i16) -> Option<bool> {
    if !net_random::event(g, Ev::RemoveTeam, &[teamid as i32]) {
        return None;
    }
    unsafe {
        if g.netplay.theHostIsMe {
            Some(outcome_state(g).g_removeResult)
        } else {
            let removed = g.score[teamid as usize].order > -1;
            Some(g.game_values.flags.teamdeadcounter as i32 + if removed { 0 } else { 1 } == g.score_cnt as i32 - 1)
        }
    }
}

pub fn net_remove_team(g: &mut Game, teamid: i16) {
    unsafe {
        if (0..game().score_cnt).contains(&teamid) {
            outcome_state(g).g_removeResult = crate::smw::gamemodes::game_mode::remove_team_now(g, teamid);
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

/// A net match's checkpoint (smw/checkpoint.rs).
pub fn checkpoint(g: &mut Game, s: &mut crate::smw::checkpoint::Snap) {
    unsafe {
        s.io(&mut *(&raw mut outcome_state(g).g_lastScores));
        s.io(&mut *(&raw mut outcome_state(g).g_lastGameOver));
        s.io(&mut *(&raw mut outcome_state(g).g_lastSent));
        s.io(&mut *(&raw mut outcome_state(g).g_hostGameOver));
        s.io(&mut *(&raw mut outcome_state(g).g_hostWinner));
    }
}
