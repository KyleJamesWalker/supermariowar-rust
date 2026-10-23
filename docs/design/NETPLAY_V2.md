# Netplay v2: rollback over WebRTC

Status: done, released in v0.2.0 (Rollout, steps 1 to 6). Decision: rollback, because the RG35XX measurement fits the tick budget (Rollout, steps 1 and 2). Background and Problem describe netplay v1, which step 5 removed.

## Summary

Replace host-authoritative netplay with input exchange over one WebRTC transport for every build. Each client runs the same deterministic match from all players' inputs and hides latency with rollback. This removes the per-event sync code and lets browser and native players share a game. It also retires ENet, the WebSocket relay's game traffic and our NAT punching.

## Background

- **Model.** The host decides random outcomes, deaths, scores and mode settings. Joiners replay those decisions a few frames later (`docs/guides/DEVIATIONS.md`, "Netplay deviations").
- **Transport.** Native clients speak ENet over UDP to `smw_server` and to the game host. Browsers tunnel the same connections over WebSocket through `smw_relay` (`docs/guides/RELAY.md`).
- **Prerequisites now met.** `Game::clone_match` copies a running match (`HANDLES.md`). CI gates cross-platform determinism across native, web, SDL3 and aarch64 builds (`ARCHITECTURE_V2.md`, phases 6 and 7).

## Problem

- Every random event needs its own sync rule. The deviation list covers block bounces, respawn fallbacks after 90 frames, hazards waiting for the host, and network IDs for objects made in events. Each new feature with randomness adds another rule.
- Joiners see host-decided events late.
- Browser and native clients cannot play together (`docs/guides/RELAY.md`, Limitations).
- Browser game traffic runs over TCP. One lost packet stalls every packet behind it, so browser players stutter on lossy links where ENet players do not.

## Proposed design

**Simulation.** Clients exchange only per-tick inputs. A match advances from the inputs of all players, and the shared RNG is ordinary game state. GGRS (the Rust port of GGPO) drives it:

- **Rollback.** A client predicts that remote players repeat their last input and runs ahead. When a real input differs, it restores the snapshot from that tick and re-runs to the present.
- **Lockstep.** The same library with prediction off and an input delay. A client waits for every input of a tick before it runs that tick.

Menus, the world map and the screens between games stay on reliable messages. Rollback covers only the match. The match start message carries everything the simulation reads: the seed, the mode settings, the map and the skins (today `NET_G2P_SYNC`, `NET_NOTICE_GAMEMODESETTINGS`, `NET_G2P_MATCH_MAP` and `NET_NOTICE_SKIN_CHANGE`). These move to a reliable data channel.

**Transport.** WebRTC data channels in browsers and native builds, through `crates/smw-peer`. Each pair of players shares a reliable channel for match setup and an unreliable, unordered one for GGRS. Peers connect directly through STUN, and fall back to a TURN relay when NAT punching fails. `coturn` is the TURN server.

- **Signaling.** `smw_relay` pairs the peers of a room on `/signal/<room>`, speaking the `matchbox_server` protocol (`apps/smw_relay/src/signal.rs`); a stock `matchbox_server` works too. Both builds speak that protocol: native builds through `webrtc` (webrtc-rs) and `tokio-tungstenite`, the web build through `web/peer_socket.js`.
- **Lobby.** `smw_relay` keeps the room list, room names, passwords and player counts. When a room starts, the lobby gives each admitted player a random `matchbox` room ID. Only admitted players learn it, so the ID is the ticket. The host reports its signaling peer ID to the lobby, which passes it to the other players (protocol 0.11), so a joiner takes the host's slot and the match start only from that peer. If that proves too weak, the relay's own signaling can check each connection against the lobby.

**Game work.**

- A rollback snapshot that skips what a re-run does not need. `clone_match` also copies the 640x480 screen surface.
- Re-runs that play no sound and draw nothing. Sound effects must not repeat after a rollback.
- A desync check: peers exchange a state hash every few ticks and log the first tick that differs.
- Delete the host-authoritative code (`net_outcomes`, `net_random`, the per-event messages) once v2 ships.

## Alternatives considered

