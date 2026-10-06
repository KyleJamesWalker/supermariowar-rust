//! Port of src/common/CmdArgs.cpp

#[derive(Clone, Debug, Default)]
pub struct Args {
    pub success: bool,
    pub show_help: bool,
    pub debug: bool,
    pub data_root: String,
    /// Not in the C++: `--replay <file>` watches a session recording (see docs/REPLAY.md, "Recordings").
    pub replay: String,
    pub replay_speed: Option<f32>,
    /// `--segment <k>` with `--replay`: watch only match k, started from its checkpoint.
    pub segment: Option<u32>,
}

/// `--debug`: a console of its own when the game was not started from one.
pub fn show_windows_console() {
    #[cfg(windows)]
    win_console::attach(true);
}

/// The GUI-subsystem executables have no console: write to the parent's when started from one,
/// leaving inherited (redirected) handles alone.
pub fn attach_parent_console() {
    #[cfg(windows)]
    win_console::attach(false);
}

#[cfg(windows)]
mod win_console {
    use std::ffi::c_void;

    type Handle = *mut c_void;
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    const STD_INPUT_HANDLE: u32 = -10i32 as u32;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const GENERIC_READ: u32 = 0x8000_0000;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const FILE_SHARE_READ: u32 = 1;
    const FILE_SHARE_WRITE: u32 = 2;
    const OPEN_EXISTING: u32 = 3;
    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;

    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
        fn AllocConsole() -> i32;
        fn GetConsoleWindow() -> Handle;
        fn GetStdHandle(std_handle: u32) -> Handle;
        fn SetStdHandle(std_handle: u32, handle: Handle) -> i32;
        fn CreateFileW(name: *const u16, access: u32, share: u32, security: *mut c_void, disposition: u32, flags: u32, template: Handle) -> Handle;
    }

    fn missing(id: u32) -> bool {
        let h = unsafe { GetStdHandle(id) };
        h.is_null() || h == INVALID_HANDLE_VALUE
    }

    fn open(name: &str) -> Handle {
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        unsafe { CreateFileW(wide.as_ptr(), GENERIC_READ | GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE, std::ptr::null_mut(), OPEN_EXISTING, 0, std::ptr::null_mut()) }
    }

    pub fn attach(alloc: bool) {
        let ids = [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE];
        if !alloc && !ids.iter().any(|&id| missing(id)) {
            return;
        }
        unsafe {
            if GetConsoleWindow().is_null() && AttachConsole(ATTACH_PARENT_PROCESS) == 0 && !(alloc && AllocConsole() != 0) {
                return;
            }
        }
        for id in ids {
            if missing(id) {
                let h = open(if id == STD_INPUT_HANDLE { "CONIN$" } else { "CONOUT$" });
                if h != INVALID_HANDLE_VALUE {
                    unsafe { SetStdHandle(id, h) };
                }
            }
        }
    }
}

pub fn print_help(title: &str, version: &str) {
    print!("{} {}\n\n", title, version);
    print!(
        "Super Mario War is a Super Mario multiplayer game. The goal is to stomp\n\
         as many other Marios as possible to win the game. It's a tribute to Nintendo\n\
         and the game Mario War by Samuele Poletto.\n\n"
    );
    println!("Options:");
    println!("  -h, --help              Prints this help");
    println!("      --datadir <DIR>     Sets the data directory to DIR (default: ./data)");
    println!("      --debug             Shows the debug console on Windows");
    println!("      --replay <FILE>     Watches a session recording");
    println!("      --replay-speed <N>  Playback speed multiplier for --replay");
    println!("      --segment <K>       With --replay: watch only match K of the recording");
}

pub fn parse_args(argv: &[String]) -> Args {
    let mut result = Args::default();
    let argc = argv.len();
    let mut i = 1;
    while i < argc {
        let arg = argv[i].as_str();
        if arg == "-h" || arg == "--help" {
            result.success = true;
            result.show_help = true;
            return result;
        }
        if arg == "--debug" {
            result.debug = true;
            i += 1;
            continue;
        }
        if arg == "--replay" || arg == "--replay-speed" || arg == "--segment" {
            i += 1;
            if i >= argc {
                eprintln!("Error: `{}` requires a parameter, see `--help`", arg);
                return result;
            }
            if arg == "--replay" {
                result.replay = argv[i].clone();
            } else if arg == "--segment" {
                match argv[i].parse::<u32>() {
                    Ok(v) if v > 0 => result.segment = Some(v),
                    _ => {
                        eprintln!("Error: `--segment` needs a match number (1, 2, ...)");
                        return result;
                    }
                }
            } else {
                match argv[i].parse::<f32>() {
                    Ok(v) if v > 0.0 => result.replay_speed = Some(v),
                    _ => {
                        eprintln!("Error: `--replay-speed` needs a positive number");
                        return result;
                    }
                }
            }
            i += 1;
            continue;
        }
        if arg == "--datadir" {
            i += 1;
            if i >= argc {
                eprintln!("Error: `--datadir` requires a parameter, see `--help`");
                return result;
            }
            result.data_root = argv[i].clone();
            i += 1;
            continue;
        }
        i += 1;
    }
    result.success = true;
    result
}
