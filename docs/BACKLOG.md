# Backlog

Work we intend to do but have not started, newest ideas last in each section. Bugs in the C++ original are tracked separately in `upstream/UPSTREAM_BACKLOG.md`. When an item starts, it moves to a design doc or a PR and leaves this list.

## Maintenance

- `soft_surface::color_key_with_blend_matches_sdl` aborted once on a misaligned SDL surface pointer under a full workspace test run; it passed 40 of 40 runs alone.

## Bugs

Possible bugs found while commenting the code in #232, not yet confirmed. Gameplay fixes change the replay goldens.

- Memory safety:
  - `smw-sdl2/audio.rs`: loading a track under an id already loaded drops the buffer the old `Mix_Music` may still stream from, and leaks the old sound.
  - `smw-sdl2/input.rs` `to_sdl` transmutes any `i32` into `SDL_Scancode`, which lacks some of SDL's values.
  - `MI_TextField::update_cursor` can split a multi-byte character with `String::from_utf8_unchecked`.
- Gameplay:
  - `gs_gameplay.rs` assigns the `musicfinished` hook to itself, so background music never resumes after a play-once track.
  - `handle_obj2obj_collisions` returns when one object dies, so the rest of that tick's pairs never collide.
  - `clean_dead_players` skips the player that moves into a removed slot.
  - `fSmoking` is never set, so smoking shells and throw blocks never happen.
  - The super fireball reads `velx` before setting it, so it always uses the right-facing row.
  - Two frozen shells that collide can each `die()` twice.
  - Shell-breakable blocks break for a slow throw box, and a player killed by `hittop_player` is still launched upward.
  - The sledge brother's repeat limits test the wrong action numbers.
  - Race goals are placed before `quantity` is set, so their spacing uses 0.
  - P-wings can be picked up while still rising out of the block.
  - `check_secret(2)` can spawn SECRET3 more than once.
  - Card collection reads `quantity` and `banktime` from the menu settings, ignoring tour, world and netplay values.
  - Survival panics when all three enemy weights are 0.
  - The fire mini-boss platform's Y is set from the screen width.
  - Block crushes test objects with the player's size.
- Menus and world map:
  - The stats menu builds 18 rows but `ROWS` is 16, so the two "(won)" rows never show.
  - Random mode settings roll Shy Guy Tag's free time from the tag-on-stomp field.
  - An AI using a door key on the world map indexes one past the team's last member.
  - Moving a powerup slider overwrites the built-in presets 5 to 16.
  - `world.rs`: `is_touching_door` compares the column with the height, and `get_next_interesting_move` uses the start tile's warp.
  - Panics: a slider with one item, `random_value` with no random-eligible items, `UI_Menu::mouse_click` with no focus, and a world music pack without a Grass song.
  - Tour and world stage points on the scoreboard are never set, and its eye candy is never drawn.
  - Leftover debug `println!` calls in `gs_menu.rs` and the netplay menus.
- Maps:
  - The 1.6 reader converts spawn-area sizes twice with `fix_spawnareas`; the 1.7 reader ignores the spawn-area overflow check.
  - Malformed map files can panic on indexes read straight from the file.
  - `draw_map_hazard` draws some hazard parts to the screen instead of the target surface.
- Editors:
  - The editor harness is always active, so the live level editor never sees real Shift or Ctrl.
  - Level editor: merge-mode paste updates the source cell, and tile-mode dragging checks `(y, y)`.
  - World editor: the edit-mode labels do not match the modes, Insert reloads the world and loses unsaved edits, and resizing deletes every warp.
- Netplay:
  - A refused lobby connection (server full) is `mem::forget`-ed, not disconnected.
  - Server info always reports 0 players.
  - Sizes and multi-byte fields go over the wire in native byte order.
  - In the web build, dropping a `PeerSocket` leaves the browser connection open.
  - The relay's `/signal` connections count against no limit.
  - A joiner's room menu shows the mode the room was created with until the match starts.
  - Dead code left from v1: unused messages in `protocol_definitions.rs`, `NetworkState` constants, unused server fields and empty server functions.
- `gfx_show_catched_error` shows the original project's issues URL in the crash text.

## Review findings

A full-codebase review on 2026-10-19, at the head of the GPU shader stack, confirmed these against the code. None changes simulated gameplay, so none changes the replay goldens. Each item gives its file and an effort estimate (S, M, L).

### Fix now

