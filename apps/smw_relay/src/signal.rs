//! Netplay v2 signaling on `/signal/<room>`: the matchbox protocol (`IdAssigned`, `NewPeer`, `PeerLeft`, `Signal`)
//! as `matchbox_server` speaks it, so a relay alone can pair a room's WebRTC peers, e.g. on a LAN without internet.

use crate::config::Config;
use crate::websocket::Poll;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::http::StatusCode;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::Message;

/// A signaling peer silent this long leaves its room.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// Messages queued for one peer; more are dropped.
const OUTBOX_LEN: usize = 256;
/// The largest signaling message, in bytes: an SDP with every ICE candidate gathered is a few KiB.
const MAX_MESSAGE_LEN: usize = 16 * 1024;
/// A match has at most four players.
const MAX_ROOM_PEERS: usize = 4;

/// Each signaling room's peers, by peer ID, with the queue of messages to each.
type Rooms = HashMap<String, HashMap<String, SyncSender<String>>>;

fn rooms() -> &'static Mutex<Rooms> {
    static ROOMS: OnceLock<Mutex<Rooms>> = OnceLock::new();
    ROOMS.get_or_init(Default::default)
}

/// A random UUID v4, the form matchbox peer IDs take.
fn new_peer_id() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("the OS random source");
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

/// Queues a message to one peer of a room; it is dropped if the peer is gone or its queue is full.
fn send_to(room: &str, peer: &str, text: String) {
    if let Some(tx) = rooms().lock().unwrap().get(room).and_then(|r| r.get(peer)) {
        let _ = tx.try_send(text);
    }
}

/// One peer's signaling session in `room`, on its connection's thread, until it closes or goes quiet.
pub fn run(stream: TcpStream, room: String, config: &Config) {
    let check_origin = |request: &Request, response: Response| -> Result<Response, ErrorResponse> {
        let origin = request.headers().get("origin").and_then(|v| v.to_str().ok());
        if config.origin_allowed(origin) {
            return Ok(response);
        }
        let mut refusal = ErrorResponse::new(Some("origin not allowed\n".to_string()));
        *refusal.status_mut() = StatusCode::FORBIDDEN;
        Err(refusal)
    };
    let ws_config = WebSocketConfig::default().max_message_size(Some(MAX_MESSAGE_LEN)).max_frame_size(Some(MAX_MESSAGE_LEN));
    let Ok(mut ws) = tungstenite::accept_hdr_with_config(stream, check_origin, Some(ws_config)) else { return };
    let mut poll = Poll::new(ws.get_ref());

    let me = new_peer_id();
    let (tx, outgoing) = sync_channel::<String>(OUTBOX_LEN);
    let _ = tx.try_send(json!({ "IdAssigned": me }).to_string());
    // Reading the room's peers and adding this one under one lock: two peers joining at once still pair up.
    let others: Option<Vec<SyncSender<String>>> = {
        let mut rooms = rooms().lock().unwrap();
        let peers = rooms.entry(room.clone()).or_default();
        (peers.len() < MAX_ROOM_PEERS).then(|| {
            let others = peers.values().cloned().collect();
            peers.insert(me.clone(), tx);
            others
        })
    };
    let Some(others) = others else {
        println!("[relay] refused a signaling peer: room full");
        let _ = ws.close(None);
        let _ = ws.flush();
        return;
    };
    for other in others {
        let _ = other.try_send(json!({ "NewPeer": me }).to_string());
    }

    let mut last_heard = Instant::now();
    'session: loop {
        loop {
            match ws.read() {
                Ok(Message::Text(text)) => {
                    last_heard = Instant::now();
                    poll.active();
                    let Ok(request) = serde_json::from_str::<Value>(&text) else { continue };
                    if let Some(signal) = request.get("Signal") {
                        let (Some(receiver), Some(data)) = (signal.get("receiver").and_then(Value::as_str), signal.get("data")) else { continue };
                        send_to(&room, receiver, json!({ "Signal": { "sender": me, "data": data } }).to_string());
                    }
                }
                Ok(Message::Close(_)) => break 'session,
                Ok(_) => last_heard = Instant::now(),
                Err(tungstenite::Error::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => break,
                Err(_) => break 'session,
            }
        }
        let mut wrote = false;
        while let Ok(text) = outgoing.try_recv() {
            if ws.write(Message::Text(text.into())).is_err() {
                break 'session;
            }
            wrote = true;
        }
        if wrote {
            poll.active();
            if ws.flush().is_err() {
                break;
            }
        }
        if last_heard.elapsed() >= IDLE_TIMEOUT {
            break;
        }
        poll.update(ws.get_ref());
    }

    let remaining: Vec<SyncSender<String>> = {
        let mut rooms = rooms().lock().unwrap();
        let peers = rooms.entry(room.clone()).or_default();
        peers.remove(&me);
        let remaining = peers.values().cloned().collect();
        if peers.is_empty() {
            rooms.remove(&room);
        }
        remaining
    };
    for other in remaining {
        let _ = other.try_send(json!({ "PeerLeft": me }).to_string());
    }
}
