//! Port of src/smw/gamemodes/Eggs.cpp

use crate::common::game_mode::*;
use crate::common::moving_object_types::movingobject_egg;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::*;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::carriable::co_egg::CO_Egg;
use crate::smw::objects::moving::mo_yoshi::MO_Yoshi;
use crate::smw::player::CPlayer;

pub struct CGM_Eggs {
    pub cgame_mode: CGameMode,
}
impl_base!(CGM_Eggs => cgame_mode: CGameMode);

//Egg mode:
//Grab the egg and return it to Yoshi
//Score 1 point for each
impl CGM_Eggs {
    pub fn new() -> Self {
        let mut this = CGM_Eggs { cgame_mode: CGameMode::new() };
        this.goal = 20;
        this.gamemode = game_mode_eggs;

        this.setup_mode_strings("Yoshi's Eggs", "Eggs", 5);
        this
    }
}

impl CGameModeTrait for CGM_Eggs {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self) {
        cgamemode_init(self);

        //Verify that at least 1 matching yoshi and egg exist
        let mut fEgg: [bool; 4] = [false, false, false, false];
        let mut fAtLeastOneMatch = false;

        unsafe {
            for iEggs in 0..4i16 {
                for iEgg in 0..game_values.gamemodesettings.egg.eggs[iEggs as usize] {
                    if iEgg > 9 {
                        break;
                    }

                    fEgg[iEggs as usize] = true;
                    objectcontainer[1].add(Ptr::new_box(CO_Egg::new(Ptr::from_mut(&mut rm.spr_egg), iEggs)));
                }
            }

            for iYoshis in 0..4i16 {
                for iYoshi in 0..game_values.gamemodesettings.egg.yoshis[iYoshis as usize] {
                    if iYoshi > 9 {
                        break;
                    }

                    if fEgg[iYoshis as usize] {
                        fAtLeastOneMatch = true;
                    }

                    objectcontainer[1].add(Ptr::new_box(MO_Yoshi::new(Ptr::from_mut(&mut rm.spr_yoshi), iYoshis)));
                }
            }

            if !fAtLeastOneMatch {
                objectcontainer[1].add(Ptr::new_box(CO_Egg::new(Ptr::from_mut(&mut rm.spr_egg), 1)));
                objectcontainer[1].add(Ptr::new_box(MO_Yoshi::new(Ptr::from_mut(&mut rm.spr_yoshi), 1)));
            }
        }
    }

    fn playerkilledplayer(&mut self, _inflictor: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgamemode_playerkilledself(self, player, style);

        let mut item = player.carriedItem;
        if !item.is_null() && item.get_moving_object_type() == movingobject_egg {
            item.as_any().downcast_mut::<CO_Egg>().expect("carried egg").place_egg();
        }

        PlayerKillType::Normal
    }

    fn playerextraguy(&mut self, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(iType);
            self.check_winner(player);
        }
    }

    fn check_winner(&mut self, mut player: Ptr<CPlayer>) -> PlayerKillType {
        if self.goal > -1 {
            if player.score().score >= self.goal {
                player.score().set_score(self.goal);

                self.winningteam = player.get_team_id();
                self.gameover = true;

                remove_players_but_team(self.winningteam);
                setup_score_board(false);
                show_score_board();
            } else if player.score().score as i32 >= self.goal as i32 - 2 && !self.playedwarningsound {
                self.playwarningsound();
            }
        }

        PlayerKillType::Normal
    }
}
