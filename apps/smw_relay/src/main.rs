//! `smw_relay`: see the library (src/lib.rs) and docs/guides/RELAY.md.

use smw_relay::Config;

fn main() {
    let config = match Config::from_args_and_env() {
        Ok(config) => config,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    if config.healthcheck {
        std::process::exit(if smw_relay::healthcheck(config.port) { 0 } else { 1 });
    }
    exit_on_signal();

    println!("SMW relay");
    if let Err(message) = smw_relay::run(&config) {
        eprintln!("[error] {message}");
        std::process::exit(1);
    }
}

/// PID 1 in a container ignores SIGTERM unless it installs a handler.
#[cfg(unix)]
fn exit_on_signal() {
    extern "C" {
        fn signal(signum: i32, handler: extern "C" fn(i32)) -> usize;
        fn _exit(status: i32) -> !;
    }
    extern "C" fn on_signal(_: i32) {
        unsafe { _exit(0) }
    }
    unsafe {
        signal(2 /* SIGINT */, on_signal);
        signal(15 /* SIGTERM */, on_signal);
    }
}

#[cfg(not(unix))]
fn exit_on_signal() {}