- **Keep host-authoritative, add a native WebSocket client.** Native builds join the relay, so mixed play works soon. It keeps the per-event sync code and puts native players in mixed games on TCP through the relay. This is the fallback if GGRS does not work out.
- **Teach the relay ENet and bridge it to UDP hosts.** Native clients stay unchanged. It adds ENet, UDP and NAT handling to the relay. The protocol version changes with every netplay release, so unchanged old clients cannot join anyway.
- **A host that serves ENet and the relay at the same time.** Native peers get direct links in mixed games. It keeps two transports and the host-authoritative model.
- **WebTransport or QUIC.** Client-to-server only, with no peer-to-peer in browsers, and browser support is uneven. It suits a server-authoritative game, which this is not.

## Risks and open questions

- **RG35XX CPU budget.** In a 4-CPU frenzy the device spends about 6 ms of its 16.67 ms tick on update and drawing (`SMW_PERF`). A rollback client snapshots every tick and, after a misprediction, re-runs up to about 8 ticks. The spike measured that this fits, with about 3 ms to spare in the worst window (Rollout, step 1). If a later feature breaks the budget, GGRS falls back to lockstep with a setting.
- **Visual snaps.** A misprediction moves a remote player at once. Input delay of one or two ticks reduces how often this happens.
- **Four players, full mesh.** Each peer talks to every other peer. Uploads stay small because only inputs travel.
- **TURN hosting.** Players behind strict NATs relay through TURN, which needs a server and bandwidth.
- **`matchbox_socket` is not used (resolved in step 3).** Its browser backend does not compile for `wasm32-unknown-emscripten`. Its native backend opens signaling through `async-tls` on rustls 0.21, which carries open advisories and an MPL-licensed root store. `smw-peer` therefore implements the same protocol itself and pins `ggrs` 0.13.0 and `webrtc` 0.17.2. `ggrs` brings two unmaintained crates, `instant` and `bincode` 1, which `deny.toml` ignores.
- **CPU players.** They run inside each client's deterministic simulation, so they need no network input. Their AI must read only game state. A room needs a setting for CPU slots, because today a room holds human slots only.
- **Disconnects.** A player who leaves mid-match needs a reliable "leave" message. When GGRS reports a disconnect, the match either continues with idle input or a CPU, or ends with the current scores.
- **Network changes (done).** GGRS drops a silent peer after 20 s. Within that time a peer whose network changed gets its place back. Its signaling connection resumes the same peer ID with the relay's resume token, and the peer that made the offer restarts ICE until the connection is back (`smw-peer`, `web/peer_socket.js`). A peer that has already been dropped cannot rejoin the match.
- **Hidden browser tabs.** Browsers pause `requestAnimationFrame` in a hidden tab, so that peer stops sending and the others stall at the prediction window. A peer reports `visibilitychange`, and the others drop it after a timeout.
- **Connecting at match start.** Every pair of players must connect before the match starts. Set a deadline, fall back to TURN, and name the player who could not connect.
- **Pause and exit.** `pausegame` and `exitinggame` live in `game_values`, which `clone_match` copies. Keep them out of the rollback state, and send neutral input while the dialog is open.
- **Local players (done).** A client brings one to three players on its own controllers: the room menu's Players field sends `NET_REQUEST_LOCAL_PLAYERS` (protocol 0.10), and the lobby seats them in free slots or refuses. `MSG_SLOT` carries all of a peer's slots, and local player i reads input slot i as GGRS local handle `L<i>`. Each room slot has a team: a new player takes an empty one, Left and Right on a player's skin send `NET_REQUEST_TEAM`, and the lobby refuses a room on one team. The match plays the room's teams.
- **Recordings.** v1 net recordings log every received packet. v2 records the confirmed inputs of each tick, which also makes a net match an ordinary replay.
- **Rooms between matches.** v1 hides a started room and breaks it up after the match. v2 returns everyone to the room for the next match.

## Rollout

