//! Port of src/smw/objectgame.cpp

use crate::game::Game;
use crate::common::eyecandy::EC_SingleAnimation;
use crate::common::game::App;
use crate::common::game_mode::game_mode_greed;
use crate::common::game_values::if_sound_on_play;
use crate::common::global_constants::*;
use crate::common::math::vec2::{Vec2f, Vec2s};
use crate::common::moving_object_types::*;
use crate::common::object_base::CObjectTrait;
use crate::common::random_number_generator::RANDOM_INT;
use crate::globals::*;

use crate::smw::objects::carriable::co_shell::{CO_Shell, ShellType};
use crate::smw::objects::moving::mo_coin::MO_Coin;
use crate::smw::objects::moving::mo_podobo::MO_Podobo;
use crate::smw::objects::moving::moving_object::{io_moving_object_collision_detection_checksides, IO_MovingObjectTrait};
use crate::smw::objects::powerup::powerup::MO_PowerupTrait;
use crate::smw::objects::powerup::pu_bobomb_powerup::PU_BobombPowerup;
use crate::smw::objects::powerup::pu_bomb_powerup::PU_BombPowerup;
use crate::smw::objects::powerup::pu_boomerang_powerup::PU_BoomerangPowerup;
use crate::smw::objects::powerup::pu_bullet_bill_powerup::PU_BulletBillPowerup;
use crate::smw::objects::powerup::pu_clock_powerup::PU_ClockPowerup;
use crate::smw::objects::powerup::pu_coin_powerup::{CoinColor, PU_CoinPowerup};
use crate::smw::objects::powerup::pu_extra_guy_powerup::PU_ExtraGuyPowerup;
use crate::smw::objects::powerup::pu_extra_heart_powerup::PU_ExtraHeartPowerup;
use crate::smw::objects::powerup::pu_extra_time_powerup::PU_ExtraTimePowerup;
use crate::smw::objects::powerup::pu_feather_powerup::PU_FeatherPowerup;
use crate::smw::objects::powerup::pu_fire_powerup::PU_FirePowerup;
use crate::smw::objects::powerup::pu_hammer_powerup::PU_HammerPowerup;
use crate::smw::objects::powerup::pu_ice_wand_powerup::PU_IceWandPowerup;
use crate::smw::objects::powerup::pu_jail_key_powerup::PU_JailKeyPowerup;
use crate::smw::objects::powerup::pu_leaf_powerup::PU_LeafPowerup;
use crate::smw::objects::powerup::pu_mod_powerup::PU_ModPowerup;
use crate::smw::objects::powerup::pu_mystery_mushroom_powerup::PU_MysteryMushroomPowerup;
use crate::smw::objects::powerup::pu_p_wings_powerup::PU_PWingsPowerup;
use crate::smw::objects::powerup::pu_podobo_powerup::PU_PodoboPowerup;
use crate::smw::objects::powerup::pu_poison_powerup::PU_PoisonPowerup;
use crate::smw::objects::powerup::pu_pow_powerup::PU_PowPowerup;
use crate::smw::objects::powerup::pu_secret_powerup::PU_SecretPowerup;
use crate::smw::objects::powerup::pu_star_powerup::PU_StarPowerup;
use crate::smw::objects::powerup::pu_tanooki::PU_Tanooki;
use crate::smw::player::get_player_from_global_id;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum PowerupType {
    #[default]
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
crate::enum_from_u8!(PowerupType, 27);

/// The `static_cast` targets `createpowerup` sorts a new object into.
enum Spawned {
    None,
    Powerup(Ptr<dyn MO_PowerupTrait>),
    Shell(Ptr<CO_Shell>),
    Feather(Ptr<PU_FeatherPowerup>),
    Leaf(Ptr<PU_LeafPowerup>),
    Coin(Ptr<MO_Coin>),
}

fn new_powerup<T: MO_PowerupTrait>(v: T) -> Spawned {
    let p = Ptr::new_box(v);
    Spawned::Powerup(Ptr::from_raw(p.as_ptr() as *mut dyn MO_PowerupTrait))
}

fn spawn_regular_powerup(g: &mut Game, r#type: PowerupType, spawnPos: Vec2s, movesRight: bool) -> Spawned {
    let mut rm = g.rm;
    match r#type {
        PowerupType::PoisonMushroom => new_powerup(PU_PoisonPowerup::new(g, rm.spr_poisonpowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1)),
        PowerupType::ExtraLife1 => new_powerup(PU_ExtraGuyPowerup::new(g, rm.spr_1uppowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1, 1)),
        PowerupType::ExtraLife2 => new_powerup(PU_ExtraGuyPowerup::new(g, rm.spr_2uppowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1, 2)),
        PowerupType::ExtraLife3 => new_powerup(PU_ExtraGuyPowerup::new(g, rm.spr_3uppowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1, 3)),
        PowerupType::ExtraLife5 => new_powerup(PU_ExtraGuyPowerup::new(g, rm.spr_5uppowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1, 5)),
        PowerupType::Fire => new_powerup(PU_FirePowerup::new(g, rm.spr_firepowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1)),
        PowerupType::Star => new_powerup(PU_StarPowerup::new(g, rm.spr_starpowerup, spawnPos, 4, movesRight, 2, 30, 30, 1, 1)),
        PowerupType::Clock => new_powerup(PU_ClockPowerup::new(g, rm.spr_clockpowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1)),
        PowerupType::Bobomb => new_powerup(PU_BobombPowerup::new(g, rm.spr_bobombpowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1)),
        PowerupType::Pow => new_powerup(PU_PowPowerup::new(g, rm.spr_powpowerup, spawnPos, 8, movesRight, 8, 30, 30, 1, 1)),
        PowerupType::BulletBill => new_powerup(PU_BulletBillPowerup::new(g, rm.spr_bulletbillpowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1)),
        PowerupType::Hammer => new_powerup(PU_HammerPowerup::new(g, rm.spr_hammerpowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1)),
        PowerupType::ShellGreen => Spawned::Shell(Ptr::new_box(CO_Shell::new(g, ShellType::Green, spawnPos, true, true, true, false))),
        PowerupType::ShellRed => Spawned::Shell(Ptr::new_box(CO_Shell::new(g, ShellType::Red, spawnPos, false, true, true, false))),
        PowerupType::ShellSpiny => Spawned::Shell(Ptr::new_box(CO_Shell::new(g, ShellType::Spiny, spawnPos, false, false, true, true))),
        PowerupType::ShellBuzzy => Spawned::Shell(Ptr::new_box(CO_Shell::new(g, ShellType::Buzzy, spawnPos, false, true, false, false))),
        PowerupType::Mod => new_powerup(PU_ModPowerup::new(g, rm.spr_modpowerup, spawnPos, 8, movesRight, 8, 30, 30, 1, 1)),
        PowerupType::Feather => Spawned::Feather(Ptr::new_box(PU_FeatherPowerup::new(g, rm.spr_featherpowerup, spawnPos, 1, 0, 30, 30, 1, 1))),
        PowerupType::MysteryMushroom => new_powerup(PU_MysteryMushroomPowerup::new(g, rm.spr_mysterymushroompowerup, spawnPos, 1, movesRight, 0, 30, 30, 1, 1)),
        PowerupType::Boomerang => new_powerup(PU_BoomerangPowerup::new(g, rm.spr_boomerangpowerup, spawnPos, 1, movesRight, 0, 30, 26, 1, 5)),
        PowerupType::Tanooki => new_powerup(PU_Tanooki::new(g, spawnPos)),
        PowerupType::IceWand => new_powerup(PU_IceWandPowerup::new(g, rm.spr_icewandpowerup, spawnPos, 1, 0, 30, 30, 1, 1)),
        PowerupType::Podobo => new_powerup(PU_PodoboPowerup::new(g, rm.spr_podobopowerup, spawnPos, 1, 0, 30, 30, 1, 1)),
        PowerupType::Bomb => new_powerup(PU_BombPowerup::new(g, rm.spr_bombpowerup, spawnPos, 1, 0, 30, 30, 1, 1)),
        PowerupType::Leaf => Spawned::Leaf(Ptr::new_box(PU_LeafPowerup::new(g, rm.spr_leafpowerup, spawnPos, 1, 0, 30, 30, 1, 1))),
        PowerupType::PWings => new_powerup(PU_PWingsPowerup::new(g, rm.spr_pwingspowerup, spawnPos)),
        PowerupType::JailKey => new_powerup(PU_JailKeyPowerup::new(g, rm.spr_jailkeypowerup, spawnPos)),
    }
}

