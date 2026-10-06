//! Faithful Rust port of Super Mario War (see docs/ARCHITECTURE.md).
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

#[macro_use]
extern crate smw_globals;
pub use smw_globals::{enum_from_u8, impl_base};

pub mod globals;

pub mod common;
pub use smw_netplay::{common_netplay, net_package};
pub mod services;
pub mod smw;
pub mod leveleditor;
pub mod worldeditor;

#[cfg(target_os = "android")]
mod android;

#[cfg(test)]
mod alias_audit;
