//! Port of src/server/Room.cpp

use super::blob::Blob;
use super::clock::{time_now, TimePoint};
use super::player::Player;
use super::util::{cstr_field, strncpy_sec};
use smw::common_netplay::protocol_definitions::*;
use smw::common_netplay::protocol_packages::{CurrentRoom, GameHostInfo, MessageHeader, PlayerInfo, RoomChatMsg};
use smw::globals::Ptr;

const REMOTE_PKG_SIZE_LIMIT: usize = 20000; /* bytes in worst case */

pub struct Room {
    pub roomID: u32,
    pub name: [u8; NET_MAX_ROOM_NAME_LENGTH],
    pub password: [u8; NET_MAX_ROOM_PASSWORD_LENGTH],
    pub visible: bool,

    pub players: [Ptr<Player>; 4],
    pub hostPlayerNumber: u8, // 0-3 index in players[]
    pub playerCount: u8,

    pub mapPackage: Blob,

    pub gamemodeID: u8,
    pub gamemodeGoal: u16,
    pub gamemodeSettingsBlob: Blob,

    pub createTime: TimePoint,
    pub lastActivityTime: TimePoint,
}

impl Room {
    pub fn new(roomID: u32, name: &[u8], password: &[u8], mut host: Ptr<Player>) -> Self {
        let createTime = time_now();
        let mut room = Room {
            roomID,
            name: [0; NET_MAX_ROOM_NAME_LENGTH],
            password: [0; NET_MAX_ROOM_PASSWORD_LENGTH],
            visible: true, // TODO
            players: [Ptr::null(); 4],
            hostPlayerNumber: 0,
            playerCount: 1,
            mapPackage: Blob::new(),
            gamemodeID: 0,    // Classic
            gamemodeGoal: 10, // 10 lives
            gamemodeSettingsBlob: Blob::new(),
            createTime,
            lastActivityTime: createTime,
        };
        strncpy_sec(&mut room.name, name, NET_MAX_ROOM_NAME_LENGTH);
        strncpy_sec(&mut room.password, password, NET_MAX_ROOM_PASSWORD_LENGTH);

        room.players[0] = host;
        host.skinPackage.set_player_id(0);

        room
    }

    pub fn try_adding_player(&mut self, mut player: Ptr<Player>) {
        let mut p: u8 = 0;
        while p < 4 && player.currentRoomID == 0 {
            if self.players[p as usize].is_null() {
                self.players[p as usize] = player;
                player.currentRoomID = self.roomID;
                self.playerCount += 1;
                debug_assert!(self.playerCount <= 4);

                self.send_room_update(); // do this first to set correct remote player IDs
                self.send_blob_to(p, BlobRef::Map);
                self.send_blob_to(p, BlobRef::GameModeSettings);

                // send new player's skin to others
                self.share_skin_of(player);
                // send skins of others to new player
                for p in 0..4u8 {
                    if !self.players[p as usize].is_null() && self.players[p as usize] != player {
                        let other = self.players[p as usize];
                        self.send_blob_to(p, BlobRef::SkinOf(other));
                    }
                }
            } else {
                println!("  R-{}: slot {} taken by {:p}", self.roomID, p, self.players[p as usize].as_ptr());
            }
            p += 1;
        }
    }

    pub fn remove_player(&mut self, mut player: Ptr<Player>) {
        let mut searching = true;
        let mut p: u8 = 0;
        while p < 4 && searching {
            if self.players[p as usize] == player {
                self.players[p as usize] = Ptr::null();
                self.playerCount = self.playerCount.wrapping_sub(1);
                debug_assert!(self.playerCount <= 4);
                searching = false;

                // if this player was the host and there are players in the room
                // TODO: check for upload/download errors
                if self.hostPlayerNumber == p && self.playerCount > 0 {
                    self.hostPlayerNumber = 0xFF;
                    let mut pnexthost: u8 = 0;
                    while pnexthost < 4 && self.hostPlayerNumber > 4 {
                        // set the first available player as host
                        if !self.players[pnexthost as usize].is_null() {
                            self.hostPlayerNumber = pnexthost;
                        }
                        pnexthost += 1;
                    }
                    debug_assert!(self.hostPlayerNumber < 4);
                }
            }
            p += 1;
        }
        player.skinPackage.set_player_id(0xFF);
    }

    pub fn set_gamemode(&mut self, id: u8, goal: u16) {
        self.gamemodeID = id;
        self.gamemodeGoal = goal;
    }

    pub fn send_room_update(&mut self) {
        // TODO: error check

        // Crete package
        let mut package = CurrentRoom::new();
        package.roomID = self.roomID;
        package.hostPlayerNumber = self.hostPlayerNumber;
        strncpy_sec(&mut package.name, &self.name, NET_MAX_ROOM_NAME_LENGTH);

        for p in 0..4usize {
            if !self.players[p].is_null() {
                let name = self.players[p].name.clone();
                strncpy_sec(&mut package.playerName[p], &name, NET_MAX_PLAYER_NAME_LENGTH);
            } else {
                strncpy_sec(&mut package.playerName[p], b"(empty)", NET_MAX_PLAYER_NAME_LENGTH);
            }
        }

        package.gamemodeID = self.gamemodeID;
        package.gamemodeGoal = self.gamemodeGoal;

        // Send every player information about the other players.
        for p in 0..4u8 {
            if !self.players[p as usize].is_null() {
                println!("Sending ROOM_CHANGED:");
                println!("  id: {}", package.roomID);
                println!("  name: {}", cstr_field(&package.name));
                println!("  p1: {}", cstr_field(&package.playerName[0]));
                println!("  p2: {}", cstr_field(&package.playerName[1]));
                println!("  p3: {}", cstr_field(&package.playerName[2]));
                println!("  p4: {}", cstr_field(&package.playerName[3]));
                package.remotePlayerNumber = p;
                self.players[p as usize].send_data(package.as_bytes());
            }
        }
    }

