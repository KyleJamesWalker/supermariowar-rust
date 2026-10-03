//! Port of src/smw/objecthazard.cpp

use crate::common::global_constants::*;
use crate::common::math::vec2::Vec2s;
use crate::globals::*;
use crate::smw::gs_gameplay::{noncolcontainer, objectcontainer};
use crate::smw::objects::io_bullet_bill_cannon::IO_BulletBillCannon;
use crate::smw::objects::io_flame_cannon::IO_FlameCannon;
use crate::smw::objects::moving::mo_pirhana_plant::MO_PirhanaPlant;
use crate::smw::objects::overmap::wo_orbit_hazard::OMO_OrbitHazard;

pub fn load_map_hazards(fPreview: bool) {
    unsafe {
        //Make sure we don't have any objects created before we create them from the map settings
        noncolcontainer.clean();
        objectcontainer[0].clean();
        objectcontainer[1].clean();
        objectcontainer[2].clean();

        let iPreview: usize = if fPreview { 1 } else { 0 };

        //Create objects for all the map hazards
        for i in 0..g_map.maphazards.len() {
            let hazard = g_map.maphazards[i].clone();
            let pos = Vec2s::new((hazard.ix as i32 * 16) as i16, (hazard.iy as i32 * 16) as i16);

            if hazard.itype == 0 {
                for iFireball in 0..hazard.iparam[0] {
                    objectcontainer[1].add(Ptr::new_box(OMO_OrbitHazard::new(
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
                    )));
                }
            } else if hazard.itype == 1 {
                let dSector: f32 = TWO_PI / hazard.iparam[0] as f32;
                for iRotoDisc in 0..hazard.iparam[0] {
                    let mut dAngle: f32 = hazard.dparam[1] + iRotoDisc as f32 * dSector;
                    if dAngle > TWO_PI {
                        dAngle -= TWO_PI;
                    }

                    objectcontainer[1].add(Ptr::new_box(OMO_OrbitHazard::new(
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
                    )));
                }
            } else if hazard.itype == 2 {
                noncolcontainer.add(Ptr::new_box(IO_BulletBillCannon::new(pos, hazard.iparam[0], hazard.dparam[0], fPreview)));
            } else if hazard.itype == 3 {
                objectcontainer[1].add(Ptr::new_box(IO_FlameCannon::new(pos, hazard.iparam[0], hazard.iparam[1])));
            } else if hazard.itype >= 4 && hazard.itype <= 7 {
                objectcontainer[1].add(Ptr::new_box(MO_PirhanaPlant::new(pos, hazard.itype - 4, hazard.iparam[0], hazard.iparam[1], fPreview)));
            }
        }
    }
}
