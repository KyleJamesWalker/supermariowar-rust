use bevy::prelude::*;

/// Types of walking/moving enemies.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyType {
    Goomba,
    Koopa,         // Green, walks off edges
    KoopaRed,      // Red, turns at edges
    Spiny,         // Cannot be stomped
    BuzzyBeetle,   // Shell-based
    CheepCheep,    // Flying fish
    Paragoomba,    // Winged goomba
    Parakoopa,     // Winged koopa
    BulletBill,    // Fires from cannons
    Podoboo,       // Flame that jumps from lava
}

/// Enemy state machine.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EnemyState {
    #[default]
    Walking,
    Stunned,
    InShell,        // Koopa in shell (can be kicked)
    ShellMoving,    // Kicked shell
    Dead,
    Falling,
}

/// Enemy behavior and stats.
#[derive(Component, Debug)]
pub struct Enemy {
    pub enemy_type: EnemyType,
    pub state: EnemyState,
    pub stompable: bool,
    pub kills_on_touch: bool,
    pub speed: f32,
}

impl Enemy {
    pub fn goomba() -> Self {
        Self {
            enemy_type: EnemyType::Goomba,
            state: EnemyState::Walking,
            stompable: true,
            kills_on_touch: true,
            speed: 1.5,
        }
    }

    pub fn koopa() -> Self {
        Self {
            enemy_type: EnemyType::Koopa,
            state: EnemyState::Walking,
            stompable: true,
            kills_on_touch: true,
            speed: 1.5,
        }
    }

    pub fn spiny() -> Self {
        Self {
            enemy_type: EnemyType::Spiny,
            state: EnemyState::Walking,
            stompable: false,
            kills_on_touch: true,
            speed: 1.2,
        }
    }
}
