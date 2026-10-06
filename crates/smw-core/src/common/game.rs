//! Port of src/common/Game.cpp

use crate::common::path::get_home_directory;
use std::ffi::CString;
#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

pub struct App;

impl App {
    pub const screenWidth: i32 = 640;
    pub const screenHeight: i32 = 480;
    pub const menuTransparency: i32 = 72;
}

pub fn ensure_settings_dir() {
    let smwHome = get_home_directory();

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

pub fn perror(msg: &str) {
    let c = CString::new(msg).unwrap();
    extern "C" {
        fn perror(s: *const std::ffi::c_char);
    }
    unsafe { perror(c.as_ptr()) };
}
