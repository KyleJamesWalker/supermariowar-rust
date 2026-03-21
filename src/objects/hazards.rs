use bevy::prelude::*;

/// Types of environmental hazards.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardType {
    Thwomp,        // Falls from ceiling
    FireBar,       // Rotating fire chain
    PiranhaPlant,  // Appears from pipes
    BulletCannon,  // Fires bullet bills
    Fireball,      // Rotating fireball (podoboo)
    Spikes,        // Static spike trap
}

/// Hazard behavior data.
#[derive(Component, Debug)]
pub struct Hazard {
    pub hazard_type: HazardType,
    pub active: bool,
    pub cycle_timer: f32,
    pub cycle_duration: f32,
}

impl Hazard {
    pub fn thwomp() -> Self {
        Self {
            hazard_type: HazardType::Thwomp,
            active: true,
            cycle_timer: 0.0,
            cycle_duration: 120.0,
        }
    }

    pub fn bullet_cannon() -> Self {
        Self {
            hazard_type: HazardType::BulletCannon,
            active: true,
            cycle_timer: 0.0,
            cycle_duration: 200.0,
        }
    }

    pub fn piranha_plant() -> Self {
        Self {
            hazard_type: HazardType::PiranhaPlant,
            active: true,
            cycle_timer: 0.0,
            cycle_duration: 180.0,
        }
    }
}
