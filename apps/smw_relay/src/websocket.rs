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

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
/// How long a client thread waits for input before it sends what the main thread queued.
const POLL: Duration = Duration::from_millis(4);
const PING_INTERVAL: Duration = Duration::from_secs(20);
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const OUTBOX_LEN: usize = 1024;
const MAX_REQUEST_HEAD: usize = 8 * 1024;

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
    pub fn into_sink(self) -> Sink {
        Rc::new(move |message| {
            if self.tx.try_send(message).is_err() {
                self.overflow.store(true, Ordering::Relaxed);
            }
        })
    }
}

pub fn serve(config: &Config, events: Sender<Event>) -> io::Result<()> {
    let listener = TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], config.port)))?;
    let config = Arc::new(config.clone());
    let active = Arc::new(AtomicUsize::new(0));
    thread::spawn(move || {
        let mut next_id: ClientId = 0;
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            next_id += 1;
            let (id, config, active, events) = (next_id, config.clone(), active.clone(), events.clone());
            thread::spawn(move || handle_connection(stream, id, &config, &active, &events));
        }
    });
    Ok(())
}

fn handle_connection(mut stream: TcpStream, id: ClientId, config: &Config, active: &AtomicUsize, events: &Sender<Event>) {
    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let _ = stream.set_nodelay(true);

    let Some((head, head_len)) = peek_request_head(&stream) else { return };
    let (path, upgrade) = parse_request_head(&head);
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

    if active.fetch_add(1, Ordering::SeqCst) >= config.max_clients {
        active.fetch_sub(1, Ordering::SeqCst);
        println!("[relay] refused a client: {} clients connected", config.max_clients);
        let mut consumed = vec![0; head_len];
        let _ = stream.read_exact(&mut consumed);
        respond(&mut stream, "503 Service Unavailable", "relay full\n");
        return;
    }
    run_client(stream, id, config, events);
    active.fetch_sub(1, Ordering::SeqCst);
}

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

    let _ = ws.get_ref().set_read_timeout(Some(POLL));
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
