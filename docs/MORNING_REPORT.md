# Morning report: platforms, workspace and upstream (2026-10-06)

**Bottom line:** 24 PRs merged since yesterday evening; every one that runs CI was green, and docs-only PRs run none. Phase 0 and phase 1 of `ARCHITECTURE_V2.md` are on `main`, and phase 2 has started: the game library now lives in `crates/smw-core`, and the clock and storage go through a services context. The port and the C++ reference are synced to upstream `c7056790`, and no golden changed. Windows, macOS and Android each got their packaging fix. Upstream PR 2 is ready on the fork for you to review; nothing went upstream.

## Decisions for you

1. **Upstream PR 2** (`fix/uninitialized-reads` on the fork, head `fa91633d`, six fixes plus a comment fix). Draft and evidence: `docs/upstream-prs/PR2-uninitialized-reads.md`. Open it upstream when you're happy with it. With the fixes, Linux x86_64, Linux arm64 and macOS play all 45 games identically (CI run 37413292776 on `c7056790`).
2. **Unique AI object IDs.** Fix 6 starts every object's network ID at 0, as macOS already behaves, so once a CPU player ignores one object it ignores all of them. Unique IDs would fix that but change CPU play; the draft offers it as a follow-up.
3. **Upstream issue for the Linux/macOS divergence**: the draft is `docs/upstream-prs/ISSUE-linux-macos-divergence.md`, not filed. Upstream PR 2 fixes the cause, so the issue may be unnecessary.
4. **macOS minimum version.** Releases now declare macOS 14 (Apple silicon) and 15 (Intel), set by the Homebrew libraries they bundle. Building SDL from source would bring it down to macOS 11, for about 3-5 CI minutes per architecture, but it swaps the graphics and audio libraries and needs its own parity run. #21 has the write-up.

## What landed

| PR | Change |
|---|---|
| #17, #18, #19 | Dependabot: `toml` 1.1, `sdl2` 0.38, 12 GitHub Actions bumps |
| #20 | Android: no gamepad crash at launch, pad hot-plug, on-screen touch controls (D-pad or floating stick), `log.txt` |
| #21 | macOS: `Info.plist` declares the real minimum per architecture |
| #22 | Windows: the game and editors open no console window; Windows CI adds editor parity (13/13) and the map sweep (290/290) |
| #23 | Phase 0: Cargo workspace, relay folded in, one `Cargo.lock` |
| #24, #25 | `ARCHITECTURE_V2.md` (you approved) and decision 5, hot-plug replay lines |
| #26 | CI and relay runs on PRs and `main` cancel superseded runs; tag runs never cancel |
| #27 | Android: every CI and release APK is signed with one project debug key from repository secrets; pixel-perfect launcher icons |
| #28 | Why the C++ reference plays differently on Linux and macOS: three uninitialized members (M9 in `UPSTREAM_BACKLOG.md`) |
| #29, #30, #31, #39 | Phase 1: `smw-globals`, `smw-netplay` and `smw-platform` crates, then `src/` moved to `crates/smw-core` and each binary to `apps/` |
| #33 | Web: the data submodule's `.git` pointer stays out of `smw.data` |
| #34, #36 | Upstream PR 2 draft and evidence, measured in CI on `c7056790` |
| #35 | Web: the clip test paces itself by game frames; the CI retry is gone |
| #37 | Port synced to upstream `c7056790` (upstream PR 480: M1, M4, M5 ported; harness patches regenerated) |
| #38 | README brought up to date with `main` |
| #40, #41 | Phase 2: clock and storage behind `smw_core::services`, new `smw-sdl2` crate |

The C++ reference merged its own sync PR 6 into `harness-latest` (`d657f2dc`), and `~/work/supermariowar-cpp-reference/build` is rebuilt from it.

## Verification

Every merged code PR passed full CI: tests, parity on macOS, Linux and Windows, the 290-map sweep, editor parity, the web build and the packages. Main at `c2710b3`, the first commit with the new layout, passed CI, web and relay.

| Check | Result |
|---|---|
| Golden regeneration from the synced C++ reference | 45 game, 290 sweep and 13 editor goldens byte-identical |
| Phase 1 and 2 PRs, local gates | parity 48/48, sweep 290/290, editor 13/13, `segment_check` 27/27, `R calls=` unchanged |
| Web clip test | 10/10 local runs, first CI run green without the retry |
| Android APK from #27 | the keystore secret was decoded; no throwaway-key warning |

## Known issues and follow-ups

- **Install the new APK once by hand.** Earlier APKs used throwaway keys, so the first install of a fixed-key APK on your Pixel needs an uninstall first. Later updates install over it.
- **C++ reference on Windows** fails to build: CORE-MATH's `__int128` isn't supported by MSVC. This predates tonight; the replay harness only runs on macOS and Linux.
- **Windows editor screenshots** are report-only, as on Linux. One shot in `leveledit/platforms_save` differs by 104 pixels; dumps and saved files must still match.
- **No editor session exercises M4 or M5.** A new session that saves a minigame stage and deletes a stage with a vehicle would cover them.
- **Upstream PR links in `UPSTREAM_BACKLOG.md`**: six markdown links to upstream PR 480 predate tonight. Links in files don't post on the upstream timeline, but plain text would match the rest of the docs.
- **Web netplay frenzy test** still fails about 1 run in 5 locally. It isn't in CI.
- **`investigate/pr2-c7056790`** stays on the C++ reference repo so CI run 37413292776 stays linked. Delete it when PR 2 is done.

## Next in the architecture

Phase 2 has three steps left, in this order:

1. **Audio output**, the largest and riskiest. The virtual mixer must decide sound state in every run, not only seeded ones, and SDL_mixer becomes output-only. Blocker: channel end times still come from SDL_mixer's decoded lengths, so they need a checked-in duration table and a test comparing it with SDL2_mixer for every file in `data/`. Gate it on the dumps' sound records and a scripted live session, because unrecorded play can shift `isPlaying()` timing.
2. **Presentation**: window and texture calls move into `smw-sdl2`. Low risk, because screenshots are taken before the flip. The Android touch overlay waits for phase 3.
3. **Event pump**: the remaining `SDL_PollEvent` sites, about 55 of them in the editors, move behind the backend together with phase 3's input events. The replay and editor goldens catch any change in event order.
