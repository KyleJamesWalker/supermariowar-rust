//! The relay's sockets: one thread accepts, and a thread per client runs its WebSocket and passes
//! messages to and from the main thread, which owns the lobby and the switch.

use crate::config::Config;
use crate::switch::{ClientId, Sink};
use smw::common_netplay::relay_frame::{HEADER_LEN, MAX_MESSAGE_LEN};
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, Sender, SyncSender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tungstenite::http::StatusCode;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::Message;

/// How long a new connection has to send its HTTP request head and finish the handshake.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
/// How long a socket thread waits for input before it sends what other threads queued, while traffic flows.
const POLL_ACTIVE: Duration = Duration::from_millis(4);
/// The same once a connection has been quiet for `ACTIVE_WINDOW`.
const POLL_IDLE: Duration = Duration::from_millis(50);
const ACTIVE_WINDOW: Duration = Duration::from_secs(2);
/// Time between pings to a lobby client.
const PING_INTERVAL: Duration = Duration::from_secs(20);
/// A lobby client silent this long is disconnected.
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
/// Messages queued for one client before it counts as fallen behind.
const OUTBOX_LEN: usize = 1024;
/// The largest HTTP request head the relay reads, in bytes.
const MAX_REQUEST_HEAD: usize = 8 * 1024;

/// What a lobby client's thread tells the main thread.
pub enum Event {
    Open { id: ClientId, outbox: Outbox },
    Message { id: ClientId, data: Vec<u8> },
    Closed { id: ClientId },
}

/// A client's queue of outgoing messages. A client that lets it fill up is disconnected.
pub struct Outbox {
    tx: SyncSender<Vec<u8>>,
    overflow: Arc<AtomicBool>,
}

impl Outbox {
    /// The main thread's handle for queueing messages to this client.
    pub fn into_sink(self) -> Sink {
        Rc::new(move |message| {
            if self.tx.try_send(message).is_err() {
                self.overflow.store(true, Ordering::Relaxed);
            }
        })
    }
}

/// Counts of the connections the relay serves, each under its own limit.
#[derive(Clone, Default)]
struct Counts {
    connections: Arc<AtomicUsize>,
    clients: Arc<AtomicUsize>,
    signaling: Arc<AtomicUsize>,
}

/// One place under a limit, given back on drop.
struct Slot(Arc<AtomicUsize>);

impl Slot {
    fn take(count: &Arc<AtomicUsize>, max: usize) -> Option<Slot> {
        count.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| (n < max).then_some(n + 1)).ok().map(|_| Slot(count.clone()))
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A socket's read timeout: `POLL_ACTIVE` while traffic flows, `POLL_IDLE` once it stops.
pub struct Poll {
    last_active: Instant,
    timeout: Duration,
}

impl Poll {
    pub fn new(stream: &TcpStream) -> Poll {
        let _ = stream.set_read_timeout(Some(POLL_ACTIVE));
        Poll { last_active: Instant::now(), timeout: POLL_ACTIVE }
    }

    pub fn active(&mut self) {
        self.last_active = Instant::now();
    }

    /// Sets the timeout for the next read.
    pub fn update(&mut self, stream: &TcpStream) {
        let timeout = if self.last_active.elapsed() < ACTIVE_WINDOW { POLL_ACTIVE } else { POLL_IDLE };
        if timeout != self.timeout {
            let _ = stream.set_read_timeout(Some(timeout));
            self.timeout = timeout;
        }
    }
}

/// Binds `config.port` and serves it from background threads; lobby client events go to `events`. Returns the port.
pub fn serve(config: &Config, events: Sender<Event>) -> io::Result<u16> {
    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], config.port)))?;
    let port = listener.local_addr()?.port();
    let config = Arc::new(config.clone());
    let counts = Counts::default();
    thread::spawn(move || {
        let mut next_id: ClientId = 0;
        let mut full = false;
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let Some(slot) = Slot::take(&counts.connections, config.max_connections) else {
                if !full {
                    println!("[relay] refusing connections: {} open", config.max_connections);
                }
                full = true;
                continue;
            };
            full = false;
            next_id += 1;
            let (id, config, counts, events) = (next_id, config.clone(), counts.clone(), events.clone());
            let _ = thread::Builder::new().spawn(move || {
                let _slot = slot;
                handle_connection(stream, id, &config, &counts, &events);
            });
        }
    });
    Ok(port)
}

