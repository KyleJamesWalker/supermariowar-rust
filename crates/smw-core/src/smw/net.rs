//! Port of src/smw/net.cpp

use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game_mode::{GameModeType, *};
use crate::common::game_mode_settings::GAMEMODESETTINGS_RAW_SIZE;

use crate::common::global_constants::WAITTIME;
use crate::common::input::{COutputControl, CPlayerInput};
use crate::common::path::get_home_directory;
use crate::common::random_number_generator::{RandomNumberGenerator, RandomNumberGeneratorType, RANDOM_INT};
use crate::common_netplay::network_interface::{NetPeer, NetworkEventHandler};
use crate::common_netplay::protocol_definitions::*;
use crate::common_netplay::protocol_packages::{self as pkgs, cstr_field, MessageHeader};
use crate::globals::{Aliased, Global, Ptr};
use crate::smw::checkpoint::{local_path, portable_path, Field, Snap};

use crate::smw::network::file_compressor::FileCompressor;
use crate::smw::network::net_config_manager::NetConfigManager;
use crate::smw::network::net_record::{self, PeerInfo, ReplayPeer, Side};
use crate::smw::network::network_layer::NetworkLayer;
use crate::smw::network::protocol_game_packages as gpkgs;
use crate::smw::objectgame::PowerupType;
use crate::smw::ui::network_list_scroll::MI_NetworkListScroll;
use std::collections::VecDeque;
use std::path::Path;

#[cfg(feature = "no_network")]
pub type NetworkHandler = crate::smw::platform::network::null::network_layer_null::NetworkLayerNULL;
#[cfg(all(not(feature = "no_network"), not(target_os = "emscripten")))]
pub type NetworkHandler = crate::smw::platform::network::enet::network_layer_enet::NetworkLayerENet;
#[cfg(all(not(feature = "no_network"), target_os = "emscripten"))]
pub type NetworkHandler = crate::smw::platform::network::websocket::network_layer_websocket::NetworkLayerWebSocket;



/// Constructs the statics C++ builds before `main`.
pub fn init_globals(g: &mut Game) {
    g.networkHandler.init(NetworkHandler::default());
    g.netplay.init(Networking::new());
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

    fn set_global(&self, g: &mut Game, mode: GameModeType) {
        let Some((off, size)) = gms_member(mode) else {
            return;
        };
        let mut raw: [u8; GAMEMODESETTINGS_RAW_SIZE] = g.game_values.gamemodesettings.to_raw_bytes();
        raw[off..off + size].copy_from_slice(&self.bytes[..size]);
        g.game_values.gamemodesettings.from_raw_bytes(&raw);
    }

    fn read_global(&mut self, g: &mut Game, mode: GameModeType) {
        let Some((off, size)) = gms_member(mode) else {
            return;
        };
        let raw = g.game_values.gamemodesettings.to_raw_bytes();
        self.bytes[..size].copy_from_slice(&raw[off..off + size]);
    }
}

/// Not in the C++, which indexes `players` by ID: a client that already removed players at the end of its game
/// would read past the vector.
fn net_player(g: &mut Game, id: u8) -> crate::smw::players::PlayerId {
    g.players.iter().copied().find(|&p| g.player(p).globalID as u8 == id).unwrap_or(crate::smw::players::PlayerId::NONE)
}

fn ticks(g: &mut Game) -> u32 {
    crate::smw::harness::net_ticks(g)
}

pub fn net_init(g: &mut Game) -> bool {
    g.netplay.active = false;
    g.netplay.connectSuccessful = false;
    g.netplay.joinSuccessful = false;
    g.netplay.gameRunning = false;

    g.netplay.myPlayerName = "Player".to_string();
    g.netplay.currentMenuChanged = false;
    g.netplay.theHostIsMe = false;
    g.netplay.selectedRoomIndex = 0;
    g.netplay.selectedServerIndex = 0;
    g.netplay.roomFilter.clear();
    g.netplay.newroom_name.clear();
    g.netplay.newroom_password.clear();
    g.netplay.mychatmessage.clear();
    g.netplay.allowMapCollisionEvent = false;

    if !net_layer(g).init(g) {
        return false;
    }

    if !g.netplay.client.init() {
        return false;
    }

    net_load_server_list(g);

    println!("[net] Network system initialized.");
    true
}

pub fn net_close(g: &mut Game) {
        let mut gp = g.ptr();
    net_save_server_list(g);

    net_end_session(g);
    gp.netplay.client.cleanup(g);

    net_layer(g).cleanup(g);
}

pub fn net_save_server_list(g: &mut Game) {
    let mut config = NetConfigManager;
    config.save(g);
}

pub fn net_load_server_list(g: &mut Game) {
    let mut config = NetConfigManager;
    config.load(g);

    #[cfg(all(not(feature = "no_network"), target_os = "emscripten"))]
    let relay_url = crate::smw::platform::network::websocket::network_layer_websocket::relay_url;
    #[cfg(not(all(not(feature = "no_network"), target_os = "emscripten")))]
    let relay_url = || None;
    // Recorded on every build, so a native replay of a browser recording lists the same servers.
    if let Some(url) = net_record::value_opt(g, "relay", relay_url) {
        g.netplay.savedServers.retain(|s| s.hostname != url);
        g.netplay.savedServers.insert(0, ServerAddress { hostname: url });
    }
}

/****************************
    Session
****************************/

pub fn net_start_session(g: &mut Game) -> bool {
        let mut gp = g.ptr();
    net_end_session(g);

    println!("[net] Session start.");
    g.netplay.active = true;
    g.netplay.connectSuccessful = false;
    g.netplay.gamestate_changed = false;
    g.netplay.frames_since_last_gamestate = 0;
    g.netplay.player_disconnected = [false; 4];

    let nullinput = COutputControl::default();
    for p in 0..4 {
        g.backup_playercontrol[p] = g.game_values.playercontrol[p];

        g.netplay.latest_playerdata.player_input[p].clear();
        g.netplay.latest_playerdata.player_input[p].push_back(nullinput);

        g.netplay.local_input_buffer.clear();
        g.netplay.remote_input_buffer[p].clear();
    }

    gp.netplay.client.start(g)
}

pub fn net_end_session(g: &mut Game) {
        let mut gp = g.ptr();
    if g.netplay.active {
        println!("[net] Session end.");

        gp.netplay.client.stop(g);

        g.netplay.active = false;
        g.netplay.connectSuccessful = false;

        for p in 0..4 {
            g.game_values.playercontrol[p] = g.backup_playercontrol[p];
        }
    }
}

/// The network layer, borrowed apart from `g` so `g` can be passed to it.
fn net_layer<'a>(g: &mut Game) -> &'a mut NetworkHandler {
    g.networkHandler.as_ptr().get()
}

/// How long the host waits for its joiners to load, and a joiner for the host's go, before playing anyway.
const START_TIMEOUT_MS: u32 = 5000;
/// How long every client shows the match settings before gameplay frame 1, at least.
const PREVIEW_MS: u32 = 2500;

