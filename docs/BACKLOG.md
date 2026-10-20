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

## Parked

- Upgrade `webrtc` past 0.17.2: 0.21 turns methods `smw-peer` awaits into synchronous calls, so it needs a port (57 compile errors on the bump). `.github/dependabot.yml` ignores its minor and major updates until then; remove that entry when this starts. The upgrade also drops the second `sha1` (0.10) that `dtls` 0.17 pulls in.
- CPU AI, from the Risks and open questions in `design/CPU_AI.md`; each idea tried so far was mixed or neutral in the benchmark:
  - Riding a moving platform along its path: the navigation graph sees a platform only where it is. Planning rides along the whole path was mixed in the benchmark.
  - Springs and breakable blocks in the navigation graph.
  - Note-block super jumps: they always rise to full height, so CPUs aiming at a lower ledge overshoot. Planning them as longer jump links, and letting go of jump above a lower target, were both mixed in the benchmark.