fn spawn_special_powerup(g: &mut Game, r#type: i16, spawnPos: Vec2s) -> Spawned {
        let mut gp = g.ptr();
    static iCoinValue: [i16; 4] = [3, 5, 2, 10];
    let mut rm = g.rm;
    static iGreedValue: [i16; 4] = [10, 15, 5, 20];

    match r#type as i32 {
        HEALTH_POWERUP => new_powerup(PU_ExtraHeartPowerup::new(g, rm.spr_extraheartpowerup, spawnPos)),
        TIME_POWERUP => new_powerup(PU_ExtraTimePowerup::new(g, rm.spr_extratimepowerup, spawnPos)),
        JAIL_KEY_POWERUP => new_powerup(PU_JailKeyPowerup::new(g, rm.spr_jailkeypowerup, spawnPos)),
        COIN_POWERUP => {
            let iRandCoin = RANDOM_INT(gp.get(), 9) as i16;
            let mut color = CoinColor::Yellow;

            if iRandCoin == 8 {
                color = CoinColor::Blue;
            } else if iRandCoin >= 6 {
                color = CoinColor::Green;
            } else if iRandCoin >= 3 {
                color = CoinColor::Red;
            }

            let colorIdx = color as usize;

            let value = if g.mode().gamemode == game_mode_greed { iGreedValue[colorIdx] } else { iCoinValue[colorIdx] };
            new_powerup(PU_CoinPowerup::new(g, rm.spr_coin, spawnPos, color, value))
        }
        MINIGAME_COIN => Spawned::Coin(Ptr::new_box(MO_Coin::new(
            g,
            gp.rm.spr_coin,
            Vec2f::new(0.0, (-VELJUMP as f64 / 2.0) as f32),
            spawnPos,
            2,
            -1,
            2,
            0,
            false,
        ))),
        SECRET1_POWERUP => new_powerup(PU_SecretPowerup::new(g, gp.rm.spr_secret1, spawnPos, 0)),
        SECRET2_POWERUP => new_powerup(PU_SecretPowerup::new(g, gp.rm.spr_secret2, spawnPos, 1)),
        SECRET3_POWERUP => new_powerup(PU_SecretPowerup::new(g, gp.rm.spr_secret3, spawnPos, 2)),
        SECRET4_POWERUP => new_powerup(PU_SecretPowerup::new(g, gp.rm.spr_secret4, spawnPos, 3)),
        _ => Spawned::None,
    }
}

