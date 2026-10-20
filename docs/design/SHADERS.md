# GPU shader presets

Status: done. Desktop builds run RetroArch `.slangp` shader presets through [librashader](https://github.com/SnowflakePowered/librashader), picked under Graphics Options > Shader. Six tuned presets ship with the game, and players can add their own. Android and the web get a built-in CRT-Easymode shader.

## Summary

Players get CRT and other display effects from the RetroArch shader collection, including CRT-Royale, crt-guest-advanced and the BVM presets. The game already draws each frame on the CPU into one 640x480 image, so the shaders only change how the backend presents that image. Gameplay, rollback, replays and goldens do not change.

## Background

`Video::present` in `smw-platform` hands the backend a finished 640x480 ARGB8888 frame. The SDL2 backend uploads it to one streaming texture and scales it with `SDL_Renderer`, which has no custom shaders. The SDL3 backend works the same way.

## Proposed design

- **Presets:** [libretro/slang-shaders](https://github.com/libretro/slang-shaders) is a shallow submodule at `shaders/slang-shaders`. Each file keeps its upstream license header. Our own presets live in `shaders/presets/`: they point at the upstream shaders and tune their parameters for a 480-line source. Players add presets under `shaders/` in the settings folder (`~/Library/Preferences/.smw/shaders/` on macOS).
- **Presenter:** with the `shaders` feature on and a preset selected, `smw-sdl2` opens an OpenGL 3.3 core window instead of the renderer (`crates/smw-sdl2/src/shader.rs`). Each frame, it uploads the image, runs the preset's filter chain into a texture at the letterboxed size, and blits that texture to the window. If the context or the preset fails, the window falls back to the renderer. The touch controls draw over it with their own small shader (`shader_overlay.rs`), and an event watch maps mouse coordinates to the 640x480 picture, as SDL's renderer does.
- **Bundled presets (`shaders/presets/`):** each `#reference`s an upstream preset and overrides a few parameters for the 480-line picture. crt-guest-advanced treats more than 375 lines as interlaced, so both guest presets set `inter` to 800. CRT Trinitron 240p also halves the internal resolution, for 240-line scanlines over a Trinitron mask. CRT Royale turns off interlace detection, and its 3-pixel mask needs 3x output or more. Captures at 2x and 4x picked these settings.
- **Packaging:** the desktop release builds use the `shaders` feature. `tools/shader_bundle.py` follows the presets' references, passes, textures and includes, and copies those 61 files (1.2 MB) and `SHADER_LICENSES.md` into the package's `shaders/`, beside `data/`.
- **Feature gates:** librashader builds SPIRV-Cross and glslang from C++, so the `shaders` feature on `smw-sdl2` (passed through as `smw/shaders`) stays off in the Android, web, muOS and aarch64 builds. The `gl` feature needs only glow and turns on the built-in presenter.
- **Built-in presenter (`crt_lite.rs`):** a GLSL ES 1.00 port of CRT-Easymode for OpenGL ES 2 (Android) and WebGL 1 (the web), drawn straight to the window. WebGL 1 has no BGRA upload, so it uploads the frame's bytes as RGBA and swaps the channels in the shader. The backend lists it through `Video::builtin_shaders`, and the menu shows it as CRT Easymode where librashader is missing. The Android library and the web page build with `gl`.
- **Settings (`smw/video_settings.rs`):** the Graphics menu's Shader option lists Off, the bundled presets, then the player's. Off keeps today's renderer. Integer Scaling keeps the picture at whole multiples of 640x480, where scanlines look even. Both save to `video.txt` in the settings folder, and the window opens with the saved preset. The two options appear only when the backend supports shaders, so the headless goldens do not change.

`guides/REPLAY.md` lists the development options. `SMW_SHADER=<preset.slangp>` selects a preset. `SMW_WINDOW_SCALE=<n>` opens the window at n times 640x480. `SMW_SHADER_SHOT=<n>` saves the window after present n as `shader_<n>.bmp` in `SMW_SHOT_DIR`.

## Alternatives considered

- **A hand-written GLSL shader per effect.** A prototype ported CRT-Easymode this way. It works on OpenGL ES too, but each new effect is a new port, and players cannot bring their own presets.
- **`SDL_GPURenderState` in the SDL3 backend.** It adds a custom fragment shader to the existing renderer. Each shader needs a build per GPU driver (SPIR-V, Metal, DXIL), though, and most players run the SDL2 build.
- **A trimmed fork of slang-shaders.** It is smaller, but it drifts from upstream. The shallow submodule clones 81 MB and the release packages only the files our presets use.

## Risks and open questions

- **Licensing:** most CRT shaders are GPL, the Royale blurs are MIT, crt-lottes is public domain, and librashader is MPL-2.0 or GPL-3.0. The game has no declared license. The release ships the shaders as unchanged source files with `SHADER_LICENSES.md`.
- **BVM Monitor** is dark: crt-potato multiplies by a mask texture and has no parameters to brighten it.
- **macOS:** OpenGL stops at 4.1 there, so presets that need GL 4.3 features, such as crt-geom (`GL_ARB_arrays_of_arrays`), fail to load. Graphics Options then shows the error and hides the preset for the rest of the session.
- **The 480-line source:** at 2x, each source line covers two screen rows, which is too few for strong scanlines. Presets tuned for 240-line games, such as CRT-Royale, look washed out unless tuned or shown at 3x and up.

## Rollout

1. Done: submodule and GL presenter behind `SMW_SHADER`.
2. Done: overlay and mouse mapping in the GL path.
3. Done: the Graphics setting, integer scaling and the player's `shaders/` folder.
4. Done: tuned bundled presets, release packaging and license notices.
5. Done: a built-in shader for OpenGL ES (Android) and WebGL (web).

Off stays the default, and turning the setting off returns to the current renderer.
