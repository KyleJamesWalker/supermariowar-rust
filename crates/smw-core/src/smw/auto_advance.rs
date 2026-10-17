//! When every player is a CPU, the winner screen, the bonus wheel and the tour and world overviews continue on their
//! own after three seconds. The Enter press goes through the platform's event queue, so a recording keeps it and a
//! replay plays it back instead of pressing again.

use crate::game::Game;
use smw_platform::{Input, InputEvent, Keycode};

/// Three seconds at the 16 ms frame time.
const DELAY_FRAMES: u16 = 188;
const SDLK_RETURN: i32 = 13;
const SDL_SCANCODE_RETURN: i32 = 40;

fn all_cpus(g: &Game) -> bool {
    let controls = &g.game_values.playercontrol;
    controls.iter().all(|&c| c != 1) && controls.iter().any(|&c| c == 2)
}

fn press(g: &mut Game, down: bool) {
    let input = Input {
        timestamp: 0,
        event: InputEvent::Key { key: Keycode(SDLK_RETURN), scancode: SDL_SCANCODE_RETURN, mods: 0, down, repeat: false, window: 0 },
    };
    crate::services::services(g).events.push(&input);
}

/// Called once a frame with whether the current screen waits for Enter to continue.
pub fn tick(g: &mut Game, waiting: bool) {
    if !waiting || !all_cpus(g) || crate::smw::harness::replaying(g) {
        g.main_loop.auto_advance = 0;
        return;
    }
    g.main_loop.auto_advance += 1;
    if g.main_loop.auto_advance == DELAY_FRAMES {
        press(g, true);
    } else if g.main_loop.auto_advance == DELAY_FRAMES + 2 {
        press(g, false);
        g.main_loop.auto_advance = 0;
    }
}
