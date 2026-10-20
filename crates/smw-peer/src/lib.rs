//! Netplay v2 transport: every player joins a matchbox signaling room and opens a WebRTC connection with a reliable
//! and an unreliable channel to every other player. Native (webrtc-rs) and web (web/peer_socket.js) peers share rooms.

#[cfg(not(target_os = "emscripten"))]
mod native;
#[cfg(not(target_os = "emscripten"))]
use native as imp;
#[cfg(target_os = "emscripten")]
mod web;
#[cfg(target_os = "emscripten")]
use web as imp;

/// Ordered and retransmitted: match start data, settings, chat.
pub const RELIABLE: usize = 0;
/// GGRS traffic; GGRS resends what it needs.
pub const UNRELIABLE: usize = 1;

/// A STUN or TURN server, as in the browser's `RTCIceServer`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}

/// How peers look for a route to each other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IceConfig {
    pub servers: Vec<IceServer>,
    /// Use TURN relays only, so other players never learn this player's address.
    pub relay_only: bool,
}

impl Default for IceConfig {
    /// Two public STUN servers and no TURN.
    fn default() -> Self {
        let urls = [
            "stun:stun.l.google.com:19302",
            "stun:stun1.l.google.com:19302",
        ];
        IceConfig {
            servers: vec![IceServer {
                urls: urls.map(String::from).to_vec(),
                ..Default::default()
            }],
            relay_only: false,
        }
    }
}

/// A peer's signaling UUID as a number; ordering it orders players the same way on every peer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PeerId(pub u128);

/// A change in a peer's connection: `Connected` once both channels are open.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PeerState {
    Connected,
    Disconnected,
}

/// A room connection, polled once per frame. Nothing blocks.
pub struct PeerSocket {
    inner: imp::Socket,
    /// Open peers, in the order they connected.
    connected: Vec<PeerId>,
}

impl PeerSocket {
    /// `room_url` is `ws[s]://<signaling server>/<room>`.
    pub fn connect(room_url: &str, ice: &IceConfig) -> PeerSocket {
        PeerSocket {
            inner: imp::Socket::connect(room_url, ice),
            connected: Vec::new(),
        }
    }

    /// None until the signaling server has assigned one.
    pub fn id(&mut self) -> Option<PeerId> {
        self.inner.id()
    }

    /// Peers that connected or left since the last call.
    pub fn update_peers(&mut self) -> Vec<(PeerId, PeerState)> {
        let changes = self.inner.update_peers();
        for &(peer, state) in &changes {
            self.connected.retain(|&p| p != peer);
            if state == PeerState::Connected {
                self.connected.push(peer);
            }
        }
        changes
    }

    /// The peers whose connections are open, as of the last `update_peers`.
    pub fn connected_peers(&self) -> &[PeerId] {
        &self.connected
    }

    /// True once the signaling connection failed or closed; established peers stay connected.
    pub fn signaling_closed(&self) -> bool {
        self.inner.signaling_closed()
    }

    /// Queues `data` on `channel` (`RELIABLE` or `UNRELIABLE`) to `peer`.
    pub fn send(&mut self, channel: usize, peer: PeerId, data: &[u8]) {
        self.inner.send(channel, peer, data);
    }

    /// What arrived on `channel` since the last call, in arrival order.
    pub fn receive(&mut self, channel: usize) -> Vec<(PeerId, Vec<u8>)> {
        self.inner.receive(channel)
    }

    /// Every connected peer and this one, in the same order on every peer: GGRS player handles.
    pub fn players(&mut self) -> Vec<ggrs::PlayerType<PeerId>> {
        let Some(me) = self.id() else {
            return Vec::new();
        };
        let mut ids: Vec<PeerId> = self.connected.iter().copied().chain([me]).collect();
        ids.sort();
        ids.into_iter()
            .map(|id| {
                if id == me {
                    ggrs::PlayerType::Local
                } else {
                    ggrs::PlayerType::Remote(id)
                }
            })
            .collect()
    }

    /// Hands the unreliable channel to a GGRS session; `receive(UNRELIABLE)` returns nothing afterwards.
    pub fn take_ggrs_socket(&mut self) -> GgrsSocket {
        GgrsSocket(self.inner.take_unreliable())
    }

    /// Closes every peer connection and the signaling connection.
    pub fn close(&mut self) {
        self.inner.close();
        self.connected.clear();
    }
}

/// `ggrs::NonBlockingSocket` over the unreliable channel. Both builds encode messages the same way.
pub struct GgrsSocket(imp::Channel);

impl ggrs::NonBlockingSocket<PeerId> for GgrsSocket {
    fn send_to(&mut self, msg: &ggrs::Message, addr: &PeerId) {
        let packet = bincode::serialize(msg).expect("a GGRS message always serializes");
        self.0.send(*addr, &packet);
    }

    fn receive_all_messages(&mut self) -> Vec<(PeerId, ggrs::Message)> {
        self.0
            .receive()
            .into_iter()
            .filter_map(|(peer, packet)| bincode::deserialize(&packet).ok().map(|msg| (peer, msg)))
            .collect()
    }
}
