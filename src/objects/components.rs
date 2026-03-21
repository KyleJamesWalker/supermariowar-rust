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

/// Marks objects that collide with map tiles and respond to solid geometry.
#[derive(Component, Debug, Default)]
pub struct InteractsWithMap;

/// Tracks which player spawned or owns this object (projectile, thrown block, etc.).
#[derive(Component, Debug)]
pub struct OwnerPlayer {
    pub index: usize,
}

/// Marks objects that should be despawned when they leave the screen boundaries.
#[derive(Component, Debug, Default)]
pub struct DespawnOffScreen;

/// Event fired when a block is hit from below by a player.
#[derive(Event, Debug)]
pub struct BlockHitEvent {
    pub block_entity: Entity,
    pub player_entity: Entity,
    pub player_index: usize,
}

/// Event fired when a powerup is collected by a player.
#[derive(Event, Debug)]
pub struct PowerupCollectEvent {
    pub powerup_entity: Entity,
    pub player_entity: Entity,
    pub player_index: usize,
}

/// Event fired when an enemy is killed.
#[derive(Event, Debug)]
pub struct EnemyDeathEvent {
    pub enemy_entity: Entity,
    pub killer_player: Option<usize>,
}

/// Event fired when a projectile hits something.
#[derive(Event, Debug)]
pub struct ProjectileHitEvent {
    pub projectile_entity: Entity,
    pub hit_entity: Entity,
    pub projectile_owner: usize,
}

/// Event fired when a hazard contacts a player.
#[derive(Event, Debug)]
pub struct HazardContactEvent {
    pub hazard_entity: Entity,
    pub player_entity: Entity,
}

/// Object is on the ground (used for non-player objects with physics).
#[derive(Component, Debug, Default)]
pub struct ObjectGrounded(pub bool);
