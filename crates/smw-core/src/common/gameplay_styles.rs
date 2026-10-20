//! The gameplay option enums the menus set and options files store as one byte each.

/// How players on the same team interact.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum TeamCollisionStyle {
    #[default]
    /// Teammates pass through each other.
    Off,
    /// Teammates bounce off each other, pass tags and swap stored powerups, but cannot kill each other.
    Assist,
    /// Teammates collide and can kill each other.
    On,
}

/// Which team gets menu control between tournament games.
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

/// What a kill does to the victim in classic, frag and time modes.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum DeathStyle {
    #[default]
    /// The victim dies and respawns.
    Respawn,
    /// The kill only resets the victim's shield, and they keep playing.
    Shield,
}

/// Which kills score in frag and time modes.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum ScoringStyle {
    #[default]
    AllKills,
    /// Only kills credited by pushing a player into a hazard score.
    PushOnly,
}

/// How jail mode frees and credits jailed players.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum JailStyle {
    #[default]
    Classic,
    Owned,
    FreeForAll,
}

/// The respawn shield's kind; the values match `PlayerShieldType`.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum ShieldStyle {
    #[default]
    NoShield,
    Soft,
    SoftWithStomp,
    Hard,
}

/// Which star mode variant is played.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum StarStyle {
    #[default]
    Ztar,
    Shine,
    Multi,
    Random,
}

/// Which warps lock, for `warplocktime` ticks, after a player uses one.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum WarpLockStyle {
    #[default]
    EntranceOnly,
    ExitOnly,
    EntranceAndExit,
    /// Both warp ends of the connection used.
    EntireConnection,
    /// Every warp on the map.
    AllWarps,
}

/// How a thrown boomerang flies.
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
