//! Not in upstream: the entry point SDLActivity calls in libmain.so, and the Android storage and log plumbing.

use std::ffi::{c_char, c_int, CStr};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::FromRawFd;
use std::sync::Mutex;

extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
}

const ANDROID_LOG_INFO: c_int = 4;

static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// A line to logcat under the "smw" tag and to `log.txt` in the storage directory.
fn log_line(line: &[u8]) {
    if let Ok(text) = std::ffi::CString::new(line) {
        unsafe { __android_log_write(ANDROID_LOG_INFO, c"smw".as_ptr(), text.as_ptr()) };
    }
    if let Some(file) = LOG_FILE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        let _ = file.write_all(line);
        let _ = file.write_all(b"\n");
    }
}

/// stdout and stderr go nowhere on Android, so forward their lines to `log_line`, which a file manager can
/// reach as Android/data/<package>/files/log.txt. A panic is logged before the process aborts.
fn redirect_output() {
    if let Ok(file) = std::fs::File::create(format!("{}/log.txt", smw_sdl2::android_storage_path())) {
        *LOG_FILE.lock().unwrap_or_else(|e| e.into_inner()) = Some(file);
    }
    std::panic::set_hook(Box::new(|info| {
        let thread = std::thread::current();
        let location = info.location().map(|l| l.to_string()).unwrap_or_default();
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        log_line(format!("thread '{}' panicked at {}:\n{}", thread.name().unwrap_or("<unnamed>"), location, message).as_bytes());
    }));
    unsafe {
        let mut fds = [0 as c_int; 2];
        if pipe(fds.as_mut_ptr()) != 0 {
            return;
        }
        dup2(fds[1], 1);
        dup2(fds[1], 2);
        let reader = BufReader::new(std::fs::File::from_raw_fd(fds[0]));
        std::thread::spawn(move || {
            for line in reader.split(b'\n').map_while(Result::ok) {
                log_line(&line);
            }
        });
    }
}

/// MainActivity reports up to two display cutouts' bounding boxes whenever the window's insets change.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn Java_com_kylejameswalker_supermariowar_MainActivity_nativeSetCutouts(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    l1: c_int,
    t1: c_int,
    r1: c_int,
    b1: c_int,
    l2: c_int,
    t2: c_int,
    r2: c_int,
    b2: c_int,
) {
    crate::smw::touch::set_cutouts(&[[l1, t1, r1, b1], [l2, t2, r2, b2]]);
}

#[no_mangle]
pub unsafe extern "C" fn SDL_main(argc: c_int, argv: *const *const c_char) -> c_int {
    redirect_output();
    let args = (0..argc.max(0) as usize).map(|i| CStr::from_ptr(*argv.add(i)).to_string_lossy().into_owned()).collect();
    crate::smw::main::run(args);
    0
}
