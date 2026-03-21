use bevy::prelude::*;

/// Marks an entity as affected by gravity.
#[derive(Component, Debug)]
pub struct GravityAffected {
    pub multiplier: f32,
}

impl Default for GravityAffected {
    fn default() -> Self {
        Self { multiplier: 1.0 }
    }
}

/// Axis-aligned bounding box collider, relative to entity transform.
#[derive(Component, Debug, Clone)]
pub struct Collider {
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl Collider {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    pub fn with_offset(mut self, x: f32, y: f32) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }
}

/// The collision layer/group for filtering which entities interact.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionLayer {
    Player,
    Block,
    Powerup,
    Projectile,
    Enemy,
    Hazard,
    Platform,
}

/// Emitted when two entities overlap.
#[derive(Event, Debug)]
pub struct CollisionEvent {
    pub entity_a: Entity,
    pub entity_b: Entity,
    pub layer_a: CollisionLayer,
    pub layer_b: CollisionLayer,
}

/// Marks an entity as a solid that others cannot pass through.
#[derive(Component, Debug, Default)]
pub struct Solid;

/// Marks an entity as a one-way platform (passable from below).
#[derive(Component, Debug, Default)]
pub struct OneWayPlatform;
