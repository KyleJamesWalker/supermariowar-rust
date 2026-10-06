# Architecture v2: platform-free core, SDL2 and SDL3 backends

Status: accepted; the decisions are under Risks and decisions. Owner: the port's maintainer. Phase 0 (the Cargo workspace) is PR #23.

## Summary

This document proposes how the faithful C++ translation becomes idiomatic Rust without losing frame-perfect replay parity on the way.

- **Crates.** A Cargo workspace holds a platform-free `smw-core` (game logic, physics, AI, menus, frame loop, replay harness, virtual mixer). Traits for video, audio, input, time and storage live in `smw-platform`. The two backends `smw-sdl2` and `smw-sdl3` implement them, and the build picks one through a Cargo feature. `smw-netplay` holds the protocol and lobby code that the game, `smw_server` and `smw_relay` share.
- **Dependency injection.** Global state moves in three behavior-preserving steps. First, platform services sit behind one context. Second, game globals move into a `Game` struct that one accessor reaches. Third, `&mut Game` is passed down from the frame loop until the accessor is gone.
- **Input.** Backends emit one normalized `InputEvent` stream (keys, pad axes, buttons and hats, device add and remove, text, touch). Live play, touch controls, hot-plug, recording and replay all use that stream. The replay format stays the same, and version 1 recordings replay unchanged.
- **Pixels and sound.** `smw-core` draws into its own software framebuffer with a pure-Rust blitter, and its virtual mixer decides all game-visible sound state. A backend only presents frames and plays sound commands. This keeps SDL2 and SDL3 builds byte-identical by construction.
- **Parity.** C++ goldens keep gating every PR through all phases. At a separate milestone that the maintainer schedules, Rust goldens become primary and the C++ goldens become a report-only check. From phase 6, CI runs every replay on every backend and platform and fails on any difference between their outputs.

## Background

The port mirrors the C++ tree one module per file (`docs/ARCHITECTURE.md`). It keeps C++ names, numeric semantics and global state, so a replay produces the same per-frame dump and screenshots as the C++ reference (`docs/REPLAY.md`). The numbers at `origin/main` (`8df9905`):

| Measure | Value |
|---|---|
| Rust source in `src/` | 92,750 lines |
| Files that use `sdl2::sys` directly | 116 (90 in `smw/`, 24 in `common/`, 1 per editor) |
| `static mut` declarations | 281 in 36 files (167 in the two editors; 23 `Global<T>`, 54 `Ptr<T>`) |
| `unsafe {` blocks | 1,324 |
| Game replays with goldens (`tools/replays`) | 47 (45 C++ goldens, 2 Rust goldens) |
| Map sweep replays (`tools/replays_sweep`) | 290 |
| Editor sessions (`tools/editor_replays`) | 13 |
| Segment replays (`tools/segment_replays`) | 9 |

Several deliberate deviations already make the Rust game the better baseline. These include correctly rounded trigonometry (`src/common/math/trig.rs`, CORE-MATH, also linked into the reference), host-decided netplay outcomes, pad translation and touch controls. `docs/PROGRESS.md` lists them all.

