use bevy::prelude::*;

use crate::core::constants::MAX_PLAYERS;

// ---------------------------------------------------------------------------
// AI difficulty
// ---------------------------------------------------------------------------

/// AI difficulty levels with preset behavior parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AiDifficulty {
    Easy,
    Medium,
    Hard,
    VeryHard,
}

impl AiDifficulty {
    /// Decision interval in seconds (how often the AI re-evaluates its goals).
    pub fn decision_interval(self) -> f32 {
        match self {
            Self::Easy => 0.5,
            Self::Medium => 0.25,
            Self::Hard => 0.1,
            Self::VeryHard => 0.05,
        }
    }

    /// Reaction time in seconds before the AI responds to new threats.
    pub fn reaction_time(self) -> f32 {
        match self {
            Self::Easy => 0.4,
            Self::Medium => 0.2,
            Self::Hard => 0.08,
            Self::VeryHard => 0.02,
        }
    }

    /// Movement accuracy from 0.0 (random) to 1.0 (perfect targeting).
    pub fn accuracy(self) -> f32 {
        match self {
            Self::Easy => 0.4,
            Self::Medium => 0.65,
            Self::Hard => 0.85,
            Self::VeryHard => 0.97,
        }
    }

    /// Aggression from 0.0 (passive) to 1.0 (always chasing).
    pub fn aggression(self) -> f32 {
        match self {
            Self::Easy => 0.3,
            Self::Medium => 0.55,
            Self::Hard => 0.75,
            Self::VeryHard => 0.9,
        }
    }

    /// Item priority from 0.0 (ignores items) to 1.0 (always goes for items first).
    pub fn item_priority(self) -> f32 {
        match self {
            Self::Easy => 0.2,
            Self::Medium => 0.45,
            Self::Hard => 0.65,
            Self::VeryHard => 0.85,
        }
    }

