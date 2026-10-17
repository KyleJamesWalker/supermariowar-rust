//! Port of src/smw/GameState.h

use crate::game::Game;
use crate::globals::*;

pub trait GameState {
    fn init(&mut self, _g: &mut Game) -> bool {
        true
    }
    fn update(&mut self, g: &mut Game);
    fn cleanup(&mut self) {}

    fn on_enter_state(&mut self, _g: &mut Game) {}
    fn on_leave_state(&mut self, _g: &mut Game) {}
}

/// Which state runs: one of the game's own, or the state a segment replay starts in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StateId {
    Splash,
    Menu,
    Gameplay,
    Segment,
}

pub struct GameStateManager {
    pub currentState: StateId,
    segment: Option<Box<dyn GameState>>,
    pub _alias: Aliased,
}

/// A segment replay's start state is not copied; it hands over to gameplay on its first frame.
impl Clone for GameStateManager {
    fn clone(&self) -> Self {
        GameStateManager { _alias: Aliased::new(), currentState: self.currentState, segment: None }
    }
}

impl GameStateManager {
    pub const fn new() -> Self {
        GameStateManager { _alias: Aliased::new(), currentState: StateId::Splash, segment: None }
    }

    /// Starts a segment replay in `state`.
    pub fn start_segment(&mut self, state: Box<dyn GameState>) {
        self.segment = Some(state);
        self.currentState = StateId::Segment;
    }

    pub fn current(&mut self, g: &mut Game) -> Ptr<dyn GameState> {
        match self.currentState {
            StateId::Splash => Ptr::from_mut(crate::smw::gs_splash_screen::SplashScreenState::instance(g) as &mut dyn GameState),
            StateId::Menu => Ptr::from_mut(crate::smw::gs_menu::MenuState::instance(g) as &mut dyn GameState),
            StateId::Gameplay => Ptr::from_mut(crate::smw::gs_gameplay::GameplayState::instance(g) as &mut dyn GameState),
            StateId::Segment => Ptr::from_mut(&mut **self.segment.as_mut().expect("segment state")),
        }
    }

    pub fn change_state_to(&mut self, g: &mut Game, newState: StateId) {
        self.current(g).get().on_leave_state(g);
        self.currentState = newState;
        self.current(g).get().on_enter_state(g);
    }

    pub fn instance(g: &mut Game) -> &'static mut GameStateManager {
        &mut g.ptr().get().gsm
    }
}
