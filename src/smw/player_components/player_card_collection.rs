//! Port of src/smw/player_components/PlayerCardCollection.cpp

use crate::common::game_mode::game_mode_collection;
use crate::globals::*;
use crate::smw::gamemodes::card_collection::CGM_Collection;
use crate::smw::player::CPlayer;

#[derive(Default)]
pub struct PlayerCardCollection {
    pub timer: u16,
    pub index: u16,
    pub _alias: Aliased,
}

impl PlayerCardCollection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.timer = 0;
        self.index = 0;
    }

    pub fn update(&mut self, player: &mut CPlayer, keymask: u8) {
        unsafe {
            if game_values.gamemode.gamemode == game_mode_collection {
                if self.index > 0 {
                    self.timer += 1;
                    if self.timer >= 32 {
                        self.index = 0;
                        self.timer = 0;
                    }
                }

                if keymask as i32 & 16 != 0 {
                    self.index += 1;
                } else if keymask as i32 & !16 != 0 {
                    self.index = 0;
                    self.timer = 0;
                }

                if self.index == 3 && self.timer < 32 {
                    self.timer = 0;
                    self.index = 0;

                    game_values.gamemode.as_any().downcast_mut::<CGM_Collection>().unwrap().release_card(Ptr::from_mut(player));
                }
            }
        }
    }
}