/// Not in the C++: the match settings every client shows while it waits to start (`waiting_to_start`).
pub fn draw_match_preview(g: &mut Game) {
        let mut gp = g.ptr();
    gp.rm.sprite(gp.rm.menu_backdrop).draw(g, 0, 0);
    gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, 144, 100, &Rect { x: 0, y: 0, w: 336, h: 264 });
    gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, 480, 100, &Rect { x: 496, y: 0, w: 16, h: 264 });
    gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, 144, 364, &Rect { x: 0, y: 464, w: 336, h: 16 });
    gp.rm.sprite(gp.rm.menu_dialog).draw_src(g, 480, 364, &Rect { x: 496, y: 464, w: 16, h: 16 });

    let gm = g.game_values.gamemode;
    let mut lines = vec![gm.szModeName.clone()];
    if !gm.szGoalName.is_empty() {
        lines.push(if gm.goal < 0 { format!("{}: Unlimited", gm.szGoalName) } else { format!("{}: {}", gm.szGoalName, gm.goal) });
    }
    lines.extend(crate::smw::gs_menu::describe_net_game_mode_settings(g));
    if !g.netplay.match_map_name.is_empty() {
        lines.push(format!("Map: {}", g.netplay.match_map_name));
    }
    gp.rm.menu_font_large.draw_centered(g, 320, 116, "Match Settings");
    for (i, line) in lines.iter().enumerate() {
        gp.rm.menu_font_large.draw_chop_centered(g, 320, 160 + 30 * i as i32, 320, line);
    }
    gp.rm.menu_font_small.draw_centered(g, 320, 340, if g.netplay.match_randomized { "Randomized by the host" } else { "Chosen by the host" });
}

/// Not in the C++: called before every gameplay frame. While true, the frame only polls the network, so no client
/// plays gameplay frame 1 before every client has loaded the match (`NET_P2G_LOADED`, then the host's `NET_G2P_GO`).
pub fn waiting_to_start(g: &mut Game) -> bool {
        let mut gp = g.ptr();
    if !g.netplay.active || !g.netplay.start_waiting {
        return false;
    }
    let now = ticks(g);
    if g.netplay.start_waited_since == 0 {
        g.netplay.start_waited_since = now.max(1);
        if !g.netplay.theHostIsMe {
            let peer = g.netplay.client.foreign_gamehost;
            gp.netplay.client.send_to(g, peer, MessageHeader::new(NET_P2G_LOADED).as_bytes());
        }
    }
    gp.netplay.client.update(g);
    if g.netplay.theHostIsMe && g.netplay.start_waiting {
        gp.netplay.client.local_gamehost.start_if_everyone_loaded(g);
    }
    let limit = if g.netplay.theHostIsMe { START_TIMEOUT_MS } else { 2 * START_TIMEOUT_MS };
    if g.netplay.start_waiting && now.wrapping_sub(g.netplay.start_waited_since) > limit {
        println!("[net] Not every client loaded the match in time; starting anyway.");
        if g.netplay.theHostIsMe {
            gp.netplay.client.local_gamehost.send_go_message(g);
        }
        g.netplay.start_waiting = false;
    }
    g.netplay.start_waiting
}

/// Not in the C++: every client draws in step from the start of the match, whatever its menus drew since the sync.
pub fn reseed_for_match(g: &mut Game) {
    let mut gp = g.ptr();
    RandomNumberGenerator::generator(gp.get()).reseed(g.netplay.common_random_seed);
}