- Netplay v2 joiners accept `MSG_START_PART`, `MSG_READY`, `MSG_GO` and `MSG_SLOT` from any peer, and parts are keyed by index only, so a second joiner can corrupt the match start. Duplicate slot claims map two GGRS handles to one player. Take the start only from the host's peer and reject duplicate slots (`net_v2.rs`, S).
- Restoring a match start panics when the host names a music or sound file the joiner lacks: `checkpoint.rs` turns the load error into a panic. Fall back to an empty track or sound and log it (S).
- A damaged map now crashes joiners remotely: the host's map goes through `load_map`, which has no `catch_runtime_error`, short reads panic in `file_io.rs`, and the 1.8 reader indexes with file values. Catch the load error, bounds-check `read_switchable_blocks`, and cap the item and hazard counts (`map.rs`, `map_reader18xx.rs`, M).
- The lobby lets any room member change the map or mode settings (`host_changes_map`, `host_changes_game_mode_settings`), and joins a room that has already started (`player_joins_room` ignores `visible`). Check the sender is the host, and refuse joins while a room is hidden (`server.rs`, S).
- The client indexes `skinids` with the server's `remotePlayerNumber` unchecked, so a value of 4 or more panics. Reject it in `handle_room_changed_message` (`net.rs`, S).
- WebRTC callbacks in `smw-peer` unwrap their mutex, so one panicking callback poisons it and the game thread panics on its next poll. Recover the poisoned lock, as `smw-sdl2/input.rs` does (`native.rs`, S).
- `smw_relay` spawns a thread per TCP connection before any limit applies, an idle socket holds one for 5 s, and signaling accepts the default 64 MiB messages. Cap accepted connections, limit signaling messages to a few KiB, and cap peers per room (`websocket.rs`, `signal.rs`, M).
- `rollback.rs` reads `SMW_V2_STATE_LOG` from the environment on every save. Read it once (S).

### Next

- The rollback snapshot deep-copies the map's 1024x1024 animated-tile surface (4 MiB), each platform's sprite surfaces and every game mode, every tick. That fits the 3 ms snapshot measured on the RG35XX, against a 0.3 ms tick. Keep surfaces out of the snapshot, save only the current mode, and reuse the previous snapshot's allocations (`rollback.rs`, M).
- The desync checksum hashes the harness dump, which counts objects but does not hash their state, so an object or RNG divergence shows only once it moves a player. Hash object positions, states and types, platform positions, the RNG and switch states (`rollback.rs`, S to M).
- Desyncs and disconnects only print to the log: a dropped player stands still until everyone leaves, and the start timeout does not name the missing peer. Show a notice, and end the match or hand the slot to a CPU (`net_v2.rs`, M).
- Netplay v2 tests run identical peers only. Add a joiner with other sound settings or packs, a host with a truncated map, a third peer sending start parts, and a mid-match disconnect (`tools/net_v2_test.py`, M).
- A shader preset that fails to load leaves only a stderr line. Show the error in Graphics Options, and hide presets that fail a test load, such as those that need OpenGL 4.3 on macOS (S).
- The web `PeerSocket` allocates a 64 KiB buffer on every receive, for both channels, every frame (`smw-peer/src/web.rs`, S).
- The display list copies the whole 640x480 map surface when animated tiles redraw, about 1.2 MB per animation step, and builds a `HashMap` per presented frame (`display_list.rs`, S).
- The lobby logs seven lines per player on every room update and one per room-list request. Put them behind a verbose flag (`server.rs`, `room.rs`, S).
- The lobby's network layer is a `static mut` singleton, so two lobbies in one process clash. Make it a field of `SMWServer` (`server.rs`, S).

### Later

- The software blitter matches the blend, color-key and modulate state for every pixel. Specialize the loop per state; results must stay bit-identical (`soft_surface.rs`, M).
- No CI job tests the shader presenters. A Linux job on Mesa llvmpipe and xvfb could compare `SMW_SHADER_SHOT` captures for CRT Easymode and one librashader preset (M).
- `smw-core` silences every clippy lint. Turn `clippy::all` on for the port's newer modules (rollback, netplay v2, checkpoint, harness, video settings, stats) and gate on it (S).
- Many accessors return `&'static mut` from `&self`, and `rollback::load` frees the boxes they point into. Nothing holds one across a tick today, but nothing checks it. Add a debug-only guard in `load` and note the rule in `CODE_CONVENTIONS.md` (S).
- Only one local player per peer can play online (`net_v2.rs` hard-codes `KeySlot::Local(0)`); couch players in an online match need a room slot model (M).
- The overlay uploads its vertex buffer once per quad. Batch the quads (`shader_overlay.rs`, S).
- Fuzz the `bincode` decode of GGRS packets from peers (`smw-peer`, S), and test the relay's connection and message limits (S).
- Unconfirmed: whether anything can move the `String` that `MI_TextField::szOutValue` points to, whether the world menu catches a world file that fails to parse, and whether `smw-peer` should retry a failed ICE handshake.

## Parked

- Upgrade `webrtc` past 0.17.2: 0.21 turns methods `smw-peer` awaits into synchronous calls, so it needs a port (57 compile errors on the bump). `.github/dependabot.yml` ignores its minor and major updates until then; remove that entry when this starts. The upgrade also drops the second `sha1` (0.10) that `dtls` 0.17 pulls in.
- CPU AI, from the Risks and open questions in `design/CPU_AI.md`; each idea tried so far was mixed or neutral in the benchmark:
  - Riding a moving platform along its path: the navigation graph sees a platform only where it is. Planning rides along the whole path was mixed in the benchmark.
  - Springs and breakable blocks in the navigation graph.
  - Note-block super jumps: they always rise to full height, so CPUs aiming at a lower ledge overshoot. Planning them as longer jump links, and letting go of jump above a lower target, were both mixed in the benchmark.
