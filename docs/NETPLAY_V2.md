# Netplay v2: rollback over WebRTC

Status: proposed, on the backlog. A spike measures the deciding number first (Rollout, step 1).

## Summary

Replace host-authoritative netplay with input exchange over one WebRTC transport for every build. Each client runs the same deterministic match from all players' inputs, with rollback if the RG35XX can afford it and delay-based lockstep if it cannot. This removes the per-event sync code and lets browser and native players share a game. It also retires ENet, the WebSocket relay's game traffic and our NAT punching.

## Background

- **Model.** The host decides random outcomes, deaths, scores and mode settings. Joiners replay those decisions a few frames later (`PROGRESS.md`, "Netplay deviations").
- **Transport.** Native clients speak ENet over UDP to `smw_server` and to the game host. Browsers tunnel the same connections over WebSocket through `smw_relay` (`RELAY.md`).
- **Prerequisites now met.** `Game::clone_match` copies a running match (`HANDLES.md`). CI gates cross-platform determinism across native, web, SDL3 and aarch64 builds (`ARCHITECTURE_V2.md`, phases 6 and 7).

## Problem

- Every random event needs its own sync rule. The deviation list covers block bounces, respawn fallbacks after 90 frames, hazards waiting for the host, and network IDs for objects made in events. Each new feature with randomness adds another rule.
- Joiners see host-decided events late.
- Browser and native clients cannot play together (`RELAY.md`, Limitations).
- Browser game traffic runs over TCP. One lost packet stalls every packet behind it, so browser players stutter on lossy links where ENet players do not.

## Proposed design

**Simulation.** Clients exchange only per-tick inputs. A match advances from the inputs of all players, and the shared RNG is ordinary game state. GGRS (the Rust port of GGPO) drives it:

- **Rollback.** A client predicts that remote players repeat their last input and runs ahead. When a real input differs, it restores the snapshot from that tick and re-runs to the present.
- **Lockstep.** The same library with prediction off and an input delay. A client waits for every input of a tick before it runs that tick.

Menus, the world map and the screens between games stay on reliable messages. Rollback covers only the match.

**Transport.** WebRTC data channels, unreliable and unordered, in browsers and native builds. Peers connect directly through STUN, and fall back to a TURN relay when NAT punching fails. `matchbox` provides one socket API for native and wasm and plugs into GGRS. Its signaling server takes over the lobby role of `smw_relay`. `coturn` is the TURN server.

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

- **RG35XX CPU budget.** In a 4-CPU frenzy the device spends about 6 ms of its 16.67 ms tick on update and drawing (`SMW_PERF`). A rollback client snapshots every tick and, after a misprediction, re-runs up to about 8 ticks. The spike measures whether that fits. If it does not, use lockstep.
- **Visual snaps.** A misprediction moves a remote player at once. Input delay of one or two ticks reduces how often this happens.
- **Four players, full mesh.** Each peer talks to every other peer. Uploads stay small because only inputs travel.
- **TURN hosting.** Players behind strict NATs relay through TURN, which needs a server and bandwidth.
- **CPU players.** They run inside each client's deterministic simulation, so they need no network input. Their AI must read only game state.

## Rollout

1. **Spike: measure the budget.** `SMW_ROLLBACK_BENCH=1` (or a frame count, default 8) logs `[rollback]` lines every 300 ticks of a match. Each line gives the tick time, the `clone_match` snapshot time every tick, and an 8-frame re-run every tenth tick. A 4-CPU frenzy on an Apple-silicon Mac: tick 0.25 ms, snapshot 0.05 ms, 8-frame re-run 2.0 ms, 2.3 ms in total. Next, run the same match on the RG35XX.
2. **Decide.** If the RG35XX total stays under the tick budget with margin, use rollback. If not, use GGRS lockstep. Record the numbers here.
3. **Transport.** `matchbox` sockets in native and web builds, a signaling server in place of the relay's lobby, and a TURN server in `deploy/`.
4. **GGRS session for matches.** Snapshot and re-run hooks, silent re-runs, and the desync check. A CI test plays a scripted mixed native and browser match under induced latency and packet loss.
5. **Remove v1.** Delete ENet, the WebSocket game tunnel, NAT punching and the host-authoritative event code. Bump the protocol version.
