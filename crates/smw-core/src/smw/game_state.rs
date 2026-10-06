//! Port of src/smw/GameState.h

use crate::globals::*;

pub trait GameState {
    fn init(&mut self) -> bool {
        true
    }
    fn update(&mut self);
    fn cleanup(&mut self) {}

    fn on_enter_state(&mut self) {}
    fn on_leave_state(&mut self) {}
}

pub struct GameStateManager {
    pub currentState: Ptr<dyn GameState>,
    pub _alias: Aliased,
}

static mut gsm: GameStateManager = GameStateManager { _alias: Aliased::new(), currentState: Ptr::null() };

impl GameStateManager {
    pub fn change_state_to(&mut self, newState: Ptr<dyn GameState>) {
        assert!(!newState.is_null());
        self.currentState.get().on_leave_state();
        self.currentState = newState;
        self.currentState.get().on_enter_state();
    }

    pub fn instance() -> &'static mut GameStateManager {
        unsafe { &mut gsm }
    }
}
