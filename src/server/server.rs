//! Port of src/server/server.cpp

use super::clock::time_now;
use super::log::{log, log_close, log_init, log_silently};
use super::network_layer::NetworkLayer;
use super::network_layer_enet::NetworkLayerENet;
use super::player::Player;
use super::room::Room;
use super::util::cstr_field;
use smw::common_netplay::network_interface::{NetPeer, NetworkEventHandler};
use smw::common_netplay::protocol_definitions::*;
use smw::common_netplay::protocol_packages::{ClientConnection, JoinRoom, MessageHeader, NewRoom, NewRoomCreated, RoomChatMsg, RoomInfo, ServerInfo};
use smw::globals::Ptr;
use super::unordered_map::UnorderedMap;

static mut netLayer: Option<NetworkLayerENet> = None;

fn net_layer() -> &'static mut NetworkLayerENet {
    unsafe { netLayer.get_or_insert_with(NetworkLayerENet::new) }
}

pub struct SMWServer {
    serverName: String,
    serverInfo: ServerInfo,

    currentPlayerCount: u32,
    maxPlayerCount: u32,
    players: UnorderedMap<u64, Player>,

    roomCreateID: u32, // temporary
    currentRoomCount: u32,
    rooms: UnorderedMap<u32, Room>,
}

impl SMWServer {
    pub fn new() -> Self {
        SMWServer {
            serverName: "SMW Server".to_string(),
            serverInfo: ServerInfo::with("SMW Server", 0, 20 /* maxPlayerCount */),
            currentPlayerCount: 0,
            maxPlayerCount: 20,
            players: UnorderedMap::new(),
            roomCreateID: 1,
            currentRoomCount: 0,
            rooms: UnorderedMap::new(),
        }
    }

    pub fn init(&mut self, config_path: &str) -> bool {
        if !log_init() {
            return false;
        }
        log_silently("-- Server started --");

        self.read_config(config_path);

        if !net_layer().init(self.maxPlayerCount as u64) {
            return false;
        }

        true
    }

    fn read_config(&mut self, config_path: &str) {
        let mut final_path = config_path.to_string();
        let mut configFile = std::fs::read(&final_path).ok();
        if configFile.is_none() {
            final_path += ".txt";
            configFile = std::fs::read(&final_path).ok();
        }
        let Some(content) = configFile else {
            log(&format!("[warning] Configuration file `{}` not found, using default values.", final_path));
            println!("  server name: {}", cstr_field(&self.serverInfo.name));
            println!("  max players: {}", { self.serverInfo.maxPlayerCount } as i32);
            return;
        };

        log(&format!("[info] Found configuration file `{}`", final_path));

        // `configFile >> keyword; std::getline(configFile, value);`
        let mut pos = 0usize;
        let mut reading = true;
        while reading {
            while pos < content.len() && content[pos].is_ascii_whitespace() {
                pos += 1;
            }
            let kstart = pos;
            while pos < content.len() && !content[pos].is_ascii_whitespace() {
                pos += 1;
            }
            let keyword = &content[kstart..pos];
            if pos >= content.len() {
                reading = false;
            } else {
                let vstart = pos;
                while pos < content.len() && content[pos] != b'\n' {
                    pos += 1;
                }
                let mut value = &content[vstart..pos];
                let eof = pos >= content.len();
                if !eof {
                    pos += 1;
                }
                if !value.is_empty() {
                    value = &value[1..]; // remove separating space
                }

                if eof {
                    reading = false;
                }

                if keyword[0] != b'#' {
                    // not a comment line
                    if keyword == b"name" {
                        let mut name = value.to_vec();
                        name.resize(31, 0);
                        let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
                        self.serverName = String::from_utf8_lossy(&name[..end]).into_owned();
                        self.serverInfo.name = [0; 32];
                        self.serverInfo.name[..end].copy_from_slice(&name[..end]);
                        self.serverInfo.name[31] = 0;
                    }
                    if keyword == b"maxplayers" {
                        let num = strtol10(value) as u16;
                        if num != 0 {
                            self.maxPlayerCount = num as u32;
                            self.serverInfo.maxPlayerCount = num as u32;
                        }
                    }

                    // TODO: implement password
                }
            }
        }
    }

