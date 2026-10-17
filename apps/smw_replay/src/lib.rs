//! The game and editors on the headless backend: the same arguments and SMW_* variables as `smw`, `leveleditor` and
//! `worldeditor`, so `SMW_BIN` can point tools/run_rust.sh and tools/parity.sh at `smw_replay`, and
//! `SMW_EDITOR_BIN_SUFFIX=_replay` points tools/editor_parity.sh at `leveleditor_replay` and `worldeditor_replay`.

use smw_platform::Services;
use std::path::PathBuf;

/// Where the SDL backend keeps settings: `$HOME/Library/Preferences/.smw/`, `%USERPROFILE%/.smw/` on Windows.
fn settings_dir() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("{}/.smw/", std::env::var("USERPROFILE").unwrap_or_default()))
    } else {
        PathBuf::from(format!("{}/Library/Preferences/.smw/", std::env::var("HOME").unwrap_or_default()))
    }
}

/// Headless services with the SDL backend's settings directory and the executable's directory as the root.
pub fn services() -> Option<Services> {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| format!("{}/", d.display()))).unwrap_or_else(|| "./".into());
    Some(smw_headless::services(settings_dir(), PathBuf::from(exe_dir)))
}
