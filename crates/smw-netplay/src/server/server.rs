//! The lobby server: player connections, rooms, and the hand-off of a starting room to netplay v2 signaling.

use super::clock::time_now;
use super::log::{log, log_close, log_init, log_silently};
use super::network_layer::NetworkLayer;
use super::player::Player;
use super::room::Room;
use super::util::cstr_field;
use smw::common_netplay::network_interface::{NetPeer, NetworkEventHandler};
use smw::common_netplay::protocol_definitions::*;
use smw::common_netplay::protocol_packages::{ClientConnection, JoinRoom, MatchRoom, MessageHeader, NewRoom, NewRoomCreated, RoomChatMsg, RoomInfo, ServerInfo};
use super::match_room;
use smw::globals::Ptr;
use super::unordered_map::UnorderedMap;

/// The network layer every [`SMWServer`] listens on.
static mut netLayer: Option<Box<dyn NetworkLayer>> = None;

/// Installs the network layer; call it before [`SMWServer::init`].
pub fn set_net_layer(layer: Box<dyn NetworkLayer>) {
    unsafe { netLayer = Some(layer) };
}

fn net_layer() -> &'static mut dyn NetworkLayer {
    unsafe { netLayer.as_deref_mut().expect("set_net_layer before SMWServer::init") }
}

/// The lobby: connected players, their rooms, and starting a room's match.
pub struct SMWServer {
    serverName: String,
    /// The `NET_RESPONSE_SERVERINFO` reply, kept in step with the config.
    serverInfo: ServerInfo,

    currentPlayerCount: u32,
    maxPlayerCount: u32,
    /// Connected players, by `NetPeer::get_player_id`.
    players: UnorderedMap<u64, Player>,

    /// The ID the next new room gets; IDs start at 1, since a player's room ID 0 means no room.
    roomCreateID: u32,
    currentRoomCount: u32,
    rooms: UnorderedMap<u32, Room>,

    /// Where a starting room's players meet.
    pub v2: V2Settings,
    /// At most this many rooms at a time; a game hosting on its own network allows one.
    pub max_rooms: Option<usize>,
}

/// Where netplay v2 players meet, from `SMW_SIGNAL_URL`, `SMW_TURN_URL` and `SMW_TURN_SECRET`.
/// Without a signaling URL the lobby sends no `NET_L2P_MATCH_ROOM`.
#[derive(Clone, Debug, Default)]
pub struct V2Settings {
    /// The signaling server; a match meets at `<signal_url>/<room ID>`. A leading `/` means the lobby's own server.
    pub signal_url: String,
    /// The TURN server handed to clients.
    pub turn_url: String,
    /// The TURN server's shared secret; without it or `turn_url` players get no TURN server.
    pub turn_secret: String,
}

