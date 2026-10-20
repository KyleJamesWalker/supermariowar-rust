//! LAN discovery: a game hosting on its own network beacons every second to multicast group 239.255.42.99 and the
//! broadcast address on UDP port 12524, and other games list the hosts heard from in the last few seconds whose relay
//! answers `/healthz`.

use crate::common_netplay::protocol_definitions::{NET_PROTOCOL_VERSION_MAJOR, NET_PROTOCOL_VERSION_MINOR};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::{Mutex, Once};
use std::time::{Duration, Instant};

const BEACON_PORT: u16 = 12524;
const BEACON_GROUP: std::net::Ipv4Addr = std::net::Ipv4Addr::new(239, 255, 42, 99);
/// A beacon is `<MAGIC> <protocol version> <relay port>`; listeners ignore other versions.
const MAGIC: &str = "SMWLAN1";
/// A host leaves the list this long after its last beacon.
const FORGET_AFTER: Duration = Duration::from_secs(4);

/// A beacon whose relay did not answer is ignored this long.
const RECHECK_AFTER: Duration = Duration::from_secs(10);
/// Relays checked at once; beacons from further new hosts wait for a later beacon.
const MAX_CHECKING: usize = 4;
const CHECK_TIMEOUT: Duration = Duration::from_secs(1);

static DISCOVERY: Mutex<Discovery> = Mutex::new(Discovery { heard: BTreeMap::new(), checks: BTreeMap::new() });

enum Check {
    Checking,
    Answered,
    Failed(Instant),
}

/// The hosts heard from, by `ws://` URL, and whether their relays answered.
struct Discovery {
    /// When each host whose relay answered last beaconed.
    heard: BTreeMap<String, Instant>,
    checks: BTreeMap<String, Check>,
}

impl Discovery {
    /// Records a beacon; true when the caller must check the host's relay and report it with `checked`.
    fn beacon(&mut self, url: &str, now: Instant) -> bool {
        match self.checks.get(url) {
            Some(Check::Answered) => {
                self.heard.insert(url.to_string(), now);
                false
            }
            Some(Check::Checking) => false,
            Some(Check::Failed(at)) if now.duration_since(*at) < RECHECK_AFTER => false,
            _ => {
                if self.checks.values().filter(|c| matches!(c, Check::Checking)).count() >= MAX_CHECKING {
                    return false;
                }
                self.checks.insert(url.to_string(), Check::Checking);
                true
            }
        }
    }

    fn checked(&mut self, url: &str, answered: bool, now: Instant) {
        if answered {
            self.checks.insert(url.to_string(), Check::Answered);
            self.heard.insert(url.to_string(), now);
        } else {
            self.checks.insert(url.to_string(), Check::Failed(now));
        }
        self.forget_old(now);
    }

    /// Forgets hosts silent for `FORGET_AFTER`, so one that beacons again is checked again.
    fn forget_old(&mut self, now: Instant) {
        self.heard.retain(|_, at| now.duration_since(*at) < FORGET_AFTER);
        let heard = &self.heard;
        self.checks.retain(|url, check| match check {
            Check::Checking => true,
            Check::Answered => heard.contains_key(url),
            Check::Failed(at) => now.duration_since(*at) < RECHECK_AFTER,
        });
    }

    fn nearby(&mut self, now: Instant) -> Vec<String> {
        self.forget_old(now);
        self.heard.keys().cloned().collect()
    }
}

/// Whether a relay at `address` answers `GET /healthz` with `ok`.
fn relay_answers(address: SocketAddr) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&address, CHECK_TIMEOUT) else { return false };
    let _ = stream.set_read_timeout(Some(CHECK_TIMEOUT));
    let _ = stream.set_write_timeout(Some(CHECK_TIMEOUT));
    if stream.write_all(b"GET /healthz HTTP/1.1\r\nHost: smw\r\nConnection: close\r\n\r\n").is_err() {
        return false;
    }
    let mut response = String::new();
    let _ = stream.take(1024).read_to_string(&mut response);
    response.starts_with("HTTP/1.1 200") && response.ends_with("\r\n\r\nok\n")
}

fn version() -> String {
    format!("{NET_PROTOCOL_VERSION_MAJOR}.{NET_PROTOCOL_VERSION_MINOR}")
}

/// The address of the interface a route to a documentation address would leave by: this device on its network.
pub fn local_address() -> Option<std::net::IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    Some(socket.local_addr().ok()?.ip())
}

