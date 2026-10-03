//! Port of src/smw/player_components/PlayerKuriboShoe.cpp
// PLACEHOLDER(foundation): only what common/ needs; the player porter completes this file.

pub type KuriboShoeType = i32;
pub const NONE: KuriboShoeType = 0;
pub const NORMAL: KuriboShoeType = 1;
pub const STICKY: KuriboShoeType = 2;

#[derive(Default)]
pub struct PlayerKuriboShoe {
    pub r#type: KuriboShoeType,
    pub animationTimer: u8,
    pub animationFrame: u8,
    pub exitTimer: u8,
    pub exitIndex: u8,
}

impl PlayerKuriboShoe {
    pub fn is_on(&self) -> bool {
        self.r#type > 0
    }
}
