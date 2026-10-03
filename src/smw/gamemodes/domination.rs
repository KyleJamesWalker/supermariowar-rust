//! Port of src/smw/gamemodes/Domination.cpp

use crate::common::game_mode::{game_mode_domination, CGameMode, CGameModeTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::players;
use crate::smw::object_container::CObjectContainer;
use crate::smw::objects::overmap::wo_area::OMO_Area;
use crate::smw::player::CPlayer;
use std::any::Any;

fn adjust_player_areas(container: &CObjectContainer, player: Ptr<CPlayer>, other: Ptr<CPlayer>) {
    unsafe {
        let n = container.list().len();
        for i in 0..n {
            let obj = container.list()[i];
            let Some(area) = obj.get().as_any().downcast_mut::<OMO_Area>() else {
                continue;
            };

            if area.get_color_id() == other.get_color_id() {
                if game_values.gamemodesettings.domination.relocateondeath {
                    area.place_area();
                }

                if game_values.gamemodesettings.domination.stealondeath && !player.is_null() {
                    area.set_owner(player);
                } else if game_values.gamemodesettings.domination.loseondeath {
                    area.reset();
                }
            }
        }
    }
}

//Domination (capture the area blocks)
//Touch all the dotted blocks to turn them your color
//The more blocks you have, the faster you rack up points
pub struct CGM_Domination {
    pub cgame_mode: CGameMode,
}
impl_base!(CGM_Domination => cgame_mode: CGameMode);

impl CGM_Domination {
    pub fn new() -> Self {
        let mut this = CGM_Domination { cgame_mode: CGameMode::new() };
        this.goal = 200;
        this.gamemode = game_mode_domination;

        this.setup_mode_strings("Domination", "Points", 50);
        this
    }
}

pub fn cgm_domination_init<T: CGameModeTrait + ?Sized>(this: &mut T) {
    gm::cgamemode_init(this);

    unsafe {
        let mut iNumAreas: i16 = game_values.gamemodesettings.domination.quantity;

        if iNumAreas < 1 {
            iNumAreas = 1;
            game_values.gamemodesettings.domination.quantity = iNumAreas;
        }

        if iNumAreas > 18 {
            iNumAreas = 2usize.wrapping_mul(players.len()).wrapping_add(iNumAreas as usize).wrapping_sub(22) as i16;
        } else if iNumAreas > 10 {
            iNumAreas = players.len().wrapping_add(iNumAreas as usize).wrapping_sub(12) as i16;
        }

        for _k in 0..iNumAreas {
            objectcontainer[0].add(Ptr::new_box(OMO_Area::new(Ptr::from_mut(&mut rm.spr_areas), iNumAreas)));
        }
    }
}

pub fn cgm_domination_playerkilledplayer<T: CGameModeTrait + ?Sized>(_this: &mut T, player: Ptr<CPlayer>, other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
    //Update areas the dead player owned
    unsafe { adjust_player_areas(&objectcontainer[0], player, other) };

    PlayerKillType::Normal
}

pub fn cgm_domination_playerkilledself<T: CGameModeTrait + ?Sized>(this: &mut T, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
    gm::cgamemode_playerkilledself(this, player, style);

    //Update areas the dead player owned
    unsafe { adjust_player_areas(&objectcontainer[0], Ptr::null(), player) };

    PlayerKillType::Normal
}

pub fn cgm_domination_playerextraguy<T: CGameModeTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>, iType: i16) {
    if !this.gm().gameover {
        player.score().adjust_score((10 * iType as i32) as i16);
        this.check_winner(player);
    }
}

pub fn cgm_domination_check_winner<T: CGameModeTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>) -> PlayerKillType {
    if this.gm().goal > -1 {
        if player.score().score >= this.gm().goal {
            player.score().set_score(this.gm().goal);
            this.gm_mut().winningteam = player.get_team_id();
            this.gm_mut().gameover = true;

            setup_score_board(false);
            show_score_board();
            remove_players_but_team(this.gm().winningteam);
        } else if player.score().score as f64 >= this.gm().goal as f64 * 0.8 && !this.gm().playedwarningsound {
            this.gm_mut().playwarningsound();
        }
    }

    PlayerKillType::Normal
}

impl CGameModeTrait for CGM_Domination {
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
        cgm_domination_init(self)
    }
    fn playerkilledplayer(&mut self, player: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_domination_playerkilledplayer(self, player, other, style)
    }
    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_domination_playerkilledself(self, player, style)
    }
    fn playerextraguy(&mut self, player: Ptr<CPlayer>, iType: i16) {
        cgm_domination_playerextraguy(self, player, iType)
    }
    fn check_winner(&mut self, player: Ptr<CPlayer>) -> PlayerKillType {
        cgm_domination_check_winner(self, player)
    }
}
