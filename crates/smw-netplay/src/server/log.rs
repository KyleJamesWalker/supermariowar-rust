//! The lobby server's log, appended to `serverlog.txt` in the working directory once `log_init` opens it.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::raw::{c_char, c_int, c_long};
use std::sync::OnceLock;

#[repr(C)]
struct tm {
    tm_sec: c_int,
    tm_min: c_int,
    tm_hour: c_int,
    tm_mday: c_int,
    tm_mon: c_int,
    tm_year: c_int,
    tm_wday: c_int,
    tm_yday: c_int,
    tm_isdst: c_int,
    tm_gmtoff: c_long,
    tm_zone: *mut c_char,
}

extern "C" {
    #[cfg_attr(windows, link_name = "_time64")]
    fn time(t: *mut i64) -> i64;
    #[cfg(not(windows))]
    fn localtime_r(t: *const i64, result: *mut tm) -> *mut tm;
    #[cfg(windows)]
    fn _localtime64_s(result: *mut tm, t: *const i64) -> c_int;
    fn strftime(s: *mut c_char, max: usize, format: *const c_char, tm: *const tm) -> usize;
}

static mut logfile: Option<File> = None;

/// Opens `serverlog.txt` for appending; false when it cannot.
pub fn log_init() -> bool {
    unsafe {
        logfile = OpenOptions::new().append(true).create(true).open("serverlog.txt").ok();
        if logfile.is_none() {
            return false;
        }

        true
    }
}

pub fn log_close() {
    unsafe {
        logfile = None;
    }
}

fn write_log(show_output: bool, parsed_message: &str) {
    unsafe {
        let rawtime = time(std::ptr::null_mut());
        let mut sTime: tm = std::mem::zeroed();
        #[cfg(not(windows))]
        localtime_r(&rawtime, &mut sTime);
        #[cfg(windows)]
        _localtime64_s(&mut sTime, &rawtime);

        let mut timeBuffer = [0u8; 80];
        let n = strftime(timeBuffer.as_mut_ptr() as *mut c_char, timeBuffer.len(), c"%Y-%b-%d %X".as_ptr(), &sTime);
        let timeStr = String::from_utf8_lossy(&timeBuffer[..n]);

        if show_output {
            println!("{}", parsed_message);
        }

        if let Some(f) = logfile.as_mut() {
            let _ = writeln!(f, "[{}] {}", timeStr, parsed_message);
            let _ = f.flush();
        }
    }
}

/// Prints `message` and appends it, timestamped, to the log file.
pub fn log(message: &str) {
    write_log(true, message);
}

/// Appends `message`, timestamped, to the log file only.
pub fn log_silently(message: &str) {
    write_log(false, message);
}

/// Whether `SMW_LOBBY_VERBOSE` is set to anything but empty or `0`.
pub fn is_verbose() -> bool {
    static VERBOSE: OnceLock<bool> = OnceLock::new();
    *VERBOSE.get_or_init(|| std::env::var_os("SMW_LOBBY_VERBOSE").is_some_and(|v| !v.is_empty() && v != "0"))
}

/// `println!` for per-message detail, shown only when [`is_verbose`].
macro_rules! verbose {
    ($($arg:tt)*) => {
        if $crate::server::log::is_verbose() {
            println!($($arg)*);
        }
    };
}
pub(crate) use verbose;
