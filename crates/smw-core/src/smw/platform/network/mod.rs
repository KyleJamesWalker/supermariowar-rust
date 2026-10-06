#[cfg(not(target_os = "emscripten"))]
pub mod enet;
pub mod null;
#[cfg(target_os = "emscripten")]
pub mod websocket;
