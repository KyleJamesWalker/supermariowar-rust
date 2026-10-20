//! Frenzy mode: frag limit with randomly spawning powerup cards.

use crate::smw::players::PlayerId;
use crate::game::Game;
use crate::common::game_mode::*;
use crate::common::global_constants::NUMFRENZYCARDS;
use crate::common::object_base::object_frenzycard;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::frag::{cgm_frag_check_winner, cgm_frag_playerextraguy, cgm_frag_playerkilledplayer, cgm_frag_playerkilledself, CGM_Frag};
use crate::smw::gamemodes::game_mode::*;
use crate::smw::objectgame::PowerupType;
use crate::smw::objects::moving::mo_frenzy_card::MO_FrenzyCard;
use crate::smw::player::CPlayer;

/// Frenzy mode: frag limit scoring while weighted-random powerup cards spawn on the map.
#[derive(Clone)]
pub struct CGM_Frenzy {
    pub cgm_frag: CGM_Frag,

    /// Ticks since the last spawn check.
    pub timer: i16,
    /// Card index (0 to `NUMFRENZYCARDS - 1`) of the most recently spawned card.
    pub iSelectedPowerup: i16,
    /// Sum of the configured card weights.
    pub iItemWeightCount: i16,
    /// Last player to pick up a card, cleared once they lose its power.
    /// With a quantity of 0, a new card spawns only while this is `NONE`.
    pub m_frenzyowner: PlayerId,
}
impl_base!(CGM_Frenzy => cgm_frag: CGM_Frag);

impl CGM_Frenzy {
    pub fn new() -> Self {
        let mut this = CGM_Frenzy { cgm_frag: CGM_Frag::new(), timer: 0, iSelectedPowerup: 0, iItemWeightCount: 0, m_frenzyowner: PlayerId::NONE };
        this.gamemode = game_mode_frenzy;
        this.szModeName = "Frenzy".to_string();
        this
    }

    pub fn set_frenzy_owner(&mut self, player: PlayerId) {
        self.m_frenzyowner = player;
    }
}

impl CGameModeTrait for CGM_Frenzy {
    fn harness_hash(&self, h: &mut crate::smw::rollback::Fnv) {
        self.gm().harness_hash(h);
        h.i16s(&[self.timer, self.iSelectedPowerup, self.iItemWeightCount]);
        h.player(self.m_frenzyowner);
    }
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self, g: &mut Game) {
        cgm_frenzy_init(g, self)
    }
    fn think(&mut self, g: &mut Game) {
        cgm_frenzy_think(g, self)
    }
    fn playerkilledplayer(&mut self, g: &mut Game, inflictor: PlayerId, other: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledplayer(g, self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, g: &mut Game, player: PlayerId, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledself(g, self, player, style)
    }
    fn playerextraguy(&mut self, g: &mut Game, player: PlayerId, iType: i16) {
        cgm_frag_playerextraguy(g, self, player, iType)
    }
    fn check_winner(&mut self, g: &mut Game, player: PlayerId) -> PlayerKillType {
        cgm_frag_check_winner(g, self, player)
    }
}

/// Resets the spawn timer and owner and totals the card weights.
pub fn cgm_frenzy_init(g: &mut Game, this: &mut CGM_Frenzy) {
    cgamemode_init(g, this);
    this.timer = 0;

    this.iItemWeightCount = 0;
    for iPowerup in 0..NUMFRENZYCARDS as usize {
        this.iItemWeightCount += g.game_values.gamemodesettings.frenzy.powerupweight[iPowerup];
    }

    this.set_frenzy_owner(PlayerId::NONE);
}

/// Picks a card by weight (the last card, a random powerup, when all weights are 0) and spawns it.
fn spawn_card(g: &mut Game, this: &mut CGM_Frenzy) {
        let mut gp = g.ptr();
    if this.iItemWeightCount == 0 {
        //If all weights are zero, then choose the random powerup
        this.iSelectedPowerup = (NUMFRENZYCARDS - 1) as i16;
    } else {
        //Randomly choose a powerup from the weighted list
        let iRandPowerup: i32 = RANDOM_INT(gp.get(), this.iItemWeightCount as i32) + 1;
        this.iSelectedPowerup = 0;
        let mut iWeightCount: i32 = g.game_values.gamemodesettings.frenzy.powerupweight[this.iSelectedPowerup as usize] as i32;

        while iWeightCount < iRandPowerup {
            this.iSelectedPowerup += 1;
            iWeightCount += g.game_values.gamemodesettings.frenzy.powerupweight[this.iSelectedPowerup as usize] as i32;
        }
    }

    let obj = Ptr::new_box(MO_FrenzyCard::new(g, gp.get().rm.spr_frenzycards, this.iSelectedPowerup));
    gp.get().objectcontainer[1].add(g, obj);
}

/// Spawns cards every `rate` ticks up to the quantity limit and drops an owner who lost the card's power.
pub fn cgm_frenzy_think(g: &mut Game, this: &mut CGM_Frenzy) {
    if this.gameover {
        this.displayplayertext(g);
    } else {
        let mut iPowerupQuantity: i16 = g.game_values.gamemodesettings.frenzy.quantity;

        let fSpawn = if iPowerupQuantity != 0 {
            this.timer += 1;
            this.timer >= g.game_values.gamemodesettings.frenzy.rate
        } else {
            false
        } || (iPowerupQuantity == 0 && this.m_frenzyowner.is_none());

        if fSpawn {
            this.timer = 0;

            if 0 == iPowerupQuantity {
                iPowerupQuantity = 1;
            }
            if 5 < iPowerupQuantity {
                iPowerupQuantity = (g.players.len() as i32 + iPowerupQuantity as i32 - 7) as i16;
            }

            if g.objectcontainer[1].count_types(object_frenzycard) < iPowerupQuantity as isize as usize {
                spawn_card(g, this);
            }
        }
    }

    if !this.m_frenzyowner.is_none() && g.players.get(this.m_frenzyowner).is_none() {
        this.m_frenzyowner = PlayerId::NONE;
    }
    if !this.m_frenzyowner.is_none() {
        let owner = this.m_frenzyowner;
        if 0 == this.iSelectedPowerup {
            if !g.player(owner).is_bobomb() {
                this.m_frenzyowner = PlayerId::NONE;
            }
        } else if 5 > this.iSelectedPowerup {
            if g.player(owner).powerup != this.iSelectedPowerup {
                this.m_frenzyowner = PlayerId::NONE;
            }
        } else if 5 == this.iSelectedPowerup {
            if g.game_values.gamepowerups[g.player(owner).get_global_id() as usize] != PowerupType::Pow as i16 {
                this.m_frenzyowner = PlayerId::NONE;
            }
        } else if 6 == this.iSelectedPowerup {
            if g.game_values.gamepowerups[g.player(owner).get_global_id() as usize] != PowerupType::Mod as i16 {
                this.m_frenzyowner = PlayerId::NONE;
            }
        } else if 7 == this.iSelectedPowerup {
            if g.game_values.gamepowerups[g.player(owner).get_global_id() as usize] != PowerupType::BulletBill as i16 {
                this.m_frenzyowner = PlayerId::NONE;
            }
        }
    }
}