/// Routes one TCP connection: signaling, `/healthz`, or a lobby WebSocket client.
fn handle_connection(mut stream: TcpStream, id: ClientId, config: &Config, counts: &Counts, events: &Sender<Event>) {
    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let _ = stream.set_nodelay(true);

    let Some((head, head_len)) = peek_request_head(&stream) else { return };
    let (path, upgrade) = parse_request_head(&head);
    if let Some(room) = path.strip_prefix("/signal/").filter(|r| upgrade && !r.is_empty()) {
        let Some(_slot) = Slot::take(&counts.signaling, config.max_clients) else {
            refuse_full(&mut stream, head_len, "signaling");
            return;
        };
        let room = room.split('?').next().unwrap_or_default().to_string();
        crate::signal::run(stream, room, config);
        return;
    }
    if !upgrade || path == "/healthz" {
        let mut consumed = vec![0; head_len];
        let _ = stream.read_exact(&mut consumed);
        match path.as_str() {
            "/healthz" => respond(&mut stream, "200 OK", "ok\n"),
            "/" => respond(&mut stream, "426 Upgrade Required", "smw_relay: connect with a WebSocket\n"),
            _ => respond(&mut stream, "404 Not Found", "not found\n"),
        }
        return;
    }

    let Some(_slot) = Slot::take(&counts.clients, config.max_clients) else {
        refuse_full(&mut stream, head_len, "lobby");
        return;
    };
    run_client(stream, id, config, events);
}

fn refuse_full(stream: &mut TcpStream, head_len: usize, kind: &str) {
    println!("[relay] refused a {kind} client: relay full");
    let mut consumed = vec![0; head_len];
    let _ = stream.read_exact(&mut consumed);
    respond(stream, "503 Service Unavailable", "relay full\n");
}

/// One lobby client's WebSocket, until it closes, falls behind or goes quiet.
fn run_client(stream: TcpStream, id: ClientId, config: &Config, events: &Sender<Event>) {
    let mut who = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    let mut origin = String::new();
    let check_origin = |request: &Request, response: Response| -> Result<Response, ErrorResponse> {
        let header = |name: &str| request.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
        if let Some(forwarded) = header("x-forwarded-for") {
            who = forwarded;
        }
        origin = header("origin").unwrap_or_default();
        if config.origin_allowed(header("origin").as_deref()) {
            return Ok(response);
        }
        let mut refusal = ErrorResponse::new(Some("origin not allowed\n".to_string()));
        *refusal.status_mut() = StatusCode::FORBIDDEN;
        Err(refusal)
    };
    let ws_config = WebSocketConfig::default().max_message_size(Some(MAX_MESSAGE_LEN)).max_frame_size(Some(MAX_MESSAGE_LEN));
    let mut ws = match tungstenite::accept_hdr_with_config(stream, check_origin, Some(ws_config)).map_err(|e| e.to_string()) {
        Ok(ws) => ws,
        Err(error) => {
            println!("[relay] {who} (origin {origin:?}): handshake refused: {error}");
            return;
        }
    };
    println!("[relay] client {id} connected from {who} (origin {origin})");

    let mut poll = Poll::new(ws.get_ref());
    let (tx, outgoing) = sync_channel(OUTBOX_LEN);
    let overflow = Arc::new(AtomicBool::new(false));
    if events.send(Event::Open { id, outbox: Outbox { tx, overflow: overflow.clone() } }).is_err() {
        return;
    }

    let mut last_heard = Instant::now();
    let mut last_ping = Instant::now();
    let reason = 'session: loop {
        loop {
            match ws.read() {
                Ok(Message::Binary(data)) => {
                    last_heard = Instant::now();
                    poll.active();
                    if data.len() < HEADER_LEN {
                        break 'session "malformed message".to_string();
                    }
                    let _ = events.send(Event::Message { id, data: data.to_vec() });
                }
                Ok(Message::Text(_)) => break 'session "text message".to_string(),
                Ok(_) => last_heard = Instant::now(),
                Err(tungstenite::Error::Io(e)) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => break,
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => break 'session "closed".to_string(),
                Err(error) => break 'session error.to_string(),
            }
        }

        if overflow.load(Ordering::Relaxed) {
            break "fell too far behind".to_string();
        }
        let mut queued = Vec::new();
        while let Ok(message) = outgoing.try_recv() {
            queued.push(Message::Binary(message.into()));
        }
        if !queued.is_empty() {
            poll.active();
        }
        if last_ping.elapsed() >= PING_INTERVAL {
            queued.push(Message::Ping(Default::default()));
            last_ping = Instant::now();
        }
        if !queued.is_empty() {
            let mut result = Ok(());
            for message in queued {
                result = result.and_then(|_| ws.write(message));
            }
            if let Err(error) = result.and_then(|_| ws.flush()) {
                break error.to_string();
            }
        }
        if last_heard.elapsed() >= IDLE_TIMEOUT {
            break "timed out".to_string();
        }
        poll.update(ws.get_ref());
    };

    let _ = events.send(Event::Closed { id });
    let _ = ws.close(None);
    let _ = ws.flush();
    println!("[relay] client {id} disconnected: {reason}");
}

