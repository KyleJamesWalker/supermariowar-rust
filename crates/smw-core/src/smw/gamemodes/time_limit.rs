//! Port of src/smw/gamemodes/TimeLimit.cpp

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::*;
use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::{DeathStyle, ScoringStyle};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::gamemodes::game_mode_timer::GameTimerDisplay;
use crate::smw::gs_gameplay::count_alive_teams;
use crate::smw::player::CPlayer;

//Timelimit
pub struct CGM_TimeLimit {
    pub cgame_mode: CGameMode,
    pub gameClock: GameTimerDisplay,
}
impl_base!(CGM_TimeLimit => cgame_mode: CGameMode);

//timelimit
impl CGM_TimeLimit {
    pub fn new() -> Self {
        let mut this = CGM_TimeLimit { cgame_mode: CGameMode::new(), gameClock: GameTimerDisplay::new() };
        this.goal = 60;
        this.gamemode = game_mode_timelimit;

        this.setup_mode_strings("Time Limit", "Time", 30);
        this
    }

    pub fn addtime(&mut self, iTime: i16) {
        if !self.gameover {
            self.gameClock.add_time(iTime);
        }
    }
}

impl CGameModeTrait for CGM_TimeLimit {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        cgm_timelimit_init(g, self)
    }
    fn think(&mut self, g: &mut Game) {
        cgm_timelimit_think(g, self)
    }
    fn draw_foreground(&mut self, g: &mut Game) {
        cgm_timelimit_draw_foreground(g, self)
    }
    fn playerkilledplayer(&mut self, g: &mut Game, inflictor: PlayerId, other: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_timelimit_playerkilledplayer(g, self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_timelimit_playerkilledself(g, self, player, style)
    }
}

pub fn cgm_timelimit_init(g: &mut Game, this: &mut CGM_TimeLimit) {
    cgamemode_init(g, this);

    if this.goal == -1 {
        this.gameClock.init(g, 0, false);
    } else {
        let goal = this.goal;
        this.gameClock.init(g, goal, true);
    }
}

pub fn cgm_timelimit_think(g: &mut Game, this: &mut CGM_TimeLimit) {
    cgamemode_think(g, this);
    let iTime: i16 = this.gameClock.run_clock();

    if this.goal > 0 {
        if iTime == 20 && !this.playedwarningsound {
            this.playwarningsound(g);
        }

        if iTime == 0 {
            //the game ends
            setup_score_board(g, false);
            show_score_board(g);

            remove_players_but_highest_scoring(g);
            this.gameover = true;

            count_alive_teams(g, Some(&mut this.cgame_mode.winningteam));
        }
    }
}

pub fn cgm_timelimit_playerkilledplayer(g: &mut Game, this: &mut CGM_TimeLimit, mut inflictor: PlayerId, mut other: PlayerId, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
    if !this.gameover {
        if g.mode().gamemode != game_mode_timelimit || g.game_values.gamemodesettings.time.scoring == ScoringStyle::AllKills || style == KillStyle::Push {
            //Penalize killing your team mates
            if g.player(inflictor).get_team_id() == g.player(other).get_team_id() {
                g.player(inflictor).score(g).adjust_score(g, -1);
            } else {
                g.player(inflictor).score(g).adjust_score(g, 1);
            }
        }

        if g.mode().gamemode == game_mode_timelimit && g.game_values.gamemodesettings.time.style == DeathStyle::Shield {
            if_sound_on_play(g, &mut gp.rm.sfx_powerdown);
            g.player(other).shield().reset(g);
            return PlayerKillType::NonKill;
        }
    }

    PlayerKillType::Normal
}

pub fn cgm_timelimit_playerkilledself(g: &mut Game, this: &mut CGM_TimeLimit, mut player: PlayerId, style: KillStyle) -> PlayerKillType {
    cgamemode_playerkilledself(g, this, player, style);

    if g.player(player).score(g).score > 0 && !this.gameover {
        g.player(player).score(g).adjust_score(g, -1);
    }

    PlayerKillType::Normal
}

pub fn cgm_timelimit_draw_foreground(g: &mut Game, this: &mut CGM_TimeLimit) {
    if !this.gameover {
        this.gameClock.draw(g);
    }
}
