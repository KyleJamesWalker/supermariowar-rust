//! Gameplay objects: blocks, powerups, enemies, projectiles, carriable items, mode objects and map hazards.

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

use crate::game::Game;
use crate::common::object_base::CObjectTrait;
use crate::globals::Ptr;

