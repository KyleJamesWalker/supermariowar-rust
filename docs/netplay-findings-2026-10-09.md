# Netplay findings, 2026-10-09

The `net_game_coins` and `net_game_classic` scenarios failed now and then because each client rolled its own random game mode settings when a net game started. #59 fixes this with option A below. Native `net_game` between a C++ and a Rust client fails because the two load a match at different speeds and upstream has no start barrier. Native C++ netplay is no longer supported, so this second finding only matters between Rust clients. A start barrier is the open follow-up. A joiner could also hang in the Servers menu after connecting, because of a timestamp race in the lobby connect handshake inherited from upstream. That race is fixed.

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
- **A (chosen, #59).** The host rolls the random settings once, at the sync, and sends them to the joiners before `NET_G2P_SYNC`. When the match starts, every client applies the host's settings and reseeds the shared generator with the sync's seed. The user approved this without a fallback for C++ peers.
- **B.** Stop randomizing in net games and play with the host's room settings. Simpler, but net games then play with the menu settings rather than random ones, which changes behaviour.
- **C.** Keep rolling locally and make both clients draw the same number of times. This breaks again whenever menu code draws one more random number.

With #59, `net_game_coins` and `net_game_classic` passed 22 of 22 consecutive browser runs and join the CI netplay list.

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
Native C++ netplay is no longer supported, so mixed pairings no longer gate anything. The same skew can still happen between two Rust clients on machines that load at different speeds.
- **Recommended follow-up (game code, for the user to decide): a start barrier for Rust-to-Rust games.** Each client reports when it has loaded the match, and the host starts gameplay for everyone together. This removes the skew and the origin frames.
- **Tool only.** `net_game_compare.py` compares each remote player's track from the first game state its client received, and reports the frames it skipped. This measures sync rather than load time, but it changes nothing in the game.
- **Cosmetic.** Until the first game state arrives, a joiner keeps remote players at their spawn positions instead of the origin.

## 3. A joiner can wait forever in the Servers menu after connecting

### Symptom
In 1 of 24 `net_game_stomp` browser runs, the joiner connected to the lobby but stayed on the Servers menu ("Connecting...") for the rest of the run. The lobby logged the connection and the skin, and the host went on to create the room.

### Cause
Inherited from upstream. The menu leaves the Servers screen only when the last message sent is the skin and the last message received is `NET_RESPONSE_CONNECT_OK`, with `lastSent.timestamp >= lastRecv.timestamp` (`crates/smw-core/src/smw/gs_menu.rs`).

`NetClient::on_receive` handles `NET_RESPONSE_CONNECT_OK` in this order (`crates/smw-core/src/smw/net.rs`):
1. It sends the skin, which sets `lastSent.timestamp`.
2. After the message is handled, it sets `lastRecv.timestamp`.

Upstream `net.cpp` uses the same order: `sendSkinChange()` inside the `NET_RESPONSE_CONNECT_OK` case, then `setAsLastReceivedMessage` after the switch.

Both timestamps are `SDL_GetTicks()` milliseconds. If the tick advances between the two steps, the receive is newer than the send, and the condition never holds again. Compressing the skin file between the two steps makes this more likely.

### Fix options
- **A (recommended).** Record the received `NET_RESPONSE_CONNECT_OK` before sending the skin, so the send is never older than the receive. This is a one-line reorder for this one message type. It keeps the menu's ordering check, which guards against stale replies.
- **B.** Relax the check to `lastSent.timestamp + 1 >= lastRecv.timestamp`, or drop the timestamp comparison for this transition. This is simpler, but it weakens a check that other transitions rely on, so it is not recommended.

Option A is done: the client now records the reply before it sends the skin. `tools/ref/net_connect_stress.sh` checks it by connecting 20 times on a clock that advances on every read, so the two timestamps always differ. The check fails on the old order and runs in CI.
