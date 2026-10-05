//! Not in upstream: the entry point SDLActivity calls in libmain.so, and the Android storage and log plumbing.

use std::ffi::{c_char, c_int, CStr};
use std::io::{BufRead, BufReader};
use std::os::fd::FromRawFd;

extern "C" {
    fn SDL_AndroidGetExternalStoragePath() -> *const c_char;
    fn SDL_AndroidGetInternalStoragePath() -> *const c_char;
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
}

const ANDROID_LOG_INFO: c_int = 4;

/// The app's external files directory, as upstream's `$EXTERNAL_STORAGE/supermariowar`; internal storage when it
/// is unavailable. MainActivity extracts the APK's data/ there.
pub fn storage_path() -> String {
    unsafe {
        let mut path = SDL_AndroidGetExternalStoragePath();
        if path.is_null() {
            path = SDL_AndroidGetInternalStoragePath();
        }
        if path.is_null() {
            return ".".to_string();
        }
        CStr::from_ptr(path).to_string_lossy().into_owned()
    }
}

/// stdout and stderr go nowhere on Android, so forward their lines to logcat under the "smw" tag.
fn redirect_output_to_logcat() {
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
                let Ok(text) = std::ffi::CString::new(line) else { continue };
                __android_log_write(ANDROID_LOG_INFO, c"smw".as_ptr(), text.as_ptr());
            }
        });
    }
}

#[no_mangle]
pub unsafe extern "C" fn SDL_main(argc: c_int, argv: *const *const c_char) -> c_int {
    redirect_output_to_logcat();
    let args = (0..argc.max(0) as usize).map(|i| CStr::from_ptr(*argv.add(i)).to_string_lossy().into_owned()).collect();
    crate::smw::main::run(args);
    0
}
