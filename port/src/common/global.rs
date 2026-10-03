//! Port of src/common/global.cpp

pub static mut RootDataDirectory: String = String::new();

pub static mut joystickcount: i16 = 0;

pub fn init_globals() {
    unsafe {
        RootDataDirectory = crate::common::path::get_root_directory() + "data";
    }
    crate::common::gfx::init_globals();
}