/// `NET_NOTICE_GAMEMODESETTINGS` with the current game mode's settings.
fn game_mode_settings_package(g: &mut Game) -> Vec<u8> {
    let mut blob = vec![0u8; std::mem::size_of::<MessageHeader>() + GMS_UNION_SIZE];

    let header = MessageHeader::new(NET_NOTICE_GAMEMODESETTINGS);
    blob[..3].copy_from_slice(header.as_bytes());

    let mut modesettings = GameModeSettingsUnion::new();
    modesettings.read_global(g, g.currentgamemode as GameModeType);
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
    /// The reply a waiting menu acts on; later messages in the same poll cannot overwrite it.
    pub lastReply: LastMessage,

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
            lastReply: LastMessage::default(),
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

    pub fn start(&mut self, g: &mut Game) -> bool {
        let mut gp = g.ptr();
        net_record::value_bool(g, "client_restart", || net_layer(gp.get()).client_restart(gp.get()))
    }

    pub fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        self.local_gamehost.update(g);
        net_record::listen(g, Side::Client, self, |g, handler| net_layer(g).client_listen(g, handler));
    }

    pub fn stop(&mut self, g: &mut Game) {
        self.local_gamehost.stop(g);

        if g.netplay.connectSuccessful {
            self.send_goodbye(g);
        }

        if !self.foreign_lobbyserver.is_null() {
            self.foreign_lobbyserver.disconnect();
            self.foreign_lobbyserver = Ptr::null();
        }
        if !self.foreign_gamehost.is_null() {
            self.foreign_gamehost.disconnect();
            self.foreign_gamehost = Ptr::null();
        }

        if !net_record::replaying(g) {
            net_layer(g).client_shutdown(g);
        }
    }

    pub fn cleanup(&mut self, g: &mut Game) {
        self.stop(g);
    }

    pub fn set_room_list_ui_control(&mut self, control: Ptr<MI_NetworkListScroll>) {
        self.uiRoomList = control;
    }

    /****************************
        Phase 1: Connect
    ****************************/

    pub fn send_connect_request_to_selected_server(&mut self, g: &mut Game) -> bool {
        let hostname = g.netplay.savedServers[g.netplay.selectedServerIndex as usize].hostname.clone();
        if !self.connect_lobby(g, &hostname, NET_LOBBYSERVER_PORT) {
            return false;
        }

        g.netplay.operationInProgress = true;
        true
    }

    fn send_goodbye(&mut self, g: &mut Game) {
        let msg = pkgs::ClientDisconnection::new();
        self.send_message_to_lobby_server(g, msg.as_bytes());
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

    pub fn request_room_list(&mut self, g: &mut Game) {
        let msg = pkgs::RoomList::new();
        self.send_message_to_lobby_server(g, msg.as_bytes());

        g.netplay.currentRooms.clear();
        if !self.uiRoomList.is_null() {
            self.uiRoomList.clear();
        }
    }

    fn handle_new_room_list_entry(&mut self, g: &mut Game, data: &[u8]) {
        let roomInfo = pkgs::RoomInfo::from_bytes(data);

        let newRoom = RoomListEntry { roomID: roomInfo.roomID, name: cstr_field(&roomInfo.name), playerCount: roomInfo.currentPlayerCount };
        println!("  Incoming room entry: [{}] {} ({}/4)", newRoom.roomID, newRoom.name, newRoom.playerCount);
        let name = newRoom.name.clone();
        g.netplay.currentRooms.push(newRoom);

        if !self.uiRoomList.is_null() {
            let playerCountString: String =
                [(b'0'.wrapping_add(roomInfo.currentPlayerCount)) as char, '/', '4'].iter().collect();
            self.uiRoomList.add(&name, &playerCountString);
        }
    }

    pub fn send_create_room_message(&mut self, g: &mut Game) {
        let mut msg = pkgs::NewRoom::with(&g.netplay.newroom_name, &g.netplay.newroom_password);
        msg.gamemodeID = g.currentgamemode as u8;
        g.game_values.gamemode = g.gamemodes[g.currentgamemode as usize];
        msg.gamemodeGoal = g.game_values.gamemode.goal as u16;

        self.send_message_to_lobby_server(g, msg.as_bytes());
        g.netplay.operationInProgress = true;
    }

    fn handle_room_created_message(&mut self, g: &mut Game, data: &[u8]) {
        println!("room created!");
        let pkg = pkgs::NewRoomCreated::from_bytes(data);

        g.netplay.currentRoom.roomID = pkg.roomID;
        g.netplay.currentRoom.hostPlayerNumber = 0;
        g.netplay.currentRoom.name = g.netplay.newroom_name.clone();
        g.netplay.currentRoom.playerNames[0] = g.netplay.myPlayerName.clone();
        g.netplay.currentRoom.playerNames[1] = "(empty)".to_string();
        g.netplay.currentRoom.playerNames[2] = "(empty)".to_string();
        g.netplay.currentRoom.playerNames[3] = "(empty)".to_string();

        g.netplay.currentMenuChanged = true;
        g.netplay.joinSuccessful = true;
        g.netplay.theHostIsMe = true;

        g.game_values.playercontrol[0] = 1;
        g.game_values.playercontrol[1] = 0;
        g.game_values.playercontrol[2] = 0;
        g.game_values.playercontrol[3] = 0;

        self.send_map_change_message(g);
        self.send_game_mode_settings_change_message(g);

        let lobby = self.foreign_lobbyserver;
        self.local_gamehost.start(g, lobby);
    }

    pub fn send_join_room_message(&mut self, g: &mut Game) {
        let entry = g.netplay.currentRooms[g.netplay.selectedRoomIndex as usize].clone();
        println!(
            "currentRooms[{}] = {{id={}, name='{}', cnt={}}}, össz: {}",
            g.netplay.selectedRoomIndex,
            entry.roomID,
            entry.name,
            entry.playerCount,
            g.netplay.currentRooms.len()
        );

        let msg = pkgs::JoinRoom::with(entry.roomID, "");
        self.send_message_to_lobby_server(g, msg.as_bytes());
        g.netplay.operationInProgress = true;
    }

    pub fn send_leave_room_message(&mut self, g: &mut Game) {
        self.local_gamehost.stop(g);

        let msg = pkgs::LeaveRoom::new();
        self.send_message_to_lobby_server(g, msg.as_bytes());
    }

    fn handle_room_changed_message(&mut self, g: &mut Game, data: &[u8]) {
        let pkg = pkgs::CurrentRoom::from_bytes(data);

        g.netplay.currentRoom.roomID = pkg.roomID;
        g.netplay.currentRoom.name = cstr_field(&pkg.name);

        println!("Room {} ({}) changed:", pkg.roomID, cstr_field(&pkg.name));
        for p in 0..4u8 {
            g.netplay.currentRoom.playerNames[p as usize] = cstr_field(&pkg.playerName[p as usize]);
            print!("  player {}: {}", p + 1, g.netplay.currentRoom.playerNames[p as usize]);
            if p == pkg.remotePlayerNumber {
                print!(" (me)");
            }
            if p == pkg.hostPlayerNumber {
                print!(" (HOST)");
            }
            println!();

            if g.netplay.currentRoom.playerNames[p as usize] == "(empty)" {
                g.game_values.playercontrol[p as usize] = 0;
            } else {
                g.game_values.playercontrol[p as usize] = 1;
            }
        }

        g.currentgamemode = pkg.gamemodeID as i16;
        g.game_values.gamemode = g.gamemodes[g.currentgamemode as usize];
        g.game_values.gamemode.goal = pkg.gamemodeGoal as i16;

        let gm = g.gamemodes[g.currentgamemode as usize];
        println!("  Game mode #{}: {} with {}: {}", g.currentgamemode, gm.get_mode_name(), gm.get_goal_name(), g.game_values.gamemode.goal);

        g.netplay.theHostIsMe = false;
        g.netplay.currentRoom.hostPlayerNumber = pkg.hostPlayerNumber;
        g.netplay.remotePlayerNumber = pkg.remotePlayerNumber;
        g.netplay.hostPlayerNumber = pkg.hostPlayerNumber;
        if g.netplay.remotePlayerNumber == pkg.hostPlayerNumber {
            g.netplay.theHostIsMe = true;
            let lobby = self.foreign_lobbyserver;
            self.local_gamehost.start(g, lobby);
        } else {
            self.local_gamehost.stop(g);
        }

        g.netplay.currentMenuChanged = true;
    }

    pub fn send_chat_message(&mut self, g: &mut Game, message: &str) {
        debug_assert!(!message.is_empty());
        let pkg = pkgs::RoomChatMsg::with(g.netplay.remotePlayerNumber, message);
        self.send_message_to_lobby_server(g, pkg.as_bytes());
    }

    pub fn send_map_change_message(&mut self, g: &mut Game) {
        let mapfilepath = g.netplay.mapfilepath.clone();
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

        self.send_message_to_lobby_server(g, &compressed.data);
    }

    fn handle_map_change_message(&mut self, g: &mut Game, data: &[u8]) {
        if data.len() <= std::mem::size_of::<MessageHeader>() + 4 || data.len() > 20000 {
            println!("[error] Corrupt map arrived");
            return;
        }

        let full_size = u16::from_ne_bytes([data[3], data[4]]);
        let compressed_size = u16::from_ne_bytes([data[5], data[6]]);

        println!("[net] Map arrived (C:{} unC:{})", compressed_size, full_size);

        g.netplay.mapfilepath = get_home_directory(g) + "net_last.map";
        let path = g.netplay.mapfilepath.clone();
        if !FileCompressor::decompress(&data[3..], &path) {
            return;
        }
    }

    pub fn send_game_mode_settings_change_message(&mut self, g: &mut Game) {
        let blob = game_mode_settings_package(g);
        self.send_message_to_lobby_server(g, &blob);
    }

    fn handle_game_mode_settings_change_message(&mut self, g: &mut Game, data: &[u8]) {
        if data.len() != std::mem::size_of::<MessageHeader>() + GMS_UNION_SIZE {
            println!("[error] Corrupt game mode settings arrived");
            return;
        }

        let mut modesettings = GameModeSettingsUnion::new();
        modesettings.bytes.copy_from_slice(&data[3..]);
        modesettings.set_global(g, g.currentgamemode as GameModeType);

        println!("[net] Game mode settings changed");
    }

    fn handle_room_chat_message(&mut self, g: &mut Game, data: &[u8]) {
        let pkg = pkgs::RoomChatMsg::from_bytes(data);

        println!("Not implemented: [net] {}: {}", g.netplay.currentRoom.playerNames[pkg.senderNum as usize], cstr_field(&pkg.message));
    }

    pub fn send_skin_change(&mut self, g: &mut Game) {
        let skinpath = g.skinlist.at(g.game_values.skinids[g.netplay.remotePlayerNumber as usize] as usize).path.to_string_lossy().into_owned();
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

        self.send_message_to_lobby_server(g, &compressed.data);
    }

    fn handle_skin_change_message(&mut self, g: &mut Game, data: &[u8]) {
        let mut gp = g.ptr();
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

        let path = format!("{}net_skin{}.bmp", get_home_directory(g), playerID);
        if !FileCompressor::decompress(&data[4..], &path) {
            return;
        }

        if !gp.rm.load_menu_skin_path(g, playerID as i16, Path::new(&path), playerID as i16, false) {
            println!("[warning] Could not load netplay skin of player {}, using default", playerID);
            gp.get().rm.load_menu_skin(g, playerID as i16, gp.get().game_values.skinids[playerID as usize], playerID as i16, false);
        }
    }

    /****************************
        Phase 2.5: Pre-game
    ****************************/

    fn handle_room_start_message(&mut self, g: &mut Game, client: &dyn NetPeer, data: &[u8]) {
        let mut pkg = pkgs::GameHostInfo::from_bytes(data);

        if pkg.host == 0x100007F {
            pkg.host = client.address_host();
        }

        let host_str = host_to_string(pkg.host);

        println!("[net] Connecting to game host... [{}:{}]", host_str, NET_GAMEHOST_PORT);
        if !self.connect_game_host(g, &host_str, NET_GAMEHOST_PORT) {
            println!("[net][error] Could not connect to game host.");
            return;
        }

        g.netplay.operationInProgress = true;
    }

    fn handle_expected_clients_message(&mut self, g: &mut Game, client: &dyn NetPeer, data: &[u8]) {
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

        self.local_gamehost.set_expected_players(g, playerCount, &hosts, &ports);
    }

    pub fn handle_start_sync_message(&mut self, g: &mut Game, data: &[u8]) {
        let mut gp = g.ptr();
        let pkg = pkgs::StartSync::from_bytes(data);

        println!("reseed: {}", pkg.commonRandomSeed as i32);
        RandomNumberGenerator::generator(gp.get()).reseed(pkg.commonRandomSeed);
        g.netplay.common_random_seed = pkg.commonRandomSeed;
        g.netplay.start_waiting = true;
        g.netplay.start_waited_since = 0;
        g.netplay.gamestate_received = false;
        crate::smw::net_random::begin_setup(g);

        g.netplay.player_disconnected = [false; 4];
        g.netplay.last_confirmed_input = 0xFF;
        g.netplay.current_input_counter = 0;
        g.netplay.local_playerdata_store_count = g.netplay.local_playerdata_store_count.wrapping_add(1);
        g.netplay.local_playerdata_store_time = [g.netplay.local_playerdata_store_count; 256];

        if g.netplay.theHostIsMe {
            self.set_as_last_sent_message(g, NET_P2G_SYNC_OK);
        } else {
            let respond_pkg = gpkgs::SyncOK::new();
            self.send_message_to_game_host(g, respond_pkg.as_bytes());
        }
    }

    pub fn handle_game_start_message(&mut self, g: &mut Game) {
        println!("[net] Game start!");
        g.netplay.gameRunning = true;
    }

    /****************************
        Phase 3: Play
    ****************************/

    pub fn send_leave_game_message(&mut self, g: &mut Game) {
        let pkg = gpkgs::LeaveGame::new();
        if g.netplay.theHostIsMe {
            self.local_gamehost.send_message_to_my_peers(g, pkg.as_bytes());
            self.local_gamehost.update(g);
            self.local_gamehost.stop(g);

            self.set_as_last_received_message(g, pkg.header.packageType);
        } else {
            self.send_message_to_game_host(g, pkg.as_bytes());
        }

        g.netplay.gameRunning = false;
    }

    pub fn store_local_input(&mut self, g: &mut Game) {
        if g.netplay.theHostIsMe {
            let c = g.game_values.playerInput.outputControls[0];
            g.netplay.local_input_buffer.push_back(c);
        }
    }

    pub fn send_local_input(&mut self, g: &mut Game) {
        if g.netplay.theHostIsMe {
            self.local_gamehost.send_local_input(g);
        } else {
            let mut pkg = gpkgs::ClientInput::new(&g.game_values.playerInput.outputControls[0]);
            pkg.input_id = g.netplay.current_input_counter;
            self.send_message_to_game_host(g, pkg.as_bytes());
        }
    }

    pub fn send_powerup_request(&mut self, g: &mut Game) {
        let pkg = gpkgs::RequestPowerup::new();
        self.send_message_to_game_host(g, pkg.as_bytes());
    }

    pub fn handle_powerup_start(&mut self, g: &mut Game, data: &[u8]) {
        let pkg = gpkgs::StartPowerup::from_bytes(data);

        if pkg.player_id > 3 {
            return;
        }

        let mut player = net_player(g, pkg.player_id);
        if player.is_none() {
            return;
        }

        let mut missed_frames: u32 = pkg.delay;
        if !g.netplay.theHostIsMe {
            missed_frames = missed_frames.wrapping_add(net_record::value_u32(g, "rtt", || self.foreign_gamehost.average_rtt()) / 2);
        }
        missed_frames /= WAITTIME as u32;

        g.player(player).powerupused = Some(PowerupType::from_u8(pkg.powerup_id));
        g.player(player).powerupradius = 100.0;
        while missed_frames > 0 {
            g.player(player).powerupradius -= g.game_values.storedpowerupdelay as f32 / 2.0;
            missed_frames -= 1;
        }

        g.player(player).net_waitingForPowerupTrigger = false;
        println!("[net] P{} used powerup {}", pkg.player_id, pkg.powerup_id);
    }

    fn handle_map_collision(&mut self, g: &mut Game, data: &[u8]) {
        let pkg = gpkgs::MapCollision::from_bytes(data);

        if pkg.player_id > 3 {
            return;
        }

        let mut player = net_player(g, pkg.player_id);
        if player.is_none() {
            return;
        }

        let temp_px = g.player(player).fx;
        let temp_py = g.player(player).fy;
        let temp_velx = g.player(player).velx;
        let temp_vely = g.player(player).vely;
        g.player(player).set_xf(pkg.player_x);
        g.player(player).set_yf(pkg.player_y);
        g.player(player).velx = pkg.player_xvel;
        g.player(player).vely = pkg.player_yvel;

        g.netplay.allowMapCollisionEvent = true;
        g.player(player).collision_detection_map(g);
        g.netplay.allowMapCollisionEvent = false;

        if g.player(player).isdead() {
            return;
        }

        g.player(player).set_xf(temp_px);
        g.player(player).set_yf(temp_py);
        g.player(player).velx = temp_velx;
        g.player(player).vely = temp_vely;

        println!("[net] Map block collision!");
    }

    fn handle_p2p_collision(&mut self, g: &mut Game, data: &[u8]) {
        let pkg = gpkgs::P2PCollision::from_bytes(data);

        if pkg.player_id[0] > 3 || pkg.player_id[1] > 3 {
            return;
        }

        let mut player1 = net_player(g, pkg.player_id[0]);
        let mut player2 = net_player(g, pkg.player_id[1]);
        if player1.is_none() || player2.is_none() {
            return;
        }

        g.player(player1).set_xf(pkg.player_x[0]);
        g.player(player1).set_yf(pkg.player_y[0]);
        g.player(player1).fOldY = pkg.player_oldy[0];

        g.player(player2).set_xf(pkg.player_x[1]);
        g.player(player2).set_yf(pkg.player_y[1]);
        g.player(player2).fOldY = pkg.player_oldy[1];

        g.player(player1).collides_with_player(g, player2);

        println!("P{} collided with P{}!", pkg.player_id[0], pkg.player_id[1]);
    }

    pub fn handle_remote_input(&mut self, g: &mut Game, data: &[u8]) {
        let pkg = gpkgs::RemoteInput::from_bytes(data);
        let mut keys = COutputControl::default();
        pkg.read_keys(&mut keys);
        g.netplay.remote_input_buffer[pkg.playerNumber as usize].push_back((0, keys));
    }

    pub fn handle_remote_game_state(&mut self, g: &mut Game, data: &[u8]) {
        let pkg = gpkgs::GameState::from_bytes(data);

        g.netplay.previous_playerdata = g.netplay.latest_playerdata.clone();

        for p in 0..g.players.len() as u8 {
            let d = &mut g.netplay.latest_playerdata.player[p as usize];
            pkg.get_player_coord(p, &mut d.x, &mut d.y);
            pkg.get_player_vel(p, &mut d.xvel, &mut d.yvel);
        }

        if !g.netplay.gamestate_received {
            g.netplay.previous_playerdata = g.netplay.latest_playerdata.clone();
        }
        g.netplay.gamestate_changed = true;
        g.netplay.gamestate_received = true;
        g.netplay.frames_since_last_gamestate = 0;
        g.netplay.last_confirmed_input = pkg.last_confirmed_local_input_id as u16;
    }

    /****************************
        Network Communication
    ****************************/

    fn connect_lobby(&mut self, g: &mut Game, hostname: &str, port: u16) -> bool {
        let mut gp = g.ptr();
        g.netplay.connectSuccessful = false;
        net_record::value_bool(g, "connect_lobby", || net_layer(gp.get()).connect_to_lobby_server(gp.get(), hostname, port))
    }

    fn connect_game_host(&mut self, g: &mut Game, hostname: &str, port: u16) -> bool {
        let mut gp = g.ptr();
        g.netplay.connectSuccessful = false;
        net_record::value_bool(g, "connect_game_host", || net_layer(gp.get()).connect_to_foreign_game_host(gp.get(), hostname, port))
    }

    /// Every client message goes reliably: a lost lobby request or start handshake leaves the client waiting forever.
    fn send_to(&mut self, g: &mut Game, mut peer: Ptr<dyn NetPeer>, data: &[u8]) -> bool {
        if !peer.send_reliable(data) {
            return false;
        }

        self.set_as_last_sent_message(g, data[2]);
        true
    }

    fn send_message_to_lobby_server(&mut self, g: &mut Game, data: &[u8]) -> bool {
        let peer = self.foreign_lobbyserver;
        self.send_to(g, peer, data)
    }

    fn send_message_to_game_host(&mut self, g: &mut Game, data: &[u8]) -> bool {
        let peer = self.foreign_gamehost;
        self.send_to(g, peer, data)
    }

    pub fn set_as_last_sent_message(&mut self, g: &mut Game, packageType: u8) {
        self.lastSentMessage.packageType = packageType;
        self.lastSentMessage.timestamp = ticks(g);
        if matches!(packageType, NET_REQUEST_CONNECT | NET_REQUEST_JOIN_ROOM | NET_REQUEST_CREATE_ROOM | NET_P2G_SYNC_OK) {
            self.lastReply = LastMessage::default();
        }
    }

    pub fn set_as_last_received_message(&mut self, g: &mut Game, packageType: u8) {
        self.lastReceivedMessage.packageType = packageType;
        self.lastReceivedMessage.timestamp = ticks(g);
        if matches!(packageType, NET_RESPONSE_CONNECT_OK | NET_RESPONSE_JOIN_OK | NET_RESPONSE_CREATE_OK | NET_G2E_GAME_START) {
            self.lastReply = self.lastReceivedMessage;
        }
    }

    /// Takes the reply a waiting menu was waiting for, so it moves on once.
    pub fn take_reply(&mut self, packageType: u8) -> bool {
        let found = self.lastReply.packageType == packageType;
        if found {
            self.lastReply = LastMessage::default();
        }
        found
    }
}

