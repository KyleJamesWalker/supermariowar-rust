//! Port of src/smw/objects/ThrowBlockType.h

/// NOTE: The elements are ordered!
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum ThrowBlockType {
    #[default]
    Blue,
    Gray,
    Red,
}

crate::enum_from_u8!(ThrowBlockType, 3);
