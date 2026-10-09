# Netplay findings, 2026-10-09

The `net_game_coins` and `net_game_classic` scenarios fail now and then because each client rolls its own random game mode settings when a net game starts. Native `net_game` between a C++ and a Rust client fails because the two load a match at different speeds and upstream has no start barrier. Neither failure is a protocol bug. Both fixes below change game code and wait for review.

## 1. Each client rolls its own game mode settings

### Symptom
- **`net_game_coins` (2 of 22 browser runs failed):** the joiner's `place` records read `out=gone`. From the first gameplay frame, container 1 holds a different number of objects on each client (host 10, joiner 2).
- **`net_game_classic` (2 of 10 runs failed):** `kill: MISMATCH`, with result `r1` (Normal) on one client and `r3` (NonKill) on the other. In one run the joiner recorded 12 deaths and the host 7.

### Cause
`MenuState::enter_gameplay` (`crates/smw-core/src/smw/gs_menu.rs`) calls `set_random_game_mode_settings` for `MatchType::NetGame`, as upstream does. Upstream's `GSMenu.cpp` marks the same call `// TODO: set from network`.

So every client throws away the settings the host sent at room creation (`NET_NOTICE_GAMEMODESETTINGS`) and rolls its own from the shared random generator. The sync reseeds the generator on both clients. The settings still match only if both clients draw the same number of times between that reseed and `enter_gameplay`, and they sometimes do not.

When the settings differ, the clients play by different rules:
- **Coins:** `coins.quantity` sets how many coins `CGM_Coins::init` makes. A different count shifts every network ID that setup assigns, so the joiner cannot find the coins the host moves.
- **Classic:** `classic.style` and `classic.scoring` decide whether a death costs a life or only a shield (`NonKill`), and whether only pushes score.

### Evidence
These come from temporary logs that were not committed.

| Run | Sent `coins.quantity` | Host at setup | Joiner at setup | Same sync seed? | Same random value at first gameplay frame? |
|---|---|---|---|---|---|
| 10 passing coins runs | 1 | 7, 4, 3, 2, 1, 2, 3, 8, 8, 5 | Same as host | Yes | Yes (e.g. 2093675983 on both) |
| Failing coins run | 1 | 10 | 6 | Yes (4315) | No (3138228400 vs 3386718563) |

- **Coin quantity:** the host always sent 1, but the quantity at setup was random from run to run.
- **Classic kill mismatches:** classic returns `NonKill` only when `classic.style` is Shield, and only one client returned it.

### Fix options
- **A (recommended).** In a game the host decides (both clients Rust, `NET_G2P_HOST_DECIDES_RANDOM`):
  - The host rolls the random settings once, when it sends the sync.
  - It sends them to the joiners before `NET_G2E_GAME_START`.
  - Each Rust client applies the host's settings in `enter_gameplay` instead of rolling.
  - A C++ peer on either side keeps the upstream behaviour.
- **B.** In Rust-to-Rust net games, stop randomizing and play with the host's room settings. This resolves the upstream TODO and is simpler, but net games then play with the menu settings rather than random ones, which changes behaviour.
- **C.** Keep rolling locally and make both clients draw the same number of times. This breaks again whenever menu code draws one more random number, so it is not recommended.

Until one of these lands, `net_game_coins` and `net_game_classic` stay out of the CI netplay list.

## 2. A C++ host and a Rust joiner start the match about 0.7 s apart

### Symptom
`tools/ref/net_game_interop.sh` reports `net_game` NOT SYNCED for cpp-rust and rust-cpp, for example: `player 0: offset +39, within 2px 529/578, worst 541.0px`.
- **Base branch:** fails the same way, 3 of 3 reruns.
- **cpp-cpp and rust-rust:** synced in every scenario.

### First divergence
In cpp-rust, the Rust joiner draws the host's player at the origin (`fx=0 fy=0` in its dump) for its first 44 gameplay frames. A C++ joiner of a C++ host, and a Rust joiner of a Rust host, do so for 1 frame.

Tick logs on the Rust joiner:

| Pairing | `NET_G2E_GAME_START` | First gameplay frame | First `NET_G2P_GAME_STATE` |
|---|---|---|---|
| cpp-rust | 8197 | 8723 | 9452 (729 ms later) |
| rust-rust | 8624 | 9143 | 9158 (15 ms later) |

### Cause
Upstream behaviour, exposed by the port being faster. After `NET_G2E_GAME_START`, each client loads the match (`StartGame`: skins, sounds, map) and then starts gameplay on its own, because there is no start barrier.

The C++ client takes about 0.7 s longer to load, so whichever client is Rust starts about 44 frames earlier:
- Until the first game state arrives, its view of the other player is the zeroed `latest_playerdata`.
- After that, its frame counter stays about 40 frames ahead of the C++ client's, which is the offset `net_game_compare.py` finds.
- The 44 origin frames are 7.6 % of the track, which drops "within 2 px" below the 95 % threshold.

The harness records what happened, and the protocol is compatible.

### Fix options
- **A (recommended, tool only).** `net_game_compare.py` compares each remote player's track from the first game state its client received. It reports how many frames it skipped, so it measures sync rather than load-time differences.
- **B (game code, Rust-to-Rust only).** Add a start barrier: each client reports when it has loaded, and the host starts gameplay for everyone together. This removes the skew between Rust clients, but a C++ peer keeps the upstream behaviour.
- **C (game code).** Until the first game state arrives, a joiner keeps remote players at their spawn positions instead of the origin. This is cosmetic, since the players are still spawning during that window.
