//! Port of src/server/Player.cpp

use super::blob::Blob;
use super::clock::{time_now, TimePoint};
use smw::common_netplay::network_interface::NetPeer;
use smw::common_netplay::protocol_definitions::NET_RESPONSE_CONNECT_OK;
use smw::common_netplay::protocol_packages::MessageHeader;

const SKINPKG_SIZE_LIMIT: usize = 20000; /* bytes in worst case */

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
    pub fn set_player_id(&mut self, id: u8) {
        if self.get_size() < 4 {
            return;
        }

        self.get_data_mut()[3] = id;
    }
}

pub struct Player {
    pub name: Vec<u8>,
    pub network_client: Option<Box<dyn NetPeer>>,

    pub currentRoomID: u32,
    pub isPlaying: bool,
    pub playerNumberInRoom: u8,
    pub synchOK: bool,

    pub skinPackage: PlayerSkin,

    pub joinTime: TimePoint,
    pub lastActivityTime: TimePoint,
}

impl Player {
    pub fn new() -> Self {
        let joinTime = time_now();
        Player {
            currentRoomID: 0,
            isPlaying: false,
            playerNumberInRoom: 0,
            synchOK: false,

            name: b"Anonymous".to_vec(),
            network_client: None,

            skinPackage: PlayerSkin::default(),

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

    pub fn set_skin(&mut self, data: &[u8]) {
        // Some basic package validation
        if data.len() <= std::mem::size_of::<MessageHeader>() + 5 /* id 1B + un-/compressed size 2*2B */
            || data.len() > SKINPKG_SIZE_LIMIT
        {
            println!("[error] Corrupt skin arrived from {}", self.to_string());
            return;
        }

        self.skinPackage.replace_with(data);
        self.lastActivityTime = time_now();
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
        self.send_code(NET_RESPONSE_CONNECT_OK)
    }
}
