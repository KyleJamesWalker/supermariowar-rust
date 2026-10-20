//! Card collection input: tapping turbo three times quickly drops a card.

use crate::game::Game;
use crate::common::game_mode::game_mode_collection;
use crate::globals::*;
use crate::smw::gamemodes::card_collection::CGM_Collection;
use crate::smw::player::CPlayer;
use crate::common::global_constants::{ticks};

#[derive(Clone, Default)]
pub struct PlayerCardCollection {
    /// Ticks since the first tap in the current sequence.
    pub timer: u16,
    /// Turbo taps in the current sequence; any other key resets it.
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

    /// Feeds this tick's pressed-key mask; in card collection mode, three turbo taps within
    /// `ticks(32)` ticks release one of the team's cards.
    pub fn update(&mut self, g: &mut Game, player: &mut CPlayer, keymask: u8) {
        if g.mode().gamemode == game_mode_collection {
            if self.index > 0 {
                self.timer += 1;
                if self.timer as i32 >= ticks(32) {
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

            if self.index == 3 && (self.timer as i32) < ticks(32) {
                self.timer = 0;
                self.index = 0;

                g.mode().get().as_any().downcast_mut::<CGM_Collection>().unwrap().release_card(g, player.id);
            }
        }
    }
}
