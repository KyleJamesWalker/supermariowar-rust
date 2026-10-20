//! A lobby room: four player slots, the host's map and mode settings, and the messages the room sends its players.

use super::blob::Blob;
use super::clock::{time_now, TimePoint};
use super::log::verbose;
use super::player::Player;
use super::util::{cstr_field, strncpy_sec};
use smw::common_netplay::protocol_definitions::*;
use smw::common_netplay::protocol_packages::{CurrentRoom, GameHostInfo, MessageHeader, PlayerInfo, RoomChatMsg};
use smw::globals::Ptr;

const REMOTE_PKG_SIZE_LIMIT: usize = 20000; /* bytes in worst case */

/// A room of the lobby, owned by `SMWServer::rooms`.
pub struct Room {
    pub roomID: u32,
    pub name: [u8; NET_MAX_ROOM_NAME_LENGTH],
    /// NUL-terminated; a NUL first byte means no password.
    pub password: [u8; NET_MAX_ROOM_PASSWORD_LENGTH],
    /// Listed in the room list; false from match start until the host reopens the room.
    pub visible: bool,

    /// The players by slot, null for a free slot; they point into `SMWServer::players`.
    pub players: [Ptr<Player>; 4],
    pub hostPlayerNumber: u8, // 0-3 index in players[]
    pub playerCount: u8,

    /// The host's last `NET_NOTICE_MAP_CHANGE` message, relayed as is; the room lists only once it has one.
    pub mapPackage: Blob,

    pub gamemodeID: u8,
    pub gamemodeGoal: u16,
    /// The host's last `NET_NOTICE_GAMEMODESETTINGS` message, relayed as is.
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
            visible: true,
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

    /// Seats `player` in the first free slot and sends it the room, map, settings and skins; a full room leaves its `currentRoomID` 0.
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
                let joined = p;
                for other in 0..4u8 {
                    if !self.players[other as usize].is_null() && self.players[other as usize] != player {
                        let other = self.players[other as usize];
                        self.send_blob_to(joined, BlobRef::SkinOf(other));
                    }
                }
            } else {
                verbose!("  R-{}: slot {} taken by {:p}", self.roomID, p, self.players[p as usize].as_ptr());
            }
            p += 1;
        }
    }

    /// Frees `player`'s slot; if the host left, the remaining player in the lowest slot becomes host.
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

    pub fn is_host(&self, player: Ptr<Player>) -> bool {
        self.players.get(self.hostPlayerNumber as usize) == Some(&player)
    }

    pub fn set_gamemode(&mut self, id: u8, goal: u16) {
        self.gamemodeID = id;
        self.gamemodeGoal = goal;
    }

    /// Sends every player `NET_NOTICE_ROOM_CHANGE`, each with its own slot as `remotePlayerNumber`.
    pub fn send_room_update(&mut self) {
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
                verbose!(
                    "Sending ROOM_CHANGED: id {}, name {}, players [{}]",
                    { package.roomID },
                    cstr_field(&package.name),
                    package.playerName.iter().map(|n| cstr_field(n)).collect::<Vec<_>>().join(", ")
                );
                package.remotePlayerNumber = p;
                self.players[p as usize].send_data(package.as_bytes());
            }
        }
    }

    /// Sends a chat message from `sender` to the whole room, `sender` included; an empty message is dropped.
    pub fn send_chat_message(&mut self, sender: Ptr<Player>, message: &[u8]) {
        let len = message.iter().position(|&c| c == 0).unwrap_or(message.len());
        if len == 0 {
            return;
        }

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

        for p in 0..4usize {
            if !self.players[p].is_null() {
                self.players[p].send_data(package.as_bytes());
            }
        }
    }

    pub fn change_and_send_map(&mut self, data: &[u8]) {
        verbose!("chageandsendmap");
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
        verbose!("chageandsendgms");
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

    /// Sends one blob to the player in slot `index`, if there is one.
    pub fn send_blob_to(&mut self, index: u8, blob: BlobRef) {
        verbose!("  sendBlobTo {}", index);
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

    /// Stamps `sender`'s slot into its skin message and sends it to the room's other players.
    pub fn share_skin_of(&mut self, mut sender: Ptr<Player>) {
        debug_assert!(!sender.is_null());

        if sender.skinPackage.empty() {
            return;
        }

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

    /// Tells every player the match starts and hides the room from the room list.
    pub fn send_match_start(&mut self) {
        let pkg = MessageHeader::new(NET_L2P_MATCH_START);
        for p in 0..4 {
            if !self.players[p].is_null() {
                self.players[p].send_data(pkg.as_bytes());
            }
        }
        self.visible = false;
    }

    pub fn start_game_if_everybody_ready(&mut self) {}
}

/// Which blob to send: the room's map or mode settings, or a player's skin.
#[derive(Clone, Copy)]
pub enum BlobRef {
    Map,
    GameModeSettings,
    SkinOf(Ptr<Player>),
}
