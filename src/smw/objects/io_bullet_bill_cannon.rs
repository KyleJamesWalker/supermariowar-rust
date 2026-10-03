//! Port of src/smw/objects/IO_BulletBillCannon.cpp

use crate::common::game_values::if_sound_on_play;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_bulletbillcannon, CObject, CObjectTrait};
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::gs_gameplay::objectcontainer;
use crate::smw::objects::moving::mo_bullet_bill::MO_BulletBill;
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// class IO_BulletBillCannon - gets update calls and shoots bullet bills based on timer
//------------------------------------------------------------------------------
pub struct IO_BulletBillCannon {
    pub cobject: CObject,

    pub m_freq: i16,
    pub m_timer: i16,
    pub m_vel: f32,
    pub m_preview: bool,
}
impl_base!(IO_BulletBillCannon => cobject: CObject);

impl IO_BulletBillCannon {
    pub fn new(pos: Vec2s, freq: i16, vel: f32, preview: bool) -> Self {
        let mut this = IO_BulletBillCannon { cobject: CObject::new(Ptr::null(), pos), m_freq: freq, m_timer: 0, m_vel: vel, m_preview: preview };
        this.objectType = object_bulletbillcannon;
        this.set_new_timer();
        this
    }

    fn set_new_timer(&mut self) {
        self.m_timer = (self.m_freq as i32 + RANDOM_INT(self.m_freq as i32)) as i16;
    }
}

impl CObjectTrait for IO_BulletBillCannon {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self) {}

    fn update(&mut self) {
        self.m_timer -= 1;
        if self.m_timer <= 0 {
            self.set_new_timer();

            let pos = Vec2s::new((self.ix as i32 + if self.m_vel < 0.0f32 { 32 } else { -32 }) as i16, self.iy);
            unsafe {
                objectcontainer[1].add(Ptr::new_box(MO_BulletBill::new(
                    Ptr::from_mut(&mut rm.spr_hazard_bulletbill[if self.m_preview { 1 } else { 0 }]),
                    Ptr::from_mut(&mut rm.spr_hazard_bulletbilldead),
                    pos,
                    self.m_vel,
                    0,
                    true,
                )));
                if_sound_on_play(&mut rm.sfx_bulletbillsound);
            }
        }
    }

    fn collide_player(&mut self, _player: Ptr<CPlayer>) -> bool {
        false
    }
}
