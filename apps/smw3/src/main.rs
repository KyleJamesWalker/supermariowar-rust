//! The game on the SDL3 backend.

#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    smw::services::set_backend(smw_sdl3::services);
    smw::smw::main::main();
}
