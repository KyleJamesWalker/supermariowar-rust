//! Port of src/server/mainServer.cpp

use super::server::SMWServer;

static mut running: bool = true;

fn cleanup() {
    //server cleans up in its destructor
}

#[cfg(all(unix, not(target_os = "macos")))]
extern "C" fn interrupt(_code: i32) {
    println!("  Goodbye!");
    unsafe { running = false };
}

extern "C" {
    fn clock() -> std::os::raw::c_ulong;
    #[cfg(all(unix, not(target_os = "macos")))]
    fn usleep(usec: u32) -> i32;
    #[cfg(all(unix, not(target_os = "macos")))]
    fn signal(signum: i32, handler: extern "C" fn(i32)) -> usize;
}

pub fn main() -> i32 {
    println!("SMW Server alpha");

    let argv: Vec<String> = std::env::args().collect();
    let config_path = if argv.len() > 1 { argv[1].clone() } else { "serverconfig".to_string() };

    let mut server = SMWServer::new();
    if !server.init(&config_path) {
        cleanup();
        return 1;
    }

    //
    // Interrupt handling (`#ifdef __unix__`, which clang does not define on macOS)
    //
    #[cfg(all(unix, not(target_os = "macos")))]
    unsafe {
        signal(2 /* SIGINT */, interrupt);
    }

    println!("Ready!");

    //
    // Main loop
    //
    unsafe {
        let mut frameStart = clock();
        let mut frameEnd;
        while running {
            server.update(&mut *std::ptr::addr_of_mut!(running));

            // Do not use 100% CPU if not necessary
            frameEnd = clock();
            if frameEnd.wrapping_sub(frameStart) < 30 {
                #[cfg(all(unix, not(target_os = "macos")))]
                usleep(((frameStart + 30 - frameEnd) * 1000) as u32); // microseconds!
            }
            frameStart = clock();
        }
    }

    cleanup();
    0
}
