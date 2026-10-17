//! The players of the current match, reached through `PlayerId` handles.

use crate::game::Game;
use crate::smw::player::CPlayer;
use std::ops::{Deref, DerefMut};

/// A player in `Game::players`. A handle to a removed player finds nothing.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PlayerId {
    slot: u8,
    generation: u8,
}

impl Default for PlayerId {
    fn default() -> Self {
        Self::NONE
    }
}

impl PlayerId {
    /// No player: what a null `CPlayer*` was in the C++.
    pub const NONE: PlayerId = PlayerId { slot: u8::MAX, generation: 0 };

    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

/// Derefs to the play order; the slots own the players.
pub struct Players {
    order: Vec<PlayerId>,
    /// Null for a free slot; otherwise a `Box::into_raw` that `remove` frees.
    slots: Vec<*mut CPlayer>,
    generations: Vec<u8>,
}

impl Clone for Players {
    fn clone(&self) -> Self {
        let slots = self.slots.iter().map(|&p| if p.is_null() { p } else { Box::into_raw(Box::new(unsafe { (*p).clone() })) }).collect();
        Players { order: self.order.clone(), slots, generations: self.generations.clone() }
    }
}

impl Drop for Players {
    fn drop(&mut self) {
        for p in self.slots.drain(..).filter(|p| !p.is_null()) {
            drop(unsafe { Box::from_raw(p) });
        }
    }
}

impl Players {
    pub const fn new() -> Self {
        Players { order: Vec::new(), slots: Vec::new(), generations: Vec::new() }
    }

    /// Takes ownership of `player`; the caller adds the handle to the play order.
    pub fn insert(&mut self, player: Box<CPlayer>) -> PlayerId {
        let slot = match self.slots.iter().position(|p| p.is_null()) {
            Some(slot) => slot,
            None => {
                self.slots.push(std::ptr::null_mut());
                self.generations.push(0);
                self.slots.len() - 1
            }
        };
        self.slots[slot] = Box::into_raw(player);
        PlayerId { slot: slot as u8, generation: self.generations[slot] }
    }

    /// Drops the player. Leaves the play order alone.
    pub fn remove(&mut self, id: PlayerId) {
        if self.get(id).is_some() {
            drop(unsafe { Box::from_raw(self.slots[id.slot as usize]) });
            self.slots[id.slot as usize] = std::ptr::null_mut();
            self.generations[id.slot as usize] = self.generations[id.slot as usize].wrapping_add(1);
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get(&self, id: PlayerId) -> Option<&'static mut CPlayer> {
        let slot = id.slot as usize;
        if slot >= self.slots.len() || self.generations[slot] != id.generation {
            return None;
        }
        let player = self.slots[slot];
        (!player.is_null()).then(|| unsafe { &mut *player })
    }
}

impl Deref for Players {
    type Target = Vec<PlayerId>;
    fn deref(&self) -> &Vec<PlayerId> {
        &self.order
    }
}

impl DerefMut for Players {
    fn deref_mut(&mut self) -> &mut Vec<PlayerId> {
        &mut self.order
    }
}

impl Game {
    /// Panics on a removed player, where the C++ read freed memory.
    pub fn player(&self, id: PlayerId) -> &'static mut CPlayer {
        self.players.get(id).unwrap_or_else(|| panic!("no player {:?}", id))
    }
}
