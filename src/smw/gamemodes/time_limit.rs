//! Port of src/smw/gamemodes/TimeLimit.cpp

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

    fn init(&mut self) {
        cgm_timelimit_init(self)
    }
    fn think(&mut self) {
        cgm_timelimit_think(self)
    }
    fn draw_foreground(&mut self) {
        cgm_timelimit_draw_foreground(self)
    }
    fn playerkilledplayer(&mut self, inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_timelimit_playerkilledplayer(self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_timelimit_playerkilledself(self, player, style)
    }
}

pub fn cgm_timelimit_init(this: &mut CGM_TimeLimit) {
    cgamemode_init(this);

    if this.goal == -1 {
        this.gameClock.init(0, false);
    } else {
        let goal = this.goal;
        this.gameClock.init(goal, true);
    }
}

pub fn cgm_timelimit_think(this: &mut CGM_TimeLimit) {
    cgamemode_think(this);
    let iTime: i16 = this.gameClock.run_clock();

    if this.goal > 0 {
        if iTime == 20 && !this.playedwarningsound {
            this.playwarningsound();
        }

        if iTime == 0 {
            //the game ends
            setup_score_board(false);
            show_score_board();

            remove_players_but_highest_scoring();
            this.gameover = true;

            count_alive_teams(Some(&mut this.cgame_mode.winningteam));
        }
    }
}

pub fn cgm_timelimit_playerkilledplayer(this: &mut CGM_TimeLimit, mut inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
    if !this.gameover {
        unsafe {
            if game_values.gamemode.gamemode != game_mode_timelimit || game_values.gamemodesettings.time.scoring == ScoringStyle::AllKills || style == KillStyle::Push {
                //Penalize killing your team mates
                if inflictor.get_team_id() == other.get_team_id() {
                    inflictor.score().adjust_score(-1);
                } else {
                    inflictor.score().adjust_score(1);
                }
            }

            if game_values.gamemode.gamemode == game_mode_timelimit && game_values.gamemodesettings.time.style == DeathStyle::Shield {
                if_sound_on_play(&mut rm.sfx_powerdown);
                other.shield().reset();
                return PlayerKillType::NonKill;
            }
        }
    }

    PlayerKillType::Normal
}

pub fn cgm_timelimit_playerkilledself(this: &mut CGM_TimeLimit, mut player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
    cgamemode_playerkilledself(this, player, style);

    if player.score().score > 0 && !this.gameover {
        player.score().adjust_score(-1);
    }

    PlayerKillType::Normal
}

pub fn cgm_timelimit_draw_foreground(this: &mut CGM_TimeLimit) {
    if !this.gameover {
        this.gameClock.draw();
    }
}
