//! Not in upstream: the platform services the game reaches through one context (docs/ARCHITECTURE_V2.md,
//! Threading the context, step 1). The SDL2 backend provides them until the apps choose a backend.

use crate::game::Game;
use smw_platform::Services;

pub fn services(g: &mut Game) -> &mut Services {
    g.services.get_or_insert_with(smw_sdl2::services)
}

/// `SDL_GetTicks()`.
pub fn ticks(g: &mut Game) -> u32 {
    services(g).clock.now_ms() as u32
}

/// `SDL_Delay(ms)`.
pub fn delay(g: &mut Game, ms: u32) {
    services(g).clock.sleep_ms(ms)
}
