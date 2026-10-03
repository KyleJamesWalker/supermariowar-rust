//! Port of src/smw/gamemodes/MiniBoss.cpp

use crate::common::game::App;
use crate::common::game_mode::{game_mode_boss_minigame, CGameMode, CGameModeTrait};
use crate::common::game_values::if_sound_on_play;
use crate::common::match_types::Boss;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::object_frenzycard;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, remove_team, setup_score_board, show_score_board};
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::{players, score, score_cnt};
use crate::smw::objects::moving::mo_frenzy_card::MO_FrenzyCard;
use crate::smw::objects::moving::mo_podobo::MO_Podobo;
use crate::smw::objects::moving::mo_sledge_brother::MO_SledgeBrother;
use crate::smw::objects::walkingenemy::we_koopa::MO_Koopa;
use crate::smw::player::CPlayer;
use std::any::Any;

//Boss Mode
//Person to score fatal hit to boss wins!
pub struct CGM_Boss_MiniGame {
    pub cgame_mode: CGameMode,

    enemytimer: i16,
    poweruptimer: i16,
    iBossType: Boss,
}
impl_base!(CGM_Boss_MiniGame => cgame_mode: CGameMode);

impl CGM_Boss_MiniGame {
    pub fn new() -> Self {
        let mut this = CGM_Boss_MiniGame { cgame_mode: CGameMode::new(), enemytimer: 0, poweruptimer: 0, iBossType: Boss::Hammer };
        this.gamemode = game_mode_boss_minigame;
        this.setup_mode_strings("Boss", "Lives", 5);
        this
    }

    pub fn set_winner(&mut self, player: Ptr<CPlayer>) -> bool {
        self.winningteam = player.get_team_id();
        self.gameover = true;

        remove_players_but_team(self.winningteam);

        unsafe {
            for iScore in 0..score_cnt {
                if self.winningteam == iScore {
                    continue;
                }

                score[iScore as usize].set_score(0);
            }

            setup_score_board(false);
            show_score_board();

            if game_values.music {
                rm.sfx_invinciblemusic.stop();
                rm.sfx_timewarning.stop();
                rm.sfx_slowdownmusic.stop();

                rm.backgroundmusic[1].play(true, false);
            }
        }

        //game_values.noexit = true;

        true
    }

    pub fn set_boss_type(&mut self, bosstype: Boss) {
        self.iBossType = bosstype;
    }

    pub fn get_boss_type(&self) -> Boss {
        self.iBossType
    }
}

impl CGameModeTrait for CGM_Boss_MiniGame {
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

        self.fReverseScoring = false;

        self.enemytimer = (RANDOM_INT(120) + 120) as i16;
        self.poweruptimer = 120;

        unsafe {
            for iScore in 0..score_cnt as usize {
                score[iScore].set_score(self.goal);
            }

            let platformY: i16 = if self.iBossType == Boss::Hammer {
                256
            } else if self.iBossType == Boss::Bomb {
                256
            } else {
                (App::screenWidth / 2) as i16
            };
            objectcontainer[0].add(Ptr::new_box(MO_SledgeBrother::new(Ptr::from_mut(&mut rm.spr_sledgebrothers), platformY, self.iBossType)));
        }
    }

    fn think(&mut self) {
        unsafe {
            if !self.gameover && players.len() == 0 {
                self.gameover = true;

                if game_values.music {
                    rm.sfx_invinciblemusic.stop();
                    rm.sfx_timewarning.stop();
                    rm.sfx_slowdownmusic.stop();
                    if_sound_on_play(&mut rm.sfx_gameover);

                    rm.backgroundmusic[1].stop();
                }
            }

            if self.gameover {
                self.displayplayertext();
            } else {
                if self.iBossType == Boss::Hammer {
                    //Randomly spawn koopas
                    self.enemytimer -= 1;
                    if self.enemytimer <= 0 {
                        objectcontainer[0].add(Ptr::new_box(MO_Koopa::new(Ptr::from_mut(&mut rm.spr_koopa), RANDOM_BOOL(), false, false, true)));
                        self.enemytimer = (RANDOM_INT(120) as i16 as i32 + 120) as i16; //Spawn koopas slowly
                    }
                } else if self.iBossType == Boss::Bomb {
                } else if self.iBossType == Boss::Fire {
                    //Only create podobos if the difficulty is moderate or greater
                    self.enemytimer -= 1;
                    if self.enemytimer <= 0 && game_values.gamemodesettings.boss.difficulty >= 2 {
                        let x = RANDOM_INT((App::screenWidth as f32 * 0.95f32) as i32) as i16;
                        let pos = Vec2s::new(x, App::screenHeight as i16);
                        let nspeed: f32 = -((RANDOM_INT(9) as f32) / 2.0f32) - 9.0f32;
                        objectcontainer[2].add(Ptr::new_box(MO_Podobo::new(Ptr::from_mut(&mut rm.spr_podobo), pos, nspeed, -1, -1, -1, false)));
                        self.enemytimer = (RANDOM_INT(80) + 60) as i16;
                    }

                    self.poweruptimer -= 1;
                    if self.poweruptimer <= 0 {
                        self.poweruptimer = (RANDOM_INT(80) + 60) as i16;

                        if objectcontainer[1].count_types(object_frenzycard) < players.len() {
                            objectcontainer[1].add(Ptr::new_box(MO_FrenzyCard::new(Ptr::from_mut(&mut rm.spr_frenzycards), 0)));
                        }
                    }
                }
            }
        }
    }

    fn draw_foreground(&mut self) {
        if self.gameover {
            if self.winningteam == -1 {
                unsafe {
                    rm.game_font_large.draw_centered(App::screenWidth / 2, 96, "You Failed To Defeat");

                    match self.iBossType {
                        Boss::Hammer => rm.game_font_large.draw_centered(App::screenWidth / 2, 118, "The Mighty Sledge Brother"),
                        Boss::Bomb => rm.game_font_large.draw_centered(App::screenWidth / 2, 118, "The Mighty Bomb Brother"),
                        Boss::Fire => rm.game_font_large.draw_centered(App::screenWidth / 2, 118, "The Mighty Flame Brother"),
                    }
                }
            }
        }
    }

    fn playerkilledplayer(&mut self, _inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        if !self.gameover {
            other.score().adjust_score(-1);

            if !self.playedwarningsound {
                let mut countscore: i16 = 0;
                unsafe {
                    for k in 0..score_cnt as usize {
                        countscore = countscore.wrapping_add(score[k].score);
                    }
                }

                if countscore <= 2 {
                    self.playwarningsound();
                }
            }

            if other.score().score <= 0 {
                remove_team(other.get_team_id());
                return PlayerKillType::Removed;
            }
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, mut player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style);

        if !self.gameover {
            player.score().adjust_score(-1);

            if !self.playedwarningsound {
                let mut countscore: i16 = 0;
                unsafe {
                    for k in 0..score_cnt as usize {
                        countscore = countscore.wrapping_add(score[k].score);
                    }
                }

                if countscore <= 2 {
                    self.playwarningsound();
                }
            }

            if player.score().score <= 0 {
                remove_team(player.get_team_id());
                return PlayerKillType::Removed;
            }
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(iType);
        }
    }

    fn has_stored_powerups(&mut self) -> bool {
        false
    }
}
