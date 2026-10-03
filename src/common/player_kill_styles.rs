//! Port of src/common/PlayerKillStyles.h

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
    Environment,
    Push,
    BuzzyBeetle,
    Spiny,
    Phanto,
}
