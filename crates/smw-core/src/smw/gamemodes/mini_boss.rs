//! Port of src/smw/gamemodes/MiniBoss.cpp

use crate::game::Game;
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

    pub fn set_winner(&mut self, g: &mut Game, player: Ptr<CPlayer>) -> bool {
        let mut gp = g.ptr();
        self.winningteam = player.get_team_id();
        self.gameover = true;

        remove_players_but_team(g, self.winningteam);

        unsafe {
            for iScore in 0..game().score_cnt {
                if self.winningteam == iScore {
                    continue;
                }

                gp.score[iScore as usize].set_score(g, 0);
            }

            setup_score_board(g, false);
            show_score_board(g);

            if g.game_values.music {
                g.rm.sfx_invinciblemusic.stop();
                g.rm.sfx_timewarning.stop();
                g.rm.sfx_slowdownmusic.stop();

                g.rm.backgroundmusic[1].play(true, false);
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

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut gp = g.ptr();
        gm::cgamemode_init(g, self);

        self.fReverseScoring = false;

        self.enemytimer = (RANDOM_INT(120) + 120) as i16;
        self.poweruptimer = 120;

        unsafe {
            for iScore in 0..game().score_cnt as usize {
                gp.score[iScore].set_score(g, self.goal);
            }

            let platformY: i16 = if self.iBossType == Boss::Hammer {
                256
            } else if self.iBossType == Boss::Bomb {
                256
            } else {
                (App::screenWidth / 2) as i16
            };
            gp.get().objectcontainer[0].add(Ptr::new_box(MO_SledgeBrother::new(g, Ptr::from_mut(&mut gp.get().rm.spr_sledgebrothers), platformY, self.iBossType)));
        }
    }

    fn think(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
        let mut gp = g.ptr();
        unsafe {
            if !self.gameover && g.players.len() == 0 {
                self.gameover = true;

                if g.game_values.music {
                    g.rm.sfx_invinciblemusic.stop();
                    g.rm.sfx_timewarning.stop();
                    g.rm.sfx_slowdownmusic.stop();
                    if_sound_on_play(g, &mut gp.rm.sfx_gameover);

                    g.rm.backgroundmusic[1].stop();
                }
            }

            if self.gameover {
                self.displayplayertext(g);
            } else {
                if self.iBossType == Boss::Hammer {
                    //Randomly spawn koopas
                    self.enemytimer -= 1;
                    if self.enemytimer <= 0 {
                        gp.objectcontainer[0].add(Ptr::new_box(MO_Koopa::new(g, Ptr::from_mut(&mut rm.spr_koopa), RANDOM_BOOL(), false, false, true)));
                        self.enemytimer = (RANDOM_INT(120) as i16 as i32 + 120) as i16; //Spawn koopas slowly
                    }
                } else if self.iBossType == Boss::Bomb {
                } else if self.iBossType == Boss::Fire {
                    //Only create podobos if the difficulty is moderate or greater
                    self.enemytimer -= 1;
                    if self.enemytimer <= 0 && g.game_values.gamemodesettings.boss.difficulty >= 2 {
                        let x = RANDOM_INT((App::screenWidth as f32 * 0.95f32) as i32) as i16;
                        let pos = Vec2s::new(x, App::screenHeight as i16);
                        let nspeed: f32 = -((RANDOM_INT(9) as f32) / 2.0f32) - 9.0f32;
                        gp.objectcontainer[2].add(Ptr::new_box(MO_Podobo::new(g, Ptr::from_mut(&mut rm.spr_podobo), pos, nspeed, -1, -1, -1, false)));
                        self.enemytimer = (RANDOM_INT(80) + 60) as i16;
                    }

                    self.poweruptimer -= 1;
                    if self.poweruptimer <= 0 {
                        self.poweruptimer = (RANDOM_INT(80) + 60) as i16;

                        if g.objectcontainer[1].count_types(object_frenzycard) < g.players.len() {
                            gp.get().objectcontainer[1].add(Ptr::new_box(MO_FrenzyCard::new(g, Ptr::from_mut(&mut gp.get().rm.spr_frenzycards), 0)));
                        }
                    }
                }
            }
        }
    }

    fn draw_foreground(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.gameover {
            if self.winningteam == -1 {
                unsafe {
                    gp.rm.game_font_large.draw_centered(g, App::screenWidth / 2, 96, "You Failed To Defeat");

                    match self.iBossType {
                        Boss::Hammer => gp.rm.game_font_large.draw_centered(g, App::screenWidth / 2, 118, "The Mighty Sledge Brother"),
                        Boss::Bomb => gp.rm.game_font_large.draw_centered(g, App::screenWidth / 2, 118, "The Mighty Bomb Brother"),
                        Boss::Fire => gp.rm.game_font_large.draw_centered(g, App::screenWidth / 2, 118, "The Mighty Flame Brother"),
                    }
                }
            }
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, _inflictor: Ptr<CPlayer>, mut other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        if !self.gameover {
            other.score().adjust_score(g, -1);

            if !self.playedwarningsound {
                let mut countscore: i16 = 0;
                unsafe {
                    for k in 0..game().score_cnt as usize {
                        countscore = countscore.wrapping_add(g.score[k].score);
                    }
                }

                if countscore <= 2 {
                    self.playwarningsound(g);
                }
            }

            if other.score().score <= 0 {
                remove_team(g, other.get_team_id());
                return PlayerKillType::Removed;
            }
        }

        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style);

        if !self.gameover {
            player.score().adjust_score(g, -1);

            if !self.playedwarningsound {
                let mut countscore: i16 = 0;
                unsafe {
                    for k in 0..game().score_cnt as usize {
                        countscore = countscore.wrapping_add(g.score[k].score);
                    }
                }

                if countscore <= 2 {
                    self.playwarningsound(g);
                }
            }

            if player.score().score <= 0 {
                remove_team(g, player.get_team_id());
                return PlayerKillType::Removed;
            }
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(g, iType);
        }
    }

    fn has_stored_powerups(&mut self) -> bool {
        false
    }
}
