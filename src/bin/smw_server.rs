//! `smw-server` lobby server (src/server, built with `IS_SERVER=1`).

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, static_mut_refs, dead_code)]

#[path = "../server/mod.rs"]
mod server;

fn main() {
    let status = server::main_server::main();
    std::process::exit(status);
}
