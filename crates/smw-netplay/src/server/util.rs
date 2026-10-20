//! Fixed-size C string helpers for protocol fields.

pub use smw::common_netplay::protocol_packages::cstr_field;

/// Copies `source` up to its first NUL into `destination[..num]`, NUL-padded, and NUL-terminates at `num - 1`.
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
