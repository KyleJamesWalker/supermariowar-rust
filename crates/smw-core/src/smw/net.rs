//! Port of src/smw/net.cpp

use crate::common::game_mode::{GameModeType, *};
use crate::common::game_mode_settings::GAMEMODESETTINGS_RAW_SIZE;
use crate::common::global::{game_values, rm, skinlist};
use crate::common::global_constants::WAITTIME;
use crate::common::input::{COutputControl, CPlayerInput};
use crate::common::path::get_home_directory;
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType, RANDOM_INT};
use crate::common_netplay::network_interface::{NetPeer, NetworkEventHandler};
use crate::common_netplay::protocol_definitions::*;
use crate::common_netplay::protocol_packages::{self as pkgs, cstr_field, MessageHeader};
use crate::globals::{Aliased, Global, Ptr};
use crate::smw::main::{currentgamemode, gamemodes, players};
use crate::smw::network::file_compressor::FileCompressor;
use crate::smw::network::net_config_manager::NetConfigManager;
use crate::smw::network::network_layer::NetworkLayer;
use crate::smw::network::protocol_game_packages as gpkgs;
use crate::smw::objectgame::PowerupType;
use crate::smw::ui::network_list_scroll::MI_NetworkListScroll;
use std::collections::VecDeque;
use std::path::Path;
use std::time::SystemTime;

#[cfg(feature = "no_network")]
pub type NetworkHandler = crate::smw::platform::network::null::network_layer_null::NetworkLayerNULL;
#[cfg(all(not(feature = "no_network"), not(target_os = "emscripten")))]
pub type NetworkHandler = crate::smw::platform::network::enet::network_layer_enet::NetworkLayerENet;
#[cfg(all(not(feature = "no_network"), target_os = "emscripten"))]
pub type NetworkHandler = crate::smw::platform::network::websocket::network_layer_websocket::NetworkLayerWebSocket;

pub type nettimepoint = SystemTime;

pub static mut networkHandler: Global<NetworkHandler> = Global::uninit();
pub static mut netplay: Global<Networking> = Global::uninit();

static mut backup_playercontrol: [i16; 4] = [0; 4];

