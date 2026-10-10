//! Not in upstream: recording and replaying a net match at the transport boundary. A client's recording keeps every
//! network event its listeners saw as `#@ net` lines; a replay hands them to the same handlers at the same frame.

use crate::smw::harness::{self, base64_decode, base64_encode};
use smw_netplay::common_netplay::network_interface::{NetPeer, NetworkEventHandler};
use std::any::Any;
use std::collections::{BTreeMap, VecDeque};

/// Which of the client's two listeners an event reached: its client (lobby or foreign game host) or its own game host.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Client,
    Host,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    Connect,
    Receive(Vec<u8>),
    Disconnect,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PeerInfo {
    pub host: u32,
    pub port: u16,
    pub player_id: u64,
    pub key: u64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NetEvent {
    pub frame: u32,
    pub side: Side,
    pub peer: PeerInfo,
    pub kind: Kind,
}

impl NetEvent {
    /// `#@ net frame=<n> side=<c|h> peer=<host>:<port>:<player id>:<peer key> <connect|disconnect|recv=<base64>>`
    pub fn line(&self) -> String {
        let side = if self.side == Side::Client { 'c' } else { 'h' };
        let kind = match &self.kind {
            Kind::Connect => "connect".to_string(),
            Kind::Disconnect => "disconnect".to_string(),
            Kind::Receive(data) => format!("recv={}", base64_encode(data)),
        };
        format!("#@ net frame={} side={} peer={}:{}:{}:{} {}", self.frame, side, self.peer.host, self.peer.port, self.peer.player_id, self.peer.key, kind)
    }

    pub fn parse(line: &str) -> Option<NetEvent> {
        let rest = line.strip_prefix("#@ net ")?;
        let mut fields = rest.split(' ');
        let frame = fields.next()?.strip_prefix("frame=")?.parse().ok()?;
        let side = match fields.next()?.strip_prefix("side=")? {
            "c" => Side::Client,
            "h" => Side::Host,
            _ => return None,
        };
        let mut peer = fields.next()?.strip_prefix("peer=")?.split(':');
        let peer = PeerInfo {
            host: peer.next()?.parse().ok()?,
            port: peer.next()?.parse().ok()?,
            player_id: peer.next()?.parse().ok()?,
            key: peer.next()?.parse().ok()?,
        };
        let kind = match fields.next()? {
            "connect" => Kind::Connect,
            "disconnect" => Kind::Disconnect,
            k => Kind::Receive(base64_decode(k.strip_prefix("recv=")?)?),
        };
        fields.next().is_none().then_some(NetEvent { frame, side, peer, kind })
    }
}

pub fn info(peer: &dyn NetPeer) -> PeerInfo {
    PeerInfo { host: peer.address_host(), port: peer.address_port(), player_id: peer.get_player_id(), key: peer.peer_key() }
}

/// A listener's handler while recording: each event is logged, then passed on unchanged.
pub struct Recording<'a> {
    pub inner: &'a mut dyn NetworkEventHandler,
    pub side: Side,
    pub frame: u32,
    pub log: &'a mut Vec<NetEvent>,
}

impl NetworkEventHandler for Recording<'_> {
    fn on_connect(&mut self, peer: Box<dyn NetPeer>) {
        self.log.push(NetEvent { frame: self.frame, side: self.side, peer: info(&*peer), kind: Kind::Connect });
        self.inner.on_connect(peer);
    }

    fn on_receive(&mut self, peer: &mut dyn NetPeer, data: &[u8]) {
        self.log.push(NetEvent { frame: self.frame, side: self.side, peer: info(peer), kind: Kind::Receive(data.to_vec()) });
        self.inner.on_receive(peer, data);
    }

    fn on_disconnect(&mut self, client: &mut dyn NetPeer) {
        self.log.push(NetEvent { frame: self.frame, side: self.side, peer: info(client), kind: Kind::Disconnect });
        self.inner.on_disconnect(client);
    }
}