impl NetworkEventHandler<Game> for NetClient {
    fn on_connect(&mut self, g: &mut Game, newPeer: Box<dyn NetPeer>) {
        let mut newPeer = Ptr::from_box(newPeer);

        if self.foreign_lobbyserver.is_null() {
            self.foreign_lobbyserver = newPeer;

            let message = pkgs::ClientConnection::with(&g.netplay.myPlayerName);
            self.send_message_to_lobby_server(g, message.as_bytes());
        } else if self.foreign_gamehost.is_null() {
            self.foreign_gamehost = newPeer;
        } else {
            newPeer.disconnect();
        }
    }

    fn on_disconnect(&mut self, g: &mut Game, client: &mut dyn NetPeer) {
        if !self.foreign_gamehost.is_null() && client.same_peer(&*self.foreign_gamehost) {
            println!("[net] Disconnected from game host.");
            let remote = g.netplay.remotePlayerNumber as usize;
            g.netplay.player_disconnected[remote] = true;
            return;
        }

        if !self.foreign_lobbyserver.is_null() && client.same_peer(&*self.foreign_lobbyserver) {
            println!("[net] Disconnected from lobby server.");
            return;
        }

        println!("[net] Client {} disconnected", client.address_as_string());
    }

    fn on_receive(&mut self, g: &mut Game, client: &mut dyn NetPeer, data: &[u8]) {
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
            NET_RESPONSE_BADPROTOCOL => println!("Not implemented: NET_RESPONSE_BADPROTOCOL"),

            NET_RESPONSE_SERVERINFO => self.handle_serverinfo_and_close(data),

            NET_RESPONSE_SERVER_MOTD => println!("Not implemented: NET_RESPONSE_SERVER_MOTD"),

            NET_RESPONSE_CONNECT_OK => {
                // Not in the C++: the Servers menu moves on only if the skin goes out no earlier than this receive.
                self.set_as_last_received_message(g, packageType);
                g.netplay.connectSuccessful = true;
                self.send_skin_change(g);
                return;
            }

            NET_RESPONSE_CONNECT_DENIED => {
                println!("Not implemented: NET_RESPONSE_CONNECT_DENIED");
                g.netplay.connectSuccessful = false;
            }

            NET_RESPONSE_CONNECT_SERVERFULL => {
                println!("Not implemented: NET_RESPONSE_CONNECT_SERVERFULL");
                g.netplay.connectSuccessful = false;
            }

            NET_RESPONSE_CONNECT_NAMETAKEN => {
                println!("Not implemented: NET_RESPONSE_CONNECT_NAMETAKEN");
                g.netplay.connectSuccessful = false;
            }

            NET_RESPONSE_ROOM_LIST_ENTRY => self.handle_new_room_list_entry(g, data),

            NET_RESPONSE_NO_ROOMS => println!("Not implemented: [net] There are no rooms currently on the server."),

            NET_RESPONSE_JOIN_OK => {
                println!("Not implemented: [net] Joined to room.");
                g.netplay.joinSuccessful = true;
                g.netplay.theHostIsMe = false;
            }

            NET_RESPONSE_ROOM_FULL => {
                println!("Not implemented: [net] The room is full");
                g.netplay.joinSuccessful = false;
            }

            NET_NOTICE_ROOM_CHANGE => self.handle_room_changed_message(g, data),

            NET_NOTICE_MAP_CHANGE => self.handle_map_change_message(g, data),

            NET_NOTICE_SKIN_CHANGE => self.handle_skin_change_message(g, data),

            NET_NOTICE_GAMEMODESETTINGS => self.handle_game_mode_settings_change_message(g, data),

            NET_NOTICE_ROOM_CHAT_MSG => self.handle_room_chat_message(g, data),

            NET_RESPONSE_CREATE_OK => self.handle_room_created_message(g, data),

            NET_RESPONSE_CREATE_ERROR => println!("Not implemented: NET_RESPONSE_CREATE_ERROR"),

            NET_L2P_GAMEHOST_INFO => self.handle_room_start_message(g, client, data),

            NET_L2G_CLIENTS_INFO => self.handle_expected_clients_message(g, client, data),

            NET_G2P_SYNC => self.handle_start_sync_message(g, data),

            NET_G2E_GAME_START => self.handle_game_start_message(g),

            NET_G2P_REMOTE_KEYS => self.handle_remote_input(g, data),

            NET_G2P_GAME_STATE => self.handle_remote_game_state(g, data),

            NET_G2P_START_POWERUP => self.handle_powerup_start(g, data),

            NET_G2P_TRIGGER_MAPCOLL => self.handle_map_collision(g, data),

            NET_G2P_TRIGGER_P2PCOLL => self.handle_p2p_collision(g, data),


            NET_G2P_GO => g.netplay.start_waiting = false,

            NET_G2P_MATCH_MAP if data.len() > 3 => {
                g.netplay.match_randomized = data[3] != 0;
                g.netplay.match_map_name = String::from_utf8_lossy(&data[4..]).into_owned();
            }

            NET_G2P_RANDOM_EVENT => crate::smw::net_random::receive(g, data),

            _ => {
                print!("Unknown: ");
                for b in data {
                    print!("{:3} ", b);
                }
                println!();
                return;
            }
        }

