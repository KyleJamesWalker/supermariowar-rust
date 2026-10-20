# GPU shader presets

Status: in progress. The desktop SDL2 build can run a RetroArch `.slangp` shader preset over each frame through [librashader](https://github.com/SnowflakePowered/librashader). A cargo feature gates it, and `SMW_SHADER` selects the preset until the Graphics setting lands.

## Summary

Players get CRT and other display effects from the RetroArch shader collection, including CRT-Royale, crt-guest-advanced and the BVM presets. The game already draws each frame on the CPU into one 640x480 image, so the shaders only change how the backend presents that image. Gameplay, rollback, replays and goldens do not change.

## Background

`Video::present` in `smw-platform` hands the backend a finished 640x480 ARGB8888 frame. The SDL2 backend uploads it to one streaming texture and scales it with `SDL_Renderer`, which has no custom shaders. The SDL3 backend works the same way.

## Proposed design

- **Presets:** [libretro/slang-shaders](https://github.com/libretro/slang-shaders) is a shallow submodule at `shaders/slang-shaders`. Each file keeps its upstream license header. Our own presets live in `shaders/presets/`: they point at the upstream shaders and tune their parameters for a 480-line source. Players add presets under `~/.smw/shaders/`.
- **Presenter:** with the `shaders` feature on and a preset selected, `smw-sdl2` opens an OpenGL 3.3 core window instead of the renderer (`crates/smw-sdl2/src/shader.rs`). Each frame, it uploads the image, runs the preset's filter chain into a texture at the letterboxed size, and blits that texture to the window. If the context or the preset fails, the window falls back to the renderer.
- **Feature gate:** librashader builds SPIRV-Cross and glslang from C++. The `shaders` feature on `smw-sdl2` (passed through as `smw/shaders`) keeps that out of the Android, web, muOS and aarch64 builds.
- **Settings:** a Graphics option lists the bundled presets, then the player's, with Off first. Off keeps today's renderer. An integer scaling option keeps the image at whole multiples of 640x480, where scanlines look even.

`guides/REPLAY.md` lists the development options. `SMW_SHADER=<preset.slangp>` selects a preset. `SMW_WINDOW_SCALE=<n>` opens the window at n times 640x480. `SMW_SHADER_SHOT=<n>` saves the shader output of present n as `shader_<n>.bmp` in `SMW_SHOT_DIR`.

## Alternatives considered

- **A hand-written GLSL shader per effect.** A prototype ported CRT-Easymode this way. It works on OpenGL ES too, but each new effect is a new port, and players cannot bring their own presets.
- **`SDL_GPURenderState` in the SDL3 backend.** It adds a custom fragment shader to the existing renderer. Each shader needs a build per GPU driver (SPIR-V, Metal, DXIL), though, and most players run the SDL2 build.
- **A trimmed fork of slang-shaders.** It is smaller, but it drifts from upstream. The shallow submodule clones 81 MB and the release packages only the files our presets use.

## Risks and open questions

- **Licensing:** most CRT shaders are GPL, crt-lottes is public domain, and librashader is MPL-2.0 or GPL-3.0. The game has no declared license. The release ships the shaders as loose source files with a notices file.
- **macOS:** OpenGL stops at 4.1 there, so presets that need GL 4.3 features, such as crt-geom (`GL_ARB_arrays_of_arrays`), fail to load. The menu should leave them out.
- **The 480-line source:** at 2x, each source line covers two screen rows, which is too few for strong scanlines. Presets tuned for 240-line games, such as CRT-Royale, look washed out unless tuned or shown at 3x and up.
- **Not drawn yet in the GL path:** the touch and F1 overlay, and mouse coordinates in game pixels.
- **Android and the web:** librashader targets desktop GL. OpenGL ES and WebGL need a lighter built-in shader.

## Rollout

1. Submodule and GL presenter behind `SMW_SHADER` (this change).
2. Overlay and mouse mapping in the GL path.
3. The Graphics setting, integer scaling and the `~/.smw/shaders/` folder.
4. Tuned bundled presets, release packaging and license notices.
5. A built-in shader for OpenGL ES (Android) and WebGL (web).

Off stays the default, and turning the setting off returns to the current renderer.
