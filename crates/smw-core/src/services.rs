//! The platform services the game reaches through one context (docs/design/ARCHITECTURE_V2.md, "Threading the
//! context through the code"). Each app picks the backend that provides them.

use crate::game::Game;
use smw_platform::Services;
use std::sync::OnceLock;

static BACKEND: OnceLock<fn() -> Services> = OnceLock::new();

#[cfg(test)]
const DEFAULT_BACKEND: Option<fn() -> Services> = Some(smw_sdl2::services);
#[cfg(not(test))]
const DEFAULT_BACKEND: Option<fn() -> Services> = None;

/// The backend whose services the game uses; apps call it before the game starts, and the first call wins.
pub fn set_backend(backend: fn() -> Services) {
    let _ = BACKEND.set(backend);
}

/// The platform services, created from the backend on first use; panics if no backend is set outside tests.
pub fn services(g: &mut Game) -> &mut Services {
    g.services.get_or_insert_with(|| BACKEND.get().copied().or(DEFAULT_BACKEND).expect("no platform backend: call smw::services::set_backend")())
}

/// Wall-clock milliseconds from the backend's clock, truncated to `u32`.
pub fn ticks(g: &mut Game) -> u32 {
    services(g).clock.now_ms() as u32
}

/// Sleeps for `ms` milliseconds.
pub fn delay(g: &mut Game, ms: u32) {
    services(g).clock.sleep_ms(ms)
}
