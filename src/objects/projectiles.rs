use bevy::prelude::*;

/// Types of projectiles players can fire.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileType {
    Fireball,
    Hammer,
    Boomerang,
    IceBall,
    Bomb,
    Shell,        // Kicked Koopa shell
    ThrowBlock,   // Picked-up block being thrown
    Feather,      // Cape attack
    TailSwipe,    // Leaf tail attack
    BulletBill,   // Player-as-bullet
}

/// Projectile behavior data.
#[derive(Component, Debug)]
pub struct Projectile {
    pub projectile_type: ProjectileType,
    pub owner: usize,
    pub bounces_remaining: u32,
    pub speed: f32,
}

impl Projectile {
    pub fn fireball(owner: usize) -> Self {
        Self {
            projectile_type: ProjectileType::Fireball,
            owner,
            bounces_remaining: 3,
            speed: 6.0,
        }
    }

    pub fn hammer(owner: usize) -> Self {
        Self {
            projectile_type: ProjectileType::Hammer,
            owner,
            bounces_remaining: 0,
            speed: 8.5,
        }
    }

    pub fn boomerang(owner: usize) -> Self {
        Self {
            projectile_type: ProjectileType::Boomerang,
            owner,
            bounces_remaining: 0,
            speed: 5.0,
        }
    }
}
