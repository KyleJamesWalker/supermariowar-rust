fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten") {
        emscripten_link_args();
    }
}

/// The settings of upstream's cmake/PlatformEmscripten.cmake, applied to the game binary.
fn emscripten_link_args() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = repo.join("data");
    let fullscreen = repo.join("web/fullscreen.js");
    let relay_socket = repo.join("web/relay_socket.js");
    println!("cargo:rerun-if-changed={}", fullscreen.display());
    println!("cargo:rerun-if-changed={}", relay_socket.display());
    println!("cargo:rerun-if-env-changed=SMW_RELAY_URL");
    for arg in [
        "-sUSE_SDL=2",
        "-sUSE_SDL_IMAGE=2",
        r#"-sSDL2_IMAGE_FORMATS=["png","bmp"]"#,
        "-sUSE_SDL_MIXER=2",
        "-sALLOW_MEMORY_GROWTH=1",
        // Not in upstream: the port builds large globals (CResourceManager, CGameValues) on the stack
        // before boxing them, which overflows emscripten's 64 KiB default. 8 MiB matches the native main thread.
        "-sSTACK_SIZE=8MB",
        // Not in upstream: ENV and FS let tools/web_replay.mjs set SMW_* variables and read the harness
        // output; web/shell.html starts the game with callMain and keeps settings and recordings in IDBFS.
        "-sEXPORTED_RUNTIME_METHODS=ENV,FS,callMain,addRunDependency,removeRunDependency",
        "-lidbfs.js",
        // Not in upstream: keeps the canvas size when SDL itself enters or leaves fullscreen (see the file).
        "--pre-js",
        &fullscreen.display().to_string(),
        // Not in upstream: the WebSocket to smw_relay for netplay (src/smw/platform/network/websocket).
        "--js-library",
        &relay_socket.display().to_string(),
        "--preload-file",
        &format!("{}@data", data.display()),
        "--exclude-file",
        "*.git",
    ] {
        println!("cargo:rustc-link-arg-bin=smw={arg}");
    }
}
