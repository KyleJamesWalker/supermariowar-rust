use bevy::prelude::*;

/// Base marker for any game object (blocks, powerups, projectiles, enemies, hazards).
#[derive(Component, Debug)]
pub struct GameObject {
    pub object_type: ObjectType,
    pub alive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Block,
    Powerup,
    Projectile,
    Enemy,
    Hazard,
}

/// Lifetime component -- object despawns when timer finishes.
#[derive(Component, Debug)]
pub struct Lifetime {
    pub timer: Timer,
}

impl Lifetime {
    pub fn from_seconds(seconds: f32) -> Self {
        Self {
            timer: Timer::from_seconds(seconds, TimerMode::Once),
        }
    }
}

/// Moving object with its own velocity (separate from player velocity).
#[derive(Component, Debug, Default)]
pub struct ObjectVelocity {
    pub x: f32,
    pub y: f32,
}

/// Marks an object that can be picked up by a player.
#[derive(Component, Debug)]
pub struct Collectible;

/// Marks an object that deals damage on contact.
#[derive(Component, Debug)]
pub struct DamageOnContact {
    pub kill: bool,
    pub owner_player: Option<usize>,
}
