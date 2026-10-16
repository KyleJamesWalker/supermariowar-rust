//! Port of src/smw/objecthazard.cpp

use crate::game::Game;
use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::globals::*;

use crate::smw::objects::io_bullet_bill_cannon::IO_BulletBillCannon;
use crate::smw::objects::io_flame_cannon::IO_FlameCannon;
use crate::smw::objects::moving::mo_pirhana_plant::MO_PirhanaPlant;
use crate::smw::objects::overmap::wo_orbit_hazard::OMO_OrbitHazard;

pub fn load_map_hazards(g: &mut Game, fPreview: bool) {
        let mut gp = g.ptr();
        let mut rm = g.rm;
    unsafe {
        //Make sure we don't have any objects created before we create them from the map settings
        g.noncolcontainer.clean();
        g.objectcontainer[0].clean();
        g.objectcontainer[1].clean();
        g.objectcontainer[2].clean();

        let iPreview: usize = if fPreview { 1 } else { 0 };

        //Create objects for all the map hazards
        for i in 0..game().g_map.maphazards.len() {
            let hazard = g.g_map.maphazards[i].clone();
            let pos = Vec2s::new((hazard.ix as i32 * 16) as i16, (hazard.iy as i32 * 16) as i16);

            if hazard.itype == 0 {
                for iFireball in 0..hazard.iparam[0] {
                    let obj = Ptr::new_box(OMO_OrbitHazard::new(
                        g,
                        Ptr::from_mut(&mut rm.spr_hazard_fireball[iPreview]),
                        pos + Vec2s::new(16, 16),
                        (iFireball as i32 * 24) as f32,
                        hazard.dparam[0],
                        hazard.dparam[1],
                        4,
                        8,
                        18,
                        18,
                        0,
                        0,
                        0,
                        if hazard.dparam[0] < 0.0 { 18 } else { 0 },
                        18,
                        18,
                    ));
                    gp.objectcontainer[1].add(g, obj);
                }
            } else if hazard.itype == 1 {
                let dSector: f32 = TWO_PI / hazard.iparam[0] as f32;
                for iRotoDisc in 0..hazard.iparam[0] {
                    let mut dAngle: f32 = hazard.dparam[1] + iRotoDisc as f32 * dSector;
                    if dAngle > TWO_PI {
                        dAngle -= TWO_PI;
                    }

                    let obj = Ptr::new_box(OMO_OrbitHazard::new(
                        g,
                        Ptr::from_mut(&mut rm.spr_hazard_rotodisc[iPreview]),
                        pos + Vec2s::new(16, 16),
                        hazard.dparam[2],
                        hazard.dparam[0],
                        dAngle,
                        21,
                        8,
                        32,
                        32,
                        0,
                        0,
                        0,
                        0,
                        32,
                        32,
                    ));
                    gp.objectcontainer[1].add(g, obj);
                }
            } else if hazard.itype == 2 {
                let obj = Ptr::new_box(IO_BulletBillCannon::new(g, pos, hazard.iparam[0], hazard.dparam[0], fPreview));
                gp.noncolcontainer.add(g, obj);
            } else if hazard.itype == 3 {
                let obj = Ptr::new_box(IO_FlameCannon::new(g, pos, hazard.iparam[0], hazard.iparam[1]));
                gp.objectcontainer[1].add(g, obj);
            } else if hazard.itype >= 4 && hazard.itype <= 7 {
                let obj = Ptr::new_box(MO_PirhanaPlant::new(g, pos, hazard.itype - 4, hazard.iparam[0], hazard.iparam[1], fPreview));
                gp.objectcontainer[1].add(g, obj);
            }
        }
    }
}
