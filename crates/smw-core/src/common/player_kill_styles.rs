//! How a player was killed, for kill awards, stats and game-mode rules.

/// What killed a player.
#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum KillStyle {
    #[default]
    Stomp,
    Star,
    Fireball,
    Bobomb,
    Bounce,
    Pow,
    Goomba,
    BulletBill,
    Hammer,
    Shell,
    ThrowBlock,
    CheepCheep,
    Koopa,
    Boomerang,
    Feather,
    IceBlast,
    Podobo,
    Bomb,
    Leaf,
    PWings,
    KuriboShoe,
    PoisonMushroom,
    /// A map hazard with no player to credit.
    Environment,
    /// A map hazard credited to the player who pushed the victim into it.
    Push,
    BuzzyBeetle,
    Spiny,
    Phanto,
}