        self.set_as_last_received_message(g, packageType);
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

    pub fn start(&mut self, g: &mut Game, lobbyserver: Ptr<dyn NetPeer>) -> bool {
        let mut gp = g.ptr();
        if self.active {
            return true;
        }

        self.foreign_lobbyserver = lobbyserver;

        if !net_record::value_bool(g, "gamehost_restart", || net_layer(gp.get()).gamehost_restart(gp.get())) {
            return false;
        }

        self.active = true;
        println!("[net] GameHost started.");
        true
    }

    pub fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if self.active {
            net_record::listen(g, Side::Host, self, |g, handler| net_layer(g).gamehost_listen(g, handler));
        }
    }

    pub fn stop(&mut self, g: &mut Game) {
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

        if !net_record::replaying(g) {
            net_layer(g).gamehost_shutdown(g);
        }
        self.active = false;
        println!("[net] GameHost stopped.");
    }

    pub fn cleanup(&mut self, g: &mut Game) {
        self.stop(g);
    }

    pub fn send_start_room_message(&mut self, g: &mut Game) {
        let pkg = pkgs::StartRoom::new();
        self.foreign_lobbyserver.send_reliable(pkg.as_bytes());
        self.set_as_last_sent_message(g, pkg.header.packageType);
    }

    fn send_sync_messages(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        println!("[net] Prepare launching the game...");

        RandomNumberGenerator::generator(gp.get()).reseed(net_record::value_u32(g, "time", || crate::smw::harness::net_seed().unwrap_or(unsafe { libc_time() } as u32)));
        if g.netplay.randomize_settings {
            crate::smw::gs_menu::roll_net_game_mode_settings(g);
        } else {
            crate::smw::gs_gameplay::set_game_mode_settings_from_menu(g);
        }
        let settings = game_mode_settings_package(g);
        let map_name = {
            let path = g.netplay.mapfilepath.clone();
            g.netplay.match_map_name = Path::new(&path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            g.netplay.match_map_name.clone()
        };
        g.netplay.match_randomized = g.netplay.randomize_settings;
        let mut map_pkg = MessageHeader::new(NET_G2P_MATCH_MAP).as_bytes().to_vec();
        map_pkg.push(g.netplay.randomize_settings as u8);
        map_pkg.extend_from_slice(map_name.as_bytes());
        let pkg = pkgs::StartSync::new(RANDOM_INT(gp.get(), 32767) as u32);
        for c in 0..3 {
            self.expected_clients[c].loaded = false;
        }
        self.send_message_to_my_peers(g, &settings);
        self.send_message_to_my_peers(g, &map_pkg);
        self.send_message_to_my_peers(g, pkg.as_bytes());

        gp.netplay.client.handle_start_sync_message(g, pkg.as_bytes());
        gp.netplay.client.set_as_last_received_message(g, pkg.header.packageType);
    }

    fn handle_sync_ok_message(&mut self, g: &mut Game, player: &dyn NetPeer, _data: &[u8]) {
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
            self.send_start_game_message(g);
        }
    }

    fn set_expected_players(&mut self, g: &mut Game, count: u8, hosts: &[u32; 3], ports: &[u16; 3]) {
        self.expected_client_count = count;
        for c in 0..count as usize {
            self.expected_clients[c].host = hosts[c];
            self.expected_clients[c].port = ports[c];
        }

        println!("[net] Game starts soon, waiting for {} players.", self.expected_client_count);

        for c in 0..self.expected_client_count as usize {
            let (h, p) = (self.expected_clients[c].host, self.expected_clients[c].port);
            if !net_record::replaying(g) {
                net_layer(g).nat_punch(g, h, p);
            }
        }
    }

    /// The host plays gameplay frame 1 once every joiner that is still connected has loaded the match.
    fn start_if_everyone_loaded(&mut self, g: &mut Game) {
        let everyone = (0..self.expected_client_count as usize)
            .all(|c| self.expected_clients[c].loaded || self.clients[c].is_null() || g.netplay.player_disconnected[c + 1]);
        if everyone && ticks(g).wrapping_sub(g.netplay.start_waited_since) >= PREVIEW_MS {
            self.send_go_message(g);
        }
    }

    fn send_go_message(&mut self, g: &mut Game) {
        self.send_message_to_my_peers(g, MessageHeader::new(NET_G2P_GO).as_bytes());
        g.netplay.start_waiting = false;
    }

    fn send_start_game_message(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let pkg = gpkgs::StartGame::new();
        self.foreign_lobbyserver.send_reliable(pkg.as_bytes());
        self.send_message_to_my_peers(g, pkg.as_bytes());

        gp.netplay.client.handle_game_start_message(g);
        gp.netplay.client.set_as_last_received_message(g, pkg.header.packageType);
    }

    pub fn send_local_input(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        if g.netplay.local_input_buffer.is_empty() {
            return;
        }

        let mut pkg = gpkgs::RemoteInput::new(0);
        let front = *g.netplay.local_input_buffer.front().unwrap();
        pkg.write_keys(&front);
        g.netplay.local_input_buffer.pop_front();

        for c in 0..3 {
            if !self.clients[c].is_null() {
                self.clients[c].send(pkg.as_bytes());
            }
        }

        gp.netplay.client.handle_remote_input(g, pkg.as_bytes());
        gp.netplay.client.set_as_last_received_message(g, pkg.header.packageType);
    }

    pub fn send_current_game_state_if_needed(&mut self, g: &mut Game) {
        if self.current_server_tick % NET_GAMESTATE_FRAMES_TO_SEND == 0 {
            self.send_current_game_state_now(g);
        }

        self.current_server_tick = self.current_server_tick.wrapping_add(1);
    }

    fn send_current_game_state_now(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        let mut pkg = gpkgs::GameState::new();

        for p in 0..g.players.len() as u8 {
            let pl = g.players[p as usize];
            pkg.set_player_coord(p, g.player(pl).fx, g.player(pl).fy);
            pkg.set_player_vel(p, g.player(pl).velx, g.player(pl).vely);
        }

        for c in 0..self.expected_client_count as usize {
            if !self.clients[c].is_null() {
                pkg.last_confirmed_local_input_id = self.last_processed_input_id[c];
                self.clients[c].send(pkg.as_bytes());
            }
        }

        gp.netplay.client.handle_remote_game_state(g, pkg.as_bytes());
        gp.netplay.client.set_as_last_received_message(g, pkg.header.packageType);
    }

    pub fn confirm_current_inputs(&mut self, g: &mut Game) {
        let g = g;
        for c in 0..self.expected_client_count as usize {
            let Some(front) = g.netplay.remote_input_buffer[c + 1].front() else {
                continue;
            };

            let input_id = front.0;
            self.last_processed_input_id[c] = self.last_processed_input_id[c].max(input_id);
            if self.last_processed_input_id[c] > 200 && input_id < 50 {
                self.last_processed_input_id[c] = input_id;
            }
        }
    }

    fn handle_remote_input(&mut self, g: &mut Game, player: &dyn NetPeer, data: &[u8]) {
        let pkg = gpkgs::ClientInput::from_bytes(data);

        for c in 0..self.expected_client_count as usize {
            if player.same_peer(&*self.clients[c]) {
                let mut keys = COutputControl::default();
                pkg.read_keys(&mut keys);
                g.netplay.remote_input_buffer[c + 1].push_back((pkg.input_id, keys));

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

    pub fn send_powerup_start_by_gh(&mut self, g: &mut Game) {
        let me = g.players[g.netplay.remotePlayerNumber as usize];
        let pkg = gpkgs::StartPowerup::new(g.netplay.remotePlayerNumber, g.player(me).powerupused.unwrap() as u8, 0);
        self.send_message_to_my_peers(g, pkg.as_bytes());

        let mut me = me;
        g.player(me).net_waitingForPowerupTrigger = false;
        println!("[net] P{} (host) used powerup {}", pkg.player_id, pkg.powerup_id);
    }

    fn handle_powerup_request(&mut self, g: &mut Game, player: &dyn NetPeer, _data: &[u8]) {
        let mut gp = g.ptr();
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

        if net_player(g, playerID).is_none() {
            return;
        }

        let powerupused: i32 = g.game_values.gamepowerups[playerID as usize] as i32;
        if powerupused <= 0 {
            return;
        }

        g.game_values.gamepowerups[playerID as usize] = -1;

        let pkg = gpkgs::StartPowerup::new(playerID, powerupused as u8, net_record::value_u32(g, "rtt", || player.average_rtt()) / 2);
        for c in 0..self.expected_client_count as usize {
            if !self.clients[c].is_null() {
                self.clients[c].send_reliable(pkg.as_bytes());
            }
        }
        gp.netplay.client.handle_powerup_start(g, pkg.as_bytes());
        gp.netplay.client.set_as_last_received_message(g, pkg.header.packageType);
    }

    pub fn prepare_map_collision_event(&mut self, player: &crate::smw::player::CPlayer) {
        self.preparedMapCollPkg.fill(player);
    }

    pub fn send_map_collision_event(&mut self, g: &mut Game) {
        let bytes = self.preparedMapCollPkg.as_bytes().to_vec();
        self.send_message_to_my_peers(g, &bytes);

        println!("[net] P{} collided with a map block", self.preparedMapCollPkg.player_id);
    }

    pub fn send_p2p_collision_event(&mut self, g: &mut Game, p1: &crate::smw::player::CPlayer, p2: &crate::smw::player::CPlayer) {
        let pkg = gpkgs::P2PCollision::from_players(p1, p2);
        self.send_message_to_my_peers(g, pkg.as_bytes());

        println!("[net] P{} collided with P{}", p1.get_global_id(), p2.get_global_id());
    }

    pub fn send_message_to_my_peers(&mut self, g: &mut Game, data: &[u8]) -> bool {
        for c in 0..3 {
            if !self.clients[c].is_null() {
                self.clients[c].send_reliable(data);
            }
        }

        self.set_as_last_sent_message(g, data[2]);
        true
    }

    fn set_as_last_sent_message(&mut self, g: &mut Game, packageType: u8) {
        self.lastSentMessage.packageType = packageType;
        self.lastSentMessage.timestamp = ticks(g);
    }

    fn set_as_last_received_message(&mut self, g: &mut Game, packageType: u8) {
        self.lastReceivedMessage.packageType = packageType;
        self.lastReceivedMessage.timestamp = ticks(g);
    }
}

impl NetworkEventHandler<Game> for NetGameHost {
    fn on_connect(&mut self, g: &mut Game, new_player: Box<dyn NetPeer>) {
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
            self.send_sync_messages(g);
        }
    }

    fn on_disconnect(&mut self, g: &mut Game, player: &mut dyn NetPeer) {
        println!("[net] Client {} disconnected", player.address_as_string());
        for c in 0..self.expected_client_count as usize {
            if !self.clients[c].is_null() && self.clients[c].same_peer(player) {
                g.netplay.player_disconnected[c + 1] = true;
                return;
            }
        }
    }

    fn on_receive(&mut self, g: &mut Game, player: &mut dyn NetPeer, data: &[u8]) {
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
            NET_P2G_SYNC_OK => self.handle_sync_ok_message(g, player, data),
            NET_P2G_LOADED => {
                for c in 0..self.expected_client_count as usize {
                    if !self.clients[c].is_null() && player.same_peer(&*self.clients[c]) {
                        self.expected_clients[c].loaded = true;
                    }
                }
            }
            NET_P2G_LOCAL_KEYS => self.handle_remote_input(g, player, data),
            NET_P2G_REQ_POWERUP => self.handle_powerup_request(g, player, data),
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
    /// When each input's player data was stored, as a count of stores.
    pub local_playerdata_store_time: [u32; 256],
    pub local_playerdata_store_count: u32,

    /// The sync's seed; every client reseeds with it again when the match starts.
    pub common_random_seed: u32,
    /// Until every client has loaded the match, nobody plays gameplay frame 1 (`waiting_to_start`).
    pub start_waiting: bool,
    pub start_waited_since: u32,
    /// A joiner leaves remote players where setup put them until the host's first game state arrives.
    pub gamestate_received: bool,
    /// The host's "Randomize" room setting: roll the mode settings at the sync instead of playing the menu's.
    pub randomize_settings: bool,
    /// The match's map name, and whether its settings were randomized, for the preview while every client loads.
    pub match_map_name: String,
    pub match_randomized: bool,

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
            local_playerdata_store_time: [0; 256],
            local_playerdata_store_count: 0,
            common_random_seed: 0,
            start_waiting: false,
            start_waited_since: 0,
            gamestate_received: false,
            randomize_settings: false,
            match_map_name: String::new(),
            match_randomized: false,
            _alias: Aliased::new(),
        }
    }
}

