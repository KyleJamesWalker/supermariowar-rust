//! Port of src/smw/objects/moving/MO_AttackZone.cpp

use crate::common::game_values::if_sound_on_play;
use crate::common::gameplay_styles::TeamCollisionStyle;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{object_moving, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::carriable::co_shell::CO_Shell;
use crate::smw::objects::carriable::co_throw_block::CO_ThrowBlock;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::mo_spin_attack::MO_SpinAttack;
use crate::smw::objects::moving::moving_object::{IO_MovingObject, IO_MovingObjectTrait};
use crate::smw::player::{get_player_from_global_id, player_killed_player, CPlayer, PlayerDeathStyle};

//------------------------------------------------------------------------------
// attack zone(invisible area that kills objects and players)
//------------------------------------------------------------------------------
pub struct MO_AttackZone {
    pub io_moving_object: IO_MovingObject,

    pub fDieOnCollision: bool,

    pub iTimer: i16,
    pub iStyle: KillStyle,
}
impl_base!(MO_AttackZone => io_moving_object: IO_MovingObject);

impl MO_AttackZone {
    pub fn new(playerId: i16, teamId: i16, pos: Vec2s, size: Vec2s, time: i16, style: KillStyle, dieoncollision: bool) -> Self {
        let mut o = MO_AttackZone {
            io_moving_object: IO_MovingObject::new(Ptr::null(), pos, 1, 0, size.x, size.y, 0, 0, -1, -1, -1, -1),
            fDieOnCollision: false,
            iTimer: 0,
            iStyle: KillStyle::Stomp,
        };

        o.iPlayerID = playerId;
        o.iTeamID = teamId;
        o.iStyle = style;

        o.objectType = object_moving;
        o.movingObjectType = movingobject_attackzone;

        o.iTimer = time;
        o.fDieOnCollision = dieoncollision;

        o.state = 1;

        o.fObjectCollidesWithMap = false;
        o
    }
}

/// `(MO_AttackZone*)object` after a `movingobject_attackzone` check: the object is an `MO_AttackZone` or an `MO_SpinAttack`.
pub fn as_attack_zone<'a>(object: Ptr<dyn IO_MovingObjectTrait>) -> &'a mut MO_AttackZone {
    let o = object.get();
    if o.as_any().is::<MO_SpinAttack>() {
        return &mut o.as_any().downcast_mut::<MO_SpinAttack>().unwrap().mo_attack_zone;
    }
    o.as_any().downcast_mut::<MO_AttackZone>().unwrap()
}

/// Virtuals with a body in `MO_AttackZone`, overridable by `MO_SpinAttack`.
pub trait MO_AttackZoneTrait: IO_MovingObjectTrait {
    fn az(&self) -> &MO_AttackZone;
    fn az_mut(&mut self) -> &mut MO_AttackZone;
}

pub fn mo_attack_zone_collide_player<T: MO_AttackZoneTrait + ?Sized>(this: &mut T, mut player: Ptr<CPlayer>) -> bool {
    unsafe {
        let (dead, iTeamID, iPlayerID, iStyle) = {
            let z = this.az();
            (z.dead, z.iTeamID, z.iPlayerID, z.iStyle)
        };

        if player.is_shielded() || player.is_invincible() || player.shyguy || dead {
            return false;
        }

        if game_values.teamcollision != TeamCollisionStyle::On && player.teamID == iTeamID {
            return false;
        }

        let killer = get_player_from_global_id(iPlayerID);

        if !killer.is_null() && killer.globalID == player.globalID {
            return false;
        }

        player_killed_player(iPlayerID, player, PlayerDeathStyle::Jump, iStyle, false, false);

        this.die();

        true
    }
}

/*void MO_AttackZone::draw()
{
        if (!dead)
        {
                SDL_Rect r = {ix, iy, collisionWidth, collisionHeight};
                SDL_FillRect(blitdest, &r, 0xf000);
        }
}*/

pub fn mo_attack_zone_update<T: MO_AttackZoneTrait + ?Sized>(this: &mut T) {
    let z = this.az_mut();
    z.iTimer -= 1;
    if z.iTimer <= 0 {
        z.dead = true;
    }
}

pub fn mo_attack_zone_collide_object<T: MO_AttackZoneTrait + ?Sized>(this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) {
    unsafe {
        if this.az().dead {
            return;
        }

        let iPlayerID = this.az().iPlayerID;

        let r#type: MovingObjectType = object.get_moving_object_type();

        if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox {
            if r#type == movingobject_shell {
                let shell = object.as_any().downcast_mut::<CO_Shell>().unwrap();
                if shell.frozen {
                    shell.shatter_die();
                } else {
                    shell.die();
                }
            } else if r#type == movingobject_throwblock {
                let block = object.as_any().downcast_mut::<CO_ThrowBlock>().unwrap();

                if block.frozen {
                    block.shatter_die();
                } else if block.owner.is_null() || block.owner.globalID != iPlayerID {
                    block.die();
                }
            } else if r#type == movingobject_throwbox {
                let r#box = object.as_any().downcast_mut::<CO_ThrowBox>().unwrap();

                if r#box.frozen {
                    r#box.shatter_die();
                } else if r#box.owner.is_null() || r#box.owner.globalID != iPlayerID {
                    r#box.die();
                }
            }

            if_sound_on_play(&mut rm.sfx_kicksound);
        }
    }
}

pub fn mo_attack_zone_die<T: MO_AttackZoneTrait + ?Sized>(this: &mut T) {
    let z = this.az_mut();
    if z.fDieOnCollision {
        z.dead = true;
    }
}

impl CObjectTrait for MO_AttackZone {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn update(&mut self) {
        mo_attack_zone_update(self)
    }
    // This is invisible
    fn draw(&mut self) {}
    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        mo_attack_zone_collide_player(self, player)
    }
    fn collide_object(&mut self, object: Ptr<dyn IO_MovingObjectTrait>) {
        mo_attack_zone_collide_object(self, object)
    }
}

impl IO_MovingObjectTrait for MO_AttackZone {
    crate::impl_io_moving_object_plumbing!();

    fn die(&mut self) {
        mo_attack_zone_die(self)
    }
}

impl MO_AttackZoneTrait for MO_AttackZone {
    fn az(&self) -> &MO_AttackZone {
        self
    }
    fn az_mut(&mut self) -> &mut MO_AttackZone {
        self
    }
}
