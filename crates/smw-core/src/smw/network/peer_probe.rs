//! Netplay v2 groundwork: when the lobby sends `NET_L2P_MATCH_ROOM`, every player also joins the room's WebRTC
//! mesh and logs whether all pairs connected. The v1 match does not use these connections.

use crate::common_netplay::protocol_packages::{cstr_field, MatchRoom};
use smw_peer::{IceConfig, IceServer, PeerSocket, PeerState};
use std::time::{Duration, Instant};

const DEADLINE: Duration = Duration::from_secs(15);

pub struct PeerProbe {
    socket: PeerSocket,
    others: usize,
    opened: Instant,
    reported: bool,
}

impl PeerProbe {
    /// `lobby` is the lobby's URL; a room URL that starts with `/` is on that server (a relay's own signaling).
    pub fn open(pkg: &MatchRoom, lobby: &str) -> PeerProbe {
        let mut ice = IceConfig::default();
        let turn_url = cstr_field(&pkg.turnURL);
        if !turn_url.is_empty() {
            ice.servers.push(IceServer {
                urls: vec![turn_url],
                username: cstr_field(&pkg.turnUsername),
                credential: cstr_field(&pkg.turnCredential),
            });
        }
        let others = (pkg.playerCount as usize).saturating_sub(1);
        println!("[net] v2: joining the match's WebRTC room ({} other players, TURN {})", others, if ice.servers.len() > 1 { "on" } else { "off" });
        let mut room = cstr_field(&pkg.roomURL);
        if room.starts_with('/') {
            room = format!("{}{}", lobby.trim_end_matches('/'), room);
        }
        PeerProbe { socket: PeerSocket::connect(&room, &ice), others, opened: Instant::now(), reported: false }
    }

    pub fn into_socket(self) -> (PeerSocket, usize) {
        (self.socket, self.others)
    }

    pub fn update(&mut self) {
        for (peer, state) in self.socket.update_peers() {
            let elapsed = self.opened.elapsed().as_millis();
            match state {
                PeerState::Connected => println!("[net] v2: peer {:08x} connected after {} ms", peer.0 >> 96, elapsed),
                PeerState::Disconnected => println!("[net] v2: peer {:08x} left after {} ms", peer.0 >> 96, elapsed),
            }
        }
        self.socket.receive(smw_peer::UNRELIABLE);
        if self.reported {
            return;
        }
        let connected = self.socket.connected_peers().len();
        if connected >= self.others {
            println!("[net] v2: all {} peers connected in {} ms", self.others, self.opened.elapsed().as_millis());
            self.reported = true;
        } else if self.opened.elapsed() >= DEADLINE {
            println!("[net] v2: only {} of {} peers connected after {} s", connected, self.others, DEADLINE.as_secs());
            self.reported = true;
        }
    }
}
