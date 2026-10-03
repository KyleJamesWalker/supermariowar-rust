//! Port of src/smw/gamemodes/ShyGuyTag.cpp

use crate::common::eyecandy::{EC_GravText, EC_SingleAnimation};
use crate::common::game_mode::{game_mode_shyguytag, CGameMode, CGameModeTrait};
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::VELJUMP;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_highest_scoring, setup_score_board, show_score_board};
use crate::smw::gamemodes::game_mode_timer::GameTimerDisplay;
use crate::smw::gs_gameplay::{count_alive_teams, eyecandy};
use crate::smw::main::{players, score, score_cnt};
use crate::smw::player::CPlayer;
use std::any::Any;

//shyguy tag mode
//First player killed becomes the shyguy
//He can then tag other players to also become shy guys
//Players that are not shy guys will be slowly scoring points
//When all players become shyguys, then the mode is reset with no shyguys
pub struct CGM_ShyGuyTag {
    pub cgame_mode: CGameMode,

    gameClock: GameTimerDisplay,
    fRunClock: bool,
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

    pub fn set_shy_guy(&mut self, iTeam: i16) {
        unsafe {
            for &player in players.iter() {
                let mut player = player;
                if player.get_team_id() == iTeam {
                    player.shyguy = true;
                    eyecandy[2].emplace(EC_GravText::new(
                        Ptr::from_mut(&mut rm.game_font_large),
                        player.center_x(),
                        player.bottom_y(),
                        "Shyguy!".to_string(),
                        (-(VELJUMP as f64) * 1.5) as f32,
                    ));
                    eyecandy[2].emplace(EC_SingleAnimation::new(
                        Ptr::from_mut(&mut rm.spr_fireballexplosion),
                        (player.center_x() as i32 - 16) as i16,
                        (player.center_y() as i32 - 16) as i16,
                        3,
                        8,
                    ));

                    player.strip_powerups();
                    player.clear_powerup_states();
                }
            }

            if_sound_on_play(&mut rm.sfx_transform);
        }
    }

    fn free_shy_guys(&mut self) {
        unsafe {
            if_sound_on_play(&mut rm.sfx_thunder);

            for &player in players.iter() {
                let mut player = player;
                player.shyguy = false;
                eyecandy[2].emplace(EC_SingleAnimation::new(
                    Ptr::from_mut(&mut rm.spr_fireballexplosion),
                    (player.center_x() as i32 - 16) as i16,
                    (player.center_y() as i32 - 16) as i16,
                    3,
                    8,
                ));
            }
        }
    }

    fn count_shy_guys(&mut self) -> i16 {
        let mut shyguycount: i16 = 0;
        unsafe {
            for &player in players.iter() {
                if player.shyguy {
                    shyguycount += 1;
                }
            }
        }

        shyguycount
    }
}

impl CGameModeTrait for CGM_ShyGuyTag {
    fn gm(&self) -> &CGameMode {
        &self.cgame_mode
    }
    fn gm_mut(&mut self) -> &mut CGameMode {
        &mut self.cgame_mode
    }
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn init(&mut self) {
        gm::cgamemode_init(self);
        self.fReverseScoring = self.goal == -1;

        self.fRunClock = false;
        self.gameClock.init(0, true);

        unsafe {
            for iScore in 0..score_cnt as usize {
                score[iScore].set_score(0);
            }

            for &player in players.iter() {
                let mut player = player;
                player.ownerColorOffsetX = (player.get_color_id() as i32 * 48) as i16;
            }
        }
    }

    fn think(&mut self) {
        if self.gameover {
            self.displayplayertext();
            return;
        }

        unsafe {
            //See how many players are shy guys
            let shyguycount: i16 = self.count_shy_guys();

            //If we are not waiting to clear, check if we need to start waiting
            if !self.fRunClock {
                if shyguycount as usize == players.len() {
                    if game_values.gamemodesettings.shyguytag.freetime > 0 {
                        self.fRunClock = true;
                        self.gameClock.set_time(game_values.gamemodesettings.shyguytag.freetime);
                        if_sound_on_play(&mut rm.sfx_starwarning);
                    } else {
                        self.free_shy_guys();
                    }
                }
            } else {
                let iTime: i16 = self.gameClock.run_clock();

                if iTime == 0 {
                    //Clear the shy guys
                    self.fRunClock = false;
                    self.free_shy_guys();
                } else if iTime > 0 {
                    if_sound_on_play(&mut rm.sfx_starwarning);
                }
            }

            //Award points to non shyguys
            if shyguycount > 0 {
                self.scorecounter += 1;
                if self.scorecounter >= game_values.pointspeed {
                    self.scorecounter = 0;

                    let mut pCheckWinner: Ptr<CPlayer> = Ptr::null();
                    let mut fAlreadyScored: [bool; 4] = [false, false, false, false];
                    for &player in players.iter() {
                        let mut player = player;
                        if !player.shyguy {
                            let iTeam: i16 = player.get_team_id();
                            if !fAlreadyScored[iTeam as usize] {
                                fAlreadyScored[iTeam as usize] = true;
                                player.score().adjust_score(shyguycount);

                                pCheckWinner = player;
                            }
                        }
                    }

                    if !pCheckWinner.is_null() && !self.fReverseScoring {
                        self.check_winner(pCheckWinner);
                    }
                }
            }
        }
    }

    //Draw count down timer here
    fn draw_foreground(&mut self) {
        if self.fRunClock {
            self.gameClock.draw();
        }
    }

    fn playerkilledplayer(&mut self, _inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        if self.gameover || other.shyguy {
            return PlayerKillType::Normal;
        }

        if self.count_shy_guys() == 0 || unsafe { game_values.gamemodesettings.shyguytag.tagtransfer } != 0 {
            self.set_shy_guy(other.get_team_id());
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style);

        if !self.gameover && unsafe { game_values.gamemodesettings.shyguytag.tagonsuicide } {
            self.set_shy_guy(player.get_team_id());
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score((10 * iType as i32) as i16);
            self.check_winner(player);
        }
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.gameover || self.goal == -1 {
            return PlayerKillType::Normal;
        }

        if player.score().score >= self.goal {
            player.score().set_score(self.goal);
            setup_score_board(false);
            show_score_board();

            remove_players_but_highest_scoring();
            self.gameover = true;

            count_alive_teams(Some(&mut self.cgame_mode.winningteam));
        } else if !self.playedwarningsound && self.goal != -1 && player.score().score as f64 >= self.goal as f64 * 0.8 {
            self.playwarningsound();
        }

        PlayerKillType::Normal
    }
}
