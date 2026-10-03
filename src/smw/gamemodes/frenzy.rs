//! Port of src/smw/gamemodes/Frenzy.cpp

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
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::players;
use crate::smw::objectgame::PowerupType;
use crate::smw::objects::moving::mo_frenzy_card::MO_FrenzyCard;
use crate::smw::player::CPlayer;

//Just like frag limit, but firepower cards appear randomly
pub struct CGM_Frenzy {
    pub cgm_frag: CGM_Frag,

    pub timer: i16,
    pub iSelectedPowerup: i16,
    pub iItemWeightCount: i16,
    pub m_frenzyowner: Ptr<CPlayer>,
}
impl_base!(CGM_Frenzy => cgm_frag: CGM_Frag);

//Fireball:
//Frag limit death match, but powerup cards appear randomly
impl CGM_Frenzy {
    pub fn new() -> Self {
        let mut this = CGM_Frenzy { cgm_frag: CGM_Frag::new(), timer: 0, iSelectedPowerup: 0, iItemWeightCount: 0, m_frenzyowner: Ptr::null() };
        this.gamemode = game_mode_frenzy;
        this.szModeName = "Frenzy".to_string();
        this
    }

    pub fn set_frenzy_owner(&mut self, player: Ptr<CPlayer>) {
        self.m_frenzyowner = player;
    }
}

impl CGameModeTrait for CGM_Frenzy {
    crate::impl_cgamemode_plumbing!();

    fn init(&mut self) {
        cgm_frenzy_init(self)
    }
    fn think(&mut self) {
        cgm_frenzy_think(self)
    }
    fn playerkilledplayer(&mut self, inflictor: Ptr<CPlayer>, other: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledplayer(self, inflictor, other, style)
    }
    fn playerkilledself(&mut self, player: Ptr<CPlayer>, style: KillStyle) -> PlayerKillType {
        cgm_frag_playerkilledself(self, player, style)
    }
    fn playerextraguy(&mut self, player: Ptr<CPlayer>, iType: i16) {
        cgm_frag_playerextraguy(self, player, iType)
    }
    fn check_winner(&mut self, player: Ptr<CPlayer>) -> PlayerKillType {
        cgm_frag_check_winner(self, player)
    }
}

pub fn cgm_frenzy_init(this: &mut CGM_Frenzy) {
    cgamemode_init(this);
    this.timer = 0;

    this.iItemWeightCount = 0;
    unsafe {
        for iPowerup in 0..NUMFRENZYCARDS as usize {
            this.iItemWeightCount += game_values.gamemodesettings.frenzy.powerupweight[iPowerup];
        }
    }

    this.set_frenzy_owner(Ptr::null());
}

pub fn cgm_frenzy_think(this: &mut CGM_Frenzy) {
    unsafe {
        if this.gameover {
            this.displayplayertext();
        } else {
            let mut iPowerupQuantity: i16 = game_values.gamemodesettings.frenzy.quantity;

            let fSpawn = if iPowerupQuantity != 0 {
                this.timer += 1;
                this.timer >= game_values.gamemodesettings.frenzy.rate
            } else {
                false
            } || (iPowerupQuantity == 0 && this.m_frenzyowner.is_null());

            if fSpawn {
                this.timer = 0;

                if 0 == iPowerupQuantity {
                    iPowerupQuantity = 1;
                }
                if 5 < iPowerupQuantity {
                    iPowerupQuantity = (players.len() as i32 + iPowerupQuantity as i32 - 7) as i16;
                }

                if objectcontainer[1].count_types(object_frenzycard) < iPowerupQuantity as isize as usize {
                    if this.iItemWeightCount == 0 {
                        //If all weights are zero, then choose the random powerup
                        this.iSelectedPowerup = (NUMFRENZYCARDS - 1) as i16;
                    } else {
                        //Randomly choose a powerup from the weighted list
                        let iRandPowerup: i32 = RANDOM_INT(this.iItemWeightCount as i32) + 1;
                        this.iSelectedPowerup = 0;
                        let mut iWeightCount: i32 = game_values.gamemodesettings.frenzy.powerupweight[this.iSelectedPowerup as usize] as i32;

                        while iWeightCount < iRandPowerup {
                            this.iSelectedPowerup += 1;
                            iWeightCount += game_values.gamemodesettings.frenzy.powerupweight[this.iSelectedPowerup as usize] as i32;
                        }
                    }

                    objectcontainer[1].add(Ptr::new_box(MO_FrenzyCard::new(Ptr::from_mut(&mut rm.spr_frenzycards), this.iSelectedPowerup)));
                }
            }
        }

        if !this.m_frenzyowner.is_null() {
            let owner = this.m_frenzyowner;
            if 0 == this.iSelectedPowerup {
                if !owner.is_bobomb() {
                    this.m_frenzyowner = Ptr::null();
                }
            } else if 5 > this.iSelectedPowerup {
                if owner.powerup != this.iSelectedPowerup {
                    this.m_frenzyowner = Ptr::null();
                }
            } else if 5 == this.iSelectedPowerup {
                if game_values.gamepowerups[owner.get_global_id() as usize] != PowerupType::Pow as i16 {
                    this.m_frenzyowner = Ptr::null();
                }
            } else if 6 == this.iSelectedPowerup {
                if game_values.gamepowerups[owner.get_global_id() as usize] != PowerupType::Mod as i16 {
                    this.m_frenzyowner = Ptr::null();
                }
            } else if 7 == this.iSelectedPowerup {
                if game_values.gamepowerups[owner.get_global_id() as usize] != PowerupType::BulletBill as i16 {
                    this.m_frenzyowner = Ptr::null();
                }
            }
        }
    }
}
