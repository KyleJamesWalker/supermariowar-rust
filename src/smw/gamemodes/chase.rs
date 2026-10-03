//! Port of src/smw/gamemodes/Chase.cpp

use crate::common::game::App;
use crate::common::game_mode::{game_mode_chase, CGameMode, CGameModeTrait};
use crate::common::global_constants::CRUNCHMAX;
use crate::common::math::vec2::Vec2s;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::{RANDOM_BOOL, RANDOM_INT};
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::carriable::co_phanto_key::CO_PhantoKey;
use crate::smw::objects::overmap::wo_phanto::OMO_Phanto;
use crate::smw::player::CPlayer;
use std::any::Any;

//Chase (hold a key for points while phantos chase you)
pub struct CGM_Chase {
    pub cgame_mode: CGameMode,

    key: Ptr<CO_PhantoKey>,
}
impl_base!(CGM_Chase => cgame_mode: CGameMode);

impl CGM_Chase {
    pub fn new() -> Self {
        let mut this = CGM_Chase { cgame_mode: CGameMode::new(), key: Ptr::null() };
        this.goal = 200;
        this.gamemode = game_mode_chase;

        this.setup_mode_strings("Phanto", "Points", 50);
        this
    }

    pub fn get_key_holder(&mut self) -> Ptr<CPlayer> {
        self.key.owner
    }
}

impl CGameModeTrait for CGM_Chase {
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

        unsafe {
            //Add phantos based on settings
            for iPhanto in 0..3i16 {
                let mut iNumPhantos: i16 = 0;
                while iNumPhantos < game_values.gamemodesettings.chase.phantoquantity[iPhanto as usize] {
                    let x = RANDOM_INT(App::screenWidth);
                    let y = if RANDOM_BOOL() { -32 - CRUNCHMAX } else { App::screenHeight };
                    objectcontainer[1].add(Ptr::new_box(OMO_Phanto::new(Ptr::from_mut(&mut rm.spr_phanto), Vec2s::new(x as i16, y as i16), 0.0f32, 0.0f32, iPhanto)));
                    iNumPhantos += 1;
                }
            }

            //Add a key
            self.key = Ptr::new_box(CO_PhantoKey::new(Ptr::from_mut(&mut rm.spr_phantokey)));
            objectcontainer[1].add(self.key);
        }
    }

    fn think(&mut self) {
        if self.gameover {
            self.displayplayertext();
            return;
        }

        let mut keyholder: Ptr<CPlayer> = self.key.owner;
        if !keyholder.is_null() {
            static mut counter: i16 = 0;

            if keyholder.isready() && !keyholder.is_tanooki_statue() {
                unsafe {
                    counter += 1;
                    if counter >= game_values.pointspeed {
                        counter = 0;
                        keyholder.score().adjust_score(1);
                        self.check_winner(keyholder);
                    }
                }
            }
        }
    }

    fn playerkilledplayer(&mut self, _player: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style);

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score((5 * iType as i32) as i16);
            self.check_winner(player);
        }
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.goal == -1 {
            return PlayerKillType::Normal;
        }

        if player.score().score >= self.goal {
            player.score().set_score(self.goal);
            self.winningteam = player.get_team_id();
            self.gameover = true;

            setup_score_board(false);
            show_score_board();
            remove_players_but_team(self.winningteam);

            return PlayerKillType::Removed;
        } else if player.score().score as f64 >= self.goal as f64 * 0.8 && !self.playedwarningsound {
            self.playwarningsound();
        }

        PlayerKillType::Normal
    }
}
