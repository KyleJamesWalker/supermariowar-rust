//! Port of src/smw/gamemodes/KingOfTheHill.cpp

use crate::game::Game;
use crate::common::game_mode::{game_mode_koth, CGameMode, CGameModeTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::domination::{cgm_domination_check_winner, CGM_Domination};
use crate::smw::gamemodes::game_mode as gm;
use crate::smw::objects::overmap::wo_king_of_the_hill_zone::OMO_KingOfTheHillZone;
use crate::smw::player::CPlayer;
use std::any::Any;

//King of the Hill (Control an area for a certain amount of time)
pub struct CGM_KingOfTheHill {
    pub cgm_domination: CGM_Domination,
}
impl_base!(CGM_KingOfTheHill => cgm_domination: CGM_Domination);

impl CGM_KingOfTheHill {
    pub fn new() -> Self {
        let mut this = CGM_KingOfTheHill { cgm_domination: CGM_Domination::new() };
        this.goal = 200;
        this.gamemode = game_mode_koth;

        this.setup_mode_strings("King Of The Hill", "Points", 50);
        this
    }
}

impl CGameModeTrait for CGM_KingOfTheHill {
    fn gm(&self) -> &CGameMode {
        &self.cgm_domination.cgame_mode
    }
    fn gm_mut(&mut self) -> &mut CGameMode {
        &mut self.cgm_domination.cgame_mode
    }
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn init(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        gm::cgamemode_init(g, self);
        let obj = Ptr::new_box(OMO_KingOfTheHillZone::new(g, Ptr::from_mut(&mut gp.get().rm.spr_kingofthehillarea)));
        gp.get().objectcontainer[2].add(g, obj);
    }

    fn playerkilledplayer(&mut self, g: &mut Game, _inflictor: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::Normal
    }

    fn playerkilledself(&mut self, g: &mut Game, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        gm::cgamemode_playerkilledself(self, player, style)
    }

    fn playerextraguy(&mut self, g: &mut Game, mut player: Ptr<CPlayer>, iType: i16) {
        if !self.gameover {
            player.score().adjust_score(g, (5 * iType as i32) as i16);
            self.check_winner(g, player);
        }
    }

    fn check_winner(&mut self, g: &mut Game, player: Ptr<CPlayer>) -> PlayerKillType {
        cgm_domination_check_winner(g, self, player)
    }
}
