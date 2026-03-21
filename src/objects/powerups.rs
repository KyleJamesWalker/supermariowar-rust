use bevy::prelude::*;

/// All powerup item types available in the game.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerupType {
    Mushroom,        // Extra life / grow
    Fire,            // Fireball throwing
    Hammer,          // Hammer throwing
    Boomerang,       // Boomerang throwing
    Feather,         // Cape/flying
    Leaf,            // Tail attack
    Bomb,            // Bob-omb
    Star,            // Invincibility
    Poison,          // Shrink/kill
    OneUp,           // Extra life
    TwoUp,           // Two extra lives
    ThreeUp,         // Three extra lives
    FiveUp,          // Five extra lives
    Clock,           // Freeze/slow enemies
    BulletBill,      // Transform into bullet
    Podoboo,         // Transform into flame
    BobOmb,          // Walk as bomb
    Tanooki,         // Turn to statue
    MysteryMushroom, // Random effect
    ExtraGuy,        // Extra life for specific team
    Jail,            // Jail key (Jail mode)
    Coin,            // Coin collection
    ShieldSoft,      // Soft shield
    ShieldHard,      // Hard shield
    SlowDown,        // Slow movement
    IceWand,         // Freeze projectile
}

/// Marker for a spawned powerup entity in the world.
#[derive(Component, Debug)]
pub struct SpawnedPowerup {
    pub powerup_type: PowerupType,
    pub spawning: bool,
    pub spawn_timer: f32,
}

impl SpawnedPowerup {
    pub fn new(powerup_type: PowerupType) -> Self {
        Self {
            powerup_type,
            spawning: true,
            spawn_timer: 0.0,
        }
    }
}