Platforms: macOS, Linux and Windows (SDL2, or sdl2-compat over SDL3 on Homebrew), web (Emscripten's SDL2 port), Android (SDL2 `SDLActivity`, `libmain.so` from `cargo ndk rustc --lib`) and muOS. On muOS the device provides `libSDL2`, which carries the GPU driver.

## Problem

1. **SDL is everywhere.** Game logic calls `sdl2::sys` in 116 files. An SDL3 backend would mean editing all of them, and SDL2 and SDL3 export the same `SDL_*` symbol names, so one binary cannot link both.
2. **Global state blocks testing and new features.** All state is process-global `static mut`. One process runs one game, tests cannot run replays in-process or in parallel, and a checkpoint has to list every global by hand (`src/smw/checkpoint.rs`). Rollback or lockstep netplay would need several game instances in one process.
3. **Input is split by device.** Keyboard and joystick events reach `CPlayerInput::update` as raw `SDL_Event`s. `pad.rs` rewrites controller events inside an SDL event filter. Android touch (`touch.rs`, on its branch) and web touch (`web/touch.js`) push fake key events. The harness pushes replay events into SDL's queue and attaches SDL virtual joysticks. Every input source has its own path.
4. **Pixels depend on the SDL build.** Screenshots come from SDL's software blitter. sdl2-compat has two known differences from SDL2 (`docs/sdl2-compat-rle.md`, `docs/sdl2-compat-convert-blend.md`), and the port turns RLE off natively to stay identical. SDL3's blitter is a third implementation.
5. **Sound timing depends on SDL_mixer.** The virtual mixer computes channel end times from SDL_mixer's decoded chunk length and `Mix_MusicDuration`. A different mixer (SDL3_mixer) or resampler can change those lengths. That moves `isPlaying()` results, and gameplay reads them.

## Proposed design

### Crate graph

```
smw (app: game binary, Android cdylib)
 ├─ smw-core ─────────┬─ smw-platform
 │                    └─ smw-globals
 ├─ smw-sdl2 | smw-sdl3 ── smw-platform     (exactly one, by feature)
 └─ smw-netplay ── smw-globals
leveleditor, worldeditor (apps) ── smw-core, smw-sdl2 | smw-sdl3
smw_server (app) ── smw-netplay
smw_relay (app) ── smw-netplay, tungstenite
smw-headless ── smw-platform                (tests and the replay runner)
```

| Crate | Contents | Depends on SDL |
|---|---|---|
| `smw-globals` | `Ptr`, `Global`, `Aliased`, `impl_base!`, `enum_from_u8!` (today `src/globals/`) | no |
| `smw-platform` | Traits and plain data: `Video`, `AudioOut`, `Clock`, `Storage`, `InputEvent`, `Keycode`, `PadSlot`, `Frame` | no |
| `smw-core` | `common/`, `smw/`, the replay harness, checkpoints, virtual mixer, input mapper, touch layout, software surfaces and blitter, image decoding | no (from phase 4) |
| `smw-netplay` | `common_netplay/`, `server/`, the `file_io` subset the relay uses today through `#[path]`, the ENet and WebSocket network layers | no |
| `smw-sdl2`, `smw-sdl3` | Window and presentation, audio output, event pump to `InputEvent`, pad mapping, platform paths, Android and Emscripten glue | yes |
| `smw-headless` | In-memory framebuffer, a sound-command log, a scripted clock, temp-dir storage | no |
| apps | `main()` per binary: select the backend, build `Services`, run the frame driver | via backend |

Rules:

- `smw-core` keeps the C++ mirror layout inside the crate (`smw_core::smw::gs_gameplay` mirrors `GSGameplay.cpp`). Upstream sync stays a file-to-file port.
- The backend is a compile-time choice: `cargo build --bin smw --features sdl3`. The default is `sdl2`. Enabling both features is a `compile_error!`, because the two libraries' symbols collide.
- Binary names stay `smw`, `leveleditor`, `worldeditor`, `smw_server` and `smw_relay`, so the tools, packaging scripts and docs keep working.
- File moves (`src/` to `crates/smw-core/src/`) happen in one rename-only PR during an agreed merge freeze, because open agent branches touch `src/`, `tools/` and `web/` (see Rollout, phase 1).

### Trait boundaries

The core gets services from the context and never calls a platform API itself. Input is pushed in by the frame driver, not pulled by the core. The frame driver owns `Services` and lends them to each frame: `FrameOutput` borrows the core's framebuffer, and the driver then presents it through the video service, which it could not do if `Game` owned the services. Sketch signatures:

```rust
// smw-platform
pub const SCREEN_W: usize = 640;
pub const SCREEN_H: usize = 480;

/// One finished frame: 640x480 ARGB8888, the same bytes SMW_SHOT_STREAM writes today.
pub struct Frame<'a> { pub pixels: &'a [u32] }

pub trait Video {
    fn present(&mut self, frame: Frame<'_>);
    fn set_fullscreen(&mut self, on: bool);
    fn set_title(&mut self, title: &str);
    fn show_error(&mut self, message: &str);
    /// Optional on-screen controls drawn outside the 4:3 picture (Android, later web).
    fn draw_overlay(&mut self, overlay: &touch::Overlay) {}
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SoundId(pub u32);
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TrackId(pub u32);

/// Output only. Nothing here returns game-visible state, and backends never call back into the game.
/// `isPlaying()`, `Mix_PlayingMusic()` and the finish callbacks all come from the core's virtual mixer.
pub trait AudioOut {
    fn load_sound(&mut self, id: SoundId, bytes: &[u8]);
    fn load_track(&mut self, id: TrackId, bytes: &[u8]);
    fn free_sound(&mut self, id: SoundId);
    fn free_track(&mut self, id: TrackId);
    fn play(&mut self, channel: u8, id: SoundId, loops: i32);   // channel chosen by the core
    fn halt(&mut self, channel: Option<u8>);                      // None: every channel
    fn play_track(&mut self, id: TrackId, once: bool);
    fn stop_track(&mut self);
    fn pause_track(&mut self, paused: bool);
    fn set_volumes(&mut self, sound: u8, music: u8);
}

pub trait Clock {
    /// Wall-clock milliseconds. Only the frame driver, the FPS overlay and netplay message timestamps read it.
    fn now_ms(&self) -> u64;
    fn sleep_ms(&self, ms: u32);
}

pub trait Storage {
    fn settings_dir(&self) -> PathBuf;      // today ~/Library/Preferences/.smw on macOS
    fn data_dir(&self) -> PathBuf;          // GetRootDirectory() + "data", or --datadir
    fn persist(&mut self) {}                // web: IDBFS sync; elsewhere a no-op
}

pub struct Services {
    pub video: Box<dyn Video>,
    pub audio: Box<dyn AudioOut>,
    pub clock: Box<dyn Clock>,
    pub storage: Box<dyn Storage>,
}
```

```rust
// smw-core
pub struct FrameOutput<'a> {
    pub frame: Frame<'a>,
    pub quit: bool,
}

impl Game {
    pub fn new(services: &mut Services, args: &CmdArgs) -> Game;
    /// One iteration of the C++ gameloop body: sound advance, input, state update, dump, screenshot.
    pub fn frame(&mut self, services: &mut Services, input: &mut dyn InputQueue) -> FrameOutput<'_>;
}

/// Events for this frame, plus the blocking wait that MI_InputControlField::SendInput needs.
pub trait InputQueue {
    fn poll(&mut self) -> Option<InputEvent>;
    fn wait(&mut self) -> Option<InputEvent>;
}
```

Three design decisions follow from the parity constraints.

1. **Pixels are the core's.** `smw-core` owns `Surface` (a `Vec<u32>` or palette-indexed buffer with a color key, alpha mod and blend mode) and a blitter that reproduces the SDL blit paths the game uses: `SDL_UpperBlit` with clipping, color key, per-surface alpha, `SDL_FillRect`, `SDL_ConvertSurface` and palette mapping. Images decode with the `png` crate and a small BMP reader. With this design, both backends present the same bytes, the web build stops depending on RLE behavior, and the sdl2-compat differences stop mattering. The screenshots of the 47 game replays, the 290 sweep replays and the 13 editor sessions verify every blit path.
2. **Sound state is the core's.** The virtual mixer (`sfx.rs`, today active only in seeded runs) becomes the only source of `isPlaying()`, channel numbers and finish events in every run. Backends play what it says on the channel it picks. Durations come from a table that the core builds from file headers, or from a checked-in table that a tool generates with SDL2_mixer. They no longer come from the backend's decoded length. A test on the SDL2 backend compares the table with `Mix_Chunk::alen` and `Mix_MusicDuration` for every file in `data/`.
3. **Time is the frame counter.** Game logic reads only the virtual clock (`1000 + N * 16` ms, already used by seeded runs). The wall clock reaches only the frame driver (`fps_limiter.rs`), the F1 overlay and the netplay message timestamps in `net.rs`, which nothing reads back.

### Threading the context through the code

The migration has three steps. Each step is a series of PRs, and each PR keeps every golden byte-identical.

**Step 1: services behind one context.** A single `static mut SERVICES: Global<Services>` with an `unsafe fn services() -> &'static mut Services` replaces the direct SDL calls, one subsystem per PR (time, storage, audio, presentation, events). Game state ownership does not change. This step removes `sdl2::sys` from every file outside `gfx` surfaces, which phase 4 replaces.

**Step 2: globals into a `Game` struct.** Fields group by subsystem (`game_values`, `map`, `rm`, `objects`, `players`, `scores`, `rng`, `sfx`, `menus`, `harness`, `net`). One accessor reaches them during the transition:

```rust
pub struct Game {
    pub game_values: CGameValues,
    pub g_map: Box<CMap>,
    pub rm: Box<CResourceManager>,
    pub objectcontainer: [CObjectContainer; 3],
    // ... one field per C++ global, declared in C++ definition order
}

static mut GAME: Ptr<Game> = Ptr::null();
#[inline(always)]
pub fn game() -> Ptr<Game> { unsafe { GAME } }
```

The accessor returns `Ptr<Game>`, not `&'static mut Game`. Step 3 passes `&mut Game` down call trees while leaf code still reaches the same object through the accessor, and two live `&mut` to one object is undefined behavior. `Ptr` keeps that aliasing in the raw-pointer world the port already lives in. Call sites change mechanically, for example from `game_values.gamemode` to `game().game_values.gamemode`. `Game::new` constructs fields in the order `globals::init_globals` uses today. Constructors that draw from the RNG or read files must run in the same order, because a different order changes the `R calls=` record. Function-local `static mut`s, for example in `sfx.rs`, `pad.rs`, `harness.rs` and `main.rs`, move into the subsystem that owns them. Checkpoint save and restore can then cover whole subsystems instead of listing globals by hand.

**Step 3: pass `&mut Game` down.** The frame loop passes `&mut Game` into `GameStateManager`, then each state's `update`/`draw`, then subsystems. A PR converts one call tree, and leaf code that still calls `game()` keeps working through the `Ptr`. When no caller remains, the static is deleted and `&mut Game` becomes the only path. `Game` is then an ordinary value, so one process can run several games: parallel in-process replay tests, and later rollback.

Step 3 meets the object graph: methods that take `&mut self` on an object inside `Game` while touching other parts of `Game`. The `Ptr`/`Aliased` convention (`docs/ARCHITECTURE.md`, Pointers and ownership) stays valid through all three steps. Replacing `Ptr` graphs with arenas and typed handles is a separate later design (decision 9).

### Normalized input

```rust
// smw-platform
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PadSlot(pub u8);              // the open index: replay `<dev>`, `which`, binding device

/// SDL2 keycode values. The core owns the name table that SDL_GetKeyName/SDL_GetKeyFromName give today.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Keycode(pub i32);

pub enum InputEvent {
    Key { key: Keycode, down: bool },
    PadAxis { pad: PadSlot, axis: u8, value: i16 },
    PadButton { pad: PadSlot, button: u8, down: bool },
    PadHat { pad: PadSlot, hat: u8, value: u8 },
    PadAdded { pad: PadSlot, name: String, guid: [u8; 16] },
    PadRemoved { pad: PadSlot },
    Touch { finger: u64, phase: TouchPhase, x: f32, y: f32 },   // normalized to the window
    Text(String),
    Mouse(MouseEvent),                                            // editors
    Quit,
}
```

There are two layers:

- **Device events (`InputEvent`).** Backends produce them, and replays and the recorder read and write them. Key values are SDL2 keycodes, because `controls.sdl2.bin`, `options.bin` and every replay use them. `smw-sdl3` translates SDL3 keycodes into those values. Letters and ASCII keys have the same values in SDL3, and scancode-derived keys keep the same `1 << 30` mask. SDL3 key events apply modifiers to the keycode by default, so Shift+A would arrive as a different value than SDL2 sends; `smw-sdl3` sets `SDL_HINT_KEYCODE_OPTIONS` to `unmodified` so shifted bindings keep their SDL2 values. The SDL2 key-name table moves into `smw-core`, so the headless backend and SDL3 parse replays without SDL.
- **Player-slot actions.** The ported `CPlayerInput::update` stays the only translator from device events to the per-player game and menu key states (`outputControls`) that game code reads. Bindings, the stick and hat merge, `assign_inputs` and the keyboard-set rules stay where they are, now fed `InputEvent` instead of `SDL_Event`.

Where each input source goes:

| Source | Today | Target |
|---|---|---|
| Keyboard, joystick | `SDL_PollEvent` in each game state | The backend's pump converts SDL events to `InputEvent`. The frame driver passes them in through `InputQueue`. |
| Controller translation (`pad.rs`, `SMW_PAD_TRANSLATE`) | An SDL event filter rewrites `SDL_Event`s | In the backend: a game controller with a mapping emits `PadButton`/`PadHat`/`PadAxis` in `LAYOUT` order. SDL3 uses its gamepad API for the same layout. |
| Hot-plug | Not handled. Pads are assigned at launch. | `PadAdded`/`PadRemoved`. The core assigns `PadSlot`s, and a slot stays stable for the session. |
| Android touch (`touch.rs`) | Pushes P1 key events and draws with `SDL_Render*` | `smw_core::input::touch` owns the layout, hit tests and hysteresis, and emits `Key` events for player 1's default keys (unchanged behavior). The backend forwards `Touch` and draws the core's overlay description. |
| Web touch (`web/touch.js`) | Page-side, dispatches DOM key events | Unchanged at first. It moves to the core module after phase 4 (decision 6). |
| Replay | `harness.rs` calls `SDL_PushEvent` and attaches SDL virtual joysticks | `ReplaySource: InputQueue` yields the frame's events in file order. Pads exist because the replay declares them, so no SDL virtual joysticks. `wait()` keeps `harness::waitEvent` semantics (next unconsumed event, which its own frame then skips). |
| Recording | An SDL event filter holds OS events and re-pushes them at frame start | `Recorder` wraps the live `InputQueue` and writes each event it hands to the core. Live and replay input then reach the core through the same path. |

Replay format compatibility:

- The version 1 line kinds (`down`/`up` key names, `jaxis`, `jbutton`, `jhat`, the `#@` directives, markers and checkpoints) stay byte-for-byte. Existing recordings, clips, `tools/replays` and `tools/checkpoint_fixtures` replay unchanged, and recordings made after the change still replay on the C++ harness.
- The recorder writes v1 lines for every event v1 can express. Touch records as keys. Pad hot-plug after frame 0 records as `jadd`/`jremove` lines (Rust only, decision 5), which the C++ harness cannot replay. `Text` events are still dropped.
- The frame-0 `0 jhat <dev> 0 0` convention keeps declaring the pad count for v1 files.

### Audio: SDL2_mixer, SDL3_mixer and the virtual mixer

The core's virtual mixer allocates the channel (16 channels, lowest free, `-1` when all are busy) and computes end times. It calls `onChannelFinished` and `musicfinished` at frame start. The backends only play sound:

| | `smw-sdl2` | `smw-sdl3` |
|---|---|---|
| Device | `Mix_OpenAudio(44100, S16, 2, 2048)` | `MIX_CreateMixerDevice` with the same spec |
| Effect on channel `c` | `Mix_PlayChannel(c, chunk, loops)` | 16 `MIX_Track`s, one per virtual channel: `MIX_SetTrackAudio` + `MIX_PlayTrack` with `MIX_PROP_PLAY_LOOPS_NUMBER` |
| Halt | `Mix_HaltChannel` | `MIX_StopTrack` |
| Music | `Mix_PlayMusic` / `Mix_HaltMusic` / `Mix_PauseMusic` | A 17th track with the same calls, and `MIX_PauseTrack`/`MIX_ResumeTrack` |
| Callbacks | none registered | `MIX_SetTrackStoppedCallback` not used |

The track API maps one-to-one onto virtual channels, so neither backend needs its own allocation logic.

Upstream's `sdl3` branch (6 commits on top of upstream master, tip `d1b58c9`, pushed 2026-10-05) changes game-visible sound behavior. `AudioSystem::playAsEffect` picks tracks round-robin (`m_next_free_sfxtrack_idx`), a play never fails, and the finish callbacks come from `MIX_SetTrackStoppedCallback`. If upstream merges that branch into master, the change reaches the port the same way any upstream gameplay change does. The virtual mixer's allocation policy changes in `smw-core`, behind the upstream-sync process, for both backends at once. The C++ reference harness then needs its virtual mixer ported to the new API before its goldens can be regenerated. The Rust SDL3 backend does not depend on upstream's branch, and neither blocks the other.

### SDL3 backend and platforms

- **Bindings.** The `sdl3` crate (`sdl3-sys` underneath) for video, events and gamepads. SDL3_mixer bindings are needed for audio. If no maintained crate covers them, a small `-sys` module generated with bindgen goes into `smw-sdl3`. SDL3_image is not needed, because the core decodes images.
- **Video.** One streaming texture, `SDL_UpdateTexture` and `SDL_RenderPresent`, with `SDL_SetRenderLogicalPresentation` for the 4:3 letterbox.
- **Settings paths** come from shared code, not from the backend, so both builds read the same `options.bin` and `controls.sdl2.bin`.
- **Desktop.** macOS, Linux and Windows build both backends in CI from phase 6.
- **Web.** It stays on `smw-sdl2` with Emscripten's SDL2 port (`-sUSE_SDL=2`, emsdk 5.0.2) until emsdk ships SDL3 and SDL3_mixer ports that the build can use. That availability is unverified. After phase 4 the web build uses SDL only for the canvas, audio and input. Emscripten glue (`emscripten_set_main_loop`, IDBFS, `relay_socket.js`) moves from `smw/main.rs` and `build.rs` into the app crate and `smw-sdl2`.
- **Android.** It stays on SDL2 first. SDL3's Java `SDLActivity` differs, so SDL3 on Android needs its own Gradle project variant. `android/build.sh` then builds the app crate's cdylib instead of the root package.
- **muOS.** It stays on `smw-sdl2`. The device's `libSDL2` carries the GPU driver, and muOS images ship no SDL3.

### Testing

- **Core without SDL.** `smw-headless` implements `Services` in memory. `cargo test -p smw-core` needs no SDL libraries or dummy drivers.
- **In-process replays.** A test helper runs a replay through `Game::frame` with `ReplaySource` and compares the dump and screenshots with the goldens. A fast subset (`start_classic` and the joystick and segment replays) runs in `cargo test`. The full sets keep running through `tools/parity.sh`, `parity_sweep.sh`, `editor_parity.sh` and `segment_check.py`, which call an `smw-replay` headless binary once it exists and the game binary until then.
- **Unit tests in the core.** Blitter paths compared with SDL2 reference images, `Keycode` name round trips against SDL2's table, virtual mixer timing against the duration table, and `InputEvent` to `outputControls` mapping.
- **Backend conformance.** Each backend runs a short suite under SDL's dummy drivers: event translation (key, pad, hat, hot-plug), `PadSlot` assignment, and the audio command sequence for a scripted session.
- **The web build** keeps `web_replay.mjs`, `web_clip_test.mjs`, `web_ui_test.mjs` and `web_netplay_test.mjs`.

## Alternatives considered

- **Rewrite on Bevy or another ECS.** This is the end state the repository name suggests. It would break frame parity at once and leave no oracle. A clean `smw-core` with `Game::frame` can later sit behind a Bevy front end without a rewrite.
- **Pass `&mut Ctx` to every function from the start.** This is correct but needs one huge diff across 92,750 lines that conflicts with every open branch. The three-step migration reaches the same end in reviewable PRs.
- **Select the backend at runtime.** SDL2 and SDL3 export the same symbol names, so both cannot load into one process without `dlopen` isolation. Build-time selection is simpler, and no platform needs both.
- **Keep SDL's blitter in each backend.** Each backend would need its own goldens, because SDL2, sdl2-compat and SDL3 differ on RLE and blend modes. The pure-Rust blitter removes that variable.
- **Get SDL3 through sdl2-compat only.** This already happens on Homebrew. It gives none of SDL3's APIs (gamepad, pen, properties) and keeps the compat-layer bugs.

## Risks and decisions

Risks:

| Risk | Mitigation |
|---|---|
| The pure-Rust blitter or image decoder differs from SDL in an edge case (palette conversion, color key on converted surfaces, clipping) | The game, sweep and editor screenshots, plus `tools/gfx_smoke` and `map_render_parity.sh`, gate phase 4. Ship it behind a switch (`SMW_BLIT=sdl`) until all of them match, then delete the switch. |
| Sound duration table disagrees with SDL2_mixer for some file | A test on `smw-sdl2` compares every file in `data/sfx` and `data/music`. |
| Moving files conflicts with open branches | One rename-only PR during a merge freeze. `git log --follow` and `merge.renames` handle later rebases. |
| `Game` initialization order changes RNG draws | The `R calls=` record in every dump catches it on frame 0. Step 2 PRs move one subsystem at a time. |
| Cross-platform float differences (armv7 Android, wasm, x86_64) | Trig already comes from CORE-MATH, and Rust never contracts to FMA. Phase 6 adds the hash comparison across platforms, including an aarch64 run under qemu-user for muOS and Android. |
| Agents keep adding `sdl2::sys` calls during the migration | A CI check counts files that use `sdl2::sys` outside the backends and fails if the count rises. |

Decisions (maintainer, 2026-10-05):

1. **Parity milestone (phase 7).** After phase 6 is green on every backend, and after the last upstream sync that the port intends to take as a C++-verified port. No date is set.
2. **The core owns all pixels.** Accepted. Per-backend goldens are not an option.
3. **Default backend per platform.** SDL3 on desktop. SDL2 on muOS, web and Android until their SDL3 ports are proven.
4. **Controller translation (`SMW_PAD_TRANSLATE`) becomes the default** on every platform at phase 7, because it changes desktop behavior. It stays opt-in until then.
5. **Hot-plug replay lines are adopted.** `<frame> jadd <dev>` and `<frame> jremove <dev>` (`REPLAY.md`) landed with the Android gamepad work (PR #20) and stay. They are written only when a pad connects or disconnects mid-session with `SMW_PAD_TRANSLATE=1`, so other recordings still replay on the C++ harness; `replay_compare.sh` refuses the ones that have them. No `#@ format=2` header is used. Text-input lines remain deferred.
6. **Web touch controls move into the core** after phase 4, once the core already draws the Android overlay. One layout and one hit test, and the web replay tests can then exercise touch.
7. **Netplay stays host-authoritative** through phase 7. Lockstep or rollback is decided once `Game` is a value (phase 5) and cross-platform determinism is CI-enforced (phase 6). Browser and native clients still cannot play together (the relay speaks WebSocket and native clients speak ENet); that is a separate transport question.
8. **`libz-sys` is replaced by `miniz_oxide`.** It can land at any time. The only compressed bytes written are checkpoint lines in recordings, and every reader, including the C++ harness, only inflates. This also removes the Emscripten zlib special case in `file_compressor.rs`.
9. **The `Ptr` object graph moves to arenas with typed handles** after step 3, under its own design document. Nothing in this document depends on it.
10. **The editors stay on SDL2 through phase 6** and follow the game to SDL3 in one PR afterwards. Their 13 sessions already gate the shared blitter in phase 4.

## Rollout

### Phases

Each phase is a series of PRs. No PR changes behavior, and each one passes every gate in the table below at its merge commit. Sizes count changed lines without pure renames.

| Phase | Work | PRs | Size | Gates beyond the standard set | Main risk |
|---|---|---|---|---|---|
| 0 | Cargo workspace: root package plus `relay` as a member, one `Cargo.lock` (PR #23) | 1 | ~250 | relay tests, `docker build`, web bundle | none found |
| 1 | Create `smw-globals`, `smw-netplay` (`relay` stops using `#[path]` includes), `smw-platform` (traits only). Move `src/` to `crates/smw-core` and the binaries to apps in one rename-only PR. | 4-5 | ~1,500 plus renames | relay image, `smw_server` interop scripts (`tools/ref/net_*`) | Branch conflicts: schedule a merge freeze |
| 2 | Context step 1: time, storage, audio output, presentation and the event pump behind `Services`, implemented in `smw-sdl2` | 6-8 | 300-1,500 each | Sound `S` records identical, web tests | The virtual mixer becomes always-on: its live behavior has to match what recordings already assume. When its table and SDL_mixer's decoded length disagree, the core may play on a channel SDL_mixer still considers busy, which cuts that sound audibly but changes no game state |
| 3 | Normalized input: `InputEvent`, `InputQueue`, `ReplaySource`, `Recorder`, `pad.rs` into the backend, touch layout into the core, no SDL virtual joysticks | 5-6 | 500-1,500 each | Recordings round trip (`SMW_LIVE_SCRIPT` dump equals replay dump), every file in `tools/segment_replays` and `checkpoint_fixtures` parses unchanged, Android touch smoke test | Event order inside a frame |
| 4 | Pure-Rust surfaces, blitter and image decoding. `smw-core` drops the `sdl2` dependency. The editors follow. | 6-8 | 500-2,000 each | Every screenshot, including the sweep and the editors; `gfx_smoke`; `map_render_parity.sh` | Blit edge cases |
| 5 | Context steps 2 and 3: the `Game` struct, then passing `&mut Game`. The game-state subsystems (`game_values`, `map`, `rm`, `objects`, `players`, `scores`, `rng`, `menus`, `net`) can start after phase 2 and run alongside phases 3-4. The `sfx`, `harness`, `pad` and gfx subsystems wait for phases 3 and 4, which rewrite those files. | 15-25 | 200-3,000 each | `R calls=` unchanged from frame 0 | Initialization order, aliasing |
| 6 | `smw-sdl3` on desktop. CI runs every replay on every backend and compares the hashes. Then Android SDL3, then web SDL3 when its ports exist. | 4-6 | ~3,000 total | Cross-backend equality (below) | SDL3_mixer bindings |
| 7 | Parity milestone: Rust goldens primary | 1-2 | data plus CI | Maintainer sign-off | None technical |

The standard gates for every PR are:

- `cargo test`, debug and release, on Linux, macOS and Windows.
- `tools/parity.sh`: every game replay MATCH.
- `parity_sweep.sh`: 290 MATCH.
- `editor_parity.sh`: 13/13.
- `segment_check.py` on `tools/segment_replays`.
- The web, Android and muOS builds.
- `web_clip_test.mjs` and `web_ui_test.mjs`.

### Parity policy timeline

- **Phases 0 to 6.** C++ goldens (`tools/golden`, `golden_sweep`, `editor_golden`) gate every PR, as today. `tools/golden_rust` holds only deliberate deviations. A PR that changes any golden is not a refactor and needs its own justification.
- **From phase 6: equality across backends and platforms.** Each CI job that runs replays writes a manifest: one line per replay with the SHA-256 of `dump.txt`, of each screenshot and of each saved file. The jobs are Linux, macOS and Windows on both backends, the headless runner, the web build in headless Chrome, and aarch64 under qemu-user. A final `determinism` job downloads every manifest and fails on any difference, naming the replay and the first differing artifact. These manifests must be identical to each other and also match the goldens.
- **Phase 7: the milestone.** One PR regenerates every golden from the Rust build at the milestone commit, into `tools/golden` (`RUST_GOLDEN=1 make_golden.sh` for all), and freezes the C++ set as `tools/golden_cpp`. CI keeps running `parity.sh` against `golden_cpp` as a report-only check, like the Linux C++ comparison today, with a checked-in list of replays expected to differ and the PR that made each differ. From then on, a game improvement may change goldens. That PR regenerates them and lists the replays that changed in its description. Cross-platform equality stays a hard gate.
- **After the milestone.** The C++ reference stays buildable (`CPP_REF_BRANCH`), so upstream gameplay changes can still be verified when they are ported.
