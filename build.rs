fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten") {
        emscripten_link_args();
        return;
    }

    for dir in ["/opt/homebrew/lib", "/usr/local/lib"] {
        if std::path::Path::new(dir).exists() {
            println!("cargo:rustc-link-search=native={dir}");
        }
    }
}

/// The settings of upstream's cmake/PlatformEmscripten.cmake, applied to the game binary.
fn emscripten_link_args() {
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    for arg in [
        "-sUSE_SDL=2",
        "-sUSE_SDL_IMAGE=2",
        r#"-sSDL2_IMAGE_FORMATS=["png","bmp"]"#,
        "-sUSE_SDL_MIXER=2",
        "-sUSE_ZLIB=1",
        "-sALLOW_MEMORY_GROWTH=1",
        // Not in upstream: the port builds large globals (CResourceManager, CGameValues) on the stack
        // before boxing them, which overflows emscripten's 64 KiB default. 8 MiB matches the native main thread.
        "-sSTACK_SIZE=8MB",
        "--preload-file",
        &format!("{}@data", data.display()),
    ] {
        println!("cargo:rustc-link-arg-bin=smw={arg}");
    }
}