/// A recorded peer with the same address and player ID; its sends go nowhere.
pub struct ReplayPeer(pub PeerInfo);

impl NetPeer for ReplayPeer {
    fn send(&mut self, _: &[u8]) -> bool {
        true
    }
    fn send_reliable(&mut self, _: &[u8]) -> bool {
        true
    }
    fn disconnect(&mut self) {}
    fn address_host(&self) -> u32 {
        self.0.host
    }
    fn address_port(&self) -> u16 {
        self.0.port
    }
    fn address_as_string(&self) -> String {
        let a = self.0.host.to_ne_bytes();
        format!("{}.{}.{}.{}:{}", a[0], a[1], a[2], a[3], self.0.port)
    }
    fn average_rtt(&self) -> u32 {
        0
    }
    fn get_player_id(&self) -> u64 {
        self.0.player_id
    }
    fn peer_key(&self) -> u64 {
        self.0.key
    }
    fn same_peer(&self, other: &dyn NetPeer) -> bool {
        other.as_any().downcast_ref::<ReplayPeer>().is_some_and(|o| o.0 == self.0)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Plays a recording's events to the listener of `side` at `frame`, in recorded order.
pub fn deliver(events: &[NetEvent], next: &mut usize, side: Side, frame: u32, handler: &mut dyn NetworkEventHandler) {
    while let Some(ev) = events.get(*next).filter(|e| e.frame == frame && e.side == side) {
        *next += 1;
        let mut peer = ReplayPeer(ev.peer.clone());
        match &ev.kind {
            Kind::Connect => handler.on_connect(Box::new(peer)),
            Kind::Receive(data) => handler.on_receive(&mut peer, data),
            Kind::Disconnect => handler.on_disconnect(&mut peer),
        }
    }
}

/// A recorded value the game read from outside it: `#@ netv frame=<n> <key>=<value>`.
fn value_line(frame: u32, key: &str, value: &str) -> String {
    format!("#@ netv frame={} {}={}", frame, key, value)
}

fn parse_value(line: &str) -> Option<(u32, String, String)> {
    let rest = line.strip_prefix("#@ netv frame=")?;
    let (frame, rest) = rest.split_once(' ')?;
    let (key, value) = rest.split_once('=')?;
    Some((frame.parse().ok()?, key.to_string(), value.to_string()))
}

struct Replay {
    events: Vec<NetEvent>,
    next: usize,
    values: BTreeMap<String, VecDeque<String>>,
}

static mut replay: Option<Replay> = None;

/// Loads a replay's net lines from `from` on. A recording without `#@ netrec=1` predates net recording,
/// so its replay keeps using the network.
pub fn load(text: &str, from: u32) {
    let lines = || text.lines().map(|l| l.strip_suffix('\r').unwrap_or(l));
    if !lines().any(|l| l == "#@ netrec=1") {
        return;
    }
    let events = lines().filter_map(NetEvent::parse).filter(|e| e.frame >= from).collect();
    let mut values: BTreeMap<String, VecDeque<String>> = BTreeMap::new();
    for (frame, key, value) in lines().filter_map(parse_value) {
        if frame >= from {
            values.entry(key).or_default().push_back(value);
        }
    }
    unsafe { replay = Some(Replay { events, next: 0, values }) };
}

/// Whether the network comes from the replay: no connections, and every listener gets the recorded events.
pub fn replaying() -> bool {
    unsafe { (*(&raw const replay)).is_some() }
}

/// A listener's poll: in a replay, the events recorded for this frame and side; otherwise `live`'s, recorded.
pub fn listen(side: Side, handler: &mut dyn NetworkEventHandler, live: impl FnOnce(&mut dyn NetworkEventHandler)) {
    let frame = harness::frame();
    let mut log = Vec::new();
    if let Some(r) = unsafe { (*(&raw mut replay)).as_mut() } {
        // Taken out while delivering: a handler may read a recorded value.
        let events = std::mem::take(&mut r.events);
        let mut next = r.next;
        deliver(&events, &mut next, side, frame, &mut Recording { inner: handler, side, frame, log: &mut log });
        if let Some(r) = unsafe { (*(&raw mut replay)).as_mut() } {
            r.events = events;
            r.next = next;
        }
    } else {
        live(&mut Recording { inner: handler, side, frame, log: &mut log });
    }
    for ev in &log {
        harness::record_line(&ev.line());
    }
}

/// A value the game reads from the network layer or the clock: `live`'s, recorded, or in a replay the recorded one.
pub fn value(key: &str, live: impl FnOnce() -> String) -> String {
    let v = if let Some(r) = unsafe { (*(&raw mut replay)).as_mut() } {
        match r.values.get_mut(key).and_then(|q| q.pop_front()) {
            Some(v) => v,
            None => harness::fail_replay(format!("the replay has no recorded {} for frame {}", key, harness::frame())),
        }
    } else {
        live()
    };
    harness::record_line(&value_line(harness::frame(), key, &v));
    v
}

/// `value` for one the game may lack (`-`); a replay without it reads none.
pub fn value_opt(key: &str, live: impl FnOnce() -> Option<String>) -> Option<String> {
    if replaying() && unsafe { (*(&raw const replay)).as_ref() }.is_some_and(|r| r.values.get(key).is_none_or(|q| q.is_empty())) {
        return None;
    }
    Some(value(key, || live().unwrap_or_else(|| "-".to_string()))).filter(|v| v != "-")
}

pub fn value_u32(key: &str, live: impl FnOnce() -> u32) -> u32 {
    value(key, || live().to_string()).parse().unwrap_or(0)
}

pub fn value_bool(key: &str, live: impl FnOnce() -> bool) -> bool {
    value(key, || (live() as u8).to_string()) == "1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_round_trip() {
        let peer = PeerInfo { host: 0x0100000a, port: 12522, player_id: 7, key: 3 };
        for kind in [Kind::Connect, Kind::Disconnect, Kind::Receive(vec![0, 93, 255, 1, 2])] {
            let ev = NetEvent { frame: 1234, side: Side::Host, peer: peer.clone(), kind };
            assert_eq!(NetEvent::parse(&ev.line()), Some(ev));
        }
        assert_eq!(NetEvent::parse("#@ net frame=1 side=x peer=1:2:3:4 connect"), None);
    }

    struct Log(Vec<String>);

    impl NetworkEventHandler for Log {
        fn on_connect(&mut self, peer: Box<dyn NetPeer>) {
            self.0.push(format!("connect {}", peer.address_as_string()));
        }
        fn on_receive(&mut self, peer: &mut dyn NetPeer, data: &[u8]) {
            self.0.push(format!("recv {} {:?}", peer.get_player_id(), data));
        }
        fn on_disconnect(&mut self, _: &mut dyn NetPeer) {
            self.0.push("disconnect".to_string());
        }
    }

    #[test]
    fn deliver_replays_one_side_and_frame_in_order() {
        let peer = PeerInfo { host: 0x0100000a, port: 12522, player_id: 2, key: 1 };
        let ev = |frame, side, kind| NetEvent { frame, side, peer: peer.clone(), kind };
        let events = [
            ev(5, Side::Client, Kind::Connect),
            ev(5, Side::Client, Kind::Receive(vec![1])),
            ev(6, Side::Client, Kind::Receive(vec![2])),
            ev(6, Side::Host, Kind::Disconnect),
        ];
        let mut log = Log(Vec::new());
        let mut next = 0;
        deliver(&events, &mut next, Side::Client, 5, &mut log);
        assert_eq!(log.0, ["connect 10.0.0.1:12522", "recv 2 [1]"]);
        deliver(&events, &mut next, Side::Client, 6, &mut log);
        deliver(&events, &mut next, Side::Host, 6, &mut log);
        assert_eq!(next, 4);
        assert_eq!(log.0[2..], ["recv 2 [2]".to_string(), "disconnect".to_string()]);
    }
}