1. **Spike: measure the budget (done).** A throwaway benchmark, `SMW_ROLLBACK_BENCH` in commit `0373ed2`, timed a `clone_match` snapshot every tick and an 8-frame re-run every tenth tick. 4-CPU frenzy averages per tick, worst 300-tick window in parentheses:

   | Device | Tick | Snapshot | 8-frame re-run | Total |
   |---|---|---|---|---|
   | Apple-silicon Mac | 0.25 ms | 0.05 ms | 2.0 ms | 2.3 ms |
   | RG35XX, Normal CPUs | 0.25-0.34 ms | 3.0-3.2 ms | 0.64-0.79 ms (worst frame 1.5 ms) | 3.9-4.3 ms |
   | RG35XX, Very Hard CPUs | 0.32-0.45 ms | 3.0-3.2 ms | 0.84-1.40 ms (worst frame 5.5 ms) | 4.2-4.9 ms |

   The tick column is simulation plus draw recording. Rasterizing a shown frame costs about 5 ms more on the RG35XX (`SMW_PERF`). The snapshot is the largest cost there: `clone_match` also copies the screen surface and the display lists, which a rollback snapshot does not need.
2. **Decide.** Rollback. The RG35XX worst case is about 3 ms snapshot, 5.5 ms re-run and 5 ms drawing, which fits the 16.67 ms tick. A trimmed snapshot widens the margin.
3. **Transport (done).** In three PRs:
   1. `smw-peer`: the peer socket for both builds and its GGRS socket. CI connects two browser pages and a native peer in one room (`tools/web_peer_test.mjs`).
   2. The lobby hands started rooms to signaling: by default the relay's own `/signal`, or the server `SMW_SIGNAL_URL` names.
   3. `coturn` in `deploy/`, and TURN credentials for clients.
4. **GGRS session for matches (done).**
   1. Rollback engine (`smw/rollback.rs`). `MatchState` holds what a match tick reads and writes: game values, map, objects, players, scores, modes, RNG, the virtual mixer, the gameplay state and the frame counter. It skips the screen surface and the display lists. `advance` runs one tick with given keys and can mute it. `checksum` hashes the harness dump plus every object's type, state, position and velocity, the platforms, the switches and the RNG state, with floats as their bits. Every replay's match runs under GGRS's sync test, which loads the state from 7 ticks back and re-runs it on every tick, and its final checksum must equal `tools/golden/rollback_checksums.txt` on every CI platform. A v2 pause belongs to the player who pressed start; only that player's start resumes it. After the match, any player's start or cancel opens the stats page on every peer at the same tick, and the next one ends the match on every peer once it is past the prediction window.
   2. The match runs over a GGRS P2P session on the peer socket (`smw/net_v2.rs`). At match start the host captures a shared checkpoint: the offline checkpoint plus the map file and the frame counter. It sends that over the reliable channel, and every joiner restores it. Every peer runs the code a local match runs. Input delay is 2 ticks, the prediction window 8, and GGRS compares checksums every 30 frames. `tools/net_v2_test.py` plays every scripted net game this way with 60 ms lag and 10% loss and checks that host and joiner states are identical on every frame.
   3. Native and browser players together. A native build reaches the relay's lobby through a `ws://` or `wss://` server entry (`smw/platform/network/websocket/`, with the relay layer's WebSocket on a background thread). `MIXED=1 tools/web_netplay_test.mjs` plays every scripted net game with a browser host and a native joiner; all 8 pass.
5. **Remove v1 (done).** The lobby starts a match with `NET_L2P_MATCH_START` (protocol 0.9). The game host connection, its start sync, NAT punching, the host-authoritative event code (`net_random`, `net_outcomes`), ENet, `smw_server` and the relay's game tunnel are gone; every build reaches the lobby over WebSocket.
6. **Local play without internet (done).** "Host on This Device" in a native build runs the lobby and the signaling server on a background thread. Players on the same network, whether shared Wi-Fi or one device's hotspot, need no internet, STUN or TURN. Their WebRTC connections use local addresses.
   - **Discovery.** The host sends a UDP beacon every second, by multicast and broadcast, with its protocol version and lobby port (`smw/network/lan_beacon.rs`). The Servers menu puts the hosts it hears, once each host's relay answers `/healthz`, at the top of the list, tagged LAN, and typing the host's address still works where broadcasts are blocked.
   - **Limits.** Networks that isolate clients, common on hotel Wi-Fi, need a hotspot instead. Browsers can neither host nor hear beacons. Joining a LAN game from a browser also needs a page served over http, because an https page cannot open `ws://` to a LAN address.
   - **Firewalls.** macOS and Windows ask once to allow incoming connections.