    pub fn update(&mut self, _running: &mut bool) {
        net_layer().listen(self);
    }

    /*

        MESSAGES

    */

    fn send_server_info(&mut self, client: &mut dyn NetPeer) {
        client.send_reliable(self.serverInfo.as_bytes());
    }

    fn send_code(&mut self, client: &mut dyn NetPeer, code: u8) {
        let msg = MessageHeader::new(code);
        client.send_reliable(msg.as_bytes());
    }

    fn send_code_player(&mut self, playerID: u64, code: u8) {
        if let Some(p) = self.players.get_mut(&playerID) {
            p.send_code(code);
        }
    }

    fn player(&mut self, playerID: u64) -> Option<Ptr<Player>> {
        self.players.get_mut(&playerID).map(Ptr::from_mut)
    }

    fn player_connects_server(&mut self, playerID: u64, data: &[u8]) {
        let Some(mut player) = self.player(playerID) else { return };

        if data.len() != std::mem::size_of::<ClientConnection>() {
            println!("[error] Corrupt package arrived from {}", playerID);
            return;
        }

        let package = ClientConnection::from_bytes(data);

        let end = package.playerName.iter().position(|&c| c == 0).unwrap_or(package.playerName.len());
        player.name = package.playerName[..end].to_vec();
        player.send_connect_ok();

        log(&format!("{} : <{}> connected.", player.network_client.as_ref().unwrap().address_as_string(), cstr_field(&package.playerName)));
    }

    fn remove_inactive_players(&mut self) {
        if self.maxPlayerCount as usize <= self.players.len() {}
    }

    fn send_visible_room_entries(&mut self, client: &mut dyn NetPeer) {
        if self.rooms.is_empty() {
            self.send_code(client, NET_RESPONSE_NO_ROOMS);
            return;
        }

        println!("Sending {} rooms:", self.rooms.len());
        for room in self.rooms.values() {
            // only visible if public and has valid map
            if room.visible && !room.mapPackage.empty() {
                let mut roomInfo = RoomInfo::new();
                {
                    roomInfo.roomID = room.roomID;

                    roomInfo.name.copy_from_slice(&room.name);
                    roomInfo.name[NET_MAX_ROOM_NAME_LENGTH - 1] = 0;

                    roomInfo.passwordRequired = 0;
                    if room.password[0] != 0 {
                        roomInfo.passwordRequired = 1;
                    }

                    roomInfo.currentPlayerCount = 0;
                    for p in 0..4 {
                        if !room.players[p].is_null() {
                            roomInfo.currentPlayerCount += 1;
                        }
                    }

                    roomInfo.gamemodeID = room.gamemodeID;

                    println!(
                        "  room {}: {{{}; pass?: {}; players: {}}}",
                        { roomInfo.roomID } as i32,
                        cstr_field(&roomInfo.name),
                        roomInfo.passwordRequired,
                        roomInfo.currentPlayerCount
                    );
                }

                client.send_reliable(roomInfo.as_bytes());
            }
        }
    }

    fn player_creates_room(&mut self, playerID: u64, data: &[u8]) {
        let Some(mut player) = self.player(playerID) else { return };
        if player.currentRoomID != 0 || player.isPlaying {
            return;
        }

        if data.len() != std::mem::size_of::<NewRoom>() {
            println!("[error] Corrupt package arrived from {}", playerID);
            return;
        }

        // input
        let pkg = NewRoom::from_bytes(data);

        // create
        let mut room = Room::new(self.roomCreateID, &pkg.name, &pkg.password, player);
        room.set_gamemode(pkg.gamemodeID, pkg.gamemodeGoal);
        let (roomID, roomName, roomPassword) = (room.roomID, cstr_field(&room.name), cstr_field(&room.password));
        self.rooms.insert(self.roomCreateID, room);

        player.currentRoomID = self.roomCreateID;
        player.isPlaying = false;

        // output
        let mut package = NewRoomCreated::new();
        package.roomID = self.roomCreateID;
        player.send_data(package.as_bytes());

        log(&format!("New room by {} : {{id: {}; name: {}; pw: {}}}", player.to_string(), roomID as i32, roomName, roomPassword));

        self.roomCreateID += 1; // increase global ID
    }

