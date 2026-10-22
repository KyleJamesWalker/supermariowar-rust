# Backlog

Work we intend to do but have not started, newest ideas last in each section. `upstream/UPSTREAM_BACKLOG.md` tracks bugs in the C++ original. When an item starts, it moves to a design doc or a PR and leaves this list.

## Decisions needed

These items wait on a choice, not on work.

- `fSmoking` is never set, so shells and throw blocks never smoke. The original comments out every trigger. These were a superkick, and a kick speed of 7.5 for a shell or 9.0 for a throw block. Turning it on adds a mechanic rather than fixing a defect.
- Moving a powerup slider overwrites the built-in presets 5 to 16. This looks intended, because Restore Defaults resets the selected preset and the game saves every preset. Decide whether to lock the built-in presets.
- Until the match starts, a joiner's room menu shows the room's creation-time mode. It also applies the host's settings bytes to that stale mode. Gameplay is unaffected. The fix adds the mode ID (`u8`) and goal (`u16`, little-endian) to `NET_NOTICE_GAMEMODESETTINGS`. That needs lobby protocol 0.10, so old clients and new relays stop working together.
- Map loads cap fireball and rotodisc chains at 9, the level editor's limit. A hand-made map with a longer chain fails to load. Decide whether to keep the cap.
- At `cpudifficulty` 4, `cplayer_ai_think` passes a threshold of 100 with a scale of 100. This trips the `debug_assert` in `get_boolean_threshold`, so the debug `rollback_sweep` fails on `opt_gameplay`. In release, `get_integer(100) > 100` is never true, so that branch never fires at the hardest difficulty. A fix changes gameplay, and the CPU AI work sits in Parked.

## After merge

Follow-ups to the merged backlog PRs.

- Run #248's SDL2 audio reload fix under AddressSanitizer.
- Check by hand what the headless tests cannot reach. These are the live level editor's Shift and Ctrl (#256), and the shader error banner and window restore (#261). Also open the sound options menu with no world music packs (#271).

## Bugs

Possible bugs found while commenting the code in #232, and gaps the fix PRs left. Gameplay fixes change the replay goldens.

- Menus and world map:
  - Tour and world stage points on the scoreboard are never set, and its eye candy is never drawn.
- Maps:
  - A 1.5 or 1.6.0 map never resets `numdrawareas`, so it keeps the previous map's draw areas. The original does the same.
  - A local match on a damaged map still starts, on the empty fallback map. The map list's summary read covers only the header, so the menu cannot refuse or hide the map.
  - The readers do not check race goal and flag base positions, draw area rectangles or warp exit positions against the screen size. The fuzz test found no panic from them.
  - The readers check platform path speed only for being finite and within 65536. A platform that moves more than a screen width per tick could carry a player past the one-screen wrap. The editor allows at most 4.16 px per tick, and 3000 fuzz cases did not hit it.
- Netplay:
  - After a v2 match, Start on the scoreboard opens the stats page only for the player who pressed it. The match's GGRS session has ended by then, so the other players stay on the scoreboard and never see the stats. Send the step to the peers, or advance every peer together.

## Review findings

A full-codebase review on 2026-10-19, at the head of the GPU shader stack, confirmed these against the code. The items below are what the fix PRs left. None changes simulated gameplay, so none changes the replay goldens. Each item gives its file and an effort estimate (S, M, L).

### Next

- A rollback save still builds a fresh `MatchState` and re-boxes the players and objects, because GGRS's `GameStateCell::save` takes the state by value. Reusing the previous save's allocations needs a cell wrapper or a GGRS change. A load still clones the `MatchState` fields other than players and objects whole (`rollback.rs`, M).
- The snapshot cost numbers come only from an Apple-silicon Mac. Re-measure on the RG35XX with `rollback_snapshot_cost` (S).
- A missing sound no longer fails at boot, but no netplay v2 test covers a joiner that lacks a sound effect the match plays. No test reproduces the in-match notice overlay or a LEAVE during the start. The harness takes screenshots at simulation frame ends, so they cannot show the lag notice during a stall (`tools/net_v2_adversarial_test.py`, M).

### Later

- A room has no CPU slots, so an online match plays only its humans. Let the host fill empty slots with bots (`net_room_menu.rs`, lobby protocol, M).
- A couch player's room name is the client's lobby name with a number (`Kyle 2`). `NET_REQUEST_LOCAL_PLAYERS` already carries names; the room menu needs a field to type them (`net_room_menu.rs`, S).
- The SDL3 audio thread still allocates each packet `lewton` decodes (`smw-sdl3/audio.rs`, S).
- The rollback snapshot carries the full mixer state. Trim it with the snapshot work above (`rollback.rs`, S).
- The pause owner cannot resume after a disconnect. A disconnect ends the match today. If that changes, the pause needs a timeout or a handover (`rollback.rs`, S).
- A dropped or failed WebRTC connection is never retried, on native or web. After a network blip the only way out is to leave the match (`smw-peer`, M). The relay caps each signaling room at 4 peers, which assumes nobody rejoins. Revisit the cap with reconnects.
- Switching the shader while fullscreen forgets the earlier windowed size (`smw-sdl2/video.rs`, S).
- The netplay v2 `ticks` count includes stalled session frames. It sets "match over after N ticks" and the `frames_ahead` skip cadence, so both count stalls (`net_v2.rs`, S).
- At CI size (48 cases per map, 30 ticks) the damaged-map fuzz test does not find the spawn-area panic without its direct check. Run a long fuzz pass nightly. Also add a byte mutator that knows the 1.8 section offsets. Then damage reaches the spawn areas, draw areas and warp exits, which `save_map` recomputes (`damaged_maps_that_load_play`, M).
- Unconfirmed: whether anything can move the `String` that `MI_TextField::szOutValue` points to, and whether the world menu catches a world file that fails to parse. Also unconfirmed: whether a host-sent tour-stop line or menu skin can panic the joiner's parser, and how `smw-peer` behaves under TURN after a dropped connection.

