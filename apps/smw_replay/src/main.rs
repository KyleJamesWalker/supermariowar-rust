//! The game on the headless backend: the same arguments and SMW_* variables as `smw`, so `SMW_BIN` can point
//! tools/run_rust.sh and tools/parity.sh at it.

use std::path::PathBuf;

/// Where the SDL backend keeps settings: `$HOME/Library/Preferences/.smw/`, `%USERPROFILE%/.smw/` on Windows.
fn settings_dir() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("{}/.smw/", std::env::var("USERPROFILE").unwrap_or_default()))
    } else {
        PathBuf::from(format!("{}/Library/Preferences/.smw/", std::env::var("HOME").unwrap_or_default()))
    }
}

fn main() {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| format!("{}/", d.display()))).unwrap_or_else(|| "./".into());
    smw::smw::main::run_with(std::env::args().collect(), Some(smw_headless::services(settings_dir(), PathBuf::from(exe_dir))));
}
