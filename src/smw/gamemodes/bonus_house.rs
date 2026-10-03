//! Port of src/smw/gamemodes/BonusHouse.cpp

use crate::common::game::App;
use crate::common::game_mode::{game_mode_bonus, CGameMode, CGameModeTrait};
use crate::common::global_constants::MAX_BONUS_CHESTS;
use crate::common::math::vec2::Vec2s;
use crate::common::player_kill_styles::KillStyle;
use crate::common::player_kill_types::PlayerKillType;
use crate::common::random_number_generator::RANDOM_INT;
use crate::common::world_tour_stop::TourStop;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gamemodes::game_mode as gm;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::main::players;
use crate::smw::objects::moving::mo_bonus_house_chest::MO_BonusHouseChest;
use crate::smw::player::CPlayer;
use sdl2::sys::SDL_Rect;
use std::any::Any;

//Bonus Mode (not really a game mode, but involves using the map so we need a mode to play)
pub struct CGM_Bonus {
    pub cgame_mode: CGameMode,

    tsTourStop: Ptr<TourStop>,
}
impl_base!(CGM_Bonus => cgame_mode: CGameMode);

impl CGM_Bonus {
    pub fn new() -> Self {
        let mut this = CGM_Bonus { cgame_mode: CGameMode::new(), tsTourStop: Ptr::null() };
        this.gamemode = game_mode_bonus;
        this.setup_mode_strings("Bonus", "", 0);
        this
    }
}

impl CGameModeTrait for CGM_Bonus {
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
            //Unlock the chests
            game_values.flags.noexit = true;

            //Will cause the flow to skip the scoreboard screen and go straight back to the world map
            game_values.worldskipscoreboard = true;

            //Add number of treasure chests to the bonus house
            self.tsTourStop = game_values.tourstops[game_values.tourstopcurrent];
            let iNumBonuses: i16 = self.tsTourStop.iNumBonuses;

            let mut fChestUsed: [bool; MAX_BONUS_CHESTS as usize] = [false; MAX_BONUS_CHESTS as usize];
            let mut iChestOrder: [i16; MAX_BONUS_CHESTS as usize] = [0; MAX_BONUS_CHESTS as usize];
            let mut iNumChests: i16 = 0;

            if self.tsTourStop.iBonusType == 0 {
                for iChest in 0..iNumBonuses {
                    iChestOrder[iNumChests as usize] = iChest;
                    iNumChests += 1;
                }
            } else {
                for iChest in 0..iNumBonuses {
                    fChestUsed[iChest as usize] = false;
                }

                for _iChest in 0..iNumBonuses {
                    let mut iRandChest: i16 = RANDOM_INT(iNumBonuses as i32) as i16;

                    while fChestUsed[iRandChest as usize] {
                        iRandChest += 1;
                        if iRandChest >= iNumBonuses {
                            iRandChest = 0;
                        }
                    }

                    fChestUsed[iRandChest as usize] = true;
                    iChestOrder[iNumChests as usize] = iRandChest;
                    iNumChests += 1;
                }
            }

            let dSpacing: f32 = (384.0f32 - (iNumBonuses as i32 * 64) as f32) / (iNumBonuses as i32 + 1) as f32;

            let mut dx: f32 = 128.0f32 + dSpacing;

            //float dx = 288.0f - (dSpacing * (float)(iNumBonuses - 1) / 2.0f);
            for iChest in 0..iNumBonuses {
                let iBonus = self.tsTourStop.wsbBonuses[iChestOrder[iChest as usize] as usize].iBonus;
                objectcontainer[0].add(Ptr::new_box(MO_BonusHouseChest::new(Ptr::from_mut(&mut rm.spr_worldbonushouse), Vec2s::new(dx as i16, 384), iBonus)));
                dx += dSpacing + 64.0f32;
            }
        }
    }

    fn draw_background(&mut self) {
        unsafe {
            //Draw Toad
            rm.spr_worldbonushouse.draw_src(544, 256, &SDL_Rect { x: if players[0].left_x() > 544 { 224 } else { 192 }, y: 0, w: 32, h: 64 });

            //Draw Bonus House Title
            rm.menu_plain_field.draw_src(0, 0, &SDL_Rect { x: 0, y: 0, w: App::screenWidth / 2, h: 32 });
            rm.menu_plain_field.draw_src(App::screenWidth / 2, 0, &SDL_Rect { x: 192, y: 0, w: App::screenWidth / 2, h: 32 });
            rm.game_font_large.draw_centered(App::screenWidth / 2, 5, &self.tsTourStop.szName);

            //Draw Bonus House Text
            if self.tsTourStop.iBonusTextLines > 0 {
                rm.spr_worldbonushouse.draw_src(128, 128, &SDL_Rect { x: 0, y: 64, w: 384, h: 128 });

                for iTextLine in 0..self.tsTourStop.iBonusTextLines {
                    rm.game_font_large.draw_chop_centered(App::screenWidth / 2, 132 + 24 * iTextLine as i32, 372, &self.tsTourStop.szBonusText[iTextLine as usize]);
                }
            }
        }
    }

    //Override so it doesn't display winner text after you choose a powerup
    fn think(&mut self) {}

    fn playerkilledplayer(&mut self, _inflictor: Ptr<CPlayer>, _other: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::None
    }
    fn playerkilledself(&mut self, _player: Ptr<CPlayer>, _style: KillStyle) -> PlayerKillType {
        PlayerKillType::None
    }

    fn playerextraguy(&mut self, _player: Ptr<CPlayer>, _iType: i16) {}

    fn has_stored_powerups(&mut self) -> bool {
        false
    }
}
