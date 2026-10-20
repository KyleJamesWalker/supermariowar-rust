//! Event polling for the editors.

use crate::game::Game;

/// The editors' poll: returns 1 with the next event in `event`, or 0 and leaves the last one there.
pub fn poll_event(g: &mut Game, event: &mut crate::common::editor_event::EditorEvent) -> i32 {
    match crate::smw::input::poll(g) {
        Some(input) => {
            event.set(input);
            1
        }
        None => 0,
    }
}
