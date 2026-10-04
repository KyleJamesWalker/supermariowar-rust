//! Not in the C++ (docs/PROGRESS.md, Netplay deviations): deaths, scores and the end of a net game.
//!
//! The harness dump records every death, every change of the team scores and the end of the game as `C`
//! records (docs/REPLAY.md), so net_game_compare.py can tell whether the clients agree on them.

use crate::common::game_values::game_values;
use crate::smw::harness;
use crate::smw::main::{score, score_cnt};
use crate::smw::net::netplay;

static mut g_lastScores: Vec<i16> = Vec::new();
static mut g_lastGameOver: bool = false;

pub fn begin_game() {
    unsafe {
        g_lastScores.clear();
        g_lastGameOver = false;
    }
}

pub fn note_death(player: i16, style: i32, removed: bool) {
    unsafe {
        if netplay.active {
            harness::note_net(format!("death p={} style={} removed={}", player, style, removed as i32));
        }
    }
}

/// Records the scores and the end of the game when they changed since the last call.
pub fn sample() {
    unsafe {
        if !netplay.active || game_values.gamemode.is_null() {
            return;
        }
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
