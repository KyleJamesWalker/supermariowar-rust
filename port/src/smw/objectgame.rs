//! Port of src/smw/objectgame.cpp

use crate::globals::Ptr;
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum PowerupType {
    PoisonMushroom,
    ExtraLife1,
    ExtraLife2,
    ExtraLife3,
    ExtraLife5,
    Fire,
    Star,
    Clock,
    Bobomb,
    Pow,
    BulletBill,
    Hammer,
    ShellGreen,
    ShellRed,
    ShellSpiny,
    ShellBuzzy,
    Mod,
    Feather,
    MysteryMushroom,
    Boomerang,
    Tanooki,
    IceWand,
    Podobo,
    Bomb,
    Leaf,
    PWings,
    JailKey,
}

pub fn removeifprojectile(_object: Ptr<dyn IO_MovingObjectTrait>, _playsound: bool, _forcedead: bool) {
    todo!("port removeifprojectile")
}