    fn player_joins_room(&mut self, playerID: u64, data: &[u8]) {
        let Some(mut player) = self.player(playerID) else { return };
        if player.currentRoomID != 0 || player.isPlaying {
            return; // TODO: warning
        }

        if data.len() != std::mem::size_of::<JoinRoom>() {
            println!("[error] Corrupt package arrived from {}", playerID);
            return;
        }

        let pkg = JoinRoom::from_bytes(data);

        println!("{} wants to join room {}", String::from_utf8_lossy(&player.name), { pkg.roomID });

        let Some(room) = self.rooms.get_mut(&{ pkg.roomID }) else {
            println!("No such room! ({})", { pkg.roomID });
            return;
        };

        // Find first empty slot
        room.try_adding_player(player);

        if player.currentRoomID != 0 {
            player.send_code(NET_RESPONSE_JOIN_OK);
        } else {
            player.send_code(NET_RESPONSE_ROOM_FULL);
        }

        player.isPlaying = false;
    }

    fn player_leaves_room(&mut self, playerID: u64) {
        let Some(mut player) = self.player(playerID) else { return };
        if player.currentRoomID == 0 || player.isPlaying {
            return;
        }

        let roomID = player.currentRoomID;
        let Some(room) = self.rooms.get_mut(&roomID) else { return };

        // Search for the room in which the player is
        room.remove_player(player);

        // If the room is now empty, delete it.
        if room.playerCount == 0 {
            self.rooms.remove(&roomID);
            log(&format!("Room {} erased.", roomID));
        } else {
            room.send_room_update();
        }

        player.currentRoomID = 0;
        player.isPlaying = false;
        player.lastActivityTime = time_now();
    }

    fn host_changes_map(&mut self, playerID: u64, data: &[u8]) {
        println!("hostChangesMap");
        let Some(player) = self.player(playerID) else { return };
        let roomID = player.currentRoomID;
        if roomID == 0 || player.isPlaying {
            return;
        }

        let Some(room) = self.rooms.get_mut(&roomID) else { return };

        room.change_and_send_map(data);
        println!("room open!");
    }

    fn host_changes_game_mode_settings(&mut self, playerID: u64, data: &[u8]) {
        println!("hostChangesGameModeSettings");
        let Some(player) = self.player(playerID) else { return };
        let roomID = player.currentRoomID;
        if roomID == 0 || player.isPlaying {
            return;
        }

        let Some(room) = self.rooms.get_mut(&roomID) else { return };

        room.change_and_send_game_mode_settings(data);
    }

    fn player_changes_skin(&mut self, playerID: u64, data: &[u8]) {
        let Some(mut player) = self.player(playerID) else { return };
        if player.isPlaying {
            return;
        }

        player.set_skin(data);
        println!("Player {} changed skin", String::from_utf8_lossy(&player.name));

        let roomID = player.currentRoomID;
        if roomID == 0 {
            return;
        }
        let Some(room) = self.rooms.get_mut(&roomID) else { return };

        room.share_skin_of(player);
    }

    fn player_sends_chat_msg(&mut self, playerID: u64, data: &[u8]) {
        let Some(mut player) = self.player(playerID) else { return };
        let roomID = player.currentRoomID;
        if roomID == 0 || player.isPlaying {
            return;
        }

        if !self.rooms.contains_key(&roomID) {
            return;
        }

        if data.len() != std::mem::size_of::<RoomChatMsg>() {
            println!("[error] Corrupt package arrived from {}", playerID);
            return;
        }

        let pkg = RoomChatMsg::from_bytes(data);

        self.rooms.get_mut(&roomID).unwrap().send_chat_message(player, &pkg.message);
        player.lastActivityTime = time_now();
    }

    fn player_starts_room(&mut self, playerID: u64) {
        let Some(mut player) = self.player(playerID) else { return };
        let roomID = player.currentRoomID;
        if roomID == 0 || player.isPlaying {
            return;
        }

        let Some(room) = self.rooms.get_mut(&roomID) else { return };
        debug_assert!(room.hostPlayerNumber < 4);

        // only host can start
        if room.players[room.hostPlayerNumber as usize] != player {
            return;
        }

        room.send_start_signal();
        player.lastActivityTime = time_now();
        log(&format!("[info] Room #{} starting.", roomID as i32));
    }

