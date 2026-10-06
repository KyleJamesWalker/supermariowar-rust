//! Not in upstream: the platform services the game reaches through one context (docs/ARCHITECTURE_V2.md,
//! Threading the context, step 1). The SDL2 backend provides them until the apps choose a backend.

use smw_platform::Services;

static mut SERVICES: Option<Services> = None;

pub fn services() -> &'static mut Services {
    unsafe { SERVICES.get_or_insert_with(smw_sdl2::services) }
}

/// `SDL_GetTicks()`.
pub fn ticks() -> u32 {
    services().clock.now_ms() as u32
}

/// `SDL_Delay(ms)`.
pub fn delay(ms: u32) {
    services().clock.sleep_ms(ms)
}
