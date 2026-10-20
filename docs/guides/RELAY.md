# smw_relay

`smw_relay` is the netplay lobby server for every build, and the signaling server for its matches. Players reach it over one WebSocket each, so the same server works for browsers, which cannot send UDP, and native games. Matches themselves run peer to peer over WebRTC (`docs/design/NETPLAY_V2.md`).

The relay is not in the C++ project. It reuses the lobby code in `crates/smw-netplay/src/server` unchanged, behind a WebSocket `NetworkLayer`. It is its own Cargo package in `apps/smw_relay/`, a member of the root workspace, so it builds without SDL2. A game that hosts on its own network runs it on a background thread (`smw_relay::run_embedded`).

## Overview

Each player opens one WebSocket to the relay. The relay gives that socket a virtual IPv4 address in `10.0.0.0/8` and a port, which the lobby reports as the player's address. Inside the socket, the player opens virtual connections to port 12521, the embedded lobby. A WebSocket to `/signal/<room>` is a signaling connection instead (`apps/smw_relay/src/signal.rs`).

## Message format

Every WebSocket binary message is `[kind u8][conn u16 big-endian][payload]`. `crates/smw-netplay/src/common_netplay/relay_frame.rs` defines the constants.

| Kind | Name | Direction | `conn` | Payload |
|---|---|---|---|---|
| 1 | `CONNECT` | player to relay | new id | address (port 12521) |
| 2 | `ACCEPTED` | relay to player | id | none |
| 3 | `REFUSED` | relay to player | id | none |
| 4 | `DATA` | both | id | one netplay package |
| 5 | `DISCONNECT` | both | id | none |
| 9 | `WELCOME` | relay to player, first | 0 | the player's own address |

An address is 6 bytes: the IPv4 octets in dotted order, then the port as u16 big-endian.

## Setup and usage

```sh
cargo run --release -p smw_relay -- --port 8080
tools/package_web.sh && python3 -m http.server -d dist/web 8000
# open http://localhost:8000/?relay=ws://localhost:8080 in two windows, or list ws://localhost:8080 in a native game
node tools/web_netplay_test.mjs out/   # two headless browsers play a scripted game
```

The relay serves plain `ws://` and `GET /healthz`. Put a TLS proxy in front of it in production, because an `https://` page can only open `wss://`.

| Setting | Default |
|---|---|
| `--port`, `SMW_RELAY_PORT` | `8080` |
| `--config` | `serverconfig`, the lobby settings file |
| `SMW_RELAY_ALLOWED_ORIGINS` | `https://www.kylejameswalker.com,http://localhost:8000`, or `*` |
| `SMW_RELAY_MAX_CLIENTS` | `64` lobby clients, and as many signaling peers |
| `SMW_RELAY_MAX_CONNECTIONS` | `256` TCP connections of any kind, handshakes included |
| `SMW_LOBBY_VERBOSE` | unset; `1` logs every room update, room list and lobby message |

The relay refuses a WebSocket whose `Origin` is not in the list. It accepts one without `Origin`: browsers always send it, so that is a native game. It also caps each player at 8 virtual connections and each message at 64 KiB, and each signaling room at 4 peers and 16 KiB per message. It closes a connection over `SMW_RELAY_MAX_CONNECTIONS` as soon as it accepts it. It drops a client that is silent for 60 seconds or whose outgoing queue fills up. When a player leaves, the relay closes its virtual connections and the lobby removes it from its room.

The browser build connects to `wss://smw-relay.vps.pocketsquirrel.com` unless the page has a `?relay=` parameter. Set `SMW_RELAY_URL` when you run `tools/package_web.sh` to change that default. The relay URL is the first entry in the saved server list. In every build, a saved entry without a scheme means `wss://<entry>`.

## Deploy

The `Relay` workflow publishes `ghcr.io/kylejameswalker/supermariowar-rust-relay` for `linux/amd64` and `linux/arm64`. `deploy/` holds a compose file that runs it behind Caddy, plus `coturn` for players whose NAT blocks a direct WebRTC connection.

1. Point a DNS `A` (or `AAAA`) record for `smw-relay.vps.pocketsquirrel.com` at the server.
2. On the server, create the shared network: `docker network create caddy`.
3. Copy `deploy/docker-compose.yml`, `deploy/Caddyfile` and `deploy/turnserver.conf` to one directory. If Caddy already runs on the `caddy` network, add the Caddyfile's site block to its config and remove the `caddy` service.
4. Create a `.env` file there with `TURN_SECRET=<hex>`, for example from `openssl rand -hex 32`. Use hex only: the coturn image evaluates its arguments in a shell. The lobby needs the same secret to issue TURN credentials.
5. Open TCP and UDP port 3478 and UDP ports 49160-49259 for `coturn`. It uses host networking.
6. Run `docker compose up -d`. Caddy gets the certificate and proxies everything to `smw-relay:8080`.
7. Check that `curl https://smw-relay.vps.pocketsquirrel.com/healthz` returns `ok`. A WebSocket client can join `wss://smw-relay.vps.pocketsquirrel.com/signal/<room>`, for example `SIGNAL_URL=wss://smw-relay.vps.pocketsquirrel.com/signal node tools/web_peer_test.mjs`.

`coturn` relays only to public addresses (`denied-peer-ip` in `turnserver.conf`), so it cannot reach the server's private network.

### Netplay v2 settings

When a room starts, the lobby sends each of its players `NET_L2P_MATCH_ROOM`. It holds a new random signaling room under `SMW_SIGNAL_URL`, plus TURN credentials for `SMW_TURN_URL`, minted from `SMW_TURN_SECRET` and valid for six hours. Then it sends every player `NET_L2P_MATCH_START`, and the match runs over WebRTC under GGRS (`docs/design/NETPLAY_V2.md`). Without `SMW_SIGNAL_URL`, the relay uses its own signaling on `/signal`, and players reach it through the same URL as the lobby. The compose file sets the TURN settings and leaves `SMW_SIGNAL_URL` unset.

## Limits

- Browser and native players share rooms. Lobby traffic passes through the relay; match traffic does not.
