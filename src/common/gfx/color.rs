//! Port of src/common/gfx/Color.h

/// Represents a solid RGB color.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub mod colors {
    use super::RGB;
    pub const BLACK: RGB = RGB { r: 0, g: 0, b: 0 };
    pub const MAGENTA: RGB = RGB { r: 255, g: 0, b: 255 };
}