impl V2Settings {
    pub fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).unwrap_or_default();
        V2Settings { signal_url: var("SMW_SIGNAL_URL"), turn_url: var("SMW_TURN_URL"), turn_secret: var("SMW_TURN_SECRET") }
    }
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
            v2: V2Settings::from_env(),
            max_rooms: None,
        }
    }

    pub fn set_max_players(&mut self, max: u32) {
        self.maxPlayerCount = max;
        self.serverInfo.maxPlayerCount = max;
    }

    /// Opens `serverlog.txt`, reads the config at `config_path` (or `<config_path>.txt`) and starts the network layer.
    pub fn init(&mut self, config_path: &str) -> bool {
        if !log_init() {
            return false;
        }
        self.init_without_log_file(config_path)
    }

    /// `init` that logs to stdout only, for a lobby inside a game, whose working directory may not be writable.
    pub fn init_without_log_file(&mut self, config_path: &str) -> bool {
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

        // Each line is `<keyword> <value>`; a keyword starting with `#` makes the line a comment.
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
                }
            }
        }
    }

    /// Handles the network events that arrived since the last call.
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

        player.name = self.unique_name(playerID, cstr_bytes(&package.playerName));
        player.send_connect_ok();

        log(&format!("{} : <{}> connected.", player.network_client.as_ref().unwrap().address_as_string(), cstr_field(&package.playerName)));
    }

    /// The requested name, or the first free `<name> <n>` if another player has it.
    fn unique_name(&self, playerID: u64, requested: &[u8]) -> Vec<u8> {
        let base: &[u8] = if requested.is_empty() { b"Player" } else { requested };
        let taken = |name: &[u8]| self.players.iter().any(|(&id, p)| id != playerID && p.name.eq_ignore_ascii_case(name));
        if !taken(base) {
            return base.to_vec();
        }
        (2..).map(|n| {
            let suffix = format!(" {}", n);
            let keep = base.len().min(NET_MAX_PLAYER_NAME_LENGTH - 1 - suffix.len());
            [&base[..keep], suffix.as_bytes()].concat()
        }).find(|name| !taken(name)).unwrap()
    }

    fn remove_inactive_players(&mut self) {
        if self.maxPlayerCount as usize <= self.players.len() {}
    }

    fn send_visible_room_entries(&mut self, client: &mut dyn NetPeer) {
        if !self.rooms.values().any(|room| room.visible && !room.mapPackage.empty()) {
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

        if self.max_rooms.is_some_and(|max| self.rooms.len() >= max) {
            player.send_code(NET_RESPONSE_CREATE_ERROR);
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
        let (roomID, roomName, hasPassword) = (room.roomID, cstr_field(&room.name), room.password[0] != 0);
        self.rooms.insert(self.roomCreateID, room);

        player.currentRoomID = self.roomCreateID;
        player.isPlaying = false;

        // output
        let mut package = NewRoomCreated::new();
        package.roomID = self.roomCreateID;
        player.send_data(package.as_bytes());

        log(&format!("New room by {} : {{id: {}; name: {}; pw: {}}}", player.to_string(), roomID as i32, roomName, hasPassword));

        self.roomCreateID += 1; // increase global ID
    }

    fn player_joins_room(&mut self, playerID: u64, data: &[u8]) {
        let Some(mut player) = self.player(playerID) else { return };
        if player.currentRoomID != 0 || player.isPlaying {
            return;
        }

        if data.len() != std::mem::size_of::<JoinRoom>() {
            println!("[error] Corrupt package arrived from {}", playerID);
            return;
        }

        let pkg = JoinRoom::from_bytes(data);

        println!("{} wants to join room {}", String::from_utf8_lossy(&player.name), { pkg.roomID });

        let Some(room) = self.rooms.get_mut(&{ pkg.roomID }).filter(|room| room.visible) else {
            println!("No such room! ({})", { pkg.roomID });
            player.send_code(NET_RESPONSE_ROOM_NOT_FOUND);
            return;
        };

        if cstr_bytes(&room.password) != cstr_bytes(&pkg.password) {
            player.send_code(NET_RESPONSE_WRONG_PASSWORD);
            return;
        }

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

    /// The room `playerID` hosts, unless the player is playing.
    fn hosted_room(&mut self, playerID: u64) -> Option<&mut Room> {
        let player = self.player(playerID)?;
        if player.isPlaying {
            return None;
        }
        self.rooms.get_mut(&player.currentRoomID).filter(|room| room.is_host(player))
    }

    fn host_changes_map(&mut self, playerID: u64, data: &[u8]) {
        println!("hostChangesMap");
        let Some(room) = self.hosted_room(playerID) else { return };

        room.change_and_send_map(data);
        println!("room open!");
    }

    fn host_changes_game_mode_settings(&mut self, playerID: u64, data: &[u8]) {
        println!("hostChangesGameModeSettings");
        let Some(room) = self.hosted_room(playerID) else { return };

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

    /// The room's host is back from a match: the room shows in the room list again, for the next one.
    fn host_reopens_room(&mut self, playerID: u64) {
        let Some(room) = self.hosted_room(playerID) else { return };
        room.visible = true;
        log(&format!("[info] Room #{} open again.", room.roomID as i32));
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
        let v2 = self.v2.clone();
        let Some(room) = self.hosted_room(playerID) else { return };

        if !Self::send_match_room(&v2, room) {
            log(&format!("[error] Room #{} cannot start: netplay needs SMW_SIGNAL_URL.", roomID as i32));
            return;
        }
        room.send_match_start();
        player.lastActivityTime = time_now();
        log(&format!("[info] Room #{} starting.", roomID as i32));
    }

    /// Gives every player of a starting room the same new signaling room and their own TURN credentials.
    fn send_match_room(v2: &V2Settings, room: &mut Room) -> bool {
        if v2.signal_url.is_empty() {
            return false;
        }
        let id = match_room::new_room_id();
        let url = format!("{}/{}", v2.signal_url.trim_end_matches('/'), id);
        let slots: Vec<usize> = (0..4).filter(|&p| !room.players[p].is_null()).collect();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        for (index, &slot) in slots.iter().enumerate() {
            let (username, credential) = if v2.turn_url.is_empty() || v2.turn_secret.is_empty() {
                (String::new(), String::new())
            } else {
                match_room::turn_credentials(&v2.turn_secret, now, &format!("{}-{}", &id[..8], index))
            };
            let turn_url = if username.is_empty() { "" } else { v2.turn_url.as_str() };
            let Some(pkg) = MatchRoom::with(slots.len() as u8, &url, turn_url, &username, &credential) else {
                log(&format!("[error] Netplay v2 settings too long for NET_L2P_MATCH_ROOM: {url}"));
                return false;
            };
            room.players[slot].send_data(pkg.as_bytes());
        }
        true
    }

    fn start_room_if_everyone_ready(&mut self, _playerID: u64) {}

    /// Drops every room and player and stops the network layer. `Drop` calls it too, so an explicit call runs it twice.
    pub fn cleanup(&mut self) {
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
    fn on_connect(&mut self, _: &mut (), new_client: Box<dyn NetPeer>) {
        println!("Connect event [{}].", new_client.get_player_id());

        // Is there a player already with this address+port?
        self.players.remove(&new_client.get_player_id());

        self.remove_inactive_players();

        if self.maxPlayerCount as usize <= self.players.len() {
            let mut new_client = new_client;
            self.send_code(new_client.as_mut(), NET_RESPONSE_CONNECT_SERVERFULL);
            log("[warning] Server full...");
            // The peer is neither kept nor disconnected.
            std::mem::forget(new_client);
        } else {
            let address = new_client.address_as_string();
            self.players.index_or_insert_with(new_client.get_player_id(), Player::new).set_client(new_client);
            log(&format!("[info] New connection from {}", address));
        }
    }

    fn on_disconnect(&mut self, _: &mut (), client: &mut dyn NetPeer) {
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

    fn on_receive(&mut self, _: &mut (), client: &mut dyn NetPeer, data: &[u8]) {
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
            NET_G2L_GAME_RESULTS => self.host_reopens_room(playerID),

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

/// The bytes before the first NUL.
fn cstr_bytes(field: &[u8]) -> &[u8] {
    &field[..field.iter().position(|&c| c == 0).unwrap_or(field.len())]
}

/// The leading base-10 integer after optional whitespace and sign; 0 when there is none.
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

#[cfg(test)]
mod tests {
    use super::*;
    use smw::common_netplay::protocol_packages::{ConnectOk, MatchRoom, RoomList, StartRoom};
    use std::any::Any;
    use std::cell::RefCell;
    use std::rc::Rc;

    type Outbox = Rc<RefCell<Vec<Vec<u8>>>>;

    struct TestPeer {
        id: u64,
        outbox: Outbox,
    }

    impl NetPeer for TestPeer {
        fn send(&mut self, data: &[u8]) -> bool {
            self.outbox.borrow_mut().push(data.to_vec());
            true
        }
        fn send_reliable(&mut self, data: &[u8]) -> bool {
            self.send(data)
        }
        fn disconnect(&mut self) {}
        fn address_host(&self) -> u32 {
            0
        }
        fn address_port(&self) -> u16 {
            self.id as u16
        }
        fn address_as_string(&self) -> String {
            format!("test:{}", self.id)
        }
        fn average_rtt(&self) -> u32 {
            0
        }
        fn get_player_id(&self) -> u64 {
            self.id
        }
        fn same_peer(&self, other: &dyn NetPeer) -> bool {
            other.get_player_id() == self.id
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    struct NoLayer;

    impl NetworkLayer for NoLayer {
        fn init(&mut self, _max_players: u64) -> bool {
            true
        }
        fn cleanup(&mut self) {}
        fn listen(&mut self, _handler: &mut dyn NetworkEventHandler) {}
    }

    fn server() -> SMWServer {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| set_net_layer(Box::new(NoLayer)));
        SMWServer::new()
    }

    fn connect(server: &mut SMWServer, id: u64, name: &str) -> Outbox {
        let outbox = Outbox::default();
        server.on_connect(&mut (), Box::new(TestPeer { id, outbox: outbox.clone() }));
        send(server, id, &outbox, ClientConnection::with(name).as_bytes());
        outbox
    }

    fn send(server: &mut SMWServer, id: u64, outbox: &Outbox, data: &[u8]) {
        server.on_receive(&mut (), &mut TestPeer { id, outbox: outbox.clone() }, data);
    }

    fn codes(outbox: &Outbox) -> Vec<u8> {
        outbox.borrow_mut().drain(..).map(|m| m[2]).collect()
    }

    fn room_with_password(server: &mut SMWServer, password: &str) -> u32 {
        let host = connect(server, 1, "Host");
        send(server, 1, &host, NewRoom::with("Room", password).as_bytes());
        let created = host.borrow().iter().find(|m| m[2] == NET_RESPONSE_CREATE_OK).cloned().unwrap();
        NewRoomCreated::from_bytes(&created).roomID
    }

    #[test]
    fn join_needs_the_room_password() {
        let mut server = server();
        let room = room_with_password(&mut server, "secret");
        let guest = connect(&mut server, 2, "Guest");
        codes(&guest);

        send(&mut server, 2, &guest, JoinRoom::with(room, "").as_bytes());
        assert_eq!(codes(&guest), [NET_RESPONSE_WRONG_PASSWORD]);
        send(&mut server, 2, &guest, JoinRoom::with(room, "Secret").as_bytes());
        assert_eq!(codes(&guest), [NET_RESPONSE_WRONG_PASSWORD]);

        send(&mut server, 2, &guest, JoinRoom::with(room, "secret").as_bytes());
        assert!(codes(&guest).contains(&NET_RESPONSE_JOIN_OK));
    }

    #[test]
    fn join_without_password_when_the_room_has_none() {
        let mut server = server();
        let room = room_with_password(&mut server, "");
        let guest = connect(&mut server, 2, "Guest");
        codes(&guest);

        send(&mut server, 2, &guest, JoinRoom::with(room, "").as_bytes());
        assert!(codes(&guest).contains(&NET_RESPONSE_JOIN_OK));
    }

    #[test]
    fn join_of_a_missing_room_is_answered() {
        let mut server = server();
        let guest = connect(&mut server, 2, "Guest");
        codes(&guest);

        send(&mut server, 2, &guest, JoinRoom::with(99, "").as_bytes());
        assert_eq!(codes(&guest), [NET_RESPONSE_ROOM_NOT_FOUND]);
    }

    #[test]
    fn room_list_without_visible_rooms_says_so() {
        let mut server = server();
        room_with_password(&mut server, "");
        let guest = connect(&mut server, 2, "Guest");
        codes(&guest);

        send(&mut server, 2, &guest, RoomList::new().as_bytes());
        assert_eq!(codes(&guest), [NET_RESPONSE_NO_ROOMS]);
    }

    fn assigned_name(outbox: &Outbox) -> String {
        let ok = outbox.borrow().iter().find(|m| m[2] == NET_RESPONSE_CONNECT_OK).cloned().unwrap();
        cstr_field(&ConnectOk::from_bytes(&ok).playerName)
    }

    #[test]
    fn duplicate_names_get_a_number() {
        let mut server = server();
        assert_eq!(assigned_name(&connect(&mut server, 1, "Player")), "Player");
        assert_eq!(assigned_name(&connect(&mut server, 2, "player")), "player 2");
        assert_eq!(assigned_name(&connect(&mut server, 3, "Player")), "Player 3");
        assert_eq!(assigned_name(&connect(&mut server, 4, "")), "Player 4");
        assert_eq!(assigned_name(&connect(&mut server, 5, "Mario")), "Mario");
    }

    #[test]
    fn numbered_long_names_still_fit() {
        let mut server = server();
        assert_eq!(assigned_name(&connect(&mut server, 1, "ABCDEFGHIJKLMNO")), "ABCDEFGHIJKLMNO");
        assert_eq!(assigned_name(&connect(&mut server, 2, "ABCDEFGHIJKLMNO")), "ABCDEFGHIJKLM 2");
    }

    #[test]
    fn a_reconnect_keeps_its_name() {
        let mut server = server();
        connect(&mut server, 1, "Luigi");
        assert_eq!(assigned_name(&connect(&mut server, 1, "Luigi")), "Luigi");
    }

    fn start_two_player_room(server: &mut SMWServer) -> (Outbox, Outbox) {
        let host = connect(server, 1, "Host");
        send(server, 1, &host, NewRoom::with("Room", "").as_bytes());
        let created = host.borrow().iter().find(|m| m[2] == NET_RESPONSE_CREATE_OK).cloned().unwrap();
        let room = NewRoomCreated::from_bytes(&created).roomID;
        let guest = connect(server, 2, "Guest");
        send(server, 2, &guest, JoinRoom::with(room, "").as_bytes());
        codes(&guest);
        send(server, 1, &host, StartRoom::new().as_bytes());
        (host, guest)
    }

    fn match_rooms(outbox: &Outbox) -> Vec<MatchRoom> {
        outbox.borrow().iter().filter(|m| m[2] == NET_L2P_MATCH_ROOM).map(|m| MatchRoom::from_bytes(m)).collect()
    }

    #[test]
    fn a_starting_room_gets_one_signaling_room_and_turn_credentials_per_player() {
        let mut server = server();
        server.v2 = V2Settings {
            signal_url: "wss://example.test/signal/".into(),
            turn_url: "turn:example.test:3478".into(),
            turn_secret: "0123abcd".into(),
        };
        let (host, guest) = start_two_player_room(&mut server);
        let (host, guest) = (match_rooms(&host), match_rooms(&guest));
        assert_eq!((host.len(), guest.len()), (1, 1));
        let (host, guest) = (host[0], guest[0]);
        assert_eq!((host.playerCount, guest.playerCount), (2, 2));
        let url = cstr_field(&host.roomURL);
        assert_eq!(url, cstr_field(&guest.roomURL));
        let id = url.strip_prefix("wss://example.test/signal/").unwrap();
        assert!(id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(cstr_field(&host.turnURL), "turn:example.test:3478");
        assert_ne!(cstr_field(&host.turnUsername), cstr_field(&guest.turnUsername));
        assert!(!cstr_field(&guest.turnCredential).is_empty());
    }

    #[test]
    fn without_a_signaling_url_rooms_start_as_before() {
        let mut server = server();
        server.v2 = V2Settings::default();
        let (host, guest) = start_two_player_room(&mut server);
        assert!(match_rooms(&host).is_empty() && match_rooms(&guest).is_empty());
    }

    fn skin(slot_hint: u8) -> Vec<u8> {
        let mut data = MessageHeader::new(NET_NOTICE_SKIN_CHANGE).as_bytes().to_vec();
        data.extend_from_slice(&[slot_hint, 10, 0, 2, 0, 1, 2]);
        data
    }

    fn skins_from(outbox: &Outbox) -> Vec<u8> {
        outbox.borrow().iter().filter(|m| m[2] == NET_NOTICE_SKIN_CHANGE).map(|m| m[3]).collect()
    }

    #[test]
    fn a_joining_player_and_the_room_swap_skins() {
        let mut server = server();
        let host = connect(&mut server, 1, "Host");
        send(&mut server, 1, &host, &skin(0xFF));
        send(&mut server, 1, &host, NewRoom::with("Room", "").as_bytes());
        let created = host.borrow().iter().find(|m| m[2] == NET_RESPONSE_CREATE_OK).cloned().unwrap();
        let room = NewRoomCreated::from_bytes(&created).roomID;
        host.borrow_mut().clear();

        let guest = connect(&mut server, 2, "Guest");
        send(&mut server, 2, &guest, &skin(0xFF));
        send(&mut server, 2, &guest, JoinRoom::with(room, "").as_bytes());
        assert_eq!(skins_from(&guest), [0], "the guest gets the host's skin, from slot 0");
        assert_eq!(skins_from(&host), [1], "the host gets the guest's skin, from slot 1");
    }

    fn map_change() -> Vec<u8> {
        let mut data = MessageHeader::new(NET_NOTICE_MAP_CHANGE).as_bytes().to_vec();
        data.extend_from_slice(&[0; 16]);
        data
    }

    fn host_and_guest(server: &mut SMWServer) -> (u32, Outbox, Outbox) {
        let host = connect(server, 1, "Host");
        send(server, 1, &host, NewRoom::with("Room", "").as_bytes());
        let created = host.borrow().iter().find(|m| m[2] == NET_RESPONSE_CREATE_OK).cloned().unwrap();
        let room = NewRoomCreated::from_bytes(&created).roomID;
        let guest = connect(server, 2, "Guest");
        send(server, 2, &guest, JoinRoom::with(room, "").as_bytes());
        host.borrow_mut().clear();
        guest.borrow_mut().clear();
        (room, host, guest)
    }

    #[test]
    fn only_the_host_changes_the_map_and_mode_settings() {
        let mut server = server();
        let (_, host, guest) = host_and_guest(&mut server);
        let mut settings = MessageHeader::new(NET_NOTICE_GAMEMODESETTINGS).as_bytes().to_vec();
        settings.extend_from_slice(&[0; 16]);

        send(&mut server, 2, &guest, &map_change());
        send(&mut server, 2, &guest, &settings);
        assert!(codes(&host).is_empty() && codes(&guest).is_empty());
        let other = connect(&mut server, 3, "Other");
        codes(&other);
        send(&mut server, 3, &other, RoomList::new().as_bytes());
        assert_eq!(codes(&other), [NET_RESPONSE_NO_ROOMS], "a joiner's map does not list the room");

        send(&mut server, 1, &host, &map_change());
        send(&mut server, 1, &host, &settings);
        assert_eq!(codes(&guest), [NET_NOTICE_MAP_CHANGE, NET_NOTICE_GAMEMODESETTINGS]);
    }

    #[test]
    fn only_the_host_starts_the_room() {
        let mut server = server();
        server.v2 = V2Settings { signal_url: "/signal".into(), ..Default::default() };
        let (_, host, guest) = host_and_guest(&mut server);
        send(&mut server, 2, &guest, StartRoom::new().as_bytes());
        assert!(codes(&host).is_empty() && codes(&guest).is_empty());
        send(&mut server, 1, &host, StartRoom::new().as_bytes());
        assert_eq!(codes(&guest), [NET_L2P_MATCH_ROOM, NET_L2P_MATCH_START]);
    }

    #[test]
    fn a_started_room_takes_no_joins_until_the_host_reopens_it() {
        let mut server = server();
        server.v2 = V2Settings { signal_url: "/signal".into(), ..Default::default() };
        let (room, host, guest) = host_and_guest(&mut server);
        send(&mut server, 1, &host, StartRoom::new().as_bytes());
        let late = connect(&mut server, 3, "Late");
        codes(&late);

        send(&mut server, 3, &late, JoinRoom::with(room, "").as_bytes());
        assert_eq!(codes(&late), [NET_RESPONSE_ROOM_NOT_FOUND]);
        send(&mut server, 2, &guest, MessageHeader::new(NET_G2L_GAME_RESULTS).as_bytes());
        send(&mut server, 3, &late, JoinRoom::with(room, "").as_bytes());
        assert_eq!(codes(&late), [NET_RESPONSE_ROOM_NOT_FOUND], "only the host reopens the room");

        send(&mut server, 1, &host, MessageHeader::new(NET_G2L_GAME_RESULTS).as_bytes());
        send(&mut server, 3, &late, JoinRoom::with(room, "").as_bytes());
        assert!(codes(&late).contains(&NET_RESPONSE_JOIN_OK));
    }

    #[test]
    fn a_lobby_with_one_room_refuses_a_second() {
        let mut server = server();
        server.max_rooms = Some(1);
        room_with_password(&mut server, "");
        let other = connect(&mut server, 2, "Other");
        codes(&other);
        send(&mut server, 2, &other, NewRoom::with("Second", "").as_bytes());
        assert_eq!(codes(&other), [NET_RESPONSE_CREATE_ERROR]);
    }
}
