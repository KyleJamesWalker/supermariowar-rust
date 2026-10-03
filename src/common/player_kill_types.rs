//! Port of src/common/PlayerKillTypes.h

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum PlayerKillType {
    #[default]
    None,
    Normal,
    Removed,
    NonKill,
}

crate::enum_from_u8!(PlayerKillType, 4);
