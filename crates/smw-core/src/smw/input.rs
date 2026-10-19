//! The game's input queue. `poll` is the only reader of the backend's queue.

use crate::game::Game;
use smw_platform::{Input, InputEvent};
use std::collections::VecDeque;

#[derive(Clone)]

pub struct InputQueue {
    /// Keys and taps from the touch controls: live input that skips the touch filter.
    touch: VecDeque<Input>,
    /// Read after the backend's queue: the recorder's input and whatever its frame start took from the backend.
    ready: VecDeque<Input>,
    /// The Guide button's quit dialog while it is open: whether Yes is selected.
    pub quit_dialog: Option<bool>,
    #[cfg(not(target_os = "emscripten"))]
    pub split: crate::smw::split_pad::SplitPad,
}

impl InputQueue {
    pub const fn new() -> Self {
        InputQueue {
            touch: VecDeque::new(),
            ready: VecDeque::new(),
            quit_dialog: None,
            #[cfg(not(target_os = "emscripten"))]
            split: crate::smw::split_pad::SplitPad::new(),
        }
    }
}

/// The next input for the game. While recording, live input goes to the recorder instead. Outside a match, a
/// `QuitRequest` opens the quit dialog instead (`quit_dialog`).
pub fn poll(g: &mut Game) -> Option<Input> {
    loop {
        let input = next(g)?;
        if input.event != InputEvent::QuitRequest || crate::smw::quit_dialog::in_match(g) {
            return Some(input);
        }
        g.input.quit_dialog.get_or_insert(false);
    }
}

fn next(g: &mut Game) -> Option<Input> {
    while let Some(input) = take_live(g) {
        if let Some(input) = crate::smw::harness::capture(g, input) {
            return Some(input);
        }
    }
    g.input.ready.pop_front()
}

/// Hands every queued live input to the recorder; the rest waits for `poll`.
pub fn drain(g: &mut Game) {
    while let Some(input) = take_live(g) {
        if let Some(input) = crate::smw::harness::capture(g, input) {
            g.input.ready.push_back(input);
        }
    }
}

/// Queues input for the game behind the backend's.
pub fn push_ready(g: &mut Game, input: Input) {
    g.input.ready.push_back(input);
}

#[cfg(not(target_os = "emscripten"))]
pub(crate) fn push_touch(g: &mut Game, input: Input) {
    g.input.touch.push_back(input);
}

fn take_live(g: &mut Game) -> Option<Input> {
    let mut gp = g.ptr();
    while let Some(input) = crate::services::services(gp.get()).events.poll() {
        #[cfg(not(target_os = "emscripten"))]
        if !crate::smw::touch::filter(g, &input) {
            continue;
        }
        #[cfg(not(target_os = "emscripten"))]
        let Some(input) = crate::smw::split_pad::filter(g, input) else { continue };
        return Some(input);
    }
    g.input.touch.pop_front()
}
