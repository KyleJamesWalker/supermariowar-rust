//! The built-in controller split between two players (Controls, "Built-in Controller: Split", or `--split-pad`).
//! Player 1 keeps the D-pad and gets L1 jump, L2 run and Select item; a second device takes Y, A and B as left,
//! right and down, X jump, R1 run and R2 item. Start stays with player 1 and pauses for everyone. Only live input
//! is split: recordings hold the two devices' events, so replays need no split.

use crate::game::{Device, Game};
use smw_platform::{Input, InputEvent, PadSlot};

/// The second half's pad, an instance number no backend gives out.
pub const SPLIT_SLOT: PadSlot = PadSlot(-2);
const SPLIT_FILE: &str = "split_pad";

const HAT_RIGHT: u8 = 2;
const HAT_DOWN: u8 = 4;
const HAT_LEFT: u8 = 8;

#[derive(Clone, Default)]
pub struct SplitPad {
    /// `--split-pad`: split for this session whatever the saved setting.
    pub forced: bool,
    saved: bool,
    hat: u8,
}

impl SplitPad {
    pub const fn new() -> Self {
        SplitPad { forced: false, saved: false, hat: 0 }
    }
}

/// Where a built-in controller button goes when split: player 1's button, player 2's button, or a player 2 D-pad bit.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Route {
    First(u8),
    Second(u8),
    SecondDpad(u8),
    Drop,
}

/// Buttons in the translated layout (smw-sdl2's pad.rs): A 0, B 1, X 2, Start 3, Back 4, Y 5, L1 6, R1 7, L2 8, R2 9.
fn route(button: u8) -> Route {
    match button {
        6 => Route::First(0),
        8 => Route::First(1),
        4 => Route::First(2),
        3 | 10 | 11 => Route::First(button),
        2 => Route::Second(0),
        7 => Route::Second(1),
        9 => Route::Second(2),
        5 => Route::SecondDpad(HAT_LEFT),
        0 => Route::SecondDpad(HAT_RIGHT),
        1 => Route::SecondDpad(HAT_DOWN),
        _ => Route::Drop,
    }
}

/// The event the game reads for one from the built-in controller `builtin`; other pads' events pass unchanged.
fn split(state: &mut SplitPad, builtin: PadSlot, event: InputEvent) -> Option<InputEvent> {
    let InputEvent::PadButton { pad, button, down } = event else { return Some(event) };
    if pad != builtin {
        return Some(event);
    }
    match route(button) {
        Route::First(button) => Some(InputEvent::PadButton { pad, button, down }),
        Route::Second(button) => Some(InputEvent::PadButton { pad: SPLIT_SLOT, button, down }),
        Route::SecondDpad(bit) => {
            let hat = if down { state.hat | bit } else { state.hat & !bit };
            if hat == state.hat {
                return None;
            }
            state.hat = hat;
            Some(InputEvent::PadHat { pad: SPLIT_SLOT, hat: 0, value: hat })
        }
        Route::Drop => None,
    }
}

/// The built-in controller: the first translated controller.
fn builtin(g: &Game) -> Option<(usize, PadSlot)> {
    g.devices.iter().enumerate().find_map(|(i, d)| match d {
        Some(Device::Pad { pad, controller: true }) if *pad != SPLIT_SLOT => Some((i, *pad)),
        _ => None,
    })
}

fn split_index(g: &Game) -> Option<usize> {
    g.devices.iter().position(|d| matches!(d, Some(Device::Pad { pad, .. }) if *pad == SPLIT_SLOT))
}

/// Live input from the built-in controller, split when the second half is open.
pub fn filter(g: &mut Game, input: Input) -> Option<Input> {
    if split_index(g).is_none() {
        return Some(input);
    }
    let Some((_, pad)) = builtin(g) else { return Some(input) };
    let event = split(&mut g.input.split, pad, input.event)?;
    Some(Input { timestamp: input.timestamp, event })
}

fn marker(g: &mut Game) -> String {
    crate::common::path::get_home_directory(g) + SPLIT_FILE
}

/// Reads the saved setting at launch and opens the second half if it is on, before the players are assigned.
pub fn init(g: &mut Game) {
    let saved = std::path::Path::new(&marker(g)).exists();
    g.input.split.saved = saved;
    g.game_values.splitpad = (saved || g.input.split.forced) as i16;
    if !crate::smw::harness::replaying(g) && g.game_values.splitpad != 0 && builtin(g).is_some() {
        if let Some(index) = crate::smw::pad::free_index(g) {
            g.devices[index] = Some(Device::Pad { pad: SPLIT_SLOT, controller: true });
        }
    }
}

