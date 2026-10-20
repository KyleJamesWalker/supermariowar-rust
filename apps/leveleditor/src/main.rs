//! The level editor on the SDL2 backend.

#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() {
    smw::services::set_backend(smw_sdl2::services);
    smw::leveleditor::leveleditor::main();
}
