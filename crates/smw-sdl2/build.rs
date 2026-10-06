fn main() {
    if matches!(std::env::var("CARGO_CFG_TARGET_OS").as_deref(), Ok("emscripten" | "android")) {
        return;
    }
    // Homebrew's SDL2; the search path reaches every crate that links smw-sdl2.
    for dir in ["/opt/homebrew/lib", "/usr/local/lib"] {
        if std::path::Path::new(dir).exists() {
            println!("cargo:rustc-link-search=native={dir}");
        }
    }
}
