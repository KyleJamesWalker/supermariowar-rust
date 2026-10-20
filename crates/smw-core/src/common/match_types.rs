//! The kinds of match the menus start and the minigames and bosses they pick from.

/// How the current match was started and what follows it.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum MatchType {
    /// One game, then back to the menu.
    #[default]
    SingleGame,
    /// A series of games until a team wins enough of them.
    Tournament,
    /// A tour: a fixed list of maps and modes.
    Tour,
    /// A minigame picked from the menu.
    MiniGame,
    /// A world map.
    World,
    /// A single game started from the main menu's quick game option.
    QuickGame,
    /// A netplay match.
    NetGame,
}

/// The minigames the menu offers.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum Minigame {
    #[default]
    PipeCoin,
    HammerBoss,
    BombBoss,
    FireBoss,
    Boxes,
}

/// The boss a boss minigame fights.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum Boss {
    #[default]
    Hammer,
    Bomb,
    Fire,
}

crate::enum_from_u8!(MatchType, 7);
crate::enum_from_u8!(Minigame, 5);
crate::enum_from_u8!(Boss, 3);
