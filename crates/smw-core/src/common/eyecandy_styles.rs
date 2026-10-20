//! Option enums for cosmetic effects: kill-streak awards, scoreboard placement and player spawning.

/// Effect shown when a player gets a kill streak.
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

/// Where the in-game scoreboard is drawn.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum ScoreboardStyle {
    #[default]
    Top,
    Bottom,
    Corners,
}

/// Animation played when a player spawns.
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