/****************************
    Checkpoints
****************************/

impl Field for PeerInfo {
    fn io(&mut self, s: &mut Snap) {
        s.io(&mut self.host);
        s.io(&mut self.port);
        s.io(&mut self.player_id);
        s.io(&mut self.key);
    }
}

impl Field for LastMessage {
    fn io(&mut self, s: &mut Snap) {
        s.io(&mut self.packageType);
        s.io(&mut self.timestamp);
    }
}

impl Field for COutputControl {
    fn io(&mut self, s: &mut Snap) {
        for key in self.keys.iter_mut() {
            s.io(&mut key.fDown);
            s.io(&mut key.fPressed);
        }
    }
}

impl Field for Net_PlayerData {
    fn io(&mut self, s: &mut Snap) {
        s.io(&mut self.x);
        s.io(&mut self.y);
        s.io(&mut self.xvel);
        s.io(&mut self.yvel);
    }
}

impl Field for Net_IndexedPlayerData {
    fn io(&mut self, s: &mut Snap) {
        s.io(&mut self.input_id);
        s.io(&mut self.data);
    }
}

impl Field for Net_AllPlayerData {
    fn io(&mut self, s: &mut Snap) {
        s.io(&mut self.player);
        s.io(&mut self.player_input);
    }
}

impl Field for RawPlayerAddress {
    fn io(&mut self, s: &mut Snap) {
        s.io(&mut self.host);
        s.io(&mut self.port);
        s.io(&mut self.sync_ok);
        s.io(&mut self.loaded);
    }
}

