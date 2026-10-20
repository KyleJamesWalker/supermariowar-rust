//! What a kill did to the victim, as a game mode reports it.

/// The outcome of a kill once the game mode has had its say.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum PlayerKillType {
    #[default]
    /// Nothing happened.
    None,
    /// The player died and respawns.
    Normal,
    /// The kill ended the match or removed the player's team.
    Removed,
    /// The mode kept the player alive (for example by taking health instead).
    NonKill,
}

crate::enum_from_u8!(PlayerKillType, 4);
