//! Shy guy tag mode: tagged players become shy guys while everyone else scores.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::eyecandy::{EC_GravText, EC_SingleAnimation};
use crate::common::game_mode::{game_mode_shyguytag, CGameMode, CGameModeTrait};
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::{ticks, VELJUMP};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_highest_scoring, setup_score_board, show_score_board};
use crate::smw::gamemodes::game_mode_timer::GameTimerDisplay;
use crate::smw::gs_gameplay::{count_alive_teams};

use crate::smw::player::CPlayer;
use std::any::Any;

/// Shy guy tag. The first player killed becomes a shy guy and tags others by killing them.
/// Non-shy-guy teams score one point per shy guy every `ticks(pointspeed)` ticks; once everyone is
/// a shy guy, all are freed after `freetime` seconds.
#[derive(Clone)]
pub struct CGM_ShyGuyTag {
    pub cgame_mode: CGameMode,

    /// Countdown to freeing the shy guys once everyone is one.
    gameClock: GameTimerDisplay,
    /// True while `gameClock` is counting down.
    fRunClock: bool,
    /// Ticks toward the next point payout.
    scorecounter: i16,
}
impl_base!(CGM_ShyGuyTag => cgame_mode: CGameMode);

impl CGM_ShyGuyTag {
    pub fn new() -> Self {
        let mut this = CGM_ShyGuyTag { cgame_mode: CGameMode::new(), gameClock: GameTimerDisplay::new(), fRunClock: false, scorecounter: 0 };
        this.goal = 200;
        this.gamemode = game_mode_shyguytag;

        this.setup_mode_strings("Shyguy Tag", "Points", 50);
        this.scorecounter = 0;
        this
    }

    /// Turns every player on `iTeam` into a shy guy, stripping their powerups.
    pub fn set_shy_guy(&mut self, g: &mut Game, iTeam: i16) {
        let mut gp = g.ptr();
        for &player in g.players.iter() {
            let mut player = player;
            if g.player(player).get_team_id() == iTeam {
                g.player(player).shyguy = true;
                g.eyecandy[2].emplace(EC_GravText::new(
                    Ptr::from_mut(&mut g.rm.game_font_large),
                    g.player(player).center_x(),
                    g.player(player).bottom_y(),
                    "Shyguy!".to_string(),
                    (-(VELJUMP as f64) * 1.5) as f32,
                ));
                g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                    g.rm.spr_fireballexplosion,
                    (g.player(player).center_x() as i32 - 16) as i16,
                    (g.player(player).center_y() as i32 - 16) as i16,
                    3,
                    8,
                ));

                g.player(player).strip_powerups();
                g.player(player).clear_powerup_states();
            }
        }

        if_sound_on_play(g, &mut gp.rm.sfx_transform);
    }

    fn free_shy_guys(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if_sound_on_play(g, &mut gp.rm.sfx_thunder);

        for &player in g.players.iter() {
            let mut player = player;
            g.player(player).shyguy = false;
            g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                g.rm.spr_fireballexplosion,
                (g.player(player).center_x() as i32 - 16) as i16,
                (g.player(player).center_y() as i32 - 16) as i16,
                3,
                8,
            ));
        }
    }

    fn count_shy_guys(&mut self, g: &mut Game) -> i16 {
        let mut shyguycount: i16 = 0;
        for &player in g.players.iter() {
            if g.player(player).shyguy {
                shyguycount += 1;
            }
        }

        shyguycount
    }
}

