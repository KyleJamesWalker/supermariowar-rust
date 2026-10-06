//! The netplay protocol (`common_netplay`, C++ src/common_netplay) and the lobby server (`server`, C++ src/server).

#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    static_mut_refs,
    dead_code,
    unused_variables,
    unused_mut,
    unused_imports,
    unused_assignments,
    unused_parens,
    clippy::all
)]

extern crate self as smw;

pub use smw_globals as globals;

pub mod common {
    /// The part of smw-core's `common::file_io` that the protocol uses.
    pub mod file_io {
        /// C strings in data files are Latin-1/ASCII bytes; map each byte to one char so lengths stay in bytes for ASCII.
        pub fn cstr_bytes_to_string(bytes: &[u8]) -> String {
            match std::str::from_utf8(bytes) {
                Ok(s) => s.to_string(),
                Err(_) => bytes.iter().map(|&b| b as char).collect(),
            }
        }
    }
}

pub mod common_netplay;

pub mod server;
