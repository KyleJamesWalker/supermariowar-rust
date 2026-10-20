//! Colors of the on/off switches and the blocks they toggle.

/// Switch color; the discriminant indexes per-color tables and sprite rows, so the order is fixed.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum SwitchColor {
    #[default]
    Red,
    Green,
    Yellow,
    Blue,
}
