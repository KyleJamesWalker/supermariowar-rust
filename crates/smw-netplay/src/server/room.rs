//! A lobby room: four player slots, which a client fills with one to three of its local players, the host's map and mode settings, and the messages the room sends its players.

use super::blob::Blob;
use super::clock::{time_now, TimePoint};
use super::log::verbose;
use super::player::Player;
use super::util::{cstr_field, strncpy_sec};
use smw::common_netplay::protocol_definitions::*;
use smw::common_netplay::protocol_packages::{CurrentRoom, MessageHeader, RoomChatMsg};
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

    /// The clients by slot, null for a free slot; they point into `SMWServer::players`. A client with several local
    /// players holds a slot for each.
    pub players: [Ptr<Player>; 4],
    /// The local player index on its client of each taken slot; a client's first player is 0.
    pub local: [u8; 4],
    /// The team of each taken slot, 0 to 3; a room with two or more players never has them all on one team.
    pub team: [u8; 4],
    pub hostPlayerNumber: u8, // 0-3 index in players[], the host's first player
    /// Taken slots.
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
            local: [0; 4],
            team: [0, 1, 2, 3],
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
        host.skins[0].set_player_id(0);

        room
    }

    /// The slot of `player`'s local player `local`.
    pub fn slot_of(&self, player: Ptr<Player>, local: usize) -> Option<u8> {
        (0..4u8).find(|&p| self.players[p as usize] == player && self.local[p as usize] as usize == local)
    }

    /// How many slots `player` holds.
    fn local_count(&self, player: Ptr<Player>) -> usize {
        self.players.iter().filter(|&&p| p == player).count()
    }

    /// The slot of each client's first player, which messages for the client go to.
    fn client_slots(&self) -> impl Iterator<Item = u8> + '_ {
        (0..4u8).filter(|&p| !self.players[p as usize].is_null() && self.local[p as usize] == 0)
    }

    /// Players on each team.
    fn team_sizes(&self) -> [usize; 4] {
        let mut sizes = [0; 4];
        for p in 0..4 {
            if !self.players[p].is_null() {
                sizes[self.team[p] as usize] += 1;
            }
        }
        sizes
    }

    /// Seats `player`'s local player `local` in the first free slot, on a team of its own: the slot's, else the first
    /// empty one. Four teams for at most four players always leave one empty.
    fn seat(&mut self, player: Ptr<Player>, local: u8) -> Option<u8> {
        let p = (0..4u8).find(|&p| self.players[p as usize].is_null())?;
        let sizes = self.team_sizes();
        self.team[p as usize] = if sizes[p as usize] == 0 { p } else { (0..4u8).find(|&t| sizes[t as usize] == 0).unwrap_or(p) };
        self.players[p as usize] = player;
        self.local[p as usize] = local;
        self.playerCount += 1;
        Some(p)
    }

    /// When a leaving player left everyone else on one team, each remaining player gets its own slot's team again.
    fn split_single_team(&mut self) {
        if self.playerCount > 1 && self.team_sizes().iter().any(|&n| n == self.playerCount as usize) {
            self.team = [0, 1, 2, 3];
        }
    }

    /// Puts `player`'s local player `local` on `team`; false, with nothing changed, for a team out of range, a player not
    /// in the room, a running match, or a move that would put every player of the room on one team.
    pub fn set_team(&mut self, player: Ptr<Player>, local: usize, team: u8) -> bool {
        let Some(slot) = self.slot_of(player, local) else { return false };
        if !self.visible || team >= 4 {
            return false;
        }
        let old = self.team[slot as usize];
        self.team[slot as usize] = team;
        if self.playerCount > 1 && self.team_sizes()[team as usize] == self.playerCount as usize {
            self.team[slot as usize] = old;
            return false;
        }
        self.send_room_update();
        true
    }

    /// Sets how many local players `player` brings and their names; false, with nothing changed, when the room has too
    /// few free slots for the new ones or its match is running.
    pub fn set_local_players(&mut self, mut player: Ptr<Player>, count: usize, names: [Vec<u8>; NET_MAX_LOCAL_PLAYERS]) -> bool {
        let current = self.local_count(player);
        if !self.visible || current == 0 || !(1..=NET_MAX_LOCAL_PLAYERS).contains(&count) || count > current + (4 - self.playerCount as usize) {
            return false;
        }
        // A name that reads as a free slot would make every client drop that player.
        player.localNames = names.map(|name| if name.eq_ignore_ascii_case(b"(empty)") { Vec::new() } else { name });
        for local in current..count {
            self.seat(player, local as u8);
        }
        for local in count..current {
            if let Some(p) = self.slot_of(player, local) {
                self.players[p as usize] = Ptr::null();
                self.playerCount -= 1;
            }
            player.skins[local].set_player_id(0xFF);
        }
        self.split_single_team();
        self.send_room_update();
        for local in current..count {
            self.share_skin_of(player, local);
        }
        true
    }

    /// Seats `player`'s first local player in the first free slot and sends it the room, map, settings and skins; a
    /// full room leaves its `currentRoomID` 0.
    pub fn try_adding_player(&mut self, mut player: Ptr<Player>) {
        let Some(joined) = self.seat(player, 0) else {
            verbose!("  R-{}: no free slot", self.roomID);
            return;
        };
        player.currentRoomID = self.roomID;

        self.send_room_update(); // do this first to set correct remote player IDs
        self.send_blob_to(joined, BlobRef::Map);
        self.send_blob_to(joined, BlobRef::GameModeSettings);

        // send new player's skin to others
        self.share_skin_of(player, 0);
        // send skins of others to new player
        for other in 0..4usize {
            let owner = self.players[other];
            if !owner.is_null() && owner != player {
                self.send_blob_to(joined, BlobRef::SkinOf(owner, self.local[other] as usize));
            }
        }
    }

    /// Frees `player`'s slots; if the host left, the client in the lowest remaining slot becomes host.
    pub fn remove_player(&mut self, mut player: Ptr<Player>) {
        for p in 0..4usize {
            if self.players[p] == player {
                self.players[p] = Ptr::null();
                self.playerCount = self.playerCount.wrapping_sub(1);
                debug_assert!(self.playerCount <= 4);
            }
        }
        self.split_single_team();
        if self.players[self.hostPlayerNumber as usize].is_null() {
            let next = self.client_slots().next();
            if let Some(next) = next {
                self.hostPlayerNumber = next;
            }
        }
        for skin in &mut player.skins {
            skin.set_player_id(0xFF);
        }
    }

    pub fn is_host(&self, player: Ptr<Player>) -> bool {
        self.players.get(self.hostPlayerNumber as usize) == Some(&player)
    }

    pub fn set_gamemode(&mut self, id: u8, goal: u16) {
        self.gamemodeID = id;
        self.gamemodeGoal = goal;
    }

    /// Sends every client `NET_NOTICE_ROOM_CHANGE` with its own first slot as `remotePlayerNumber` and its slots marked.
    pub fn send_room_update(&mut self) {
        let mut package = CurrentRoom::new();
        package.roomID = self.roomID;
        package.hostPlayerNumber = self.hostPlayerNumber;
        strncpy_sec(&mut package.name, &self.name, NET_MAX_ROOM_NAME_LENGTH);

        for p in 0..4usize {
            if !self.players[p].is_null() {
                let name = self.players[p].local_name(self.local[p] as usize);
                strncpy_sec(&mut package.playerName[p], &name, NET_MAX_PLAYER_NAME_LENGTH);
            } else {
                strncpy_sec(&mut package.playerName[p], b"(empty)", NET_MAX_PLAYER_NAME_LENGTH);
            }
        }

        package.gamemodeID = self.gamemodeID;
        package.gamemodeGoal = self.gamemodeGoal;
        package.team = self.team;

        // Send every player information about the other players.
        for p in self.client_slots().collect::<Vec<_>>() {
            let client = self.players[p as usize];
            for q in 0..4 {
                package.localPlayer[q] = if self.players[q] == client { self.local[q] } else { 0xFF };
            }
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

    /// Sends a chat message from `sender` to the whole room, `sender` included; an empty message is dropped.
    pub fn send_chat_message(&mut self, sender: Ptr<Player>, message: &[u8]) {
        let len = message.iter().position(|&c| c == 0).unwrap_or(message.len());
        if len == 0 {
            return;
        }

        let Some(senderNum) = self.slot_of(sender, 0) else { return };

        let mut package = RoomChatMsg::new();
        package.senderNum = senderNum;
        strncpy_sec(&mut package.message, message, NET_MAX_CHAT_MSG_LENGTH);

        for p in self.client_slots().collect::<Vec<_>>() {
            self.players[p as usize].send_data(package.as_bytes());
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
        for p in self.client_slots().collect::<Vec<_>>() {
            self.send_blob_to(p, blob);
        }
    }

    pub fn share_blob_except_host(&mut self, blob: BlobRef) {
        for p in self.client_slots().collect::<Vec<_>>() {
            if p == self.hostPlayerNumber {
                continue;
            }

            self.send_blob_to(p, blob);
        }
    }

    /// Sends one blob to the client in slot `index`, if there is one.
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
            BlobRef::SkinOf(player, local) => player.skins[local].get_data().to_vec(),
        };
        self.players[index as usize].send_data(&data);
    }

    /// Stamps the slot of `sender`'s local player `local` into its skin message and sends it to the room's other clients.
    pub fn share_skin_of(&mut self, mut sender: Ptr<Player>, local: usize) {
        debug_assert!(!sender.is_null());

        let Some(slot) = self.slot_of(sender, local) else { return };
        if sender.skins[local].empty() {
            return;
        }
        sender.skins[local].set_player_id(slot);

        for p in self.client_slots().collect::<Vec<_>>() {
            if self.players[p as usize] != sender {
                self.send_blob_to(p, BlobRef::SkinOf(sender, local));
            }
        }
    }

    /// Tells every player the match starts and hides the room from the room list.
    pub fn send_match_start(&mut self) {
        let pkg = MessageHeader::new(NET_L2P_MATCH_START);
        for p in self.client_slots().collect::<Vec<_>>() {
            self.players[p as usize].send_data(pkg.as_bytes());
        }
        self.visible = false;
    }
}

/// Which blob to send: the room's map or mode settings, or the skin of a client's local player.
#[derive(Clone, Copy)]
pub enum BlobRef {
    Map,
    GameModeSettings,
    SkinOf(Ptr<Player>, usize),
}