/// Opens or closes the second half to match the setting, outside replays, and saves a changed setting. True when
/// it opened, so the players are reassigned as for a pad plugged in.
pub fn update(g: &mut Game) -> bool {
    if crate::smw::harness::replaying(g) {
        return false;
    }
    let on = g.game_values.splitpad != 0;
    if on != g.input.split.saved && !g.input.split.forced {
        let path = marker(g);
        let _ = if on { std::fs::write(&path, "") } else { std::fs::remove_file(&path) };
        g.input.split.saved = on;
    }
    let open = split_index(g);
    if on && open.is_none() && builtin(g).is_some() {
        let Some(index) = crate::smw::pad::free_index(g) else { return false };
        g.devices[index] = Some(Device::Pad { pad: SPLIT_SLOT, controller: true });
        crate::smw::pad::devices_changed();
        crate::smw::harness::record_device(g, index, true);
        return true;
    }
    if let Some(index) = open.filter(|_| !on || builtin(g).is_none()) {
        g.devices[index] = None;
        g.input.split.hat = 0;
        crate::smw::pad::devices_changed();
        crate::smw::harness::record_device(g, index, false);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILTIN: PadSlot = PadSlot(0);

    fn press(state: &mut SplitPad, button: u8, down: bool) -> Option<InputEvent> {
        split(state, BUILTIN, InputEvent::PadButton { pad: BUILTIN, button, down })
    }

    #[test]
    fn player_one_gets_the_left_side() {
        let mut s = SplitPad::default();
        assert_eq!(press(&mut s, 6, true), Some(InputEvent::PadButton { pad: BUILTIN, button: 0, down: true }));
        assert_eq!(press(&mut s, 8, true), Some(InputEvent::PadButton { pad: BUILTIN, button: 1, down: true }));
        assert_eq!(press(&mut s, 4, false), Some(InputEvent::PadButton { pad: BUILTIN, button: 2, down: false }));
        assert_eq!(press(&mut s, 3, true), Some(InputEvent::PadButton { pad: BUILTIN, button: 3, down: true }));
        let hat = InputEvent::PadHat { pad: BUILTIN, hat: 0, value: 1 };
        assert_eq!(split(&mut s, BUILTIN, hat.clone()), Some(hat));
    }

    #[test]
    fn player_two_gets_the_face_buttons_and_right_shoulders() {
        let mut s = SplitPad::default();
        assert_eq!(press(&mut s, 2, true), Some(InputEvent::PadButton { pad: SPLIT_SLOT, button: 0, down: true }));
        assert_eq!(press(&mut s, 7, true), Some(InputEvent::PadButton { pad: SPLIT_SLOT, button: 1, down: true }));
        assert_eq!(press(&mut s, 9, true), Some(InputEvent::PadButton { pad: SPLIT_SLOT, button: 2, down: true }));
        assert_eq!(press(&mut s, 5, true), Some(InputEvent::PadHat { pad: SPLIT_SLOT, hat: 0, value: HAT_LEFT }));
        assert_eq!(press(&mut s, 1, true), Some(InputEvent::PadHat { pad: SPLIT_SLOT, hat: 0, value: HAT_LEFT | HAT_DOWN }));
        assert_eq!(press(&mut s, 5, false), Some(InputEvent::PadHat { pad: SPLIT_SLOT, hat: 0, value: HAT_DOWN }));
        assert_eq!(press(&mut s, 1, false), Some(InputEvent::PadHat { pad: SPLIT_SLOT, hat: 0, value: 0 }));
        assert_eq!(press(&mut s, 0, true), Some(InputEvent::PadHat { pad: SPLIT_SLOT, hat: 0, value: HAT_RIGHT }));
        assert_eq!(press(&mut s, 0, true), None);
    }

    #[test]
    fn other_pads_pass_unchanged() {
        let mut s = SplitPad::default();
        let other = InputEvent::PadButton { pad: PadSlot(1), button: 0, down: true };
        assert_eq!(split(&mut s, BUILTIN, other.clone()), Some(other));
        assert_eq!(press(&mut s, 12, true), None);
    }
}
