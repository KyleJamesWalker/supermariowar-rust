fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        android_link_args();
    }
}

/// SDL's Android loader looks for SDL_main in libmain.so, linked against the SDL libraries android/build.sh builds.
fn android_link_args() {
    println!("cargo:rerun-if-env-changed=SMW_SDL_LIB_DIR");
    let dir = std::env::var("SMW_SDL_LIB_DIR").expect("SMW_SDL_LIB_DIR must point at the Android SDL libraries");
    println!("cargo:rustc-link-search=native={dir}");
    println!("cargo:rustc-link-lib=dylib=SDL2_image");
    println!("cargo:rustc-link-lib=dylib=SDL2_mixer");
    println!("cargo:rustc-cdylib-link-arg=-Wl,--no-undefined");
    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,libmain.so");
}
