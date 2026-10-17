//! Port of src/common/util/SdlHelpers.cpp

use crate::game::Game;

/// The editors' poll: the last event stays in `event` when the queue runs dry, as sdl2-compat leaves it.
pub fn poll_event(g: &mut Game, event: &mut crate::common::editor_event::EditorEvent) -> i32 {
    match crate::smw::input::poll(g) {
        Some(input) => {
            event.set(input);
            1
        }
        None => 0,
    }
}
