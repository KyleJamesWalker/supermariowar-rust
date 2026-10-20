//! Option enums for cosmetic effects: kill-streak awards, scoreboard placement and player spawning.

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
/// Effect shown when a player gets a kill streak.
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
/// Where the in-game scoreboard is drawn.
pub enum ScoreboardStyle {
    #[default]
    Top,
    Bottom,
    Corners,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
/// Animation played when a player spawns.
pub enum SpawnStyle {
    #[default]
    Instant,
    Door,
    Swirl,
}

crate::enum_from_u8!(AwardStyle, 6);
crate::enum_from_u8!(ScoreboardStyle, 3);
crate::enum_from_u8!(SpawnStyle, 3);
