# Draft issue: first-use CPU detection races, so surfaces get misaligned pixels (SDL3)

Draft for the SDL issue tracker. Not filed.

---

**Title:** `SDL_GetSIMDAlignment` can return `0xFFFFFFFF` while another thread is detecting CPU features, and `SDL_CreateSurface` then allocates misaligned pixels

**Versions:** SDL 3.4.18 and sdl2-compat 2.32.74 (Homebrew), macOS 26.6, arm64.

## Summary

SDL detects CPU features lazily, on first use, without synchronization. If several threads create their first surface at the same time, one of them can read `SDL_SIMDAlignment` before it is set and pass `0xFFFFFFFF` to `SDL_aligned_alloc`. The surface's `pixels` pointer is then not even 4-byte aligned. Writing it as 32-bit pixels is undefined behavior; Rust's debug checks abort on it.

We hit this in a test suite that creates surfaces from parallel test threads without calling `SDL_Init` first.

## Cause

`src/cpuinfo/SDL_cpuinfo.c`:

```c
static Uint32 SDL_CPUFeatures = SDL_CPUFEATURES_RESET_VALUE;   /* 0xFFFFFFFF */
static Uint32 SDL_SIMDAlignment = 0xFFFFFFFF;

static Uint32 SDL_GetCPUFeatures(void)
{
    if (SDL_CPUFeatures == SDL_CPUFEATURES_RESET_VALUE) {
        CPU_calcCPUIDFeatures();
        SDL_CPUFeatures = 0;
        SDL_SIMDAlignment = sizeof(void *);
        ...
    }
    return SDL_CPUFeatures;
}

size_t SDL_GetSIMDAlignment(void)
{
    if (SDL_SIMDAlignment == 0xFFFFFFFF) {
        SDL_GetCPUFeatures();
    }
    return SDL_SIMDAlignment;
}
```

1. Thread A enters `SDL_GetCPUFeatures` and stores `SDL_CPUFeatures = 0`.
2. Thread B calls `SDL_GetSIMDAlignment`, sees `SDL_SIMDAlignment == 0xFFFFFFFF`, and calls `SDL_GetCPUFeatures`. Features are no longer the reset value, so it returns at once.
3. Thread B returns `SDL_SIMDAlignment`, still `0xFFFFFFFF`, because A has not stored `sizeof(void *)` yet. Nothing orders the two stores, so the compiler or CPU may also make them visible in either order.
4. `SDL_CreateSurface` calls `SDL_aligned_alloc(0xFFFFFFFF, size)`, which offsets the result by `alignment - (ptr % alignment)`. That gives an arbitrary, usually odd, pointer.

The same window can also return a smaller alignment than the final one, for example `sizeof(void *)` before NEON or SSE raises it to 16. SIMD code that relies on the full alignment would then read misaligned buffers. The audio work buffers and `SDL_audiocvt.c` also use `SDL_GetSIMDAlignment`.

`SDL_InitAudio` probes CPU features through `SDL_ChooseAudioConverters`, so a program that starts audio before it starts other threads is safe. We did not check whether other subsystems' init does the same.

## Reproduction

32 threads create their first surface at the same moment. Built against sdl2-compat; the same calls through SDL3 directly take the same path.

```c
#include <SDL2/SDL.h>
#include <pthread.h>
#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#define N 32
static atomic_int ready;
static atomic_int go;
static uintptr_t got[N];
static void *run(void *arg) {
    long i = (long)arg;
    atomic_fetch_add(&ready, 1);
    while (!atomic_load(&go)) {}
    SDL_Surface *s = SDL_CreateRGBSurfaceWithFormat(0, 8, 8, 32, SDL_PIXELFORMAT_ARGB8888);
    got[i] = s ? (uintptr_t)s->pixels : 1;
    return NULL;
}
int main(void) {
    pthread_t t[N];
    /* SDL_SIMDGetAlignment();   uncomment to prime detection: no failures */
    for (long i = 0; i < N; i++) pthread_create(&t[i], NULL, run, (void *)i);
    while (atomic_load(&ready) < N) {}
    atomic_store(&go, 1);
    for (int i = 0; i < N; i++) pthread_join(t[i], NULL);
    int bad = 0;
    for (int i = 0; i < N; i++) if (got[i] % 4) bad++;
    if (bad) printf("misaligned %d\n", bad);
    return bad ? 1 : 0;
}
```

```sh
cc -O2 race.c $(sdl2-config --cflags) $(sdl2-config --libs) -o race
for i in $(seq 3000); do ./race; done | sort | uniq -c
```

| Run | Processes with a misaligned surface |
|---|---|
| As written | 11 of 3000 |
| With `SDL_SIMDGetAlignment()` on the main thread before the threads start | 0 of 3000 |

## Suggested fix

Publish the result only once it is complete:

- Compute the features and the alignment into locals, store `SDL_SIMDAlignment` first, then store `SDL_CPUFeatures` last with release ordering (`SDL_SetAtomicU32`), and read it with acquire ordering. The work is idempotent, so a thread that loses the race can still compute the same values.
- Or guard the detection with `SDL_ShouldInit` / `SDL_SetInitialized`, as other lazily initialized SDL state does.
- `SDL_GetSIMDAlignment` should test `SDL_CPUFeatures`, the value it waits on, rather than `SDL_SIMDAlignment`.

## Workaround

Call `SDL_GetSIMDAlignment()` (`SDL_SIMDGetAlignment()` in SDL2) once on one thread before any other thread uses SDL. Super Mario War's Rust port does this in its SDL comparison tests. The game itself starts audio on the main thread, which probes CPU features before SDL starts any thread, and it creates no surfaces off the main thread.