    fn start_room_if_everyone_ready(&mut self, _playerID: u64) {}

    pub fn cleanup(&mut self) {
        // TODO: Make sure it runs only once.
        self.rooms.clear();
        self.players.clear();

        net_layer().cleanup();

        log_silently("-- Server stopped --");
        log_close();
    }
}

impl Drop for SMWServer {
    fn drop(&mut self) {
        self.cleanup();
    }
}

impl NetworkEventHandler for SMWServer {
    fn on_connect(&mut self, new_client: Box<dyn NetPeer>) {
        println!("Connect event [{}].", new_client.get_player_id());

        // Is there a player already with this address+port?
        self.players.remove(&new_client.get_player_id());

        self.remove_inactive_players();

        if self.maxPlayerCount as usize <= self.players.len() {
            let mut new_client = new_client;
            self.send_code(new_client.as_mut(), NET_RESPONSE_CONNECT_SERVERFULL);
            log("[warning] Server full...");
            // TODO: disconnect
            std::mem::forget(new_client);
        } else {
            let address = new_client.address_as_string();
            self.players.index_or_insert_with(new_client.get_player_id(), Player::new).set_client(new_client);
            log(&format!("[info] New connection from {}", address));
        }
    }

    fn on_disconnect(&mut self, client: &mut dyn NetPeer) {
        println!("Disconnect event [{}].", client.get_player_id());
        let playerID = client.get_player_id();
        if self.players.contains_key(&playerID) {
            self.player_leaves_room(playerID);
            let player = self.players.get(&playerID).unwrap();
            log(&format!("[info] {}@{} disconnected.", String::from_utf8_lossy(&player.name), player.network_client.as_ref().unwrap().address_as_string()));
            self.players.remove(&playerID);
        } else {
            println!("No such player!");
        }
    }

    fn on_receive(&mut self, client: &mut dyn NetPeer, data: &[u8]) {
        if data.len() < 3 {
            return;
        }

        let versionMajor = data[0];
        let versionMinor = data[1];
        let messageType = data[2];

        let playerID = client.get_player_id();

        if versionMajor != NET_PROTOCOL_VERSION_MAJOR || versionMinor != NET_PROTOCOL_VERSION_MINOR {
            self.send_code(client, NET_RESPONSE_BADPROTOCOL);
            return;
        }

        match messageType {
            //
            // Server-related
            //
            NET_REQUEST_SERVERINFO => self.send_server_info(client),

            NET_REQUEST_CONNECT => self.player_connects_server(playerID, data),

            NET_REQUEST_LEAVE_SERVER => self.player_leaves_room(playerID),

            //
            // Room-related
            //
            NET_REQUEST_ROOM_LIST => self.send_visible_room_entries(client),

            NET_REQUEST_CREATE_ROOM => self.player_creates_room(playerID, data),

            NET_REQUEST_JOIN_ROOM => self.player_joins_room(playerID, data),

            NET_REQUEST_LEAVE_ROOM => self.player_leaves_room(playerID),

            NET_NOTICE_MAP_CHANGE => self.host_changes_map(playerID, data),

            NET_NOTICE_GAMEMODESETTINGS => self.host_changes_game_mode_settings(playerID, data),

            NET_NOTICE_SKIN_CHANGE => self.player_changes_skin(playerID, data),

            NET_NOTICE_ROOM_CHAT_MSG => self.player_sends_chat_msg(playerID, data),

            //
            // Room start
            //
            NET_G2L_START_ROOM => self.player_starts_room(playerID),

            _ => {
                print!("Unknown: {}", messageType);
                println!();
            }
        }
    }
}

/// `strtol(value.c_str(), NULL, 10)`
fn strtol10(s: &[u8]) -> i64 {
    let mut i = 0;
    while i < s.len() && (s[i] == b' ' || (b'\t'..=b'\r').contains(&s[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((s[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        -v
    } else {
        v
    }
}
