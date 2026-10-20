//! The relay layer's one WebSocket: web/relay_socket.js in the browser, a background thread natively. Both are polled.

/// `state()` values; it is -1 when no socket was opened.
pub const CONNECTING: i32 = 0;
pub const OPEN: i32 = 1;
pub const CLOSED: i32 = 2;

#[cfg(target_os = "emscripten")]
mod imp {
    use std::ffi::CString;
    use std::os::raw::c_char;

    extern "C" {
        fn smw_ws_open(url: *const c_char);
        fn smw_ws_state() -> i32;
        fn smw_ws_send(data: *const u8, len: usize) -> i32;
        fn smw_ws_recv(buf: *mut u8, cap: usize) -> i32;
        fn smw_ws_close();
    }

    /// Closes any open socket and starts connecting to `url`; `state()` reports the outcome.
    pub fn open(url: &str) {
        let url = CString::new(url).unwrap_or_default();
        unsafe { smw_ws_open(url.as_ptr()) }
    }

    pub fn state() -> i32 {
        unsafe { smw_ws_state() }
    }

    /// Sends one binary message; dropped unless the socket is open.
    pub fn send(data: &[u8]) {
        unsafe { smw_ws_send(data.as_ptr(), data.len()) };
    }

    pub fn recv(buf: &mut [u8]) -> i32 {
        unsafe { smw_ws_recv(buf.as_mut_ptr(), buf.len()) }
    }

    pub fn close() {
        unsafe { smw_ws_close() }
    }
}

#[cfg(not(target_os = "emscripten"))]
mod imp {
    use super::{CLOSED, CONNECTING, OPEN};
    use std::collections::VecDeque;
    use std::net::TcpStream;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::mpsc::{channel, Sender, TryRecvError};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tungstenite::stream::MaybeTlsStream;
    use tungstenite::{Message, WebSocket};

    struct Socket {
        state: Arc<AtomicI32>,
        inbox: Arc<Mutex<VecDeque<Vec<u8>>>>,
        outbox: Sender<Vec<u8>>,
    }

    static SOCKET: Mutex<Option<Socket>> = Mutex::new(None);

    fn set_read_timeout(ws: &mut WebSocket<MaybeTlsStream<TcpStream>>, timeout: Duration) {
        let _ = match ws.get_mut() {
            MaybeTlsStream::Plain(s) => s.set_read_timeout(Some(timeout)),
            MaybeTlsStream::Rustls(s) => s.get_mut().set_read_timeout(Some(timeout)),
            _ => Ok(()),
        };
    }

    /// Closes any open socket and starts a thread that connects to `url`, then pumps it every 4 ms.
    pub fn open(url: &str) {
        close();
        let state = Arc::new(AtomicI32::new(CONNECTING));
        let inbox = Arc::new(Mutex::new(VecDeque::new()));
        let (outbox, outgoing) = channel::<Vec<u8>>();
        let (thread_state, thread_inbox, url) = (state.clone(), inbox.clone(), url.to_string());
        std::thread::Builder::new()
            .name("smw-relay-socket".into())
            .spawn(move || {
                let _ = rustls::crypto::ring::default_provider().install_default();
                let mut ws = match tungstenite::connect(url.as_str()) {
                    Ok((ws, _)) => ws,
                    Err(e) => {
                        eprintln!("[net] relay {url}: {e}");
                        thread_state.store(CLOSED, Ordering::Release);
                        return;
                    }
                };
                set_read_timeout(&mut ws, Duration::from_millis(4));
                thread_state.store(OPEN, Ordering::Release);
                loop {
                    loop {
                        match outgoing.try_recv() {
                            Ok(data) => {
                                if ws.write(Message::Binary(data.into())).is_err() {
                                    thread_state.store(CLOSED, Ordering::Release);
                                    return;
                                }
                            }
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => {
                                let _ = ws.close(None);
                                let _ = ws.flush();
                                thread_state.store(CLOSED, Ordering::Release);
                                return;
                            }
                        }
                    }
                    let _ = ws.flush();
                    match ws.read() {
                        Ok(Message::Binary(data)) => thread_inbox.lock().unwrap().push_back(data.to_vec()),
                        Ok(_) => {}
                        Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
                        Err(_) => {
                            thread_state.store(CLOSED, Ordering::Release);
                            return;
                        }
                    }
                }
            })
            .expect("spawn the relay socket thread");
        *SOCKET.lock().unwrap() = Some(Socket { state, inbox, outbox });
    }

    pub fn state() -> i32 {
        SOCKET.lock().unwrap().as_ref().map_or(-1, |s| s.state.load(Ordering::Acquire))
    }

    /// Queues one binary message for the socket thread; dropped when no socket is open.
    pub fn send(data: &[u8]) {
        if let Some(s) = SOCKET.lock().unwrap().as_ref() {
            let _ = s.outbox.send(data.to_vec());
        }
    }

    /// The message's length, -1 when none is waiting, -2 when it is longer than `buf` (dropped).
    pub fn recv(buf: &mut [u8]) -> i32 {
        let guard = SOCKET.lock().unwrap();
        let Some(s) = guard.as_ref() else { return -1 };
        let Some(message) = s.inbox.lock().unwrap().pop_front() else { return -1 };
        if message.len() > buf.len() {
            return -2;
        }
        buf[..message.len()].copy_from_slice(&message);
        message.len() as i32
    }

    /// Drops the socket; its thread sends a close frame and exits.
    pub fn close() {
        SOCKET.lock().unwrap().take();
    }
}

pub use imp::{close, open, recv, send, state};
