# Super Mario War on Android

`SuperMarioWar-<version>.apk` is a debug-signed APK for Android 5.0 (API 21) and newer on arm64-v8a, armeabi-v7a and x86_64. `.github/workflows/build_android.yml` builds it, plays replays on an x86_64 emulator, and uploads it as the `smw-android-apk` artifact.

It plays with on-screen touch controls, a gamepad or a keyboard; see [Controls](#controls) and [Touch controls](#touch-controls).

## Install

Allow installs from unknown sources, then open the APK on the device, or run `adb install SuperMarioWar-<version>.apk`.

CI artifacts and releases are signed with the project's debug key, which the workflow decodes from the repository secrets `ANDROID_DEBUG_KEYSTORE_B64` and `ANDROID_DEBUG_KEYSTORE_PASSWORD` (key alias `smw-debug`), so each one installs over the last and keeps settings and recordings. A local build uses that key when `SMW_DEBUG_KEYSTORE` (the keystore file) and `SMW_DEBUG_KEYSTORE_PASSWORD` are set, and the standard `~/.android/debug.keystore` otherwise. Without the secrets (a fork's pull request, say) CI signs with a throwaway key and warns. An APK signed with a different key does not install over an existing one: uninstall first, which deletes settings and recordings. The workflow summary prints each APK's signing certificate (SHA-256). A release key for a store listing is not set up yet.

## Layout

Upstream's Android port ([mmatyas/supermariowar-android](https://github.com/mmatyas/supermariowar-android), built by upstream's disabled `build_android.yml`) is SDL's `android-project` template with a `MainActivity` that loads SDL2, SDL2_image, SDL2_mixer and `libmain.so`. This build follows it:

| Piece | Here |
|---|---|
| Java and Gradle | SDL 2.32.10's `android-project` template, with `app/`, `build.gradle` (AGP 8.5.2) and the Gradle 8.7 wrapper properties from this directory on top (`build.sh`) |
| Game | the `smw` library as `libmain.so`, exporting `SDL_main` (`crates/smw-core/src/android.rs`), built with cargo-ndk |
| SDL2, SDL2_image, SDL2_mixer | built from checksummed release tarballs with CMake and the NDK (`build-sdl-libs.sh`), PNG and OGG through stb as on muOS |
| ENet | built by `enet-sys`. Compression uses flate2's Rust backend, so the game links no zlib |
| Icon | `resources/smw.png` (32x32), scaled by whole numbers with nearest neighbour to every mipmap density, plus an adaptive icon on the sky blue of its frame (`make_icons.py`, run by `build.sh`) |

## Data and settings

The game lists directories and reads files under `data/` with plain filesystem calls, which cannot see inside an APK. Upstream reads `data/` from `$EXTERNAL_STORAGE/supermariowar/`, which the user copies there by hand. Here `data/` ships as APK assets and `MainActivity` copies it to the app's external files directory, `/sdcard/Android/data/com.kylejameswalker.supermariowar/files/data/`, on the first launch after each install or update (about a second). Files added to that folder stay.

Settings (`options.bin`, `controls.sdl2.bin`) and recordings (`replays/`) live in the same files directory, upstream's settings directory on Android, falling back to internal storage when there is no external storage. Uninstalling deletes them. Game output (stdout and stderr) goes to logcat under the `smw` tag (`adb logcat -s smw SDL`) and to `log.txt` in the same files directory, rewritten at each launch, which a file manager can open without adb. It lists every joystick at launch and when one connects or disconnects (`[pad]` lines), and ends with the message and source location of a Rust panic.

## Controls

