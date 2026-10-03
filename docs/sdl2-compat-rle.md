# Draft issue: every blit re-encodes RLE surfaces (sdl2-compat)

Draft for the sdl2-compat issue tracker. Not filed.

---

**Title:** `SDL_UpperBlit` invalidates the blit map on every call, so RLE surfaces are re-encoded on every blit

**Versions:** sdl2-compat 2.32.72 and SDL 3.4.16 (Homebrew), macOS 26, arm64.

## Summary

Every SDL2 blit of an RLE-accelerated surface re-encodes the whole surface. A 32×32 blit from a 480×96 RLE sprite sheet costs about 44 µs. Without `SDL_SetSurfaceRLE` it costs well under 1 µs. With real SDL2, the RLE encoding is built once and reused.

This makes RLE, which is meant as an optimisation, the dominant cost of an SDL2 game. In Super Mario War, 80–90% of main-thread time goes to `RLEAlphaSurface` / `copy_32`, called from `SDL_CalculateBlit` on almost every blit.

## Cause

1. `SDL_LowerBlit` calls `Surface2to3()` for both surfaces (`src/sdl2_compat.c`).
2. `Surface2to3()` calls `SDL3_SetSurfacePalette(surface3, surface->format->palette)` unconditionally, even when the palette is `NULL` and has not changed.
3. In SDL3, `SDL_SetSurfacePalette()` ends with an unconditional `SDL_InvalidateMap(&surface->map)` (`src/video/SDL_surface.c`, release-3.4.x).
4. On the next `SDL_BlitSurfaceUnchecked`, `SDL_ValidateMap` → `SDL_CalculateBlit` sees the RLE-desired flag and calls `SDL_RLESurface`. That re-encodes the entire source surface.

Possible fixes:
- Only call `SDL3_SetSurfacePalette` when the palette actually changed.
- Make SDL3's `SDL_SetSurfacePalette` skip the invalidation when the palette is unchanged.

## Reproduction

```c
#include <SDL.h>
#include <SDL_image.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv)
{
    SDL_Init(SDL_INIT_VIDEO);
    SDL_Surface *dst = SDL_CreateRGBSurface(0, 640, 480, 32, 0, 0, 0, 0);
    SDL_Surface *raw = IMG_Load(argv[1]);   /* any PNG sprite sheet, e.g. 480x96 */
    SDL_Surface *img = SDL_ConvertSurface(raw, dst->format, 0);
    if (!getenv("NORLE"))
        SDL_SetSurfaceRLE(img, 1);
    SDL_SetColorKey(img, 1, SDL_MapRGB(img->format, 255, 0, 255));

    SDL_Rect src = {0, 0, 32, 32};
    Uint64 t0 = SDL_GetPerformanceCounter();
    for (int i = 0; i < 2000; i++) {
        SDL_Rect d = {i % 600, i % 400, 32, 32};
        SDL_BlitSurface(img, &src, dst, &d);
    }
    Uint64 t1 = SDL_GetPerformanceCounter();
    printf("%dx%d %.3f ms per blit\n", img->w, img->h,
           (t1 - t0) * 1000.0 / SDL_GetPerformanceFrequency() / 2000);
    return 0;
}
```

```sh
cc -O2 t.c $(sdl2-config --cflags) $(sdl2-config --libs) -lSDL2_image -o t
./t blocks.png            # 480x96 0.044 ms per blit
NORLE=1 ./t blocks.png    # 480x96 0.000 ms per blit
```

The destination is the same surface on every iteration, so nothing legitimately invalidates the map.

## Impact measured in a game

Super Mario War (C++ reference and its Rust port, both on sdl2-compat), headless, frame limiter off. Times are CPU per frame on a heavily loaded machine.

| scene | with RLE | RLE disabled |
|---|---|---|
| world map menu (`flow_world`, 6000 frames) | 18.9 ms | 1.2 ms |
| heavy battle map (`gg_death_valley`, 1214 frames) | 11.6 ms | 0.9 ms |

Disabling RLE is not an acceptable workaround for us. SDL3's RLE alpha blit rounds differently from the non-RLE blend, which changes the rendered pixels.
