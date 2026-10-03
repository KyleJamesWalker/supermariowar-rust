//! Port of src/server/Util.h

pub use smw::common_netplay::protocol_packages::cstr_field;

/// `strncpy_sec(destination, source, num)`: `strncpy` then terminate at `num - 1`.
pub fn strncpy_sec(destination: &mut [u8], source: &[u8], num: usize) {
    let mut i = 0;
    while i < num && i < source.len() && source[i] != 0 {
        destination[i] = source[i];
        i += 1;
    }
    while i < num {
        destination[i] = 0;
        i += 1;
    }
    destination[num - 1] = 0;
}
