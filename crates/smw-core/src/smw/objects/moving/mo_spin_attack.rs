//! Port of src/smw/objects/moving/MO_SpinAttack.cpp

use crate::common::game::App;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::*;
use crate::common::io_block::IO_BlockTrait;
use crate::common::math::vec2::Vec2s;
use crate::common::moving_object_types::*;
use crate::common::object_base::{object_moving, CObjectTrait};
use crate::common::player_kill_styles::KillStyle;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::carriable::co_shell::CO_Shell;
use crate::smw::objects::carriable::co_throw_block::CO_ThrowBlock;
use crate::smw::objects::carriable::co_throw_box::CO_ThrowBox;
use crate::smw::objects::moving::mo_attack_zone::{mo_attack_zone_collide_player, mo_attack_zone_die, mo_attack_zone_update, MO_AttackZone, MO_AttackZoneTrait};
use crate::smw::objects::moving::moving_object::IO_MovingObjectTrait;
use crate::smw::player::{get_player_from_global_id, CPlayer};

//------------------------------------------------------------------------------
// class spin death (spinning cape or tail)
//------------------------------------------------------------------------------
pub struct MO_SpinAttack {
    pub mo_attack_zone: MO_AttackZone,

    pub fDirection: bool,
    pub iOffsetY: i16,
}
impl_base!(MO_SpinAttack => mo_attack_zone: MO_AttackZone);

impl MO_SpinAttack {
    pub fn new(playerId: i16, teamId: i16, style: KillStyle, direction: bool, offsety: i16) -> Self {
        let mut o = MO_SpinAttack {
            mo_attack_zone: MO_AttackZone::new(playerId, teamId, Vec2s::zero(), Vec2s::new(24, 12), 16, style, true),
            fDirection: false,
            iOffsetY: 0,
        };

        o.fDirection = direction;
        o.iOffsetY = offsety;

        o.state = 0;
        o.objectType = object_moving;
        o
    }
}

impl CObjectTrait for MO_SpinAttack {
    crate::impl_cobject_plumbing!();
    fn as_io_moving_object(&mut self) -> Option<&mut dyn IO_MovingObjectTrait> {
        Some(self)
    }

    fn collide_player(&mut self, player: Ptr<CPlayer>) -> bool {
        if self.iTimer > 11 {
            return false;
        }

        mo_attack_zone_collide_player(self, player)
    }

    /*
    void MO_SpinAttack::draw()
    {
            if (iTimer <= 11 && !dead)
            {
                    SDL_Rect r = {ix, iy, collisionWidth, collisionHeight};
                    crate::common::gfx::blit::fill_rect(blitdest, &r, 0xff00);
            }
    }
    */
    fn draw(&mut self) {}

