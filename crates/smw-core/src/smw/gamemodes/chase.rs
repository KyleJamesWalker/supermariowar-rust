//! Port of src/smw/gamemodes/Chase.cpp

use crate::game::Game;
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
use crate::smw::objects::carriable::co_phanto_key::CO_PhantoKey;
use crate::smw::objects::overmap::wo_phanto::OMO_Phanto;
use crate::smw::player::CPlayer;
use std::any::Any;

//Chase (hold a key for points while phantos chase you)
pub struct CGM_Chase {
    pub cgame_mode: CGameMode,

    key: Ptr<CO_PhantoKey>,    /// Frames toward the next point; never reset between games, like the C++ function-local static.
    counter: i16,
}
impl_base!(CGM_Chase => cgame_mode: CGameMode);

impl CGM_Chase {
    pub fn new() -> Self {
        let mut this = CGM_Chase { cgame_mode: CGameMode::new(), key: Ptr::null(), counter: 0 };
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

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut gp = g.ptr();
        gm::cgamemode_init(g, self);

        unsafe {
            //Add phantos based on settings
            for iPhanto in 0..3i16 {
                let mut iNumPhantos: i16 = 0;
                while iNumPhantos < g.game_values.gamemodesettings.chase.phantoquantity[iPhanto as usize] {
                    let x = RANDOM_INT(App::screenWidth);
                    let y = if RANDOM_BOOL() { -32 - CRUNCHMAX } else { App::screenHeight };
                    let obj = Ptr::new_box(OMO_Phanto::new(g, Ptr::from_mut(&mut gp.get().rm.spr_phanto), Vec2s::new(x as i16, y as i16), 0.0f32, 0.0f32, iPhanto));
                    gp.get().objectcontainer[1].add(g, obj);
                    iNumPhantos += 1;
                }
            }

            //Add a key
            self.key = Ptr::new_box(CO_PhantoKey::new(g, Ptr::from_mut(&mut gp.rm.spr_phantokey)));
            gp.objectcontainer[1].add(g, self.key);
        }
    }

    fn think(&mut self, g: &mut Game) {
        if self.gameover {
            self.displayplayertext(g);
            return;
        }

        let mut keyholder: Ptr<CPlayer> = self.key.owner;
        if !keyholder.is_null() {
            if keyholder.isready() && !keyholder.is_tanooki_statue() {
                unsafe {
                    self.counter += 1;
                    if self.counter >= g.game_values.pointspeed {
                        self.counter = 0;
                        keyholder.score().adjust_score(g, 1);
                        self.check_winner(g, keyholder);
                    }
                }
            }
        }
    }

    fn playerkilledplayer(&mut self, g: &mut Game, _player: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style);

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(g, (5 * iType as i32) as i16);
            self.check_winner(g, player);
        }
    }

    fn check_winner(&mut self, g: &mut Game, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.goal == -1 {
            return PlayerKillType::Normal;
        }

        if player.score().score >= self.goal {
            player.score().set_score(g, self.goal);
            self.winningteam = player.get_team_id();
            self.gameover = true;

            setup_score_board(g, false);
            show_score_board(g);
            remove_players_but_team(g, self.winningteam);

            return PlayerKillType::Removed;
        } else if player.score().score as f64 >= self.goal as f64 * 0.8 && !self.playedwarningsound {
            self.playwarningsound(g);
        }

        PlayerKillType::Normal
    }
}
