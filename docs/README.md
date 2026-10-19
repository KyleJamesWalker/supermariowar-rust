# Docs

Start with `guides/` for how things work today. `design/` holds design docs with a status line each, `upstream/` the work aimed at the C++ original, and `archive/` dated reports kept for history.

- `BACKLOG.md`: work we intend to do and have not started.

## Guides

- `guides/GAME_MANUAL.md`: the player's manual (menus, controls, game modes, blocks and items). The web build renders it as `manual.html`.
- `guides/DEVIATIONS.md`: where the port behaves differently from the C++ on purpose, including netplay.
- `guides/CODE_CONVENTIONS.md`: crate layout, naming, numeric semantics, global state, handles and pointers.
- `guides/REPLAY.md`: replays, recordings, the dump format, goldens and the parity tools.
- `guides/EDITOR_REPLAY.md`: scripted sessions for the level and world editors.
- `guides/REPLAY_VIDEO.md`: rendering a replay to MP4, alone or side by side.
- `guides/RELAY.md`: `smw_relay`, the lobby and WebSocket relay for the browser build.
- `guides/RELEASING.md`: publishing a release and what it builds.

## Design

| Doc | Status |
|---|---|
| `design/ARCHITECTURE_V2.md`: platform-free core, SDL2 and SDL3 backends | done |
| `design/HANDLES.md`: typed handles replace the `Ptr` object graph | done |
| `design/PADS_AND_RECORDER.md`: pads and the recorder move into the backend | done |
| `design/CPU_AI.md`: CPU AI overhaul | in progress |
| `design/NETPLAY_V2.md`: rollback over WebRTC | done |

## Upstream

- `upstream/UPSTREAM_BACKLOG.md`: bugs found in the C++ and fixes worth offering upstream.
- `upstream/UPSTREAM_SYNC.md`: the pinned upstream commit and how to sync.
- `upstream/prs/`: drafts and evidence for upstream PRs and issues.
- `upstream/sdl2-compat-rle.md`, `upstream/sdl2-compat-convert-blend.md`: draft sdl2-compat issues, not filed.

## Archive

- `archive/PORTING_PROGRESS.md`: the module checklist from the C++ translation.
- `archive/MORNING_REPORT.md`: status report of 2026-10-06.
- `archive/netplay-findings-2026-10-09.md`: netplay desync findings, all fixed.
