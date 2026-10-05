# Super Mario War on Android

`SuperMarioWar-<version>.apk` is a debug-signed APK for Android 5.0 (API 21) and newer on arm64-v8a, armeabi-v7a and x86_64. `.github/workflows/build_android.yml` builds it, plays replays on an x86_64 emulator, and uploads it as the `smw-android-apk` artifact.

This version needs a gamepad or a keyboard. It has no on-screen controls; see [Touch controls](#touch-controls).

## Install

Allow installs from unknown sources, then open the APK on the device, or run `adb install SuperMarioWar-<version>.apk`. The APK is signed with a throwaway debug key, so installing a build from another machine over an existing one fails with a signature mismatch: uninstall first, which deletes settings and recordings.

## Layout

Upstream's Android port ([mmatyas/supermariowar-android](https://github.com/mmatyas/supermariowar-android), built by upstream's disabled `build_android.yml`) is SDL's `android-project` template with a `MainActivity` that loads SDL2, SDL2_image, SDL2_mixer and `libmain.so`. This build follows it:

| Piece | Here |
|---|---|
| Java and Gradle | SDL 2.32.10's `android-project` template, with `app/` from this directory on top (`build.sh`) |
| Game | the `smw` library as `libmain.so`, exporting `SDL_main` (`src/android.rs`), built with cargo-ndk |
| SDL2, SDL2_image, SDL2_mixer | built from checksummed release tarballs with CMake and the NDK (`build-sdl-libs.sh`), PNG and OGG through stb as on muOS |
| ENet, zlib | ENet built by `enet-sys`; the system `libz.so` |
| Icon | `resources/smw.png` (32x32) |

## Data and settings

The game lists directories and reads files under `data/` with plain filesystem calls, which cannot see inside an APK. Upstream reads `data/` from `$EXTERNAL_STORAGE/supermariowar/`, which the user copies there by hand. Here `data/` ships as APK assets and `MainActivity` copies it to the app's external files directory, `/sdcard/Android/data/net.smwstuff.supermariowar/files/data/`, on the first launch after each install or update (about a second). Files added to that folder stay.

Settings (`options.bin`, `controls.sdl2.bin`) and recordings (`replays/`) live in the same files directory, upstream's settings directory on Android, falling back to internal storage when there is no external storage. Uninstalling deletes them. Game output (stdout and stderr) goes to logcat under the `smw` tag: `adb logcat -s smw SDL`.

## Controls

SDL reads USB and Bluetooth gamepads and keyboards. The manifest sets `SMW_PAD_TRANSLATE=1`, as muOS does, so every pad with an SDL controller mapping uses the layout in [`device/muos/README.md`](../device/muos/README.md#controls) (A jump, B run, X item, Start pause) and joins as the next human player. Keyboards use the desktop keys. The Android Back button does nothing in the game; leave through **Exit** on the main menu.

The manifest also turns off SDL's accelerometer joystick, which would otherwise take player 1 on phones, and locks the screen to landscape.

## Touch controls

Upstream's Android port has no on-screen controls, and this build adds none. A follow-up could reuse the web build's touch overlay design (`web/touch.js`, which presses player 1's keyboard keys) as a native layer drawn by the game or as an Android view over `SDLSurface` that injects key events.

## Build

Needs the Android SDK (`ANDROID_HOME`) with NDK 27.2.12479018 (`ANDROID_NDK_HOME`), JDK 17, CMake, rsync, `cargo install cargo-ndk`, and the Rust targets:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
android/build.sh arm64-v8a armeabi-v7a x86_64   # default arm64-v8a only
```

The APK lands in `dist/SuperMarioWar-<Cargo version>.apk`, with `versionName` the Cargo version and `versionCode` `major*10000 + minor*100 + patch`. The SDL builds are kept in `target/android/sdl/<abi>/`; delete that to rebuild them.

## Replay check

`android/smoke.sh <apk> [replay.txt ...]` installs the APK on the connected device or emulator and, for each replay, clears the app's data, starts it with the replay harness variables as intent extras, waits for it to quit, and diffs the dump and screenshots against `tools/golden/` like `tools/parity.sh`. `MainActivity` turns extras named `SMW_*` into environment variables and an `args` string array into the command line, in debuggable builds only:

```sh
adb shell am start -n net.smwstuff.supermariowar/.MainActivity --es SMW_REPLAY /data/data/net.smwstuff.supermariowar/files/r.txt --es SMW_NOLIMIT 1
```
