use bevy::prelude::*;

/// Marker indicating this player is controlled by AI.
#[derive(Component, Debug)]
pub struct AiControlled {
    pub difficulty: AiDifficulty,
    pub decision_timer: f32,
    pub decision_interval: f32,
}

impl AiControlled {
    pub fn new(difficulty: AiDifficulty) -> Self {
        let interval = match difficulty {
            AiDifficulty::Easy => 0.5,
            AiDifficulty::Medium => 0.25,
            AiDifficulty::Hard => 0.1,
            AiDifficulty::VeryHard => 0.05,
        };
        Self {
            difficulty,
            decision_timer: 0.0,
            decision_interval: interval,
        }
    }
}

/// AI difficulty levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,
    Medium,
    Hard,
    VeryHard,
}

/// What the AI currently intends to do.
#[derive(Component, Debug, Default)]
pub struct AiIntent {
    pub move_left: bool,
    pub move_right: bool,
    pub jump: bool,
    pub use_item: bool,
    pub target_entity: Option<Entity>,
    pub target_position: Option<Vec2>,
}

/// AI awareness of nearby threats and opportunities.
#[derive(Component, Debug, Default)]
pub struct AiPerception {
    pub nearest_player: Option<Entity>,
    pub nearest_player_distance: f32,
    pub nearest_powerup: Option<Entity>,
    pub nearest_hazard: Option<Entity>,
    pub on_edge: bool,
    pub above_pit: bool,
}
