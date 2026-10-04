pub mod io_bullet_bill_cannon;
pub mod io_flame_cannon;
pub mod mystery_mushroom_temp_player;
pub mod switch_color;
pub mod throw_block_type;
pub mod blocks;
pub mod carriable;
pub mod moving;
pub mod overmap;
pub mod powerup;
pub mod walkingenemy;

use crate::common::object_base::CObjectTrait;
use crate::globals::Ptr;
use crate::smw::net_random::arg_f32;

/// `net_random::Ev::Place`: relocates the object as its own placement function would.
pub fn net_place(mut obj: Ptr<dyn CObjectTrait>, extra: i32) {
    if obj.is_null() {
        return;
    }
    let any = obj.as_any();
    if let Some(o) = any.downcast_mut::<moving::mo_coin::MO_Coin>() {
        o.place_coin();
    } else if let Some(o) = any.downcast_mut::<moving::mo_collection_card::MO_CollectionCard>() {
        o.place_card();
    } else if let Some(o) = any.downcast_mut::<moving::mo_frenzy_card::MO_FrenzyCard>() {
        o.place_card();
    } else if let Some(o) = any.downcast_mut::<moving::mo_yoshi::MO_Yoshi>() {
        o.place_yoshi();
    } else if let Some(o) = any.downcast_mut::<overmap::wo_area::OMO_Area>() {
        o.place_area();
    } else if let Some(o) = any.downcast_mut::<overmap::wo_king_of_the_hill_zone::OMO_KingOfTheHillZone>() {
        o.place_area();
    } else if let Some(o) = any.downcast_mut::<carriable::co_egg::CO_Egg>() {
        o.place_egg();
    } else if let Some(o) = any.downcast_mut::<carriable::co_flag::CO_Flag>() {
        o.place_flag();
    } else if let Some(o) = any.downcast_mut::<moving::mo_flag_base::MO_FlagBase>() {
        o.place_flag_base(extra != 0);
    } else if let Some(o) = any.downcast_mut::<carriable::co_phanto_key::CO_PhantoKey>() {
        o.place_key();
    } else if let Some(o) = any.downcast_mut::<powerup::pu_secret_powerup::PU_SecretPowerup>() {
        o.place();
    }
}

/// `net_random::Ev::Wander`: takes the game host's position and heading, then turns as the host did.
pub fn net_wander(mut obj: Ptr<dyn CObjectTrait>, args: &[i32]) {
    if obj.is_null() {
        return;
    }
    let (fx, fy, angle) = (arg_f32(args[1]), arg_f32(args[2]), arg_f32(args[3]));
    let any = obj.as_any();
    if let Some(o) = any.downcast_mut::<moving::mo_flag_base::MO_FlagBase>() {
        o.set_xf(fx);
        o.set_yf(fy);
        o.angle = angle;
        o.change_angle();
    } else if let Some(o) = any.downcast_mut::<overmap::wo_race_goal::OMO_RaceGoal>() {
        o.set_xf(fx);
        o.set_yf(fy);
        o.angle = angle;
        o.change_angle();
    } else if let Some(o) = any.downcast_mut::<overmap::wo_phanto::OMO_Phanto>() {
        o.set_xf(fx);
        o.set_yf(fy);
        o.change_speed();
    }
}

/// `net_random::Ev::HazardTimer`.
pub fn net_hazard_timer(mut obj: Ptr<dyn CObjectTrait>) {
    if obj.is_null() {
        return;
    }
    let any = obj.as_any();
    if let Some(o) = any.downcast_mut::<io_flame_cannon::IO_FlameCannon>() {
        o.set_new_timer();
    } else if let Some(o) = any.downcast_mut::<moving::mo_pirhana_plant::MO_PirhanaPlant>() {
        o.set_new_timer();
    } else if let Some(o) = any.downcast_mut::<io_bullet_bill_cannon::IO_BulletBillCannon>() {
        o.fire();
    }
}

pub fn net_hazard_state(mut obj: Ptr<dyn CObjectTrait>) -> String {
    if obj.is_null() {
        return "gone".to_string();
    }
    let any = obj.as_any();
    if let Some(o) = any.downcast_mut::<io_flame_cannon::IO_FlameCannon>() {
        format!("t{}", o.iTimer)
    } else if let Some(o) = any.downcast_mut::<moving::mo_pirhana_plant::MO_PirhanaPlant>() {
        format!("t{}f{}", o.iTimer, o.iFrame)
    } else if let Some(o) = any.downcast_mut::<io_bullet_bill_cannon::IO_BulletBillCannon>() {
        format!("t{}", o.m_timer)
    } else {
        String::new()
    }
}
