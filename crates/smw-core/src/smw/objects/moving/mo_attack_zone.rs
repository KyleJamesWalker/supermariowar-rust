//! Invisible attack zone: a timed hitbox that kills players and breaks thrown objects for its owner.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
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
/// Invisible hitbox owned by a player; expires after `iTimer` ticks.
#[derive(Clone)]
pub struct MO_AttackZone {
    pub io_moving_object: IO_MovingObject,

    /// The zone disappears after its first hit.
    pub fDieOnCollision: bool,

    /// Ticks left before the zone expires.
    pub iTimer: i16,
    /// Kill style credited to the owner for a kill.
    pub iStyle: KillStyle,
}
impl_base!(MO_AttackZone => io_moving_object: IO_MovingObject);

impl MO_AttackZone {
    pub fn new(g: &mut Game, playerId: i16, teamId: i16, pos: Vec2s, size: Vec2s, time: i16, style: KillStyle, dieoncollision: bool) -> Self {
        let mut o = MO_AttackZone {
            io_moving_object: IO_MovingObject::new(g, SpriteId::NONE, pos, 1, 0, size.x, size.y, 0, 0, -1, -1, -1, -1),
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

/// Views a `movingobject_attackzone` object as its `MO_AttackZone`, which may sit inside an `MO_SpinAttack`.
pub fn as_attack_zone<'a>(object: Ptr<dyn IO_MovingObjectTrait>) -> &'a mut MO_AttackZone {
    let o = object.get();
    if o.as_any().is::<MO_SpinAttack>() {
        return &mut o.as_any().downcast_mut::<MO_SpinAttack>().unwrap().mo_attack_zone;
    }
    o.as_any().downcast_mut::<MO_AttackZone>().unwrap()
}

/// Access to the `MO_AttackZone` state shared by attack zones and spin attacks.
pub trait MO_AttackZoneTrait: IO_MovingObjectTrait {
    fn az(&self) -> &MO_AttackZone;
    fn az_mut(&mut self) -> &mut MO_AttackZone;
}

/// Kills a vulnerable player other than the owner (teammates only with team collision on); returns true on a kill.
pub fn mo_attack_zone_collide_player<T: MO_AttackZoneTrait + ?Sized>(g: &mut Game, this: &mut T, mut player: PlayerId) -> bool {
    let (dead, iTeamID, iPlayerID, iStyle) = {
        let z = this.az();
        (z.dead, z.iTeamID, z.iPlayerID, z.iStyle)
    };

    if g.player(player).is_shielded() || g.player(player).is_invincible() || g.player(player).shyguy || dead {
        return false;
    }

    if g.game_values.teamcollision != TeamCollisionStyle::On && g.player(player).teamID == iTeamID {
        return false;
    }

    let killer = get_player_from_global_id(g, iPlayerID);

    if !killer.is_none() && g.player(killer).globalID == g.player(player).globalID {
        return false;
    }

    player_killed_player(g, iPlayerID, player, PlayerDeathStyle::Jump, iStyle, false, false);

    this.die(g);

    true
}

/// Counts down `iTimer` and removes the zone when it runs out.
pub fn mo_attack_zone_update<T: MO_AttackZoneTrait + ?Sized>(this: &mut T) {
    let z = this.az_mut();
    z.iTimer -= 1;
    if z.iTimer <= 0 {
        z.dead = true;
    }
}

/// Breaks shells and other players' thrown blocks and boxes; shatters frozen ones.
pub fn mo_attack_zone_collide_object<T: MO_AttackZoneTrait + ?Sized>(g: &mut Game, this: &mut T, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        let mut gp = g.ptr();
    if this.az().dead {
        return;
    }

    let iPlayerID = this.az().iPlayerID;

    let r#type: MovingObjectType = object.get_moving_object_type();

    if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox {
        if r#type == movingobject_shell {
            let shell = object.as_any().downcast_mut::<CO_Shell>().unwrap();
            if shell.frozen {
                shell.shatter_die(g);
            } else {
                shell.die(g);
            }
        } else if r#type == movingobject_throwblock {
            let block = object.as_any().downcast_mut::<CO_ThrowBlock>().unwrap();

            if block.frozen {
                block.shatter_die(g);
            } else if block.owner.is_none() || g.player(block.owner).globalID != iPlayerID {
                block.die(g);
            }
        } else if r#type == movingobject_throwbox {
            let r#box = object.as_any().downcast_mut::<CO_ThrowBox>().unwrap();

            if r#box.frozen {
                r#box.shatter_die(g);
            } else if r#box.owner.is_none() || g.player(r#box.owner).globalID != iPlayerID {
                r#box.die(g);
            }
        }

        if_sound_on_play(g, &mut gp.rm.sfx_kicksound);
    }
}

/// Removes the zone only if `fDieOnCollision` is set.
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

    fn update(&mut self, g: &mut Game) {
        mo_attack_zone_update(self)
    }
    // This is invisible
    fn draw(&mut self, g: &mut Game) {}
    fn collide_player(&mut self, g: &mut Game, player: PlayerId) -> bool {
        mo_attack_zone_collide_player(g, self, player)
    }
    fn collide_object(&mut self, g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) {
        mo_attack_zone_collide_object(g, self, object)
    }
}

impl IO_MovingObjectTrait for MO_AttackZone {
    crate::impl_io_moving_object_plumbing!();

    fn die(&mut self, g: &mut Game) {
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
