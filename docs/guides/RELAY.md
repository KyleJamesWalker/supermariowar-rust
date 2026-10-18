# smw_relay

`smw_relay` lets the browser build play online. It runs the netplay lobby server and switches each player's game traffic to the game host, all over WebSocket. Browsers cannot send UDP or accept connections, so ENet's direct player-to-host links do not work in a page.

The relay is not in the C++ project. It reuses the lobby code in `crates/smw-netplay/src/server` unchanged, behind a WebSocket `NetworkLayer`, through the `smw-netplay` crate built without its `enet` feature. It is its own Cargo package in `apps/smw_relay/`, a member of the root workspace, so it builds without SDL2 or ENet.

## Overview

Each browser opens one WebSocket to the relay. The relay gives that socket a virtual IPv4 address in `10.0.0.0/8` and a port, and uses them for all of the browser's connections. The lobby therefore reports addresses and the game host checks expected peers exactly as with ENet.

Inside the socket, the browser opens virtual connections:

- A connection to port 12521 (any host) goes to the embedded lobby.
- A connection to `<virtual address>:12522` goes to the browser with that address, if it listens on 12522 (it runs the room's game host).

The game host's NAT punch is a no-op. Reliable and unreliable sends both go over the socket, and TCP delivers them in order.

## Message format

Every WebSocket binary message is `[kind u8][conn u16 big-endian][payload]`. Ids below `0x8000` name connections the browser opened. Ids from `0x8000` up name connections the relay opened to the browser's game host. `crates/smw-netplay/src/common_netplay/relay_frame.rs` defines the constants.

| Kind | Name | Direction | `conn` | Payload |
|---|---|---|---|---|
| 1 | `CONNECT` | browser to relay | new id | address |
| 2 | `ACCEPTED` | relay to browser | id | none |
| 3 | `REFUSED` | relay to browser | id | none |
| 4 | `DATA` | both | id | one netplay package |
| 5 | `DISCONNECT` | both | id | none |
| 6 | `LISTEN` | browser to relay | 0 | port, u16 big-endian |
| 7 | `UNLISTEN` | browser to relay | 0 | port, u16 big-endian |
| 8 | `INCOMING` | relay to browser | new id | the connecting player's address |
| 9 | `WELCOME` | relay to browser, first | 0 | the browser's own address |

An address is 6 bytes: the IPv4 octets in dotted order, then the port as u16 big-endian.

## Setup and usage

```sh
cargo run --release -p smw_relay -- --port 8080
tools/package_web.sh && python3 -m http.server -d dist/web 8000
# open http://localhost:8000/?relay=ws://localhost:8080 in two windows
node tools/web_netplay_test.mjs out/   # two headless browsers play a scripted game
```

The relay serves plain `ws://` and `GET /healthz`. Put a TLS proxy in front of it in production, because an `https://` page can only open `wss://`.

| Setting | Default |
|---|---|
| `--port`, `SMW_RELAY_PORT` | `8080` |
| `--config` | `serverconfig`, the lobby settings file `smw_server` reads |
| `SMW_RELAY_ALLOWED_ORIGINS` | `https://www.kylejameswalker.com,http://localhost:8000`, or `*` |
| `SMW_RELAY_MAX_CLIENTS` | `64` |

The relay refuses a WebSocket whose `Origin` is not in the list. It accepts one without `Origin`: browsers always send it, so that is a native game. It also caps each browser at 8 virtual connections and each message at 64 KiB. It drops a client that is silent for 60 seconds or whose outgoing queue fills up. When a browser leaves, the relay closes its virtual connections and the lobby removes it from its room.

The browser build connects to `wss://smw-relay.vps.pocketsquirrel.com` unless the page has a `?relay=` parameter. Set `SMW_RELAY_URL` when you run `tools/package_web.sh` to change that default. The relay URL is the first entry in the saved server list. A saved entry without a scheme means `wss://<entry>`.

## Deploy

The `Relay` workflow publishes two images for `linux/amd64` and `linux/arm64`:

- `ghcr.io/kylejameswalker/supermariowar-rust-relay`: the relay.
- `ghcr.io/kylejameswalker/supermariowar-rust-signal`: `matchbox_server` 0.14.0, the netplay v2 signaling server (`deploy/signal/Dockerfile`).

`deploy/` holds a compose file that runs both behind Caddy, plus `coturn` for netplay v2 players whose NAT blocks a direct connection.

1. Point a DNS `A` (or `AAAA`) record for `smw-relay.vps.pocketsquirrel.com` at the server.
2. On the server, create the shared network: `docker network create caddy`.
3. Copy `deploy/docker-compose.yml`, `deploy/Caddyfile` and `deploy/turnserver.conf` to one directory. If Caddy already runs on the `caddy` network, add the Caddyfile's site block to its config and remove the `caddy` service.
4. Create a `.env` file there with `TURN_SECRET=<hex>`, for example from `openssl rand -hex 32`. Use hex only: the coturn image evaluates its arguments in a shell. The lobby needs the same secret to issue TURN credentials.
5. Open TCP and UDP port 3478 and UDP ports 49160-49259 for `coturn`. It uses host networking.
6. Run `docker compose up -d`. Caddy gets the certificate and proxies `/signal/*` to `smw-signal:3536` and everything else to `smw-relay:8080`.
7. Check that `curl https://smw-relay.vps.pocketsquirrel.com/healthz` returns `ok`. A WebSocket client can join `wss://smw-relay.vps.pocketsquirrel.com/signal/<room>`, for example `SIGNAL_URL=wss://smw-relay.vps.pocketsquirrel.com/signal node tools/web_peer_test.mjs`.

`coturn` relays only to public addresses (`denied-peer-ip` in `turnserver.conf`), so it cannot reach the server's private network.

### Netplay v2 settings

When a room starts, the lobby (`smw_relay` or `smw_server`) sends each of its players `NET_L2P_MATCH_ROOM`. It holds a new random signaling room under `SMW_SIGNAL_URL`, plus TURN credentials for `SMW_TURN_URL`, minted from `SMW_TURN_SECRET` and valid for six hours. Without `SMW_SIGNAL_URL` the lobby sends nothing and rooms start as before. The compose file sets all three for the relay. For now the game only connects the peers and logs the result (`[net] v2:` lines); the match itself still runs over v1.

## Limits

- A native client joins the relay's lobby when its server list holds the relay as a `ws://` or `wss://` URL, for example `wss://smw-relay.vps.pocketsquirrel.com`; a bare host name still means ENet and `smw_server`. Browser and native players then share rooms. Their match runs over the relay's game tunnel (v1) or, with `SMW_NETPLAY_V2=1`, over WebRTC (`docs/design/NETPLAY_V2.md`).
- All game traffic passes through the relay, so its round trip adds to each player's latency.