/// Announces the relay on `port` to the local network until the process ends.
pub fn broadcast(port: u16) {
    let message = format!("{MAGIC} {} {port}", version());
    let _ = std::thread::Builder::new().name("smw-lan-beacon".into()).spawn(move || {
        let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else { return };
        let _ = socket.set_broadcast(true);
        let _ = socket.set_multicast_ttl_v4(1);
        // Multicast also reaches listeners on this machine; some systems never loop a broadcast back.
        let targets = [BEACON_GROUP, std::net::Ipv4Addr::BROADCAST];
        loop {
            for target in &targets {
                let _ = socket.send_to(message.as_bytes(), (*target, BEACON_PORT));
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

/// The beacon port, shared with other games on this device (several players may run on one machine).
fn shared_listener() -> Option<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).ok()?;
    socket.set_reuse_address(true).ok()?;
    #[cfg(unix)]
    socket.set_reuse_port(true).ok()?;
    socket.bind(&std::net::SocketAddr::from(([0, 0, 0, 0], BEACON_PORT)).into()).ok()?;
    let _ = socket.join_multicast_v4(&BEACON_GROUP, &std::net::Ipv4Addr::UNSPECIFIED);
    Some(socket.into())
}

/// Starts listening for beacons, once.
pub fn listen() {
    // Tests set SMW_LAN_DISCOVERY=0, so real games on the network stay out of their server lists.
    if std::env::var("SMW_LAN_DISCOVERY").is_ok_and(|v| v == "0") {
        return;
    }
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        let _ = std::thread::Builder::new().name("smw-lan-listen".into()).spawn(|| {
            let Some(socket) = shared_listener() else { return };
            let mut buf = [0u8; 256];
            while let Ok((len, from)) = socket.recv_from(&mut buf) {
                let text = String::from_utf8_lossy(&buf[..len]);
                let mut fields = text.split(' ');
                if fields.next() != Some(MAGIC) || fields.next() != Some(version().as_str()) {
                    continue;
                }
                let Some(port) = fields.next().and_then(|p| p.parse::<u16>().ok()) else { continue };
                let url = format!("ws://{}:{port}", from.ip());
                if !discovery().beacon(&url, Instant::now()) {
                    continue;
                }
                let relay = SocketAddr::new(from.ip(), port);
                let _ = std::thread::Builder::new().name("smw-lan-check".into()).spawn(move || {
                    let answered = relay_answers(relay);
                    discovery().checked(&url, answered, Instant::now());
                });
            }
        });
    });
}

fn discovery() -> std::sync::MutexGuard<'static, Discovery> {
    DISCOVERY.lock().unwrap_or_else(|e| e.into_inner())
}

/// The hosts heard from recently whose relays answered, as server list entries.
pub fn nearby() -> Vec<String> {
    discovery().nearby(Instant::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn empty() -> Discovery {
        Discovery { heard: BTreeMap::new(), checks: BTreeMap::new() }
    }

    #[test]
    fn a_host_is_listed_only_once_its_relay_answers() {
        let mut d = empty();
        let now = Instant::now();
        assert!(d.beacon("ws://10.0.0.5:12523", now));
        assert!(!d.beacon("ws://10.0.0.5:12523", now), "one check at a time");
        assert!(d.nearby(now).is_empty());
        d.checked("ws://10.0.0.5:12523", true, now);
        assert_eq!(d.nearby(now), ["ws://10.0.0.5:12523"]);
        assert!(!d.beacon("ws://10.0.0.5:12523", now + Duration::from_secs(3)));
        assert_eq!(d.nearby(now + Duration::from_secs(6)), ["ws://10.0.0.5:12523"], "its beacons keep it listed");
        assert!(d.nearby(now + Duration::from_secs(10)).is_empty());
        assert!(d.beacon("ws://10.0.0.5:12523", now + Duration::from_secs(10)), "a forgotten host is checked again");
    }

    #[test]
    fn a_host_whose_relay_does_not_answer_is_not_listed_or_rechecked_at_once() {
        let mut d = empty();
        let now = Instant::now();
        assert!(d.beacon("ws://10.0.0.6:12523", now));
        d.checked("ws://10.0.0.6:12523", false, now);
        assert!(!d.beacon("ws://10.0.0.6:12523", now + Duration::from_secs(1)));
        assert!(d.nearby(now + Duration::from_secs(1)).is_empty());
        assert!(d.beacon("ws://10.0.0.6:12523", now + RECHECK_AFTER));
    }

    #[test]
    fn at_most_a_few_hosts_are_checked_at_once() {
        let mut d = empty();
        let now = Instant::now();
        let started = (0..10).filter(|i| d.beacon(&format!("ws://10.0.0.{i}:1"), now)).count();
        assert_eq!(started, MAX_CHECKING);
    }

    fn serve_once(response: &'static str) -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.read(&mut [0; 512]);
            let _ = stream.write_all(response.as_bytes());
        });
        address
    }

    #[test]
    fn only_a_relay_health_answer_counts() {
        assert!(relay_answers(serve_once("HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\nok\n")));
        assert!(!relay_answers(serve_once("HTTP/1.1 404 Not Found\r\n\r\nnot found\n")));
        assert!(!relay_answers(serve_once("SMWLAN1\n")));
        let closed = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap();
        assert!(!relay_answers(closed));
    }
}
