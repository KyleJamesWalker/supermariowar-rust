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

The relay refuses a WebSocket whose `Origin` is not in the list. It also caps each browser at 8 virtual connections and each message at 64 KiB. It drops a client that is silent for 60 seconds or whose outgoing queue fills up. When a browser leaves, the relay closes its virtual connections and the lobby removes it from its room.

The browser build connects to `wss://smw-relay.vps.pocketsquirrel.com` unless the page has a `?relay=` parameter. Set `SMW_RELAY_URL` when you run `tools/package_web.sh` to change that default. The relay URL is the first entry in the saved server list. A saved entry without a scheme means `wss://<entry>`.

## Deploy

The `Relay` workflow publishes `ghcr.io/kylejameswalker/supermariowar-rust-relay` for `linux/amd64` and `linux/arm64`. `deploy/` holds a compose file and a Caddyfile that put it behind Caddy.

1. Point a DNS `A` (or `AAAA`) record for `smw-relay.vps.pocketsquirrel.com` at the server.
2. On the server, create the shared network: `docker network create caddy`.
3. Copy `deploy/docker-compose.yml` and `deploy/Caddyfile` to one directory. If Caddy already runs on the `caddy` network, add the Caddyfile's site block to its config and remove the `caddy` service.
4. Run `docker compose up -d`. Caddy gets the certificate and proxies the WebSocket to `smw-relay:8080`.
5. Check `curl https://smw-relay.vps.pocketsquirrel.com/healthz` returns `ok`.

## Limits

- Browser and native clients cannot play together. A native client speaks ENet to `smw_server`, and its game host listens on UDP. Mixed play would need the relay to also speak ENet and bridge the two.
- All game traffic passes through the relay, so its round trip adds to each player's latency.
