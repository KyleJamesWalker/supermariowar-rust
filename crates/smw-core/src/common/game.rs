//! Screen dimensions and setup of the settings directory.

use crate::game::Game;
use crate::common::path::get_home_directory;
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

/// Constants shared by the game and both editors.
pub struct App;

impl App {
    /// The logical screen size in pixels.
    pub const screenWidth: i32 = 640;
    pub const screenHeight: i32 = 480;
    /// Alpha (0 to 255) of the shade drawn behind menus.
    pub const menuTransparency: i32 = 72;
}

/// Creates the settings directory if it is missing, reporting failure on stderr.
pub fn ensure_settings_dir(g: &mut Game) {
    let smwHome = get_home_directory(g);

    match std::fs::metadata(&smwHome) {
        Err(_) => {
            let mut builder = std::fs::DirBuilder::new();
            #[cfg(unix)]
            builder.mode(0o775);
            if builder.create(&smwHome).is_err() {
                perror("[error] Could not create settings directory");
            }
        }
        Ok(meta) if !meta.is_dir() => {
            perror("[error] Could not access settings directory");
        }
        Ok(_) => {}
    }
}

/// Prints `msg` and the current `errno` text to stderr.
pub fn perror(msg: &str) {
    let c = CString::new(msg).unwrap();
    extern "C" {
        fn perror(s: *const std::ffi::c_char);
    }
    unsafe { perror(c.as_ptr()) };
}