    /// Probability (0.0-1.0) that any given input frame is dropped as a "mistake".
    pub fn input_drop_chance(self) -> f32 {
        match self {
            Self::Easy => 0.30,
            Self::Medium => 0.15,
            Self::Hard => 0.05,
            Self::VeryHard => 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Core AI marker
// ---------------------------------------------------------------------------

/// Marker component indicating this player entity is controlled by AI.
#[derive(Component, Debug)]
pub struct AiControlled {
    pub difficulty: AiDifficulty,
    /// Timer accumulator for periodic decision re-evaluation.
    pub decision_timer: f32,
    /// How often the AI re-evaluates goals (derived from difficulty).
    pub decision_interval: f32,
    /// Delay before responding to new threats (derived from difficulty).
    pub reaction_time: f32,
    /// 0.0-1.0, higher means more precise targeting.
    pub accuracy: f32,
    /// 0.0-1.0, higher means more aggressive pursuit.
    pub aggression: f32,
    /// 0.0-1.0, higher means prioritizing items over combat.
    pub item_priority: f32,
    /// Reaction cooldown accumulator; while > 0 the AI ignores new threats.
    pub reaction_cooldown: f32,
}

impl AiControlled {
    pub fn new(difficulty: AiDifficulty) -> Self {
        Self {
            difficulty,
            decision_timer: 0.0,
            decision_interval: difficulty.decision_interval(),
            reaction_time: difficulty.reaction_time(),
            accuracy: difficulty.accuracy(),
            aggression: difficulty.aggression(),
            item_priority: difficulty.item_priority(),
            reaction_cooldown: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// AI goal
// ---------------------------------------------------------------------------

/// High-level goal the AI is currently pursuing.
#[derive(Debug, Clone)]
pub enum AiGoal {
    /// Pursue a target player.
    Chase(Entity),
    /// Run away from a threat.
    Flee(Entity),
    /// Move to pick up a powerup/coin.
    CollectItem(Entity),
    /// Move to an objective point (zone center, hill, etc.).
    CaptureObjective(Vec2),
    /// Wander back and forth with no specific target.
    Patrol,
    /// Dodge a hazard at the given position.
    AvoidHazard(Vec2),
    /// Bring a captured flag back to the team's base (CTF).
    ReturnFlag(Vec2),
    /// Stay near a zone to defend it (Domination / KotH).
    DefendZone(Vec2),
}

impl Default for AiGoal {
    fn default() -> Self {
        Self::Patrol
    }
}

// ---------------------------------------------------------------------------
// AI intent (desired outputs)
// ---------------------------------------------------------------------------

/// What the AI currently intends to do this frame. Converted to `PlayerInput`
/// by the apply-intent system.
#[derive(Component, Debug)]
pub struct AiIntent {
    /// Desired movement direction: -1.0 = full left, 0.0 = stop, 1.0 = full right.
    pub move_direction: f32,
    pub want_jump: bool,
    pub want_turbo: bool,
    pub want_down: bool,
    pub want_use_item: bool,
    pub current_goal: AiGoal,
}

impl Default for AiIntent {
    fn default() -> Self {
        Self {
            move_direction: 0.0,
            want_jump: false,
            want_turbo: false,
            want_down: false,
            want_use_item: false,
            current_goal: AiGoal::Patrol,
        }
    }
}

// ---------------------------------------------------------------------------
// AI perception
// ---------------------------------------------------------------------------

/// Snapshot of what the AI can "see" around it, rebuilt every perception tick.
#[derive(Component, Debug)]
pub struct AiPerception {
    /// (entity, world position, distance, is_same_team) for each visible player.
    pub visible_players: Vec<(Entity, Vec2, f32, bool)>,
    /// (entity, world position, distance) for each visible collectible item.
    pub visible_items: Vec<(Entity, Vec2, f32)>,
    /// (position, distance) for each nearby hazard.
    pub nearby_hazards: Vec<(Vec2, f32)>,
    /// True when standing near the edge of a platform.
    pub on_edge: bool,
    /// True when there is a gap (non-solid) directly below.
    pub above_pit: bool,
    /// Distance (in pixels) to the nearest solid tile below.
    pub nearest_solid_below: f32,
    /// True when the tile immediately to the left is solid/blocked.
    pub path_blocked_left: bool,
    /// True when the tile immediately to the right is solid/blocked.
    pub path_blocked_right: bool,
    /// Mode-specific objective location (zone center, flag base, hill, etc.).
    pub objective_position: Option<Vec2>,
}

impl Default for AiPerception {
    fn default() -> Self {
        Self {
            visible_players: Vec::new(),
            visible_items: Vec::new(),
            nearby_hazards: Vec::new(),
            on_edge: false,
            above_pit: false,
            nearest_solid_below: 0.0,
            path_blocked_left: false,
            path_blocked_right: false,
            objective_position: None,
        }
    }
}

// ---------------------------------------------------------------------------
// AI memory
// ---------------------------------------------------------------------------

/// Short-term memory allowing the AI to track past events and detect stuck states.
#[derive(Component, Debug)]
pub struct AiMemory {
    /// Player index of whoever hit us last (for revenge targeting).
    pub last_hit_by: Option<usize>,
    /// Last known positions of up to 4 players: (position, age in seconds).
    pub last_known_positions: [Option<(Vec2, f32)>; MAX_PLAYERS],
    /// Time (seconds) spent without making meaningful progress.
    pub stuck_timer: f32,
    /// Position snapshot for stuck detection.
    pub last_position: Vec2,
    /// Direction used for patrol wandering.
    pub patrol_direction: f32,
    /// Cooldown preventing the AI from spamming jump to get unstuck.
    pub jump_cooldown: f32,
    /// Cooldown preventing the AI from spamming use_item.
    pub item_use_cooldown: f32,
}

impl Default for AiMemory {
    fn default() -> Self {
        Self {
            last_hit_by: None,
            last_known_positions: [None; MAX_PLAYERS],
            stuck_timer: 0.0,
            last_position: Vec2::ZERO,
            patrol_direction: 1.0,
            jump_cooldown: 0.0,
            item_use_cooldown: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// AI pathfinding hints
// ---------------------------------------------------------------------------

/// Lightweight pathfinding state guiding the AI toward its current goal.
#[derive(Component, Debug, Default)]
pub struct AiPathfinding {
    /// World position of the navigation target.
    pub target_position: Option<Vec2>,
    /// A jump is needed to reach the target (e.g. target is above or obstacle ahead).
    pub jump_needed: bool,
    /// The AI should press down to drop through a pass-through platform.
    pub should_drop_through: bool,
}
