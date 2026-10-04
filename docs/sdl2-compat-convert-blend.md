# Draft issue: `SDL_ConvertSurface` to an alpha format returns a blended surface (sdl2-compat)

Draft for the sdl2-compat issue tracker. Not filed.

---

**Title:** `SDL_ConvertSurface` of an opaque surface to an alpha format sets `SDL_BLENDMODE_BLEND`, so a later colour key is ignored under RLE

**Versions:** sdl2-compat 2.32.72 and SDL 3.4.18 (Homebrew), macOS 26, arm64. Compared with SDL 2.32.10 (Emscripten port).

## Summary

With SDL2, converting an opaque RGB24 surface to ARGB8888 gives a surface with `SDL_BLENDMODE_NONE`. With sdl2-compat it gives `SDL_BLENDMODE_BLEND`. On an RLE surface with a blend mode and an alpha channel, the RLE encoder uses per-pixel alpha and ignores the colour key, so colour-keyed pixels (alpha 255) are drawn opaque.

Super Mario War loads its map foreground layer this way, fills it with the magenta colour key, and draws tiles onto it. Under sdl2-compat, maps with a foreground layer show magenta where the layer should be transparent.

## Reproduction

```c
#include <SDL.h>
#include <stdio.h>

int main(void)
{
    SDL_Surface *dst = SDL_CreateRGBSurfaceWithFormat(0, 64, 64, 32, SDL_PIXELFORMAT_ARGB8888);
    SDL_Surface *raw = SDL_CreateRGBSurfaceWithFormat(0, 64, 64, 24, SDL_PIXELFORMAT_RGB24);
    SDL_Surface *fg = SDL_ConvertSurface(raw, dst->format, 0);   /* opaque image -> screen format */
    SDL_BlendMode mode;
    SDL_GetSurfaceBlendMode(fg, &mode);
    SDL_SetSurfaceRLE(fg, 1);

    Uint32 magenta = SDL_MapRGB(fg->format, 255, 0, 255);
    SDL_LockSurface(fg);
    SDL_FillRect(fg, NULL, magenta);
    SDL_SetColorKey(fg, SDL_TRUE, magenta);
    SDL_UnlockSurface(fg);

    SDL_FillRect(dst, NULL, SDL_MapRGB(dst->format, 0, 0, 255));
    SDL_BlitSurface(fg, NULL, dst, NULL);
    printf("blend mode %d, dst pixel %08x (expect ff0000ff)\n", (int)mode, ((Uint32 *)dst->pixels)[0]);
    return 0;
}
```

```
sdl2-compat 2.32.72:  blend mode 1, dst pixel ffff00ff (expect ff0000ff)
SDL 2.32.10:          blend mode 0, dst pixel ff0000ff (expect ff0000ff)
```

Without `SDL_SetSurfaceRLE`, or after `SDL_SetSurfaceBlendMode(fg, SDL_BLENDMODE_NONE)`, sdl2-compat also prints `ff0000ff`. SDL 2.32.10 prints `ffff00ff` too if the surface is set to `SDL_BLENDMODE_BLEND` first, so the RLE behaviour is the same in both; the difference is the blend mode `SDL_ConvertSurface` returns.

Native SDL3 behaves like sdl2-compat. The same steps in SDL3's API print `blend mode 1` and a magenta pixel, and a transparent one after `SDL_SetSurfaceBlendMode(fg, SDL_BLENDMODE_NONE)`:

```c
SDL_Surface *dst = SDL_CreateSurface(64, 64, SDL_PIXELFORMAT_ARGB8888);
SDL_Surface *raw = SDL_CreateSurface(64, 64, SDL_PIXELFORMAT_RGB24);
SDL_Surface *fg = SDL_ConvertSurface(raw, SDL_PIXELFORMAT_ARGB8888);
SDL_SetSurfaceRLE(fg, true);
Uint32 magenta = SDL_MapSurfaceRGB(fg, 255, 0, 255);
SDL_LockSurface(fg);
SDL_FillSurfaceRect(fg, NULL, magenta);
SDL_SetSurfaceColorKey(fg, true, magenta);
SDL_UnlockSurface(fg);
SDL_FillSurfaceRect(dst, NULL, SDL_MapSurfaceRGB(dst, 0, 0, 255));
SDL_BlitSurface(fg, NULL, dst, NULL);   /* dst is magenta on SDL 3.4.18 */
```

SDL3 may consider this intended, but sdl2-compat should return what SDL2 does. A direct SDL3 port that keeps RLE needs `SDL_BLENDMODE_NONE` on opaque converted images.

## Impact

10 of the port's 44 replays and 121 of its 290 map replays have magenta in their screenshots, in both the C++ build and the Rust port. Setting `SDL_BLENDMODE_NONE` on the foreground surface, or drawing without RLE, removes it. The result then matches the Emscripten build pixel for pixel.
