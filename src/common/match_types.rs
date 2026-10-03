//! Port of src/common/MatchTypes.h

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum MatchType {
    #[default]
    SingleGame,
    Tournament,
    Tour,
    MiniGame,
    World,
    QuickGame,
    NetGame,
}

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
