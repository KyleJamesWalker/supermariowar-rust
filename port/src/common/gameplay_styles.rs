//! Port of src/common/GameplayStyles.h

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum TeamCollisionStyle {
    #[default]
    Off,
    Assist,
    On,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum TournamentControlStyle {
    #[default]
    All,
    GameWinner,
    GameLoser,
    LeadingTeams,
    TrailingTeams,
    Random,
    RandomLoser,
    RoundRobin,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum DeathStyle {
    #[default]
    Respawn,
    Shield,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum ScoringStyle {
    #[default]
    AllKills,
    PushOnly,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum JailStyle {
    #[default]
    Classic,
    Owned,
    FreeForAll,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum ShieldStyle {
    #[default]
    NoShield,
    Soft,
    SoftWithStomp,
    Hard,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum StarStyle {
    #[default]
    Ztar,
    Shine,
    Multi,
    Random,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum WarpLockStyle {
    #[default]
    EntranceOnly,
    ExitOnly,
    EntranceAndExit,
    EntireConnection,
    AllWarps,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum BoomerangStyle {
    #[default]
    Flat,
    SMB3,
    Zelda,
    Random,
}

crate::enum_from_u8!(TeamCollisionStyle, 3);
crate::enum_from_u8!(TournamentControlStyle, 8);
crate::enum_from_u8!(DeathStyle, 2);
crate::enum_from_u8!(ScoringStyle, 2);
crate::enum_from_u8!(JailStyle, 3);
crate::enum_from_u8!(ShieldStyle, 4);
crate::enum_from_u8!(StarStyle, 4);
crate::enum_from_u8!(WarpLockStyle, 5);
crate::enum_from_u8!(BoomerangStyle, 4);
