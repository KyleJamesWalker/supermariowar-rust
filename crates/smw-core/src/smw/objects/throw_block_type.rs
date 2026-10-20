//! Kinds of throwable brick block.

/// Throw block kind; the discriminant selects the sprite row, so the order is fixed.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum ThrowBlockType {
    #[default]
    Blue,
    Gray,
    Red,
}

crate::enum_from_u8!(ThrowBlockType, 3);