    pub fn send_chat_message(&mut self, sender: Ptr<Player>, message: &[u8]) {
        let len = message.iter().position(|&c| c == 0).unwrap_or(message.len());
        if len == 0 {
            return;
        }

        // TODO: verify
        let mut senderNum: u8 = 0xFF;
        for p in 0..4u8 {
            if self.players[p as usize] == sender {
                senderNum = p;
                break;
            }
        }
        debug_assert!(senderNum != 0xFF);

        let mut package = RoomChatMsg::new();
        package.senderNum = senderNum;
        strncpy_sec(&mut package.message, message, NET_MAX_CHAT_MSG_LENGTH);

        // Send every player information about the other players.
        for p in 0..4usize {
            if !self.players[p].is_null() {
                self.players[p].send_data(package.as_bytes());
            }
        }
    }

    pub fn change_and_send_map(&mut self, data: &[u8]) {
        println!("chageandsendmap");
        debug_assert!(self.hostPlayerNumber < 4);

        // Some basic package validation
        if data.len() <= std::mem::size_of::<MessageHeader>() + 4 /* un-/compressed size 2*2B */ || data.len() > REMOTE_PKG_SIZE_LIMIT {
            println!("[warning] Corrupt map arrived from host in room {}", self.roomID);
            return;
        }

        self.mapPackage.replace_with(data);
        self.share_blob_except_host(BlobRef::Map);
    }

    pub fn change_and_send_game_mode_settings(&mut self, data: &[u8]) {
        println!("chageandsendgms");
        debug_assert!(self.hostPlayerNumber < 4);

        // Some basic package validation
        if data.len() <= std::mem::size_of::<MessageHeader>() || data.len() > REMOTE_PKG_SIZE_LIMIT {
            println!("[warning] Corrupt settings arrived from host in room {}", self.roomID);
            return;
        }

        self.gamemodeSettingsBlob.replace_with(data);
        self.share_blob_except_host(BlobRef::GameModeSettings);
    }

    pub fn share_blob(&mut self, blob: BlobRef) {
        for p in 0..4u8 {
            self.send_blob_to(p, blob);
        }
    }

    pub fn share_blob_except_host(&mut self, blob: BlobRef) {
        for p in 0..4u8 {
            if p == self.hostPlayerNumber {
                continue;
            }

            self.send_blob_to(p, blob);
        }
    }

    pub fn send_blob_to(&mut self, index: u8, blob: BlobRef) {
        println!("  sendBlobTo {}", index);
        debug_assert!(self.hostPlayerNumber < 4);
        debug_assert!(index < 4);
        if self.players[index as usize].is_null() {
            return;
        }

        let data: Vec<u8> = match blob {
            BlobRef::Map => self.mapPackage.get_data().to_vec(),
            BlobRef::GameModeSettings => self.gamemodeSettingsBlob.get_data().to_vec(),
            BlobRef::SkinOf(player) => player.skinPackage.get_data().to_vec(),
        };
        self.players[index as usize].send_data(&data);
    }

    pub fn share_skin_of(&mut self, mut sender: Ptr<Player>) {
        debug_assert!(!sender.is_null());

        if sender.skinPackage.empty() {
            return;
        }

        // TODO: verify

        // find out and set player id
        for p in 0..4u8 {
            if self.players[p as usize] == sender {
                sender.skinPackage.set_player_id(p);
                break;
            }
        }

        // send the fixed package to others
        for p in 0..4u8 {
            if !self.players[p as usize].is_null() && self.players[p as usize] != sender {
                self.send_blob_to(p, BlobRef::SkinOf(sender));
            }
        }
    }

    pub fn send_start_signal(&mut self) {
        debug_assert!(self.hostPlayerNumber < 4);

        let mut gamehost = self.players[self.hostPlayerNumber as usize];

        println!("sendStartSignal");

        let (gh_host, gh_ptr) = {
            let gh_netclient = gamehost.network_client.as_ref().unwrap();
            (gh_netclient.address_host(), gh_netclient.as_ref() as *const _ as *const u8)
        };
        let pkg_to_players = GameHostInfo::new(gh_host);

        println!("  pkg->players: [{}:{}]", { pkg_to_players.host }, NET_GAMEHOST_PORT);

        let mut pkg_to_gh = PlayerInfo::new();
        let mut playerIndex: u8 = 0;
        for p in 0..4u8 {
            if p != self.hostPlayerNumber && !self.players[p as usize].is_null() {
                let (host, port, ptr) = {
                    let client = self.players[p as usize].network_client.as_ref().unwrap();
                    (client.address_host(), client.address_port(), client.as_ref() as *const _ as *const u8)
                };
                debug_assert!(ptr != gh_ptr);

                pkg_to_gh.set_player(playerIndex, host, port);
                self.players[p as usize].send_data(pkg_to_players.as_bytes());
                playerIndex += 1;
            }
        }

        gamehost.send_data(pkg_to_gh.as_bytes());

        self.visible = false;
    }

    pub fn start_game_if_everybody_ready(&mut self) {}
}

/// `const Blob&` argument: which blob of the room (or of a player's skin) to send.
#[derive(Clone, Copy)]
pub enum BlobRef {
    Map,
    GameModeSettings,
    SkinOf(Ptr<Player>),
}
