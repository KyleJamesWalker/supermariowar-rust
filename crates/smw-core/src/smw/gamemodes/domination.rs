//! Domination mode: claim area blocks; owned blocks earn points over time.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::{game_mode_domination, CGameMode, CGameModeTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode::{self as gm, remove_players_but_team, setup_score_board, show_score_board};
use crate::smw::object_container::CObjectContainer;
use crate::smw::objects::overmap::wo_area::OMO_Area;
use crate::smw::player::CPlayer;
use std::any::Any;

/// Applies the domination death rules to every area owned by `other`'s color: relocate,
/// steal for `player` (when not `NONE`) or reset to unowned.
fn adjust_player_areas(g: &mut Game, container: &CObjectContainer, player: PlayerId, other: PlayerId) {
    let n = container.list().len();
    for i in 0..n {
        let obj = container.list()[i];
        let Some(area) = obj.get().as_any().downcast_mut::<OMO_Area>() else {
            continue;
        };

        if area.get_color_id() == g.player(other).get_color_id() {
            if g.game_values.gamemodesettings.domination.relocateondeath {
                area.place_area(g);
            }

            if g.game_values.gamemodesettings.domination.stealondeath && !player.is_none() {
                area.set_owner(g, player);
            } else if g.game_values.gamemodesettings.domination.loseondeath {
                area.reset();
            }
        }
    }
}

/// Domination: touch area blocks to claim them; the more blocks a team owns, the faster it scores.
#[derive(Clone)]
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

/// Spawns the configured number of areas. Settings above 10 and 18 scale with player count.
pub fn cgm_domination_init<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T) {
        let mut gp = g.ptr();
    gm::cgamemode_init(g, this);

    let mut iNumAreas: i16 = g.game_values.gamemodesettings.domination.quantity;

    if iNumAreas < 1 {
        iNumAreas = 1;
        g.game_values.gamemodesettings.domination.quantity = iNumAreas;
    }

    if iNumAreas > 18 {
        iNumAreas = 2usize.wrapping_mul(g.players.len()).wrapping_add(iNumAreas as usize).wrapping_sub(22) as i16;
    } else if iNumAreas > 10 {
        iNumAreas = g.players.len().wrapping_add(iNumAreas as usize).wrapping_sub(12) as i16;
    }

    for _k in 0..iNumAreas {
        let obj = Ptr::new_box(OMO_Area::new(g, gp.get().rm.spr_areas, iNumAreas));
        gp.get().objectcontainer[0].add(g, obj);
    }
}

/// Applies the area death rules to the victim's areas; kills score nothing.
pub fn cgm_domination_playerkilledplayer<T: CGameModeTrait + ?Sized>(g: &mut Game, _this: &mut T, player: PlayerId, other: PlayerId, _style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
    //Update areas the dead player owned
    adjust_player_areas(g, &gp.objectcontainer[0], player, other);

    PlayerKillType::Normal
}

/// Applies the area death rules to the player's areas with nobody to steal them.
pub fn cgm_domination_playerkilledself<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T, player: PlayerId, style: KillStyle) -> PlayerKillType {
        let mut gp = g.ptr();
    gm::cgamemode_playerkilledself(g, this, player, style);

    //Update areas the dead player owned
    adjust_player_areas(g, &gp.objectcontainer[0], PlayerId::NONE, player);

    PlayerKillType::Normal
}

/// Awards 10 points per extra life.
pub fn cgm_domination_playerextraguy<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId, iType: i16) {
    if !this.gm().gameover {
        g.player(player).score(g).adjust_score(g, (10 * iType as i32) as i16);
        this.check_winner(g, player);
    }
}

/// Ends the match when the player's team reaches the goal; warns at 80%.
pub fn cgm_domination_check_winner<T: CGameModeTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId) -> PlayerKillType {
    if this.gm().goal > -1 {
        if g.player(player).score(g).score >= this.gm().goal {
            g.player(player).score(g).set_score(g, this.gm().goal);
            this.gm_mut().winningteam = g.player(player).get_team_id();
            this.gm_mut().gameover = true;

            setup_score_board(g, false);
            show_score_board(g);
            remove_players_but_team(g, this.gm().winningteam);
        } else if g.player(player).score(g).score as f64 >= this.gm().goal as f64 * 0.8 && !this.gm().playedwarningsound {
            this.gm_mut().playwarningsound(g);
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

    fn init(&mut self, g: &mut Game) {
        cgm_domination_init(g, self)
    }
    fn playerkilledplayer(&mut self, g: &mut Game, player: PlayerId, other: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_domination_playerkilledplayer(g, self, player, other, style)
    }
    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_domination_playerkilledself(g, self, player, style)
    }
    fn playerextraguy(&mut self, g: &mut Game, player: PlayerId, iType: i16) {
        cgm_domination_playerextraguy(g, self, player, iType)
    }
    fn check_winner(&mut self, g: &mut Game, player: PlayerId) -> PlayerKillType {
        cgm_domination_check_winner(g, self, player)
    }
}
