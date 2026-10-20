//! Map Bullet Bill cannon that fires on a random interval.

use crate::smw::players::PlayerId;
use crate::common::resource_manager::SpriteId;
use crate::game::Game;
use crate::common::global_constants::{ticks, vel};
use crate::common::game_values::if_sound_on_play;
use crate::common::math::vec2::Vec2s;
use crate::common::object_base::{object_bulletbillcannon, CObject, CObjectTrait};
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;
use crate::impl_base;
use crate::smw::objects::moving::mo_bullet_bill::MO_BulletBill;
use crate::smw::player::CPlayer;

//------------------------------------------------------------------------------
// Invisible map object that fires Bullet Bills on a timer.
//------------------------------------------------------------------------------
#[derive(Clone)]
pub struct IO_BulletBillCannon {
    pub cobject: CObject,

    /// Base interval between shots in 62.5 Hz ticks; each wait is one to two times this.
    pub m_freq: i16,
    /// Ticks left before the next shot.
    pub m_timer: i16,
    /// Bullet speed (pixels per tick); its sign sets the firing direction.
    pub m_vel: f32,
    /// Cannon in the map preview; fires preview-size bullets.
    pub m_preview: bool,
}
impl_base!(IO_BulletBillCannon => cobject: CObject);

impl IO_BulletBillCannon {
    pub fn new(g: &mut Game, pos: Vec2s, freq: i16, velocity: f32, preview: bool) -> Self {
        let mut gp = g.ptr();
        let mut this = IO_BulletBillCannon { cobject: CObject::new(gp.get(), SpriteId::NONE, pos), m_freq: freq, m_timer: 0, m_vel: vel(velocity), m_preview: preview };
        this.objectType = object_bulletbillcannon;
        this.set_new_timer(g);
        this
    }

    fn set_new_timer(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.m_timer = ticks(self.m_freq as i32 + RANDOM_INT(gp.get(), self.m_freq as i32)) as i16;
    }
}

impl CObjectTrait for IO_BulletBillCannon {
    crate::impl_cobject_plumbing!();

    fn draw(&mut self, g: &mut Game) {}

    fn update(&mut self, g: &mut Game) {
        self.m_timer -= 1;
        if self.m_timer <= 0 {
            self.fire(g);
        }
    }

    fn collide_player(&mut self, g: &mut Game, _player: PlayerId) -> bool {
        false
    }
}

impl IO_BulletBillCannon {
    /// Fires a Bullet Bill from beside the cannon and rolls the next wait.
    pub fn fire(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.set_new_timer(g);

        let pos = Vec2s::new((self.ix as i32 + if self.m_vel < 0.0f32 { 32 } else { -32 }) as i16, self.iy);
        let obj = Ptr::new_box(MO_BulletBill::new(
            g,
            gp.get().rm.spr_hazard_bulletbill[if self.m_preview { 1 } else { 0 }],
            gp.get().rm.spr_hazard_bulletbilldead,
            pos,
            self.m_vel,
            0,
            true,
        ));
        gp.get().objectcontainer[1].add(g, obj);
        if_sound_on_play(g, &mut gp.rm.sfx_bulletbillsound);
    }
}