fn as_object<T: CObjectTrait>(p: Ptr<T>) -> Ptr<dyn CObjectTrait> {
    Ptr::from_raw(p.as_ptr() as *mut dyn CObjectTrait)
}

fn as_moving<T: IO_MovingObjectTrait>(p: Ptr<T>) -> Ptr<dyn IO_MovingObjectTrait> {
    Ptr::from_raw(p.as_ptr() as *mut dyn IO_MovingObjectTrait)
}

pub fn removeifprojectile(g: &mut Game, mut object: Ptr<dyn IO_MovingObjectTrait>, playsound: bool, forcedead: bool) {
        let mut gp = g.ptr();
    if object.dead {
        return;
    }

    let r#type = object.movingObjectType;
    if r#type == movingobject_fireball
        || r#type == movingobject_hammer
        || r#type == movingobject_boomerang
        || r#type == movingobject_iceblast
        || r#type == movingobject_superfireball
        || r#type == movingobject_sledgehammer
    {
        let iPlayerID = object.iPlayerID;
        let mut fDie = true;

        if r#type == movingobject_hammer && !g.game_values.hammerpower {
            fDie = false;
        }

        if fDie || forcedead {
            let mut player = get_player_from_global_id(g, iPlayerID);

            if !player.is_none() {
                g.player(player).decrease_projectiles_count();
            }

            object.dead = true;
            g.eyecandy[2].emplace(EC_SingleAnimation::new(&gp.rm, 
                g.rm.spr_fireballexplosion,
                (object.x() + (object.iw as i32 >> 1) - 16) as i16,
                (object.y() + (object.ih as i32 >> 1) - 16) as i16,
                3,
                4,
            ));
        }

        if playsound {
            if_sound_on_play(g, &mut gp.rm.sfx_hit);
        }
    }
}