/// Constructs the statics C++ builds before `main`.
pub fn init_globals() {
    unsafe {
        networkHandler.init(NetworkHandler::default());
        netplay.init(Networking::new());
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LastMessage {
    pub packageType: u8,
    pub timestamp: u32,
}

#[derive(Clone, Debug, Default)]
pub struct ServerAddress {
    pub hostname: String,
}

#[derive(Clone, Debug, Default)]
pub struct RoomListEntry {
    pub roomID: u32,
    pub name: String,
    pub playerCount: u8,
}

#[derive(Clone, Debug)]
pub struct Room {
    pub roomID: u32,
    pub name: String,
    pub playerNames: [String; 4],
    pub hostPlayerNumber: u8,
    pub gamemodeID: u8,
    pub gamemodeGoal: u16,
}

impl Room {
    pub fn new() -> Self {
        Room {
            roomID: 0,
            name: String::new(),
            playerNames: std::array::from_fn(|_| "(empty)".to_string()),
            hostPlayerNumber: 0,
            gamemodeID: 0,
            gamemodeGoal: 0,
        }
    }

    pub fn player_count(&self) -> u8 {
        self.playerNames.iter().filter(|n| *n != "(empty)").count() as u8
    }
}

/// `union GameModeSettingsUnion`: the raw bytes of one mode's settings struct, zero-padded to the largest member.
const GMS_UNION_SIZE: usize = 44;

/// (offset, size) of each mode's settings inside the `GameModeSettings` raw image (clang layout).
fn gms_member(mode: GameModeType) -> Option<(usize, usize)> {
    Some(match mode {
        game_mode_classic => (0, 2),
        game_mode_frag => (2, 2),
        game_mode_timelimit => (4, 4),
        game_mode_jail => (8, 6),
        game_mode_coins => (14, 6),
        game_mode_stomp => (20, 20),
        game_mode_eggs => (40, 18),
        game_mode_ctf => (58, 8),
        game_mode_chicken => (66, 2),
        game_mode_tag => (68, 1),
        game_mode_star => (70, 6),
        game_mode_domination => (76, 8),
        game_mode_koth => (84, 6),
        game_mode_race => (90, 6),
        game_mode_frenzy => (96, 44),
        game_mode_survival => (140, 12),
        game_mode_greed => (152, 8),
        game_mode_health => (160, 6),
        game_mode_collection => (166, 8),
        game_mode_chase => (174, 8),
        game_mode_shyguytag => (182, 6),
        _ => return None,
    })
}

struct GameModeSettingsUnion {
    bytes: [u8; GMS_UNION_SIZE],
}

impl GameModeSettingsUnion {
    fn new() -> Self {
        GameModeSettingsUnion { bytes: [0; GMS_UNION_SIZE] }
    }

    fn set_global(&self, mode: GameModeType) {
        let Some((off, size)) = gms_member(mode) else {
            return;
        };
        unsafe {
            let mut raw: [u8; GAMEMODESETTINGS_RAW_SIZE] = game_values.gamemodesettings.to_raw_bytes();
            raw[off..off + size].copy_from_slice(&self.bytes[..size]);
            game_values.gamemodesettings.from_raw_bytes(&raw);
        }
    }

    fn read_global(&mut self, mode: GameModeType) {
        let Some((off, size)) = gms_member(mode) else {
            return;
        };
        unsafe {
            let raw = game_values.gamemodesettings.to_raw_bytes();
            self.bytes[..size].copy_from_slice(&raw[off..off + size]);
        }
    }
}

/// Not in the C++, which indexes `players` by ID: a client that already removed players at the end of its game
/// would read past the vector.
fn net_player(id: u8) -> Ptr<crate::smw::player::CPlayer> {
    unsafe { players.iter().copied().find(|p| p.globalID as u8 == id).unwrap_or(Ptr::null()) }
}

fn ticks() -> u32 {
    crate::services::ticks()
}

pub fn net_init() -> bool {
    unsafe {
        netplay.active = false;
        netplay.connectSuccessful = false;
        netplay.joinSuccessful = false;
        netplay.gameRunning = false;

        netplay.myPlayerName = "Player".to_string();
        netplay.currentMenuChanged = false;
        netplay.theHostIsMe = false;
        netplay.selectedRoomIndex = 0;
        netplay.selectedServerIndex = 0;
        netplay.roomFilter.clear();
        netplay.newroom_name.clear();
        netplay.newroom_password.clear();
        netplay.mychatmessage.clear();
        netplay.allowMapCollisionEvent = false;

        if !networkHandler.deref_handler().init() {
            return false;
        }

        if !netplay.client.init() {
            return false;
        }

        net_load_server_list();

        println!("[net] Network system initialized.");
        true
    }
}

pub fn net_close() {
    net_save_server_list();

    net_end_session();
    unsafe {
        netplay.client.cleanup();

        networkHandler.deref_handler().cleanup();
    }
}

pub fn net_save_server_list() {
    let mut config = NetConfigManager;
    config.save();
}

pub fn net_load_server_list() {
    let mut config = NetConfigManager;
    config.load();

    #[cfg(all(not(feature = "no_network"), target_os = "emscripten"))]
    unsafe {
        if let Some(url) = crate::smw::platform::network::websocket::network_layer_websocket::relay_url() {
            netplay.savedServers.retain(|s| s.hostname != url);
            netplay.savedServers.insert(0, ServerAddress { hostname: url });
        }
    }
}

/****************************
    Session
****************************/

pub fn net_start_session() -> bool {
    net_end_session();

    unsafe {
        println!("[net] Session start.");
        netplay.active = true;
        netplay.connectSuccessful = false;
        netplay.gamestate_changed = false;
        netplay.frames_since_last_gamestate = 0;
        netplay.player_disconnected = [false; 4];

        let nullinput = COutputControl::default();
        for p in 0..4 {
            backup_playercontrol[p] = game_values.playercontrol[p];

            netplay.latest_playerdata.player_input[p].clear();
            netplay.latest_playerdata.player_input[p].push_back(nullinput);

            netplay.local_input_buffer.clear();
            netplay.remote_input_buffer[p].clear();
        }

        netplay.client.start()
    }
}

pub fn net_end_session() {
    unsafe {
        if netplay.active {
            println!("[net] Session end.");

            netplay.client.stop();

            netplay.active = false;
            netplay.connectSuccessful = false;

            for p in 0..4 {
                game_values.playercontrol[p] = backup_playercontrol[p];
            }
        }
    }
}

trait HandlerAccess {
    fn deref_handler(&mut self) -> &mut NetworkHandler;
}

impl HandlerAccess for Global<NetworkHandler> {
    fn deref_handler(&mut self) -> &mut NetworkHandler {
        self
    }
}

/// How long the host waits for its joiners to load, and a joiner for the host's go, before playing anyway.
const START_TIMEOUT_MS: u32 = 5000;

/// Not in the C++: called before every gameplay frame. While true, the frame only polls the network, so no client
/// plays gameplay frame 1 before every client has loaded the match (`NET_P2G_LOADED`, then the host's `NET_G2P_GO`).
pub fn waiting_to_start() -> bool {
    unsafe {
        if !netplay.active || !netplay.start_waiting {
            return false;
        }
        let now = ticks();
        if netplay.start_waited_since == 0 {
            netplay.start_waited_since = now.max(1);
            if !netplay.theHostIsMe {
                let peer = netplay.client.foreign_gamehost;
                netplay.client.send_to(peer, MessageHeader::new(NET_P2G_LOADED).as_bytes(), false);
            }
        }
        netplay.client.update();
        if netplay.theHostIsMe && netplay.start_waiting {
            netplay.client.local_gamehost.start_if_everyone_loaded();
        }
        let limit = if netplay.theHostIsMe { START_TIMEOUT_MS } else { 2 * START_TIMEOUT_MS };
        if netplay.start_waiting && now.wrapping_sub(netplay.start_waited_since) > limit {
            println!("[net] Not every client loaded the match in time; starting anyway.");
            if netplay.theHostIsMe {
                netplay.client.local_gamehost.send_go_message();
            }
            netplay.start_waiting = false;
        }
        netplay.start_waiting
    }
}

/// Not in the C++: every client draws in step from the start of the match, whatever its menus drew since the sync.
pub fn reseed_for_match() {
    RandomNumberGenerator::generator().reseed(unsafe { netplay.common_random_seed });
}

/// `NET_NOTICE_GAMEMODESETTINGS` with the current game mode's settings.
fn game_mode_settings_package() -> Vec<u8> {
    let mut blob = vec![0u8; std::mem::size_of::<MessageHeader>() + GMS_UNION_SIZE];

    let header = MessageHeader::new(NET_NOTICE_GAMEMODESETTINGS);
    blob[..3].copy_from_slice(header.as_bytes());

    let mut modesettings = GameModeSettingsUnion::new();
    modesettings.read_global(unsafe { currentgamemode } as GameModeType);
    blob[3..].copy_from_slice(&modesettings.bytes);
    blob
}

/// `host_bytes[0].host_bytes[1]...` of an address stored in network byte order.
fn host_to_string(host: u32) -> String {
    let b = host.to_ne_bytes();
    format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
}

/********************************************************************
 * NetClient
 ********************************************************************/

pub struct NetClient {
    pub lastSentMessage: LastMessage,
    pub lastReceivedMessage: LastMessage,

    pub local_gamehost: NetGameHost,

    foreign_lobbyserver: Ptr<dyn NetPeer>,
    foreign_gamehost: Ptr<dyn NetPeer>,

    uiRoomList: Ptr<MI_NetworkListScroll>,

    pub _alias: Aliased,
}

impl NetClient {
    pub fn new() -> Self {
        NetClient {
            lastSentMessage: LastMessage::default(),
            lastReceivedMessage: LastMessage::default(),
            local_gamehost: NetGameHost::new(),
            foreign_lobbyserver: Ptr::null(),
            foreign_gamehost: Ptr::null(),
            uiRoomList: Ptr::null(),
            _alias: Aliased::new(),
        }
    }

    pub fn init(&mut self) -> bool {
        if !self.local_gamehost.init() {
            return false;
        }

        true
    }

    pub fn start(&mut self) -> bool {
        unsafe {
            if !networkHandler.deref_handler().client_restart() {
                return false;
            }
        }

        true
    }

    pub fn update(&mut self) {
        self.local_gamehost.update();
        unsafe { networkHandler.deref_handler().client_listen(self) };
    }

    pub fn stop(&mut self) {
        self.local_gamehost.stop();

        if unsafe { netplay.connectSuccessful } {
            self.send_goodbye();
        }

        if !self.foreign_lobbyserver.is_null() {
            self.foreign_lobbyserver.disconnect();
            self.foreign_lobbyserver = Ptr::null();
        }
        if !self.foreign_gamehost.is_null() {
            self.foreign_gamehost.disconnect();
            self.foreign_gamehost = Ptr::null();
        }

        unsafe { networkHandler.deref_handler().client_shutdown() };
    }

    pub fn cleanup(&mut self) {
        self.stop();
    }

    pub fn set_room_list_ui_control(&mut self, control: Ptr<MI_NetworkListScroll>) {
        self.uiRoomList = control;
    }

    /****************************
        Phase 1: Connect
    ****************************/

    pub fn send_connect_request_to_selected_server(&mut self) -> bool {
        let hostname = unsafe { netplay.savedServers[netplay.selectedServerIndex as usize].hostname.clone() };
        if !self.connect_lobby(&hostname, NET_LOBBYSERVER_PORT) {
            return false;
        }

        unsafe { netplay.operationInProgress = true };
        true
    }

    fn send_goodbye(&mut self) {
        let msg = pkgs::ClientDisconnection::new();
        self.send_message_to_lobby_server(msg.as_bytes());
    }

    fn handle_serverinfo_and_close(&mut self, data: &[u8]) {
        let serverInfo = pkgs::ServerInfo::from_bytes(data);

        println!(
            "[net] Server information: Name: {}, Protocol version: {}.{}  Players/Max: {} / {}",
            cstr_field(&serverInfo.name),
            serverInfo.header.protocolMajorVersion,
            serverInfo.header.protocolMinorVersion,
            serverInfo.currentPlayerCount as i32,
            serverInfo.maxPlayerCount as i32
        );

        self.foreign_lobbyserver.disconnect();
        self.foreign_lobbyserver = Ptr::null();
    }

    /****************************
        Phase 2: Join room
    ****************************/

    pub fn request_room_list(&mut self) {
        let msg = pkgs::RoomList::new();
        self.send_message_to_lobby_server(msg.as_bytes());

        unsafe { netplay.currentRooms.clear() };
        if !self.uiRoomList.is_null() {
            self.uiRoomList.clear();
        }
    }

    fn handle_new_room_list_entry(&mut self, data: &[u8]) {
        let roomInfo = pkgs::RoomInfo::from_bytes(data);

        let newRoom = RoomListEntry { roomID: roomInfo.roomID, name: cstr_field(&roomInfo.name), playerCount: roomInfo.currentPlayerCount };
        println!("  Incoming room entry: [{}] {} ({}/4)", newRoom.roomID, newRoom.name, newRoom.playerCount);
        let name = newRoom.name.clone();
        unsafe { netplay.currentRooms.push(newRoom) };

        if !self.uiRoomList.is_null() {
            let playerCountString: String =
                [(b'0'.wrapping_add(roomInfo.currentPlayerCount)) as char, '/', '4'].iter().collect();
            self.uiRoomList.add(&name, &playerCountString);
        }
    }

    pub fn send_create_room_message(&mut self) {
        unsafe {
            let mut msg = pkgs::NewRoom::with(&netplay.newroom_name, &netplay.newroom_password);
            msg.gamemodeID = currentgamemode as u8;
            game_values.gamemode = gamemodes[currentgamemode as usize];
            msg.gamemodeGoal = game_values.gamemode.goal as u16;

            self.send_message_to_lobby_server(msg.as_bytes());
            netplay.operationInProgress = true;
        }
    }

    fn handle_room_created_message(&mut self, data: &[u8]) {
        println!("room created!");
        let pkg = pkgs::NewRoomCreated::from_bytes(data);

        unsafe {
            netplay.currentRoom.roomID = pkg.roomID;
            netplay.currentRoom.hostPlayerNumber = 0;
            netplay.currentRoom.name = netplay.newroom_name.clone();
            netplay.currentRoom.playerNames[0] = netplay.myPlayerName.clone();
            netplay.currentRoom.playerNames[1] = "(empty)".to_string();
            netplay.currentRoom.playerNames[2] = "(empty)".to_string();
            netplay.currentRoom.playerNames[3] = "(empty)".to_string();

            netplay.currentMenuChanged = true;
            netplay.joinSuccessful = true;
            netplay.theHostIsMe = true;

            game_values.playercontrol[0] = 1;
            game_values.playercontrol[1] = 0;
            game_values.playercontrol[2] = 0;
            game_values.playercontrol[3] = 0;
        }

        self.send_map_change_message();
        self.send_game_mode_settings_change_message();

        let lobby = self.foreign_lobbyserver;
        self.local_gamehost.start(lobby);
    }

    pub fn send_join_room_message(&mut self) {
        unsafe {
            let entry = netplay.currentRooms[netplay.selectedRoomIndex as usize].clone();
            println!(
                "currentRooms[{}] = {{id={}, name='{}', cnt={}}}, össz: {}",
                netplay.selectedRoomIndex,
                entry.roomID,
                entry.name,
                entry.playerCount,
                netplay.currentRooms.len()
            );

            let msg = pkgs::JoinRoom::with(entry.roomID, "");
            self.send_message_to_lobby_server(msg.as_bytes());
            netplay.operationInProgress = true;
        }
    }

    pub fn send_leave_room_message(&mut self) {
        self.local_gamehost.stop();

        let msg = pkgs::LeaveRoom::new();
        self.send_message_to_lobby_server(msg.as_bytes());
    }

    fn handle_room_changed_message(&mut self, data: &[u8]) {
        let pkg = pkgs::CurrentRoom::from_bytes(data);

        unsafe {
            netplay.currentRoom.roomID = pkg.roomID;
            netplay.currentRoom.name = cstr_field(&pkg.name);

            println!("Room {} ({}) changed:", pkg.roomID, cstr_field(&pkg.name));
            for p in 0..4u8 {
                netplay.currentRoom.playerNames[p as usize] = cstr_field(&pkg.playerName[p as usize]);
                print!("  player {}: {}", p + 1, netplay.currentRoom.playerNames[p as usize]);
                if p == pkg.remotePlayerNumber {
                    print!(" (me)");
                }
                if p == pkg.hostPlayerNumber {
                    print!(" (HOST)");
                }
                println!();

                if netplay.currentRoom.playerNames[p as usize] == "(empty)" {
                    game_values.playercontrol[p as usize] = 0;
                } else {
                    game_values.playercontrol[p as usize] = 1;
                }
            }

            currentgamemode = pkg.gamemodeID as i16;
            game_values.gamemode = gamemodes[currentgamemode as usize];
            game_values.gamemode.goal = pkg.gamemodeGoal as i16;

            let gm = gamemodes[currentgamemode as usize];
            println!("  Game mode #{}: {} with {}: {}", currentgamemode, gm.get_mode_name(), gm.get_goal_name(), game_values.gamemode.goal);

            netplay.theHostIsMe = false;
            netplay.currentRoom.hostPlayerNumber = pkg.hostPlayerNumber;
            netplay.remotePlayerNumber = pkg.remotePlayerNumber;
            netplay.hostPlayerNumber = pkg.hostPlayerNumber;
            if netplay.remotePlayerNumber == pkg.hostPlayerNumber {
                netplay.theHostIsMe = true;
                let lobby = self.foreign_lobbyserver;
                self.local_gamehost.start(lobby);
            } else {
                self.local_gamehost.stop();
            }

            netplay.currentMenuChanged = true;
        }
    }

    pub fn send_chat_message(&mut self, message: &str) {
        debug_assert!(!message.is_empty());
        let pkg = pkgs::RoomChatMsg::with(unsafe { netplay.remotePlayerNumber }, message);
        self.send_message_to_lobby_server(pkg.as_bytes());
    }

    pub fn send_map_change_message(&mut self) {
        let mapfilepath = unsafe { netplay.mapfilepath.clone() };
        println!("[net] Sending map: {}", mapfilepath);

        let mut compressed = FileCompressor::compress(&mapfilepath, std::mem::size_of::<MessageHeader>());
        if !compressed.is_valid() {
            return;
        }
        if compressed.size() <= 4 + std::mem::size_of::<MessageHeader>() {
            return;
        }

        let header = MessageHeader::new(NET_NOTICE_MAP_CHANGE);
        compressed.data[..3].copy_from_slice(header.as_bytes());

        self.send_message_to_lobby_server(&compressed.data);
    }

    fn handle_map_change_message(&mut self, data: &[u8]) {
        if data.len() <= std::mem::size_of::<MessageHeader>() + 4 || data.len() > 20000 {
            println!("[error] Corrupt map arrived");
            return;
        }

        let full_size = u16::from_ne_bytes([data[3], data[4]]);
        let compressed_size = u16::from_ne_bytes([data[5], data[6]]);

        println!("[net] Map arrived (C:{} unC:{})", compressed_size, full_size);

        unsafe {
            netplay.mapfilepath = get_home_directory() + "net_last.map";
            let path = netplay.mapfilepath.clone();
            if !FileCompressor::decompress(&data[3..], &path) {
                return;
            }
        }
    }

    pub fn send_game_mode_settings_change_message(&mut self) {
        let blob = game_mode_settings_package();
        self.send_message_to_lobby_server(&blob);
    }

    fn handle_game_mode_settings_change_message(&mut self, data: &[u8]) {
        if data.len() != std::mem::size_of::<MessageHeader>() + GMS_UNION_SIZE {
            println!("[error] Corrupt game mode settings arrived");
            return;
        }

        let mut modesettings = GameModeSettingsUnion::new();
        modesettings.bytes.copy_from_slice(&data[3..]);
        modesettings.set_global(unsafe { currentgamemode } as GameModeType);

        println!("[net] Game mode settings changed");
    }

    fn handle_room_chat_message(&mut self, data: &[u8]) {
        let pkg = pkgs::RoomChatMsg::from_bytes(data);

        unsafe {
            println!("Not implemented: [net] {}: {}", netplay.currentRoom.playerNames[pkg.senderNum as usize], cstr_field(&pkg.message));
        }
    }

    pub fn send_skin_change(&mut self) {
        let skinpath = unsafe { skinlist.at(game_values.skinids[netplay.remotePlayerNumber as usize] as usize).path.to_string_lossy().into_owned() };
        println!("[net] Sending skin: {}", skinpath);

        let mut compressed = FileCompressor::compress(&skinpath, std::mem::size_of::<MessageHeader>() + 1);
        if !compressed.is_valid() {
            return;
        }
        if compressed.size() <= 1 + std::mem::size_of::<MessageHeader>() {
            return;
        }

        let header = MessageHeader::new(NET_NOTICE_SKIN_CHANGE);
        compressed.data[..3].copy_from_slice(header.as_bytes());
        compressed.data[3] = 0xFF;

        self.send_message_to_lobby_server(&compressed.data);
    }

    fn handle_skin_change_message(&mut self, data: &[u8]) {
        if data.len() <= std::mem::size_of::<MessageHeader>() + 5 || data.len() > 20000 {
            println!("[error] Corrupt skin arrived");
            return;
        }

        let playerID = data[3] as i32;
        if playerID > 3 {
            println!("[error] Corrupt skin arrived: bad player id");
            return;
        }

        let full_size = u16::from_ne_bytes([data[4], data[5]]);
        let compressed_size = u16::from_ne_bytes([data[6], data[7]]);

        println!("[net] Skin arrived (from: {}, C:{} unC:{})", playerID, compressed_size, full_size);

        let path = format!("{}net_skin{}.bmp", get_home_directory(), playerID);
        if !FileCompressor::decompress(&data[4..], &path) {
            return;
        }

        unsafe {
            if !rm.load_menu_skin_path(playerID as i16, Path::new(&path), playerID as i16, false) {
                println!("[warning] Could not load netplay skin of player {}, using default", playerID);
                rm.load_menu_skin(playerID as i16, game_values.skinids[playerID as usize], playerID as i16, false);
            }
        }
    }

    /****************************
        Phase 2.5: Pre-game
    ****************************/

    fn handle_room_start_message(&mut self, client: &dyn NetPeer, data: &[u8]) {
        let mut pkg = pkgs::GameHostInfo::from_bytes(data);

        if pkg.host == 0x100007F {
            pkg.host = client.address_host();
        }

        let host_str = host_to_string(pkg.host);

        println!("[net] Connecting to game host... [{}:{}]", host_str, NET_GAMEHOST_PORT);
        if !self.connect_game_host(&host_str, NET_GAMEHOST_PORT) {
            println!("[net][error] Could not connect to game host.");
            return;
        }

        unsafe { netplay.operationInProgress = true };
    }

    fn handle_expected_clients_message(&mut self, client: &dyn NetPeer, data: &[u8]) {
        let pkg = pkgs::PlayerInfo::from_bytes(data);

        let mut playerCount: u8 = 0;
        let mut hosts = [0u32; 3];
        let mut ports = [0u16; 3];

        println!("[net] Expecting the following clients:");
        for p in 0..3usize {
            if pkg.host[p] != 0 {
                hosts[playerCount as usize] = pkg.host[p];
                ports[playerCount as usize] = pkg.port[p];

                if hosts[playerCount as usize] == 0x100007F {
                    hosts[playerCount as usize] = client.address_host();
                }

                println!("  Client {} is {}:{}", p, host_to_string(pkg.host[p]), pkg.port[p]);
                playerCount += 1;
            }
        }

        self.local_gamehost.set_expected_players(playerCount, &hosts, &ports);
    }

    pub fn handle_start_sync_message(&mut self, data: &[u8]) {
        let pkg = pkgs::StartSync::from_bytes(data);

        println!("reseed: {}", pkg.commonRandomSeed as i32);
        RandomNumberGenerator::generator().reseed(pkg.commonRandomSeed);
        unsafe {
            netplay.common_random_seed = pkg.commonRandomSeed;
            netplay.start_waiting = true;
            netplay.start_waited_since = 0;
            netplay.gamestate_received = false;
        }
        crate::smw::net_random::begin_setup();

        unsafe {
            netplay.player_disconnected = [false; 4];
            netplay.last_confirmed_input = 0xFF;
            netplay.current_input_counter = 0;
            netplay.local_playerdata_store_time = [SystemTime::now(); 256];

            if netplay.theHostIsMe {
                self.set_as_last_sent_message(NET_P2G_SYNC_OK);
            } else {
                let respond_pkg = gpkgs::SyncOK::new();
                self.send_message_to_game_host_reliable(respond_pkg.as_bytes());
            }
        }
    }

    pub fn handle_game_start_message(&mut self) {
        println!("[net] Game start!");
        unsafe { netplay.gameRunning = true };
    }

    /****************************
        Phase 3: Play
    ****************************/

    pub fn send_leave_game_message(&mut self) {
        let pkg = gpkgs::LeaveGame::new();
        unsafe {
            if netplay.theHostIsMe {
                self.local_gamehost.send_message_to_my_peers(pkg.as_bytes());
                self.local_gamehost.update();
                self.local_gamehost.stop();

                self.set_as_last_received_message(pkg.header.packageType);
            } else {
                self.send_message_to_game_host_reliable(pkg.as_bytes());
            }

            netplay.gameRunning = false;
        }
    }

    pub fn store_local_input(&mut self) {
        unsafe {
            if netplay.theHostIsMe {
                let c = game_values.playerInput.outputControls[0];
                netplay.local_input_buffer.push_back(c);
            }
        }
    }

    pub fn send_local_input(&mut self) {
        unsafe {
            if netplay.theHostIsMe {
                self.local_gamehost.send_local_input();
            } else {
                let mut pkg = gpkgs::ClientInput::new(&game_values.playerInput.outputControls[0]);
                pkg.input_id = netplay.current_input_counter;
                self.send_message_to_game_host(pkg.as_bytes());
            }
        }
    }

    pub fn send_powerup_request(&mut self) {
        let pkg = gpkgs::RequestPowerup::new();
        self.send_message_to_game_host_reliable(pkg.as_bytes());
    }

    pub fn handle_powerup_start(&mut self, data: &[u8]) {
        let pkg = gpkgs::StartPowerup::from_bytes(data);

        if pkg.player_id > 3 {
            return;
        }

        unsafe {
            let mut player = net_player(pkg.player_id);
            if player.is_null() {
                return;
            }

            let mut missed_frames: u32 = pkg.delay;
            if !netplay.theHostIsMe {
                missed_frames = missed_frames.wrapping_add(self.foreign_gamehost.average_rtt() / 2);
            }
            missed_frames /= WAITTIME as u32;

            player.powerupused = Some(PowerupType::from_u8(pkg.powerup_id));
            player.powerupradius = 100.0;
            while missed_frames > 0 {
                player.powerupradius -= game_values.storedpowerupdelay as f32 / 2.0;
                missed_frames -= 1;
            }

            player.net_waitingForPowerupTrigger = false;
            println!("[net] P{} used powerup {}", pkg.player_id, pkg.powerup_id);
        }
    }

    fn handle_map_collision(&mut self, data: &[u8]) {
        let pkg = gpkgs::MapCollision::from_bytes(data);

        if pkg.player_id > 3 {
            return;
        }

        unsafe {
            let mut player = net_player(pkg.player_id);
            if player.is_null() {
                return;
            }

            let temp_px = player.fx;
            let temp_py = player.fy;
            let temp_velx = player.velx;
            let temp_vely = player.vely;
            player.set_xf(pkg.player_x);
            player.set_yf(pkg.player_y);
            player.velx = pkg.player_xvel;
            player.vely = pkg.player_yvel;

            netplay.allowMapCollisionEvent = true;
            player.collision_detection_map();
            netplay.allowMapCollisionEvent = false;

            if player.isdead() {
                return;
            }

            player.set_xf(temp_px);
            player.set_yf(temp_py);
            player.velx = temp_velx;
            player.vely = temp_vely;

            println!("[net] Map block collision!");
        }
    }

    fn handle_p2p_collision(&mut self, data: &[u8]) {
        let pkg = gpkgs::P2PCollision::from_bytes(data);

        if pkg.player_id[0] > 3 || pkg.player_id[1] > 3 {
            return;
        }

        unsafe {
            let mut player1 = net_player(pkg.player_id[0]);
            let mut player2 = net_player(pkg.player_id[1]);
            if player1.is_null() || player2.is_null() {
                return;
            }

            player1.set_xf(pkg.player_x[0]);
            player1.set_yf(pkg.player_y[0]);
            player1.fOldY = pkg.player_oldy[0];

            player2.set_xf(pkg.player_x[1]);
            player2.set_yf(pkg.player_y[1]);
            player2.fOldY = pkg.player_oldy[1];

            player1.collides_with_player(player2);

            println!("P{} collided with P{}!", pkg.player_id[0], pkg.player_id[1]);
        }
    }

    pub fn handle_remote_input(&mut self, data: &[u8]) {
        let pkg = gpkgs::RemoteInput::from_bytes(data);
        let mut keys = COutputControl::default();
        pkg.read_keys(&mut keys);
        unsafe { netplay.remote_input_buffer[pkg.playerNumber as usize].push_back((0, keys)) };
    }

    pub fn handle_remote_game_state(&mut self, data: &[u8]) {
        let pkg = gpkgs::GameState::from_bytes(data);

        unsafe {
            netplay.previous_playerdata = netplay.latest_playerdata.clone();

            for p in 0..players.len() as u8 {
                let d = &mut netplay.latest_playerdata.player[p as usize];
                pkg.get_player_coord(p, &mut d.x, &mut d.y);
                pkg.get_player_vel(p, &mut d.xvel, &mut d.yvel);
            }

            if !netplay.gamestate_received {
                netplay.previous_playerdata = netplay.latest_playerdata.clone();
            }
            netplay.gamestate_changed = true;
            netplay.gamestate_received = true;
            netplay.frames_since_last_gamestate = 0;
            netplay.last_confirmed_input = pkg.last_confirmed_local_input_id as u16;
        }
    }

    /****************************
        Network Communication
    ****************************/

    fn connect_lobby(&mut self, hostname: &str, port: u16) -> bool {
        unsafe {
            netplay.connectSuccessful = false;
            networkHandler.deref_handler().connect_to_lobby_server(hostname, port)
        }
    }

    fn connect_game_host(&mut self, hostname: &str, port: u16) -> bool {
        unsafe {
            netplay.connectSuccessful = false;
            networkHandler.deref_handler().connect_to_foreign_game_host(hostname, port)
        }
    }

    /// The C++ swaps the channels: `reliable` sends unreliably and vice versa.
    fn send_to(&mut self, mut peer: Ptr<dyn NetPeer>, data: &[u8], reliable: bool) -> bool {
        if reliable {
            if !peer.send(data) {
                return false;
            }
        } else if !peer.send_reliable(data) {
            return false;
        }

        self.set_as_last_sent_message(data[2]);
        true
    }

    fn send_message_to_lobby_server(&mut self, data: &[u8]) -> bool {
        let peer = self.foreign_lobbyserver;
        self.send_to(peer, data, true)
    }

    fn send_message_to_game_host(&mut self, data: &[u8]) -> bool {
        let peer = self.foreign_gamehost;
        self.send_to(peer, data, false)
    }

    fn send_message_to_game_host_reliable(&mut self, data: &[u8]) -> bool {
        let peer = self.foreign_gamehost;
        self.send_to(peer, data, true)
    }

    pub fn set_as_last_sent_message(&mut self, packageType: u8) {
        self.lastSentMessage.packageType = packageType;
        self.lastSentMessage.timestamp = ticks();
    }

    pub fn set_as_last_received_message(&mut self, packageType: u8) {
        self.lastReceivedMessage.packageType = packageType;
        self.lastReceivedMessage.timestamp = ticks();
    }
}

impl NetworkEventHandler for NetClient {
    fn on_connect(&mut self, newPeer: Box<dyn NetPeer>) {
        let mut newPeer = Ptr::from_box(newPeer);

        if self.foreign_lobbyserver.is_null() {
            self.foreign_lobbyserver = newPeer;

            let message = pkgs::ClientConnection::with(unsafe { &netplay.myPlayerName });
            self.send_message_to_lobby_server(message.as_bytes());
        } else if self.foreign_gamehost.is_null() {
            self.foreign_gamehost = newPeer;
        } else {
            newPeer.disconnect();
        }
    }

    fn on_disconnect(&mut self, client: &mut dyn NetPeer) {
        if !self.foreign_gamehost.is_null() && client.same_peer(&*self.foreign_gamehost) {
            println!("[net] Disconnected from game host.");
            unsafe { netplay.player_disconnected[netplay.remotePlayerNumber as usize] = true };
            return;
        }

        if !self.foreign_lobbyserver.is_null() && client.same_peer(&*self.foreign_lobbyserver) {
            println!("[net] Disconnected from lobby server.");
            return;
        }

        println!("[net] Client {} disconnected", client.address_as_string());
    }

    fn on_receive(&mut self, client: &mut dyn NetPeer, data: &[u8]) {
        if data.len() < 3 {
            return;
        }

        let protocolMajor = data[0];
        let protocolMinor = data[1];
        let packageType = data[2];
        if protocolMajor != NET_PROTOCOL_VERSION_MAJOR || protocolMinor != NET_PROTOCOL_VERSION_MINOR {
            println!("Not implemented: bad protocol version");
            return;
        }

        unsafe {
            match packageType {
                NET_RESPONSE_BADPROTOCOL => println!("Not implemented: NET_RESPONSE_BADPROTOCOL"),

                NET_RESPONSE_SERVERINFO => self.handle_serverinfo_and_close(data),

                NET_RESPONSE_SERVER_MOTD => println!("Not implemented: NET_RESPONSE_SERVER_MOTD"),

                NET_RESPONSE_CONNECT_OK => {
                    // Not in the C++: the Servers menu moves on only if the skin goes out no earlier than this receive.
                    self.set_as_last_received_message(packageType);
                    netplay.connectSuccessful = true;
                    self.send_skin_change();
                    return;
                }

                NET_RESPONSE_CONNECT_DENIED => {
                    println!("Not implemented: NET_RESPONSE_CONNECT_DENIED");
                    netplay.connectSuccessful = false;
                }

                NET_RESPONSE_CONNECT_SERVERFULL => {
                    println!("Not implemented: NET_RESPONSE_CONNECT_SERVERFULL");
                    netplay.connectSuccessful = false;
                }

                NET_RESPONSE_CONNECT_NAMETAKEN => {
                    println!("Not implemented: NET_RESPONSE_CONNECT_NAMETAKEN");
                    netplay.connectSuccessful = false;
                }

                NET_RESPONSE_ROOM_LIST_ENTRY => self.handle_new_room_list_entry(data),

                NET_RESPONSE_NO_ROOMS => println!("Not implemented: [net] There are no rooms currently on the server."),

                NET_RESPONSE_JOIN_OK => {
                    println!("Not implemented: [net] Joined to room.");
                    netplay.joinSuccessful = true;
                    netplay.theHostIsMe = false;
                }

                NET_RESPONSE_ROOM_FULL => {
                    println!("Not implemented: [net] The room is full");
                    netplay.joinSuccessful = false;
                }

                NET_NOTICE_ROOM_CHANGE => self.handle_room_changed_message(data),

                NET_NOTICE_MAP_CHANGE => self.handle_map_change_message(data),

                NET_NOTICE_SKIN_CHANGE => self.handle_skin_change_message(data),

                NET_NOTICE_GAMEMODESETTINGS => self.handle_game_mode_settings_change_message(data),

                NET_NOTICE_ROOM_CHAT_MSG => self.handle_room_chat_message(data),

                NET_RESPONSE_CREATE_OK => self.handle_room_created_message(data),

                NET_RESPONSE_CREATE_ERROR => println!("Not implemented: NET_RESPONSE_CREATE_ERROR"),

                NET_L2P_GAMEHOST_INFO => self.handle_room_start_message(client, data),

                NET_L2G_CLIENTS_INFO => self.handle_expected_clients_message(client, data),

                NET_G2P_SYNC => self.handle_start_sync_message(data),

                NET_G2E_GAME_START => self.handle_game_start_message(),

                NET_G2P_REMOTE_KEYS => self.handle_remote_input(data),

                NET_G2P_GAME_STATE => self.handle_remote_game_state(data),

                NET_G2P_START_POWERUP => self.handle_powerup_start(data),

                NET_G2P_TRIGGER_MAPCOLL => self.handle_map_collision(data),

                NET_G2P_TRIGGER_P2PCOLL => self.handle_p2p_collision(data),


                NET_G2P_GO => netplay.start_waiting = false,

                NET_G2P_RANDOM_EVENT => crate::smw::net_random::receive(data),

                _ => {
                    print!("Unknown: ");
                    for b in data {
                        print!("{:3} ", b);
                    }
                    println!();
                    return;
                }
            }
        }

        self.set_as_last_received_message(packageType);
        if packageType == NET_RESPONSE_CREATE_OK {
            println!("last: {}, {}", self.lastSentMessage.packageType, self.lastReceivedMessage.packageType);
        }
    }
}

/********************************************************************
 * NetGameHost
 ********************************************************************/

#[derive(Clone, Copy, Debug, Default)]
struct RawPlayerAddress {
    host: u32,
    port: u16,
    sync_ok: bool,
    loaded: bool,
}

impl RawPlayerAddress {
    fn reset(&mut self) {
        self.host = 0;
        self.port = 0;
        self.sync_ok = false;
        self.loaded = false;
    }

    fn same(&self, other: &RawPlayerAddress) -> bool {
        self.host == other.host && self.port == other.port
    }
}

pub struct NetGameHost {
    pub lastSentMessage: LastMessage,
    pub lastReceivedMessage: LastMessage,

    active: bool,

    current_server_tick: u32,

    clients: [Ptr<dyn NetPeer>; 3],
    foreign_lobbyserver: Ptr<dyn NetPeer>,

    expected_clients: [RawPlayerAddress; 3],
    expected_client_count: u8,
    next_free_client_slot: u8,
    last_processed_input_id: [u8; 3],

    preparedMapCollPkg: Box<gpkgs::MapCollision>,

    pub _alias: Aliased,
}

impl NetGameHost {
    pub fn new() -> Self {
        NetGameHost {
            lastSentMessage: LastMessage::default(),
            lastReceivedMessage: LastMessage::default(),
            active: false,
            current_server_tick: 0,
            clients: [Ptr::null(); 3],
            foreign_lobbyserver: Ptr::null(),
            expected_clients: [RawPlayerAddress::default(); 3],
            expected_client_count: 0,
            next_free_client_slot: 0,
            last_processed_input_id: [0xFF; 3],
            preparedMapCollPkg: Box::new(gpkgs::MapCollision::new()),
            _alias: Aliased::new(),
        }
    }

    pub fn init(&mut self) -> bool {
        true
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn start(&mut self, lobbyserver: Ptr<dyn NetPeer>) -> bool {
        if self.active {
            return true;
        }

        self.foreign_lobbyserver = lobbyserver;

        unsafe {
            if !networkHandler.deref_handler().gamehost_restart() {
                return false;
            }
        }

        self.active = true;
        println!("[net] GameHost started.");
        true
    }

    pub fn update(&mut self) {
        if self.active {
            unsafe { networkHandler.deref_handler().gamehost_listen(self) };
        }
    }

    pub fn stop(&mut self) {
        if !self.active {
            return;
        }

        self.foreign_lobbyserver = Ptr::null();
        self.next_free_client_slot = 0;
        self.expected_client_count = 0;
        for p in 0..3 {
            if !self.clients[p].is_null() {
                self.clients[p].disconnect();
                self.clients[p] = Ptr::null();
            }
            self.expected_clients[p].reset();
            self.last_processed_input_id[p] = 0xFF;
        }

        unsafe { networkHandler.deref_handler().gamehost_shutdown() };
        self.active = false;
        println!("[net] GameHost stopped.");
    }

    pub fn cleanup(&mut self) {
        self.stop();
    }

    pub fn send_start_room_message(&mut self) {
        let pkg = pkgs::StartRoom::new();
        self.foreign_lobbyserver.send_reliable(pkg.as_bytes());
        self.set_as_last_sent_message(pkg.header.packageType);
    }

    fn send_sync_messages(&mut self) {
        println!("[net] Prepare launching the game...");

        RandomNumberGenerator::generator().reseed(unsafe { libc_time() } as u32);
        crate::smw::gs_menu::roll_net_game_mode_settings();
        let settings = game_mode_settings_package();
        let pkg = pkgs::StartSync::new(RANDOM_INT(32767) as u32);
        for c in 0..3 {
            self.expected_clients[c].loaded = false;
        }
        self.send_message_to_my_peers(&settings);
        self.send_message_to_my_peers(pkg.as_bytes());

        unsafe {
            netplay.client.handle_start_sync_message(pkg.as_bytes());
            netplay.client.set_as_last_received_message(pkg.header.packageType);
        }
    }

    fn handle_sync_ok_message(&mut self, player: &dyn NetPeer, _data: &[u8]) {
        let mut readyPlayerCount: u8 = 0;
        for c in 0..self.expected_client_count as usize {
            if player.same_peer(&*self.clients[c]) {
                self.expected_clients[c].sync_ok = true;
                println!("Client {}/{} ready.", c + 1, self.expected_client_count);
            }

            if self.expected_clients[c].sync_ok {
                readyPlayerCount += 1;
            }
        }

        if readyPlayerCount == self.expected_client_count {
            self.send_start_game_message();
        }
    }

    fn set_expected_players(&mut self, count: u8, hosts: &[u32; 3], ports: &[u16; 3]) {
        self.expected_client_count = count;
        for c in 0..count as usize {
            self.expected_clients[c].host = hosts[c];
            self.expected_clients[c].port = ports[c];
        }

        println!("[net] Game starts soon, waiting for {} players.", self.expected_client_count);

        for c in 0..self.expected_client_count as usize {
            let (h, p) = (self.expected_clients[c].host, self.expected_clients[c].port);
            unsafe { networkHandler.deref_handler().nat_punch(h, p) };
        }
    }

    /// The host plays gameplay frame 1 once every joiner that is still connected has loaded the match.
    fn start_if_everyone_loaded(&mut self) {
        unsafe {
            let everyone = (0..self.expected_client_count as usize)
                .all(|c| self.expected_clients[c].loaded || self.clients[c].is_null() || netplay.player_disconnected[c + 1]);
            if everyone {
                self.send_go_message();
            }
        }
    }

    fn send_go_message(&mut self) {
        self.send_message_to_my_peers(MessageHeader::new(NET_G2P_GO).as_bytes());
        unsafe { netplay.start_waiting = false };
    }

    fn send_start_game_message(&mut self) {
        let pkg = gpkgs::StartGame::new();
        self.foreign_lobbyserver.send_reliable(pkg.as_bytes());
        self.send_message_to_my_peers(pkg.as_bytes());

        unsafe {
            netplay.client.handle_game_start_message();
            netplay.client.set_as_last_received_message(pkg.header.packageType);
        }
    }

    pub fn send_local_input(&mut self) {
        unsafe {
            if netplay.local_input_buffer.is_empty() {
                return;
            }

            let mut pkg = gpkgs::RemoteInput::new(0);
            let front = *netplay.local_input_buffer.front().unwrap();
            pkg.write_keys(&front);
            netplay.local_input_buffer.pop_front();

            for c in 0..3 {
                if !self.clients[c].is_null() {
                    self.clients[c].send(pkg.as_bytes());
                }
            }

            netplay.client.handle_remote_input(pkg.as_bytes());
            netplay.client.set_as_last_received_message(pkg.header.packageType);
        }
    }

    pub fn send_current_game_state_if_needed(&mut self) {
        if self.current_server_tick % NET_GAMESTATE_FRAMES_TO_SEND == 0 {
            self.send_current_game_state_now();
        }

        self.current_server_tick = self.current_server_tick.wrapping_add(1);
    }

    fn send_current_game_state_now(&mut self) {
        let mut pkg = gpkgs::GameState::new();

        unsafe {
            for p in 0..players.len() as u8 {
                let pl = players[p as usize];
                pkg.set_player_coord(p, pl.fx, pl.fy);
                pkg.set_player_vel(p, pl.velx, pl.vely);
            }

            for c in 0..self.expected_client_count as usize {
                if !self.clients[c].is_null() {
                    pkg.last_confirmed_local_input_id = self.last_processed_input_id[c];
                    self.clients[c].send(pkg.as_bytes());
                }
            }

            netplay.client.handle_remote_game_state(pkg.as_bytes());
            netplay.client.set_as_last_received_message(pkg.header.packageType);
        }
    }

    pub fn confirm_current_inputs(&mut self) {
        unsafe {
            for c in 0..self.expected_client_count as usize {
                let Some(front) = netplay.remote_input_buffer[c + 1].front() else {
                    continue;
                };

                let input_id = front.0;
                self.last_processed_input_id[c] = self.last_processed_input_id[c].max(input_id);
                if self.last_processed_input_id[c] > 200 && input_id < 50 {
                    self.last_processed_input_id[c] = input_id;
                }
            }
        }
    }

    fn handle_remote_input(&mut self, player: &dyn NetPeer, data: &[u8]) {
        let pkg = gpkgs::ClientInput::from_bytes(data);

        for c in 0..self.expected_client_count as usize {
            if player.same_peer(&*self.clients[c]) {
                let mut keys = COutputControl::default();
                pkg.read_keys(&mut keys);
                unsafe { netplay.remote_input_buffer[c + 1].push_back((pkg.input_id, keys)) };

                let pkg_out = gpkgs::RemoteInput::with_input((c + 1) as u8, pkg.input);
                for c_out in 0..self.expected_client_count as usize {
                    if !self.clients[c_out].is_null() && c != c_out {
                        self.clients[c_out].send(pkg_out.as_bytes());
                    }
                }

                return;
            }
        }
    }

    pub fn send_powerup_start_by_gh(&mut self) {
        unsafe {
            let me = players[netplay.remotePlayerNumber as usize];
            let pkg = gpkgs::StartPowerup::new(netplay.remotePlayerNumber, me.powerupused.unwrap() as u8, 0);
            self.send_message_to_my_peers(pkg.as_bytes());

            let mut me = me;
            me.net_waitingForPowerupTrigger = false;
            println!("[net] P{} (host) used powerup {}", pkg.player_id, pkg.powerup_id);
        }
    }

    fn handle_powerup_request(&mut self, player: &dyn NetPeer, _data: &[u8]) {
        let mut playerID: u8 = 0xFF;
        for c in 0..self.expected_client_count as usize {
            if player.same_peer(&*self.clients[c]) {
                playerID = (c + 1) as u8;
                break;
            }
        }

        if playerID == 0xFF {
            return;
        }

        unsafe {
            if net_player(playerID).is_null() {
                return;
            }

            let powerupused: i32 = game_values.gamepowerups[playerID as usize] as i32;
            if powerupused <= 0 {
                return;
            }

            game_values.gamepowerups[playerID as usize] = -1;

            let pkg = gpkgs::StartPowerup::new(playerID, powerupused as u8, player.average_rtt() / 2);
            for c in 0..self.expected_client_count as usize {
                if !self.clients[c].is_null() {
                    self.clients[c].send_reliable(pkg.as_bytes());
                }
            }
            netplay.client.handle_powerup_start(pkg.as_bytes());
            netplay.client.set_as_last_received_message(pkg.header.packageType);
        }
    }

    pub fn prepare_map_collision_event(&mut self, player: &crate::smw::player::CPlayer) {
        self.preparedMapCollPkg.fill(player);
    }

    pub fn send_map_collision_event(&mut self) {
        let bytes = self.preparedMapCollPkg.as_bytes().to_vec();
        self.send_message_to_my_peers(&bytes);

        println!("[net] P{} collided with a map block", self.preparedMapCollPkg.player_id);
    }

    pub fn send_p2p_collision_event(&mut self, p1: &crate::smw::player::CPlayer, p2: &crate::smw::player::CPlayer) {
        let pkg = gpkgs::P2PCollision::from_players(p1, p2);
        self.send_message_to_my_peers(pkg.as_bytes());

        println!("[net] P{} collided with P{}", p1.get_global_id(), p2.get_global_id());
    }

    pub fn send_message_to_my_peers(&mut self, data: &[u8]) -> bool {
        for c in 0..3 {
            if !self.clients[c].is_null() {
                self.clients[c].send_reliable(data);
            }
        }

        self.set_as_last_sent_message(data[2]);
        true
    }

    fn set_as_last_sent_message(&mut self, packageType: u8) {
        self.lastSentMessage.packageType = packageType;
        self.lastSentMessage.timestamp = ticks();
    }

    fn set_as_last_received_message(&mut self, packageType: u8) {
        self.lastReceivedMessage.packageType = packageType;
        self.lastReceivedMessage.timestamp = ticks();
    }
}

impl NetworkEventHandler for NetGameHost {
    fn on_connect(&mut self, new_player: Box<dyn NetPeer>) {
        let mut new_player = Ptr::from_box(new_player);

        if self.next_free_client_slot >= 3 {
            new_player.disconnect();
            println!("[error] More than 3 players want to join.");
            return;
        }

        let incoming = RawPlayerAddress { host: new_player.address_host(), port: new_player.address_port(), sync_ok: false, loaded: false };

        let mut valid_address = false;
        let mut c = 0;
        while (c as u8) < self.expected_client_count && !valid_address {
            if incoming.same(&self.expected_clients[c]) {
                valid_address = true;
            }
            c += 1;
        }

        if !valid_address {
            println!("[warning] An unexpected client wants to join the game.");
            new_player.disconnect();
            return;
        }

        self.clients[self.next_free_client_slot as usize] = new_player;

        self.next_free_client_slot += 1;
        println!("{}/{} player connected.", self.next_free_client_slot, self.expected_client_count);

        if self.next_free_client_slot == self.expected_client_count {
            self.send_sync_messages();
        }
    }

    fn on_disconnect(&mut self, player: &mut dyn NetPeer) {
        println!("[net] Client {} disconnected", player.address_as_string());
        for c in 0..self.expected_client_count as usize {
            if !self.clients[c].is_null() && self.clients[c].same_peer(player) {
                unsafe { netplay.player_disconnected[c + 1] = true };
                return;
            }
        }
    }

    fn on_receive(&mut self, player: &mut dyn NetPeer, data: &[u8]) {
        if data.len() < 3 {
            return;
        }

        let protocolMajor = data[0];
        let protocolMinor = data[1];
        let packageType = data[2];
        if protocolMajor != NET_PROTOCOL_VERSION_MAJOR || protocolMinor != NET_PROTOCOL_VERSION_MINOR {
            println!("Not implemented: bad protocol version");
            return;
        }

        match packageType {
            NET_P2G_SYNC_OK => self.handle_sync_ok_message(player, data),
            NET_P2G_LOADED => {
                for c in 0..self.expected_client_count as usize {
                    if !self.clients[c].is_null() && player.same_peer(&*self.clients[c]) {
                        self.expected_clients[c].loaded = true;
                    }
                }
            }
            NET_P2G_LOCAL_KEYS => self.handle_remote_input(player, data),
            NET_P2G_REQ_POWERUP => self.handle_powerup_request(player, data),
            _ => {
                print!("GH Unknown: ");
                for b in data {
                    print!("{:3} ", b);
                }
                println!();
            }
        }
    }
}

extern "C" {
    #[cfg_attr(not(windows), link_name = "time")]
    #[cfg_attr(windows, link_name = "_time64")]
    fn libc_time_raw(t: *mut i64) -> i64;
}

unsafe fn libc_time() -> i64 {
    libc_time_raw(std::ptr::null_mut())
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Net_PlayerData {
    pub x: f32,
    pub y: f32,
    pub xvel: f32,
    pub yvel: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Net_AllPlayerData {
    pub player: [Net_PlayerData; 4],
    pub player_input: [VecDeque<COutputControl>; 4],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Net_IndexedPlayerData {
    pub data: Net_PlayerData,
    pub input_id: u8,
}

impl Net_IndexedPlayerData {
    pub fn new(id: u8) -> Self {
        Net_IndexedPlayerData { data: Net_PlayerData::default(), input_id: id }
    }
}

pub type NetworkState = i32;
pub const INACTIVE: NetworkState = 0;
pub const DISCONNECTED: NetworkState = 1;
pub const CONNECTED: NetworkState = 2;
pub const JOINED: NetworkState = 3;
pub const PLAYING: NetworkState = 4;

pub struct Networking {
    pub active: bool,

    pub connectSuccessful: bool,
    pub joinSuccessful: bool,
    pub gameRunning: bool,

    pub currentMenuChanged: bool,
    pub operationInProgress: bool,

    pub client: NetClient,
    pub myPlayerName: String,

    pub selectedServerIndex: u16,
    pub savedServers: Vec<ServerAddress>,

    pub roomFilter: String,
    pub selectedRoomIndex: u16,
    pub currentRooms: Vec<RoomListEntry>,
    pub currentRoom: Room,

    pub newroom_name: String,
    pub newroom_password: String,
    pub mychatmessage: String,
    pub mapfilepath: String,

    pub theHostIsMe: bool,
    pub remotePlayerNumber: u8,
    pub hostPlayerNumber: u8,
    pub netPlayerInput: CPlayerInput,

    pub waitingForPowerupTrigger: bool,
    pub allowMapCollisionEvent: bool,

    pub gamestate_changed: bool,
    pub frames_since_last_gamestate: u32,
    pub previous_playerdata: Net_AllPlayerData,
    pub latest_playerdata: Net_AllPlayerData,
    pub last_confirmed_input: u16,
    pub current_input_counter: u8,
    pub remote_input_buffer: [VecDeque<(u8, COutputControl)>; 4],
    pub player_disconnected: [bool; 4],

    pub local_input_buffer: VecDeque<COutputControl>,
    pub local_playerdata_buffer: VecDeque<Net_IndexedPlayerData>,
    pub local_playerdata_store_time: [nettimepoint; 256],

    /// The sync's seed; every client reseeds with it again when the match starts.
    pub common_random_seed: u32,
    /// Until every client has loaded the match, nobody plays gameplay frame 1 (`waiting_to_start`).
    pub start_waiting: bool,
    pub start_waited_since: u32,
    /// A joiner leaves remote players where setup put them until the host's first game state arrives.
    pub gamestate_received: bool,

    pub _alias: Aliased,
}

impl Networking {
    /// Zero-initialized like the C++ global, plus the member constructors.
    pub fn new() -> Self {
        Networking {
            active: false,
            connectSuccessful: false,
            joinSuccessful: false,
            gameRunning: false,
            currentMenuChanged: false,
            operationInProgress: false,
            client: NetClient::new(),
            myPlayerName: String::new(),
            selectedServerIndex: 0,
            savedServers: Vec::new(),
            roomFilter: String::new(),
            selectedRoomIndex: 0,
            currentRooms: Vec::new(),
            currentRoom: Room::new(),
            newroom_name: String::new(),
            newroom_password: String::new(),
            mychatmessage: String::new(),
            mapfilepath: String::new(),
            theHostIsMe: false,
            remotePlayerNumber: 0,
            hostPlayerNumber: 0,
            netPlayerInput: CPlayerInput::new(),
            waitingForPowerupTrigger: false,
            allowMapCollisionEvent: false,
            gamestate_changed: false,
            frames_since_last_gamestate: 0,
            previous_playerdata: Net_AllPlayerData::default(),
            latest_playerdata: Net_AllPlayerData::default(),
            last_confirmed_input: 0,
            current_input_counter: 0,
            remote_input_buffer: Default::default(),
            player_disconnected: [false; 4],
            local_input_buffer: VecDeque::new(),
            local_playerdata_buffer: VecDeque::new(),
            local_playerdata_store_time: [SystemTime::UNIX_EPOCH; 256],
            common_random_seed: 0,
            start_waiting: false,
            start_waited_since: 0,
            gamestate_received: false,
            _alias: Aliased::new(),
        }
    }
}
