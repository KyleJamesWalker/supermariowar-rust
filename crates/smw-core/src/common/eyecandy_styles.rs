//! Port of src/common/EyecandyStyles.h

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum AwardStyle {
    #[default]
    None,
    Fireworks,
    Swirl,
    Halo,
    Souls,
    Text,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum ScoreboardStyle {
    #[default]
    Top,
    Bottom,
    Corners,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum SpawnStyle {
    #[default]
    Instant,
    Door,
    Swirl,
}

crate::enum_from_u8!(AwardStyle, 6);
crate::enum_from_u8!(ScoreboardStyle, 3);
crate::enum_from_u8!(SpawnStyle, 3);
