//! The dialog a controller's Guide button opens: "Quit Game", Yes or No. In a match it is the exit dialog
//! (`gs_gameplay::update_exit_pause_dialog`); elsewhere it holds the current state until a player answers.

use crate::game::Game;
use crate::smw::game_state::{GameStateManager, StateId};
use smw_platform::{Input, InputEvent};

/// In a match the exit dialog asks instead.
pub fn in_match(g: &mut Game) -> bool {
    crate::smw::net_v2::owns_tick(g) || !matches!(GameStateManager::instance(g).currentState, StateId::Menu | StateId::Splash)
}

/// Runs the open dialog for one frame. False when the current state should update this frame.
pub fn update(g: &mut Game) -> bool {
    let Some(mut yes) = g.input.quit_dialog else { return false };
    if in_match(g) {
        return false;
    }
    let mut gp = g.ptr();
    if g.netplay.active {
        gp.netplay.client.update(g);
    }
    g.game_values.playerInput.clear_pressed_keys(1);
    while let Some(input) = crate::smw::input::poll(g) {
        if input.event == InputEvent::Quit {
            return close(g, true);
        }
        gp.game_values.playerInput.update_input(g, &input, 1);
    }
    for player in 0..4 {
        let keys = &g.game_values.playerInput.outputControls[player];
        if keys.menu_left().fPressed {
            yes = true;
        } else if keys.menu_right().fPressed {
            yes = false;
        }
        if keys.menu_select().fPressed {
            return close(g, yes);
        }
        if keys.menu_cancel().fPressed {
            return close(g, false);
        }
    }
    g.input.quit_dialog = Some(yes);
    crate::smw::gs_gameplay::draw_yes_no_dialog(g, "Quit Game", yes);
    true
}

/// Closes the dialog; `quit` hands the state a `Quit`, as closing the window does.
fn close(g: &mut Game, quit: bool) -> bool {
    g.input.quit_dialog = None;
    g.game_values.playerInput.reset_keys();
    if quit {
        crate::smw::input::push_ready(g, Input { timestamp: 0, event: InputEvent::Quit });
        return false;
    }
    true
}
