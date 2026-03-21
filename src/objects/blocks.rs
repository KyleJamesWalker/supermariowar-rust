use bevy::prelude::*;

/// Types of interactive blocks in the game.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Breakable,
    PowerupBlock,
    DonutBlock,      // Falls after being stood on
    FlipBlock,       // Flips when hit from below
    BounceBlock,     // Bounces entities upward
    NoteBlock,       // Stronger bounce
    OnOffSwitch,     // Toggles on/off blocks
    OnOffBlock,      // Solid/passable depending on switch state
    WeaponBreakable, // Only broken by projectiles
    ViewBlock,       // Blocks line-of-sight (for specific modes)
    ThrowBlock,      // Can be picked up and thrown
}

/// State of a block that can be hit/activated.
#[derive(Component, Debug)]
pub struct BlockState {
    pub hit: bool,
    pub hit_timer: f32,
    pub empty: bool,
    pub item_count: u32,
}

impl Default for BlockState {
    fn default() -> Self {
        Self {
            hit: false,
            hit_timer: 0.0,
            empty: false,
            item_count: 1,
        }
    }
}

/// Donut block that crumbles after a player stands on it.
#[derive(Component, Debug)]
pub struct DonutBlockState {
    pub weight_timer: f32,
    pub crumbling: bool,
    pub crumble_threshold: f32,
}

impl Default for DonutBlockState {
    fn default() -> Self {
        Self {
            weight_timer: 0.0,
            crumbling: false,
            crumble_threshold: 50.0,
        }
    }
}

/// On/Off switch state shared across all on/off blocks.
#[derive(Resource, Debug, Default)]
pub struct OnOffSwitchState {
    pub active: bool,
}
