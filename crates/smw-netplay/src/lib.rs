//! The netplay protocol (src/common_netplay) and the lobby server (src/server).

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

#[path = "../../../src/common"]
pub mod common {
    pub mod file_io;
}

#[path = "../../../src/common_netplay/mod.rs"]
pub mod common_netplay;

#[path = "../../../src/server/mod.rs"]
pub mod server;