SDL reads USB and Bluetooth gamepads and keyboards. The manifest sets `SMW_PAD_TRANSLATE=1`, as muOS does, so every pad with an SDL controller mapping uses the layout in [`device/muos/README.md`](../device/muos/README.md#controls) (A jump, B run, X item, Start pause) and joins as the next human player, also when it connects after launch. A is the bottom face button (Android's `KEYCODE_BUTTON_A`). Keyboards use the desktop keys. The Android Back button does nothing in the game; leave through **Exit** on the main menu.

The manifest also turns off SDL's accelerometer joystick, which would otherwise take player 1 on phones, keeps keyboards and remotes that have a D-pad but no stick or hat from becoming joysticks (`SDL_TV_REMOTE_AS_JOYSTICK=0`), and locks the screen to landscape.

## Touch controls

Upstream's Android port has no on-screen controls. Here the manifest sets `SMW_TOUCH=1`, and the game draws controls in the black bars beside the 4:3 picture (`crates/smw-core/src/smw/touch.rs`), laid out like the web build's (`web/touch.js`): a D-pad bottom left, Jump, Run and Item bottom right, Back and Start at the top. They press player 1's default keys, so a recording holds ordinary key input and replays exactly. Every finger is tracked on its own, so a direction, Run and Jump can be held together, and a thumb between two buttons presses both. Keys are released when the finger lifts, the touch is cancelled or the app loses focus.

- **D-pad or stick.** The button under Back switches between the D-pad (the default) and a floating stick: touching anywhere left of the picture puts the stick under the thumb, dragging presses up to two directions past a small dead zone, and the stick follows a thumb that goes past its edge but never leaves the left bar. The choice is kept in `touch_controls.txt` in the settings directory.
- **Players.** With no pad at launch the touch controls are player 1 and pads connected later take the next players. With a pad at launch the pad is player 1 and the touch controls player 2. The keyboard sets get no other player, and the remaining players are bots, as with `SMW_NO_KEYBOARD`. A recording made with touch controls says so (`#@ touch=1`), so its replay assigns players the same way.
- **Hiding.** The controls fade out at the first pad or keyboard input, return at the next touch, and start hidden when a pad is connected at launch.
- Touches do not reach the game as mouse clicks (`SDL_TOUCH_MOUSE_EVENTS=0`).
- The game runs in immersive full screen (the system bars come back with a swipe from the edge). A control that would sit on a display cutout moves sideways off it; MainActivity reports the cutouts' bounding boxes to the game.

## Build

Needs the Android SDK (`ANDROID_HOME`) with NDK 27.2.12479018 (`ANDROID_NDK_HOME`) and build-tools 35 or newer, Python 3, JDK 17, CMake, rsync, `cargo install cargo-ndk`, and the Rust targets:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
android/build.sh arm64-v8a armeabi-v7a x86_64   # default arm64-v8a only
```

The APK lands in `dist/SuperMarioWar-<Cargo version>.apk`, with `versionName` the Cargo version and `versionCode` `major*10000 + minor*100 + patch`. The SDL builds are kept in `target/android/sdl/<abi>/`; delete that to rebuild them. `check_page_size.sh` then fails the build unless every 64-bit library is aligned for 16 KB pages and stored uncompressed and 16 KB-aligned in the APK, which Android 15 and later check.

## Replay and pad checks

`android/smoke.sh <apk> [replay.txt ...]` installs the APK on the connected device or emulator and, for each replay, clears the app's data, starts it with the replay harness variables as intent extras, waits for it to quit, and diffs the dump and screenshots against `tools/golden/` like `tools/parity.sh`. `MainActivity` turns extras named `SMW_*` into environment variables and an `args` string array into the command line, in debuggable builds only:

```sh
adb shell am start -n com.kylejameswalker.supermariowar/.MainActivity --es SMW_REPLAY /data/data/com.kylejameswalker.supermariowar/files/r.txt --es SMW_NOLIMIT 1
```

`android/pad_smoke.sh <vpad>` plugs a virtual Bluetooth Xbox pad into a rootable emulator through `/dev/uinput` (`vpad.c`, built for the emulator's ABI with the NDK's clang), launches the installed APK with it connected, presses the D-pad, unplugs and replugs it, and checks the `[pad]` and `[input]` lines in logcat and `log.txt`. CI runs both after building the APK. Its replays include `text_shift` and `osk_pad`, which open Multiplayer and with it the ENet client socket; the manifest's `INTERNET` permission is what lets that socket open.