pub fn createpowerup(g: &mut Game, iType: i16, pos: Vec2s, side: bool, spawn: bool) -> Ptr<dyn IO_MovingObjectTrait> {
        let mut gp = g.ptr();
    let mut rm = g.rm;
    let specialPos = pos + Vec2s::new(1, -1);

    let mut spawned = Spawned::None;

    if iType < 0 {
        spawned = spawn_special_powerup(g, iType, specialPos);
    } else if (iType as i32) < NUM_POWERUPS {
        spawned = spawn_regular_powerup(g, PowerupType::from_u8(iType as u8), specialPos, side);
    }

    match spawned {
        Spawned::Coin(coin) => {
            if_sound_on_play(g, &mut gp.rm.sfx_coin);
            if gp.objectcontainer[1].add(g, coin) {
                return as_moving(coin);
            }
        }
        Spawned::Powerup(mut powerup) => {
            if gp.objectcontainer[0].add_dyn(g, Ptr::from_raw(powerup.as_ptr() as *mut dyn CObjectTrait)) {
                if !spawn {
                    powerup.nospawn(pos.y);
                    io_moving_object_collision_detection_checksides(g, powerup.get());
                }

                return Ptr::from_raw(powerup.as_ptr() as *mut dyn IO_MovingObjectTrait);
            }
        }
        Spawned::Shell(mut shell) => {
            if gp.objectcontainer[1].add(g, shell) {
                if !spawn {
                    shell.nospawn(pos.y, true);
                    io_moving_object_collision_detection_checksides(g, shell.get());
                }

                return as_moving(shell);
            }
        }
        Spawned::Feather(mut feather) => {
            if gp.objectcontainer[0].add(g, feather) {
                if !spawn {
                    feather.nospawn(pos.y);
                }

                return as_moving(feather);
            }
        }
        Spawned::Leaf(mut leaf) => {
            if gp.objectcontainer[0].add(g, leaf) {
                if !spawn {
                    leaf.nospawn(pos.y);
                }

                return as_moving(leaf);
            }
        }
        Spawned::None => {
            //If no powerups were selected for this block, then fire out a podobo
            let dVelY = vel(-(RANDOM_INT(gp.get(), 5) as f32 / 2.0) - 6.0);
            let podobo = Ptr::new_box(MO_Podobo::new(
                g,
                rm.spr_podobo,
                Vec2s::new((pos.x as i32 + 2) as i16, pos.y),
                dVelY,
                -1,
                -1,
                -1,
                true,
            ));
            gp.objectcontainer[2].add(g, podobo);
            return as_moving(podobo);
        }
    }

    Ptr::null()
}

fn check_secret_poof(g: &mut Game, object: Ptr<dyn IO_MovingObjectTrait>) {
    if !object.is_null() {
        g.eyecandy[2].emplace(EC_SingleAnimation::new(&g.rm, g.rm.spr_poof, (object.x() - 8) as i16, (object.y() - 8) as i16, 4, 5));
    }
}

pub fn check_secret(g: &mut Game, id: i16) {
    if !g.game_values.secretsenabled {
        return;
    }
    let mut gp = g.ptr();
    let rx = RANDOM_INT(gp.get(), App::screenWidth) as i16;
    let ry = RANDOM_INT(gp.get(), App::screenHeight) as i16;
    let randomPos = Vec2s::new(rx, ry);

    if id == 0 && !g.game_values.unlocksecretunlocked[0] {
        let mut iCountTeams: i16 = 0;
        for iPlayer in 0..MAX_PLAYERS as usize {
            if g.game_values.unlocksecret1part1[iPlayer] {
                iCountTeams += 1;
            }
        }

        if iCountTeams >= 2 && g.game_values.unlocksecret1part2 >= 8 {
            g.game_values.unlocksecretunlocked[0] = true;
            if_sound_on_play(g, &mut gp.rm.sfx_transform);

            let object = createpowerup(g, SECRET1_POWERUP as i16, randomPos, true, false);
            check_secret_poof(g, object);
        }
    } else if id == 1 && !g.game_values.unlocksecretunlocked[1] {
        if g.game_values.unlocksecret2part1 && g.game_values.unlocksecret2part2 >= 3 {
            g.game_values.unlocksecretunlocked[1] = true;
            if_sound_on_play(g, &mut gp.rm.sfx_transform);

            let object = createpowerup(g, SECRET2_POWERUP as i16, randomPos, true, false);
            check_secret_poof(g, object);
        }
    } else if id == 2 && !g.game_values.unlocksecretunlocked[2] {
        for iPlayer in 0..MAX_PLAYERS as usize {
            //number of songs on thriller + number of released albums (figure it out :))
            if g.game_values.unlocksecret3part1[iPlayer] >= 9 && g.game_values.unlocksecret3part2[iPlayer] >= 13 {
                g.game_values.unlocksecretunlocked[2] = true;
                if_sound_on_play(g, &mut gp.rm.sfx_transform);

                let object = createpowerup(g, SECRET3_POWERUP as i16, randomPos, true, false);
                check_secret_poof(g, object);
            }
        }
    } else if id == 3 && !g.game_values.unlocksecretunlocked[3] {
        g.game_values.unlocksecretunlocked[3] = true;
        if_sound_on_play(g, &mut gp.rm.sfx_transform);

        let object = createpowerup(g, SECRET4_POWERUP as i16, randomPos, true, false);
        check_secret_poof(g, object);
    }
}