/// The HTTP request head and its length in bytes, left unread in the socket.
fn peek_request_head(stream: &TcpStream) -> Option<(String, usize)> {
    let mut buf = vec![0; MAX_REQUEST_HEAD];
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    loop {
        let n = stream.peek(&mut buf).ok()?;
        if n == 0 {
            return None;
        }
        if let Some(end) = buf[..n].windows(4).position(|w| w == b"\r\n\r\n") {
            return Some((String::from_utf8_lossy(&buf[..end]).into_owned(), end + 4));
        }
        if n == buf.len() || Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// The path, and whether the request asks for a WebSocket.
fn parse_request_head(head: &str) -> (String, bool) {
    let mut lines = head.lines();
    let target = lines.next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("");
    let path = target.split('?').next().unwrap_or("").to_string();
    let upgrade = lines.any(|l| {
        let (name, value) = l.split_once(':').unwrap_or((l, ""));
        name.trim().eq_ignore_ascii_case("upgrade") && value.to_ascii_lowercase().contains("websocket")
    });
    (path, upgrade)
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// True when `127.0.0.1:<port>` answers `GET /healthz` with 200.
pub fn healthcheck(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&SocketAddr::from(([127, 0, 0, 1], port)), Duration::from_secs(3)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    if stream.write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").is_err() {
        return false;
    }
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    response.starts_with("HTTP/1.1 200")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::{channel, Receiver};
    use tungstenite::stream::MaybeTlsStream;
    use tungstenite::WebSocket;

    fn relay(config: impl FnOnce(&mut Config)) -> (u16, Receiver<Event>) {
        let mut c = Config::embedded(0);
        config(&mut c);
        let (tx, rx) = channel();
        (serve(&c, tx).unwrap(), rx)
    }

    fn open(port: u16) -> TcpStream {
        let stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        stream
    }

    /// Whether the relay closed `stream` rather than waiting for its request.
    fn closed(stream: &mut TcpStream) -> bool {
        match stream.read(&mut [0; 64]) {
            Ok(0) => true,
            Ok(_) => false,
            Err(e) => !matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut),
        }
    }

    fn signal(port: u16, room: &str) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, String> {
        let stream = open(port);
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        tungstenite::client::client(format!("ws://127.0.0.1:{port}/signal/{room}"), MaybeTlsStream::Plain(stream))
            .map(|(ws, _)| ws)
            .map_err(|e| e.to_string())
    }

    fn read_text(ws: &mut WebSocket<MaybeTlsStream<TcpStream>>) -> Option<String> {
        match ws.read() {
            Ok(Message::Text(text)) => Some(text.to_string()),
            _ => None,
        }
    }

    #[test]
    fn connections_over_the_limit_are_closed_at_once() {
        let (port, _events) = relay(|c| c.max_connections = 2);
        let mut held = [open(port), open(port)];
        let mut extra = open(port);
        assert!(closed(&mut extra));
        assert!(!closed(&mut held[1]), "the relay waits for an open connection's request");

        drop(held);
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let mut retry = open(port);
            if !closed(&mut retry) {
                break;
            }
            assert!(Instant::now() < deadline, "closed connections free their places");
        }
    }

    #[test]
    fn signaling_peers_count_against_max_clients() {
        let (port, _events) = relay(|c| c.max_clients = 1);
        let mut first = signal(port, "limit-a").unwrap();
        assert!(read_text(&mut first).unwrap().contains("IdAssigned"));
        assert!(signal(port, "limit-b").unwrap_err().contains("503"));
    }

    #[test]
    fn signaling_rooms_hold_four_peers() {
        let (port, _events) = relay(|_| {});
        let mut peers: Vec<_> = (0..4).map(|_| signal(port, "four").unwrap()).collect();
        for peer in &mut peers {
            assert!(read_text(peer).unwrap().contains("IdAssigned"));
        }
        let mut fifth = signal(port, "four").unwrap();
        assert_eq!(read_text(&mut fifth), None);
    }

    #[test]
    fn signaling_closes_on_an_oversize_message() {
        let (port, _events) = relay(|_| {});
        let mut peer = signal(port, "oversize").unwrap();
        assert!(read_text(&mut peer).unwrap().contains("IdAssigned"));
        let _ = peer.send(Message::Text("x".repeat(64 * 1024).into()));
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if let Err(error) = peer.read() {
                assert!(!matches!(error, tungstenite::Error::Io(ref e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)), "{error}");
                return;
            }
        }
        panic!("the relay kept the connection");
    }

    #[test]
    fn request_heads() {
        assert_eq!(parse_request_head("GET /healthz HTTP/1.1\r\nHost: x"), ("/healthz".to_string(), false));
        assert_eq!(
            parse_request_head("GET /?a=b HTTP/1.1\r\nHost: x\r\nConnection: Upgrade\r\nUpgrade: WebSocket"),
            ("/".to_string(), true)
        );
        assert_eq!(parse_request_head(""), (String::new(), false));
    }
}
