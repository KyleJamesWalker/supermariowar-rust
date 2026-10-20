//! LAN discovery: a game hosting on its own network beacons every second to multicast group 239.255.42.99 and the
//! broadcast address on UDP port 12524, and other games list the hosts heard from in the last few seconds.

use crate::common_netplay::protocol_definitions::{NET_PROTOCOL_VERSION_MAJOR, NET_PROTOCOL_VERSION_MINOR};
use std::collections::BTreeMap;
use std::net::UdpSocket;
use std::sync::{Mutex, Once};
use std::time::{Duration, Instant};

const BEACON_PORT: u16 = 12524;
const BEACON_GROUP: std::net::Ipv4Addr = std::net::Ipv4Addr::new(239, 255, 42, 99);
/// A beacon is `<MAGIC> <protocol version> <relay port>`; listeners ignore other versions.
const MAGIC: &str = "SMWLAN1";
/// A host leaves the list this long after its last beacon.
const FORGET_AFTER: Duration = Duration::from_secs(4);

/// Each heard host's `ws://` URL and when its last beacon arrived.
static HEARD: Mutex<BTreeMap<String, Instant>> = Mutex::new(BTreeMap::new());

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
                HEARD.lock().unwrap().insert(format!("ws://{}:{port}", from.ip()), Instant::now());
            }
        });
    });
}

/// The hosts heard from recently, as server list entries.
pub fn nearby() -> Vec<String> {
    let mut heard = HEARD.lock().unwrap();
    heard.retain(|_, at| at.elapsed() < FORGET_AFTER);
    heard.keys().cloned().collect()
}