impl Field for Room {
    fn io(&mut self, s: &mut Snap) {
        s.io(&mut self.roomID);
        s.io(&mut self.name);
        s.io(&mut self.playerNames);
        s.io(&mut self.hostPlayerNumber);
        s.io(&mut self.gamemodeID);
        s.io(&mut self.gamemodeGoal);
    }
}

/// A peer as its address; a segment replay's peers are `ReplayPeer`s like the replayed events'.
fn peer_io(s: &mut Snap, peer: &mut Ptr<dyn NetPeer>) {
    let mut info = (!peer.is_null()).then(|| net_record::info(&**peer));
    s.io(&mut info);
    if s.loading() {
        *peer = match info {
            Some(info) => Ptr::from_box(Box::new(ReplayPeer(info)) as Box<dyn NetPeer>),
            None => Ptr::null(),
        };
    }
}

/// The skin a net game's remote player plays with, as `MenuState::start_game` loaded it.
pub fn net_skin_path(g: &mut Game, player: usize) -> Option<String> {
    if !g.netplay.active || player == g.netplay.remotePlayerNumber as usize {
        return None;
    }
    let path = format!("{}net_skin{}.bmp", get_home_directory(g), player);
    Path::new(&path).exists().then_some(path)
}

/// An online match's checkpoint (smw/checkpoint.rs): the net session, its peers and the files the host sent.
pub fn checkpoint(g: &mut Game, s: &mut Snap) {
    let mut gp = g.ptr();
    unsafe {
        let home = get_home_directory(g);
        let mut files: Vec<(String, Vec<u8>)> = Vec::new();
        if !s.loading() {
            let mut names = Vec::new();
            if g.netplay.mapfilepath == home.clone() + "net_last.map" {
                names.push("net_last.map".to_string());
            }
            for k in 0..4 {
                if g.game_values.playercontrol[k] > 0 && k != g.netplay.remotePlayerNumber as usize {
                    names.push(format!("net_skin{}.bmp", k));
                }
            }
            files = names.into_iter().filter_map(|n| Some((n.clone(), std::fs::read(home.clone() + &n).ok()?))).collect();
        }
        s.io(&mut files);
        if s.loading() {
            for (name, bytes) in &files {
                if name == "net_last.map" || (name.starts_with("net_skin") && name.ends_with(".bmp") && !name.contains('/')) {
                    let _ = std::fs::write(home.clone() + name, bytes);
                }
            }
        }

        let n = &mut *gp.netplay;
        s.io(&mut n.active);
        s.io(&mut n.connectSuccessful);
        s.io(&mut n.joinSuccessful);
        s.io(&mut n.gameRunning);
        s.io(&mut n.currentMenuChanged);
        s.io(&mut n.operationInProgress);
        s.io(&mut n.myPlayerName);
        s.io(&mut n.currentRoom);
        let mut map = portable_path(g, &n.mapfilepath);
        s.io(&mut map);
        n.mapfilepath = local_path(g, &map);
        s.io(&mut n.theHostIsMe);
        s.io(&mut n.remotePlayerNumber);
        s.io(&mut n.hostPlayerNumber);
        s.io(&mut n.netPlayerInput.outputControls);
        s.io(&mut n.waitingForPowerupTrigger);
        s.io(&mut n.allowMapCollisionEvent);
        s.io(&mut n.gamestate_changed);
        s.io(&mut n.frames_since_last_gamestate);
        s.io(&mut n.previous_playerdata);
        s.io(&mut n.latest_playerdata);
        s.io(&mut n.last_confirmed_input);
        s.io(&mut n.current_input_counter);
        s.io(&mut n.remote_input_buffer);
        s.io(&mut n.player_disconnected);
        s.io(&mut n.local_input_buffer);
        s.io(&mut n.local_playerdata_buffer);
        s.io(&mut n.local_playerdata_store_time);
        s.io(&mut n.local_playerdata_store_count);
        s.io(&mut n.common_random_seed);
        s.io(&mut n.start_waiting);
        s.io(&mut n.start_waited_since);
        s.io(&mut n.gamestate_received);
        s.io(&mut *(&raw mut g.backup_playercontrol));

        let c = &mut n.client;
        s.io(&mut c.lastSentMessage);
        s.io(&mut c.lastReceivedMessage);
        peer_io(s, &mut c.foreign_lobbyserver);
        peer_io(s, &mut c.foreign_gamehost);

        let g = &mut c.local_gamehost;
        s.io(&mut g.lastSentMessage);
        s.io(&mut g.lastReceivedMessage);
        s.io(&mut g.active);
        s.io(&mut g.current_server_tick);
        for peer in g.clients.iter_mut() {
            peer_io(s, peer);
        }
        peer_io(s, &mut g.foreign_lobbyserver);
        s.io(&mut g.expected_clients);
        s.io(&mut g.expected_client_count);
        s.io(&mut g.next_free_client_slot);
        s.io(&mut g.last_processed_input_id);
    }
    crate::smw::net_random::checkpoint(g, s);
    crate::smw::net_outcomes::checkpoint(g, s);
}