    fn update(&mut self) {
        mo_attack_zone_update(self);

        if self.iTimer <= 11 {
            self.state = 1;
        }

        unsafe {
            let owner = get_player_from_global_id(self.iPlayerID);

            if !owner.is_null() {
                // Move to the owner
                let xi = (owner.ix as i32 - PWOFFSET + if self.fDirection { 24 } else { -16 }) as i16;
                self.set_xi(xi);
                let yi = (owner.iy as i32 + PH - self.iOffsetY as i32) as i16;
                self.set_yi(yi);

                if self.iTimer < 5 || self.iy as i32 + (self.collisionHeight as i32) < 0 {
                    return;
                }

                // Check block collisions
                let iTop: i16 = (self.iy as i32 / TILESIZE) as i16;
                let iBottom: i16 = ((self.iy as i32 + self.collisionHeight as i32) / TILESIZE) as i16;

                let mut iLeft: i16;
                if self.ix < 0 {
                    iLeft = ((self.ix as i32 + App::screenWidth) / TILESIZE) as i16;
                } else {
                    iLeft = (self.ix as i32 / TILESIZE) as i16;
                }

                let mut iRight: i16 = ((self.ix as i32 + self.collisionWidth as i32) / TILESIZE) as i16;

                if iLeft < 0 {
                    iLeft += 20;
                }

                if iLeft >= 20 {
                    iLeft -= 20;
                }

                if iRight < 0 {
                    iRight += 20;
                }

                if iRight >= 20 {
                    iRight -= 20;
                }

                let mut topleftblock: Ptr<dyn IO_BlockTrait> = Ptr::null();
                let mut toprightblock: Ptr<dyn IO_BlockTrait> = Ptr::null();
                let mut bottomleftblock: Ptr<dyn IO_BlockTrait> = Ptr::null();
                let mut bottomrightblock: Ptr<dyn IO_BlockTrait> = Ptr::null();

                if iTop >= 0 && iTop < 15 {
                    topleftblock = g_map.block(iLeft, iTop);
                    toprightblock = g_map.block(iRight, iTop);
                }

                if iBottom >= 0 && iBottom < 15 {
                    bottomleftblock = g_map.block(iLeft, iBottom);
                    bottomrightblock = g_map.block(iRight, iBottom);
                }

                let mut fHitBlock = false;
                if !topleftblock.is_null() && !topleftblock.is_transparent() && !topleftblock.is_hidden() {
                    fHitBlock = topleftblock.collide_object_dir(self.as_mo_ptr(), 3);
                }

                if !fHitBlock && !toprightblock.is_null() && !toprightblock.is_transparent() && !toprightblock.is_hidden() {
                    fHitBlock = toprightblock.collide_object_dir(self.as_mo_ptr(), 1);
                }

                if !fHitBlock && !bottomleftblock.is_null() && !bottomleftblock.is_transparent() && !bottomleftblock.is_hidden() {
                    fHitBlock = bottomleftblock.collide_object_dir(self.as_mo_ptr(), 3);
                }

                if !fHitBlock && !bottomrightblock.is_null() && !bottomrightblock.is_transparent() && !bottomrightblock.is_hidden() {
                    fHitBlock = bottomrightblock.collide_object_dir(self.as_mo_ptr(), 1);
                }

                if fHitBlock {
                    self.dead = true;
                }
            } else {
                self.dead = true;
            }
        }
    }

    fn collide_object(&mut self, mut object: Ptr<dyn IO_MovingObjectTrait>) {
        if self.dead || self.iTimer > 11 {
            return;
        }

        unsafe {
            let r#type: MovingObjectType = object.get_moving_object_type();

            if r#type == movingobject_shell || r#type == movingobject_throwblock || r#type == movingobject_throwbox {
                if r#type == movingobject_shell {
                    let shell = object.as_any().downcast_mut::<CO_Shell>().unwrap();

                    if shell.frozen {
                        shell.shatter_die();
                    } else {
                        shell.flip();
                        if_sound_on_play(&mut rm.sfx_kicksound);
                    }

                    self.die();
                } else if r#type == movingobject_throwblock {
                    let block = object.as_any().downcast_mut::<CO_ThrowBlock>().unwrap();

                    if block.frozen {
                        block.shatter_die();
                    } else if block.owner.is_null() || block.owner.globalID != self.iPlayerID {
                        block.die();
                        if_sound_on_play(&mut rm.sfx_kicksound);
                        self.die();
                    }
                } else if r#type == movingobject_throwbox {
                    let r#box = object.as_any().downcast_mut::<CO_ThrowBox>().unwrap();

                    if r#box.frozen {
                        r#box.shatter_die();
                    } else if r#box.owner.is_null() || r#box.owner.globalID != self.iPlayerID {
                        r#box.die();
                        if_sound_on_play(&mut rm.sfx_kicksound);
                        self.die();
                    }
                }
            }
        }
    }
}

impl IO_MovingObjectTrait for MO_SpinAttack {
    crate::impl_io_moving_object_plumbing!();

    fn die(&mut self) {
        mo_attack_zone_die(self)
    }
}

impl MO_AttackZoneTrait for MO_SpinAttack {
    fn az(&self) -> &MO_AttackZone {
        &self.mo_attack_zone
    }
    fn az_mut(&mut self) -> &mut MO_AttackZone {
        &mut self.mo_attack_zone
    }
}
