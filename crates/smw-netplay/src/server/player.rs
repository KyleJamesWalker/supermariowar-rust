//! A lobby player: its connection, name, room and skin.

use super::blob::Blob;
use super::clock::{time_now, TimePoint};
use smw::common_netplay::network_interface::NetPeer;
use smw::common_netplay::protocol_definitions::{NET_MAX_LOCAL_PLAYERS, NET_MAX_PLAYER_NAME_LENGTH};
use smw::common_netplay::protocol_packages::{ConnectOk, MessageHeader};

const SKINPKG_SIZE_LIMIT: usize = 20000; /* bytes in worst case */

/// A player's last `NET_NOTICE_SKIN_CHANGE` message; byte 3 carries its room slot.
#[derive(Default)]
pub struct PlayerSkin {
    pub blob: Blob,
}

impl std::ops::Deref for PlayerSkin {
    type Target = Blob;
    fn deref(&self) -> &Blob {
        &self.blob
    }
}

impl std::ops::DerefMut for PlayerSkin {
    fn deref_mut(&mut self) -> &mut Blob {
        &mut self.blob
    }
}

impl PlayerSkin {
    /// Stamps the sender's room slot into the message; 0xFF outside a room.
    pub fn set_player_id(&mut self, id: u8) {
        if self.get_size() < 4 {
            return;
        }

        self.get_data_mut()[3] = id;
    }
}

/// A connected client, owned by `SMWServer::players`, with its one to `NET_MAX_LOCAL_PLAYERS` players.
pub struct Player {
    /// The name the lobby gave this player, unique ignoring ASCII case.
    pub name: Vec<u8>,
    /// Sent `NET_REQUEST_CONNECT`; a connection that only asks for server info never does.
    pub connected: bool,
    pub network_client: Option<Box<dyn NetPeer>>,

    /// The player's room, 0 for none.
    pub currentRoomID: u32,

    /// Each local player's skin, by local index.
    pub skins: [PlayerSkin; NET_MAX_LOCAL_PLAYERS],
    /// The names the client asked for its local players after the first; empty for the numbered lobby name.
    pub localNames: [Vec<u8>; NET_MAX_LOCAL_PLAYERS],

    pub joinTime: TimePoint,
    pub lastActivityTime: TimePoint,
}

impl Player {
    pub fn new() -> Self {
        let joinTime = time_now();
        Player {
            currentRoomID: 0,

            name: b"Anonymous".to_vec(),
            connected: false,
            network_client: None,

            skins: Default::default(),
            localNames: Default::default(),

            joinTime,
            lastActivityTime: joinTime,
        }
    }

    pub fn set_client(&mut self, client: Box<dyn NetPeer>) {
        debug_assert!(self.network_client.is_none());
        self.network_client = Some(client);

        self.lastActivityTime = time_now();
    }

    pub fn set_name(&mut self, name: &[u8]) {
        self.name = name.to_vec();

        self.lastActivityTime = time_now();
    }

    /// Keeps the skin of the local player byte 3 names; the local index of a valid one, else None.
    pub fn set_skin(&mut self, data: &[u8]) -> Option<usize> {
        // Some basic package validation
        if data.len() <= std::mem::size_of::<MessageHeader>() + 5 /* id 1B + un-/compressed size 2*2B */
            || data.len() > SKINPKG_SIZE_LIMIT
            || data[3] as usize >= NET_MAX_LOCAL_PLAYERS
        {
            println!("[error] Corrupt skin arrived from {}", self.to_string());
            return None;
        }

        let local = data[3] as usize;
        self.skins[local].replace_with(data);
        self.lastActivityTime = time_now();
        Some(local)
    }

    /// The room name of local player `local`: the lobby name for the first, else its requested name or the lobby
    /// name numbered from 2.
    pub fn local_name(&self, local: usize) -> Vec<u8> {
        if local == 0 {
            return self.name.clone();
        }
        if !self.localNames[local].is_empty() {
            return self.localNames[local].clone();
        }
        let suffix = format!(" {}", local + 1);
        let keep = self.name.len().min(NET_MAX_PLAYER_NAME_LENGTH - 1 - suffix.len());
        [&self.name[..keep], suffix.as_bytes()].concat()
    }

    // Printing stuff
    pub fn to_string(&self) -> String {
        let mut out = String::from("[");
        out += &String::from_utf8_lossy(&self.name);
        out += "@";
        out += &self.address_to_string();
        out += "]";
        out
    }

    pub fn address_to_string(&self) -> String {
        match &self.network_client {
            None => "(null)".to_string(),
            Some(c) => c.address_as_string(),
        }
    }

    // Network stuff
    /// Sends reliably; false without a connection or for data shorter than a message header.
    pub fn send_data(&mut self, data: &[u8]) -> bool {
        if self.network_client.is_none() || data.len() < 3 {
            return false;
        }

        self.lastActivityTime = time_now();
        self.network_client.as_mut().unwrap().send_reliable(data)
    }

    pub fn send_code(&mut self, code: u8) -> bool {
        let msg = MessageHeader::new(code);
        self.send_data(msg.as_bytes())
    }

    pub fn send_connect_ok(&mut self) -> bool {
        let msg = ConnectOk::with(&self.name);
        self.send_data(msg.as_bytes())
    }
}
