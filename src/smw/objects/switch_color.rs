//! Port of src/smw/objects/SwitchColor.h

/// NOTE: The elements are ordered!
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum SwitchColor {
    #[default]
    Red,
    Green,
    Yellow,
    Blue,
}
