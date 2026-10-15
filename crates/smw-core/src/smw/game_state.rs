//! Port of src/smw/GameState.h

use crate::game::Game;
use crate::globals::*;

pub trait GameState {
    fn init(&mut self) -> bool {
        true
    }
    fn update(&mut self, g: &mut Game);
    fn cleanup(&mut self) {}

    fn on_enter_state(&mut self, _g: &mut Game) {}
    fn on_leave_state(&mut self, _g: &mut Game) {}
}

pub struct GameStateManager {
    pub currentState: Ptr<dyn GameState>,
    pub _alias: Aliased,
}

impl GameStateManager {
    pub const fn new() -> Self {
        GameStateManager { _alias: Aliased::new(), currentState: Ptr::null() }
    }

    pub fn change_state_to(&mut self, g: &mut Game, newState: Ptr<dyn GameState>) {
        assert!(!newState.is_null());
        self.currentState.get().on_leave_state(g);
        self.currentState = newState;
        self.currentState.get().on_enter_state(g);
    }

    pub fn instance() -> &'static mut GameStateManager {
        &mut game().get().gsm
    }
}