## Performance

A read-only profile on 2026-10-20 of a headless 4-CPU replay on an Apple-silicon Mac. None of these changes simulated gameplay unless the item says so. Device numbers are scaled from `design/NETPLAY_V2.md` unless an item says it measured them.

Two RG35XX handhelds measured the performance PRs (#255, #262, #263, #265, #284). Menus went from 9.3 to 3.2 ms of update and draw. A local Frenzy match went from 4.8 to 2.7 ms, on another map. A 3-player Greed netplay match on Log Balancing ran 5.4 to 5.9 ms on the host. Its GGRS frames took 1.1 to 1.3 ms, with no frame over budget and no desync. Present stays at 3.8 to 4.0 ms, now more than half of a match frame.

- Every loaded image blits through the per-pixel alpha blend. `convert_to_screen` turns the color key into alpha 0 and sets no `color_key`, so the opaque 640x480 map layers blend too. The backmap blit takes 78% of gameplay samples: 255 us blended against 20 us copied. Mark a surface opaque when every pixel has alpha 255, clear the mark on any non-opaque write, and copy opaque sources. Output stays bit-identical (`soft_image.rs`, `soft_surface.rs`, M). #284 adds the alpha 0 and 255 shortcut inside the blend loop, about 1.8x on blended pixels.
- Check whether keyed sprites can set `color_key` at load and take the key loop (66 us against 90 us for 300 sprites). This holds only if the alpha-0 pixels are exactly the key pixels. Confirm with `tools/blit_unverified.sh` (S).
- `SMW_PERF`'s present column adds the texture upload to the vsync wait, so the device's 5 ms cannot be split. Time `display_list::render`, `SDL_UpdateTexture` and `SDL_RenderPresent` apart, and log the renderer's texture formats once. A GLES2 driver without BGRA support converts the 1.2 MB frame on the CPU every present (`main.rs`, `video.rs`, S). Do this before more device tuning.
- A Very Hard CPU rebuilds its navigation graph every 8 ticks on maps with platforms, with 300 or more allocations per build, and `route` allocates its scratch buffers per call. Reuse the buffers and store the edges as two flat vectors (`ai_nav.rs`, S). One graph shared by all CPUs per tick changes gameplay. Batch it with other AI work.
- On a 60 Hz display, interpolation records each tick and replays it once per present, for no visual gain. Rasterize directly when the refresh rate is at or below the tick rate (`main.rs`, `gs_gameplay.rs`, S).
- Small per-frame allocations: `draw_platforms` clones two vectors six times a frame (`map.rs`), `touch::overlay` clones the cutouts and builds a new quad vector every present (`touch.rs`), and `display_list::begin` regrows its command vector every tick (S each).
- The map browser stalls on the RG35XX while it loads each map's preview (`mi_map_browser.rs`). Two 5 s reports averaged 78 ms and 113 ms per frame, with single frames up to 1.5 s. Cache the previews, or load them off the frame (M).
- Startup on the device is unmeasured: about 230 PNGs decode through `flate2`'s Rust backend, and a missing `maps/cache/mapsummary.txt` loads every map. Log per-phase startup times first, then consider `zlib-rs` and shipping the cache (S).

## Parked

- Upgrade `webrtc` past 0.17.2: 0.21 turns methods `smw-peer` awaits into synchronous calls, so it needs a port (57 compile errors on the bump). Until then, `.github/dependabot.yml` ignores its minor and major updates. Remove that entry when this starts. The upgrade also drops the second `sha1` (0.10) that `dtls` 0.17 pulls in.
- CPU AI, from the Risks and open questions in `design/CPU_AI.md`; each idea tried so far was mixed or neutral in the benchmark:
  - Riding a moving platform along its path: the navigation graph sees a platform only where it is. Planning rides along the whole path was mixed in the benchmark.
  - Springs and breakable blocks in the navigation graph.
  - Note-block super jumps: they always rise to full height, so CPUs aiming at a lower ledge overshoot. Planning them as longer jump links, and letting go of jump above a lower target, were both mixed in the benchmark.
  - The boss minigames: the AI has no notion of the boss, so CPUs play them as deathmatch and hunt the other players. The final hit on the boss wins. A CPU should mostly attack the boss, and stomp a rival only when that is cheap.