impl CGameModeTrait for CGM_ShyGuyTag {
    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.gm().harness_hash(h);
        self.gameClock.harness_hash(h);
        h.bools(&[self.fRunClock]);
        h.i16s(&[self.scorecounter]);
    }
    fn gm(&self) -> &CGameMode {
        &self.cgame_mode
    }
    fn gm_mut(&mut self) -> &mut CGameMode {
        &mut self.cgame_mode
    }
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        gm::cgamemode_init(g, self);
        self.fReverseScoring = self.goal == -1;

        self.fRunClock = false;
        self.gameClock.init(g, 0, true);

        for iScore in 0..g.score_cnt as usize {
            gp.score[iScore].set_score(g, 0);
        }

        for &player in g.players.iter() {
            let mut player = player;
            g.player(player).ownerColorOffsetX = (g.player(player).get_color_id() as i32 * 48) as i16;
        }
    }

    fn think(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.gameover {
            self.displayplayertext(g);
            return;
        }

        //See how many players are shy guys
        let shyguycount: i16 = self.count_shy_guys(g);

        //If we are not waiting to clear, check if we need to start waiting
        if !self.fRunClock {
            if shyguycount as usize == g.players.len() {
                if g.game_values.gamemodesettings.shyguytag.freetime > 0 {
                    self.fRunClock = true;
                    self.gameClock.set_time(g.game_values.gamemodesettings.shyguytag.freetime);
                    if_sound_on_play(g, &mut gp.rm.sfx_starwarning);
                } else {
                    self.free_shy_guys(g);
                }
            }
        } else {
            let iTime: i16 = self.gameClock.run_clock();

            if iTime == 0 {
                //Clear the shy guys
                self.fRunClock = false;
                self.free_shy_guys(g);
            } else if iTime > 0 {
                if_sound_on_play(g, &mut gp.rm.sfx_starwarning);
            }
        }

        //Award points to non shyguys
        if shyguycount > 0 {
            self.scorecounter += 1;
            if self.scorecounter as i32 >= ticks(g.game_values.pointspeed as i32) {
                self.scorecounter = 0;

                let mut pCheckWinner: PlayerId = PlayerId::NONE;
                let mut fAlreadyScored: [bool; 4] = [false, false, false, false];
                for &player in gp.players.iter() {
                    let mut player = player;
                    if !g.player(player).shyguy {
                        let iTeam: i16 = g.player(player).get_team_id();
                        if !fAlreadyScored[iTeam as usize] {
                            fAlreadyScored[iTeam as usize] = true;
                            g.player(player).score(g).adjust_score(g, shyguycount);

                            pCheckWinner = player;
                        }
                    }
                }

                if !pCheckWinner.is_none() && !self.fReverseScoring {
                    self.check_winner(g, pCheckWinner);
                }
            }
        }
    }

    fn draw_foreground(&mut self, g: &mut Game) {
        if self.fRunClock {
            self.gameClock.draw(g);
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, _inflictor: PlayerId, other: PlayerId, _style: KillStyle) -> PlayerKillType {
        if self.gameover || g.player(other).shyguy {
            return PlayerKillType::Normal;
        }

        if self.count_shy_guys(g) == 0 || g.game_values.gamemodesettings.shyguytag.tagtransfer != 0 {
            self.set_shy_guy(g, g.player(other).get_team_id());
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(g, self, player, style);

        if !self.gameover && g.game_values.gamemodesettings.shyguytag.tagonsuicide {
            self.set_shy_guy(g, g.player(player).get_team_id());
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: PlayerId, iType: i16) {
        if !self.gameover {
            g.player(player).score(g).adjust_score(g, (10 * iType as i32) as i16);
            self.check_winner(g, player);
        }
    }

    fn check_winner(&mut self, g: &mut Game, mut player: PlayerId) -> PlayerKillType {
        if self.gameover || self.goal == -1 {
            return PlayerKillType::Normal;
        }

        if g.player(player).score(g).score >= self.goal {
            g.player(player).score(g).set_score(g, self.goal);
            setup_score_board(g, false);
            show_score_board(g);

            remove_players_but_highest_scoring(g);
            self.gameover = true;

            count_alive_teams(g, Some(&mut self.cgame_mode.winningteam));
        } else if !self.playedwarningsound && self.goal != -1 && g.player(player).score(g).score as f64 >= self.goal as f64 * 0.8 {
            self.playwarningsound(g);
        }

        PlayerKillType::Normal
    }
}
