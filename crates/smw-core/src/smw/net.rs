//! Port of src/smw/net.cpp

use crate::common::uicontrol::{ControlId, Ctl};
use crate::common::gfx::soft_surface::Rect;
use crate::game::Game;
use crate::common::game_mode::{GameModeType, *};
use crate::common::game_mode_settings::GAMEMODESETTINGS_RAW_SIZE;

use crate::common::global_constants::{TICKS_PER_SECOND, vel};
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
use crate::smw::network::peer_probe::PeerProbe;
use crate::smw::objectgame::PowerupType;
use crate::smw::ui::network_list_scroll::MI_NetworkListScroll;
use std::collections::VecDeque;
use std::path::Path;

#[cfg(feature = "no_network")]
pub type NetworkHandler = crate::smw::platform::network::null::network_layer_null::NetworkLayerNULL;
#[cfg(not(feature = "no_network"))]
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoomListEntry {
    pub roomID: u32,
    pub name: String,
    pub playerCount: u8,
    pub passwordRequired: bool,
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
    #[cfg(not(target_os = "emscripten"))]
    if !net_record::replaying(g) {
        crate::smw::network::lan_beacon::listen();
    }
    #[cfg(not(target_os = "emscripten"))]
    if std::env::var("SMW_LAN_HOST").is_ok_and(|v| v == "1") {
        host_on_this_device(g);
    }
}

/// Starts this game's own lobby and signaling for its network (the Servers menu's Host on This Device, or
/// `SMW_LAN_HOST=1`) and selects it. False on a build that cannot host.
pub fn host_on_this_device(g: &mut Game) -> bool {
    #[cfg(target_os = "emscripten")]
    return false;
    #[cfg(not(target_os = "emscripten"))]
    {
        let url = lan_host();
        g.netplay.savedServers.retain(|s| s.hostname != url);
        g.netplay.savedServers.insert(0, ServerAddress { hostname: url });
        g.netplay.selectedServerIndex = 0;
        true
    }
}

/// Puts the LAN hosts heard from recently at the top of the server list and drops the ones gone quiet. Replays read
/// the list from the recording. True when the list changed.
pub fn merge_nearby(g: &mut Game) -> bool {
    #[cfg(not(target_os = "emscripten"))]
    let live = || Some(crate::smw::network::lan_beacon::nearby().join(" "));
    #[cfg(target_os = "emscripten")]
    let live = || None;
    let heard = net_record::value_opt(g, "nearby", live).unwrap_or_default();
    let mut heard: Vec<String> = heard.split(' ').filter(|s| !s.is_empty()).map(String::from).collect();
    let n = &mut *g.netplay;
    heard.retain(|url| n.nearby.contains(url) || !n.savedServers.iter().any(|s| &s.hostname == url));
    if heard == n.nearby {
        return false;
    }
    let selected = n.savedServers.get(n.selectedServerIndex as usize).map(|s| s.hostname.clone());
    n.savedServers.retain(|s| !n.nearby.contains(&s.hostname));
    for url in heard.iter().rev() {
        n.savedServers.insert(0, ServerAddress { hostname: url.clone() });
    }
    n.nearby = heard;
    n.selectedServerIndex = selected.and_then(|h| n.savedServers.iter().position(|s| s.hostname == h)).unwrap_or(0) as u16;
    true
}

/// The relay port a game hosting on its own network listens on.
#[cfg(not(target_os = "emscripten"))]
pub const LAN_HOST_PORT: u16 = 12523;

/// Runs the lobby and signaling for this game's network once (docs/design/NETPLAY_V2.md, step 6) and returns its URL
/// here. Nearby games find it by its beacon, or players add `ws://<this device's address>:12523`.
#[cfg(not(target_os = "emscripten"))]
fn lan_host() -> String {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        smw_relay::run_embedded(LAN_HOST_PORT);
        crate::smw::network::lan_beacon::broadcast(LAN_HOST_PORT);
        let address = crate::smw::network::lan_beacon::local_address().map_or("this device's address".to_string(), |a| a.to_string());
        println!("[net] Hosting on this device: other players add ws://{address}:{LAN_HOST_PORT}");
    });
    format!("ws://127.0.0.1:{LAN_HOST_PORT}")
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
    g.backup_playercontrol = g.game_values.playercontrol;

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

/// How long the Servers menu waits for the lobby to accept the connection.
const CONNECT_DEADLINE_FRAMES: u32 = 8 * TICKS_PER_SECOND;
/// How long the lobby menus wait for the server to answer a join or create request.
const REQUEST_DEADLINE_FRAMES: u32 = 5 * TICKS_PER_SECOND;
/// How often the lobby menu asks for the room list again.
const ROOM_LIST_REFRESH_FRAMES: u32 = 3 * TICKS_PER_SECOND;

/// Why a lobby request failed, for the menu to show.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LobbyError {
    Unreachable,
    ConnectTimeout,
    ClientOutdated,
    ServerOutdated,
    VersionMismatch,
    Denied,
    ServerFull,
    NameTaken,
    ConnectionLost,
    RequestTimeout,
    RoomFull,
    RoomNotFound,
    WrongPassword,
    CreateFailed,
}

impl LobbyError {
    pub fn message(self) -> &'static str {
        match self {
            LobbyError::Unreachable => "Could not reach the server",
            LobbyError::ConnectTimeout | LobbyError::RequestTimeout => "The server did not answer",
            LobbyError::ClientOutdated => "Update the game for this server",
            LobbyError::ServerOutdated => "The server needs an update",
            LobbyError::VersionMismatch => "Game and server versions differ",
            LobbyError::Denied => "The server refused to connect",
            LobbyError::ServerFull => "The server is full",
            LobbyError::NameTaken => "That name is already taken",
            LobbyError::ConnectionLost => "Lost connection to the server",
            LobbyError::RoomFull => "The room is full",
            LobbyError::RoomNotFound => "That room no longer exists",
            LobbyError::WrongPassword => "Wrong room password",
            LobbyError::CreateFailed => "Could not create the room",
        }
    }

    /// Whether the lobby connection is unusable, so the player starts over from the Servers menu.
    pub fn disconnects(self) -> bool {
        !matches!(
            self,
            LobbyError::RequestTimeout | LobbyError::RoomFull | LobbyError::RoomNotFound | LobbyError::WrongPassword | LobbyError::CreateFailed
        )
    }
}

/// How long the host waits for its joiners to load, and a joiner for the host's go, before playing anyway.
const START_TIMEOUT_MS: u32 = 5000;

/// Not in the C++: every client draws in step from the start of the match, whatever its menus drew since the sync.
pub fn reseed_for_match(g: &mut Game) {
    let mut gp = g.ptr();
    RandomNumberGenerator::generator(gp.get()).reseed(g.netplay.common_random_seed);
}

/// The host's match: seed and game mode settings.
fn prepare_match(g: &mut Game) {
    let mut gp = g.ptr();
    RandomNumberGenerator::generator(gp.get()).reseed(net_record::value_u32(g, "time", || crate::smw::harness::net_seed().unwrap_or(unsafe { libc_time() } as u32)));
    if g.netplay.randomize_settings {
        crate::smw::gs_menu::roll_net_game_mode_settings(g);
    } else {
        crate::smw::gs_gameplay::set_game_mode_settings_from_menu(g);
    }
    g.netplay.common_random_seed = RANDOM_INT(gp.get(), 32767) as u32;
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
    lobby_error: Option<LobbyError>,
    lobby_connecting: bool,
    /// Menu frames left to wait for the pending request's reply; 0 when nothing is pending.
    deadline: u32,
    deadline_error: LobbyError,

    /// Every room the server listed; `Networking::currentRooms` holds the ones the lobby's search shows.
    rooms: Vec<RoomListEntry>,
    room_list_pending: bool,
    room_list_timer: u32,
    /// The room selected when the pending list's first entry arrived.
    room_list_keep: Option<u32>,
    shown_filter: String,

    peer_probe: Option<PeerProbe>,
    pub v2: Option<Box<crate::smw::net_v2::V2Match>>,

    foreign_lobbyserver: Ptr<dyn NetPeer>,

    uiRoomList: Ctl<MI_NetworkListScroll>,

    pub _alias: Aliased,
}

/// A copy (`Game::clone_match`) holds no connections.
impl Clone for NetClient {
    fn clone(&self) -> Self {
        NetClient {
            lastSentMessage: self.lastSentMessage.clone(),
            lastReceivedMessage: self.lastReceivedMessage.clone(),
            lastReply: self.lastReply.clone(),
            lobby_error: self.lobby_error,
            lobby_connecting: false,
            deadline: self.deadline,
            deadline_error: self.deadline_error,
            rooms: self.rooms.clone(),
            room_list_pending: self.room_list_pending,
            room_list_timer: self.room_list_timer,
            room_list_keep: self.room_list_keep,
            shown_filter: self.shown_filter.clone(),
            peer_probe: None,
            v2: None,
            foreign_lobbyserver: Ptr::null(),
            uiRoomList: self.uiRoomList,
            _alias: Aliased::new(),
        }
    }
}

impl NetClient {
    pub fn new() -> Self {
        NetClient {
            lastSentMessage: LastMessage::default(),
            lastReceivedMessage: LastMessage::default(),
            lastReply: LastMessage::default(),
            lobby_error: None,
            lobby_connecting: false,
            deadline: 0,
            deadline_error: LobbyError::RequestTimeout,
            rooms: Vec::new(),
            room_list_pending: false,
            room_list_timer: 0,
            room_list_keep: None,
            shown_filter: String::new(),
            peer_probe: None,
            v2: None,
            foreign_lobbyserver: Ptr::null(),
            uiRoomList: Default::default(),
            _alias: Aliased::new(),
        }
    }

    pub fn init(&mut self) -> bool {
        true
    }

    pub fn start(&mut self, g: &mut Game) -> bool {
        let mut gp = g.ptr();
        net_record::value_bool(g, "client_restart", || net_layer(gp.get()).client_restart(gp.get()))
    }

    /// The room's WebRTC socket, whose peers may still be connecting, and how many other players it will reach.
    pub fn take_peer_socket(&mut self) -> Option<(smw_peer::PeerSocket, usize)> {
        self.peer_probe.take().map(PeerProbe::into_socket)
    }

    pub fn update(&mut self, g: &mut Game) {
        let mut gp = g.ptr();
        net_record::listen(g, Side::Client, self, |g, handler| net_layer(g).client_listen(g, handler));
        if let Some(probe) = &mut self.peer_probe {
            probe.update();
        }
    }

    pub fn stop(&mut self, g: &mut Game) {
        self.peer_probe = None;
        self.v2 = None;
        g.netplay.v2_match = false;

        if g.netplay.connectSuccessful {
            self.send_goodbye(g);
        }

        if !self.foreign_lobbyserver.is_null() {
            self.foreign_lobbyserver.disconnect();
            self.foreign_lobbyserver = Ptr::null();
        }
        self.lobby_connecting = false;
        self.deadline = 0;

        if !net_record::replaying(g) {
            net_layer(g).client_shutdown(g);
        }
    }

    /// Reports a failed lobby request to the menu; the first failure until the menu takes it wins.
    fn fail(&mut self, error: LobbyError) {
        println!("[net] {}", error.message());
        self.deadline = 0;
        self.lobby_error.get_or_insert(error);
    }

    pub fn take_lobby_error(&mut self) -> Option<LobbyError> {
        self.lobby_error.take()
    }

    fn start_deadline(&mut self, frames: u32, error: LobbyError) {
        self.deadline = frames;
        self.deadline_error = error;
    }

    /// Counts down the pending request's deadline, once per menu frame.
    pub fn tick_deadline(&mut self, g: &mut Game) {
        if !g.netplay.operationInProgress {
            self.deadline = 0;
        } else if self.deadline > 0 {
            self.deadline -= 1;
            if self.deadline == 0 {
                let error = self.deadline_error;
                self.fail(error);
            }
        }
    }

    pub fn cleanup(&mut self, g: &mut Game) {
        self.stop(g);
    }

    pub fn set_room_list_ui_control(&mut self, control: Ctl<MI_NetworkListScroll>) {
        self.uiRoomList = control;
    }

    /****************************
        Phase 1: Connect
    ****************************/

    pub fn send_connect_request_to_selected_server(&mut self, g: &mut Game) -> bool {
        let hostname = g.netplay.savedServers[g.netplay.selectedServerIndex as usize].hostname.clone();
        self.lobby_error = None;
        if !self.connect_lobby(g, &hostname, NET_LOBBYSERVER_PORT) {
            self.fail(LobbyError::Unreachable);
            return false;
        }

        g.netplay.operationInProgress = true;
        self.lobby_connecting = true;
        self.start_deadline(CONNECT_DEADLINE_FRAMES, LobbyError::ConnectTimeout);
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
        self.rooms.clear();
        g.netplay.currentRooms.clear();
        if !self.uiRoomList.is_none() {
            g.ctl(self.uiRoomList).clear();
        }
        self.poll_room_list(g);
    }

    /// Asks for the room list again, showing the current one until the answer arrives.
    pub fn poll_room_list(&mut self, g: &mut Game) {
        let msg = pkgs::RoomList::new();
        self.send_message_to_lobby_server(g, msg.as_bytes());
        self.room_list_pending = true;
        self.room_list_timer = 0;
    }

    /// Called every frame the lobby menu is shown: refreshes the room list now and then, and applies the search text.
    pub fn update_room_list(&mut self, g: &mut Game) {
        if !g.netplay.operationInProgress {
            self.room_list_timer += 1;
            if self.room_list_timer >= ROOM_LIST_REFRESH_FRAMES {
                self.poll_room_list(g);
            }
        }
        if g.netplay.roomFilter != self.shown_filter {
            let selected = self.selected_room_id(g);
            self.show_rooms(g, selected);
        }
    }

    fn selected_room_id(&self, g: &Game) -> Option<u32> {
        g.netplay.currentRooms.get(g.netplay.selectedRoomIndex as usize).map(|r| r.roomID)
    }

    fn start_room_list_answer(&mut self, g: &mut Game) {
        if self.room_list_pending {
            self.room_list_pending = false;
            self.room_list_keep = self.selected_room_id(g);
            self.rooms.clear();
        }
    }

    /// Shows the rooms whose name contains the search text, ignoring case, keeping `selected` selected.
    fn show_rooms(&mut self, g: &mut Game, selected: Option<u32>) {
        self.shown_filter = g.netplay.roomFilter.clone();
        let filter = self.shown_filter.to_lowercase();
        let shown: Vec<RoomListEntry> = self.rooms.iter().filter(|r| r.name.to_lowercase().contains(&filter)).cloned().collect();
        if shown == g.netplay.currentRooms {
            return;
        }
        let index = selected.and_then(|id| shown.iter().position(|r| r.roomID == id)).unwrap_or(0);
        let items: Vec<(String, String)> = shown
            .iter()
            .map(|r| (r.name.clone(), format!("{}{}/4", if r.passwordRequired { "PW " } else { "" }, r.playerCount)))
            .collect();
        g.netplay.currentRooms = shown;
        if self.uiRoomList.is_none() {
            g.netplay.selectedRoomIndex = 0;
        } else {
            g.ctl(self.uiRoomList).replace_items(&items, index);
        }
    }

    fn handle_new_room_list_entry(&mut self, g: &mut Game, data: &[u8]) {
        let roomInfo = pkgs::RoomInfo::from_bytes(data);

        let newRoom = RoomListEntry {
            roomID: roomInfo.roomID,
            name: cstr_field(&roomInfo.name),
            playerCount: roomInfo.currentPlayerCount,
            passwordRequired: roomInfo.passwordRequired != 0,
        };
        println!("  Incoming room entry: [{}] {} ({}/4)", newRoom.roomID, newRoom.name, newRoom.playerCount);
        self.start_room_list_answer(g);
        self.rooms.push(newRoom);
        let keep = self.room_list_keep;
        self.show_rooms(g, keep);
    }

    pub fn send_create_room_message(&mut self, g: &mut Game) {
        let mut msg = pkgs::NewRoom::with(&g.netplay.newroom_name, &g.netplay.newroom_password);
        msg.gamemodeID = g.currentgamemode as u8;
        g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((g.currentgamemode as usize) as u8));
        msg.gamemodeGoal = g.mode().goal as u16;

        self.send_message_to_lobby_server(g, msg.as_bytes());
        g.netplay.operationInProgress = true;
        self.start_deadline(REQUEST_DEADLINE_FRAMES, LobbyError::RequestTimeout);
    }

    fn handle_room_created_message(&mut self, g: &mut Game, data: &[u8]) {
        println!("room created!");
        let pkg = pkgs::NewRoomCreated::from_bytes(data);

        g.netplay.chat.clear();
        g.netplay.currentRoom.roomID = pkg.roomID;
        g.netplay.currentRoom.hostPlayerNumber = 0;
        g.netplay.currentRoom.name = g.netplay.newroom_name.clone();
        g.netplay.currentRoom.playerNames[0] = g.netplay.lobby_name.clone();
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
    }

    /// After a match the room stays together for the next one: the host reopens the room and resends the map and mode
    /// it will play next.
    pub fn back_in_room(&mut self, g: &mut Game) {
        g.netplay.gameRunning = false;
        g.netplay.operationInProgress = false;
        if g.netplay.theHostIsMe {
            self.send_room_settings(g);
            let reopen = MessageHeader::new(NET_G2L_GAME_RESULTS);
            self.send_message_to_lobby_server(g, reopen.as_bytes());
        }
    }

    pub fn send_start_room_message(&mut self, g: &mut Game) {
        let pkg = pkgs::StartRoom::new();
        self.send_message_to_lobby_server(g, pkg.as_bytes());
    }

    /// The host's map and game mode settings, to the room.
    pub fn send_room_settings(&mut self, g: &mut Game) {
        self.send_map_change_message(g);
        self.send_game_mode_settings_change_message(g);
    }

    pub fn send_join_room_message(&mut self, g: &mut Game, password: &str) {
        let entry = g.netplay.currentRooms[g.netplay.selectedRoomIndex as usize].clone();
        println!(
            "currentRooms[{}] = {{id={}, name='{}', cnt={}}}, össz: {}",
            g.netplay.selectedRoomIndex,
            entry.roomID,
            entry.name,
            entry.playerCount,
            g.netplay.currentRooms.len()
        );

        let msg = pkgs::JoinRoom::with(entry.roomID, password);
        self.send_message_to_lobby_server(g, msg.as_bytes());
        g.netplay.operationInProgress = true;
        self.start_deadline(REQUEST_DEADLINE_FRAMES, LobbyError::RequestTimeout);
    }

    pub fn send_leave_room_message(&mut self, g: &mut Game) {
        g.netplay.chat.clear();

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
        g.game_values.gamemode = Some(crate::common::game_mode::ModeId::Standard((g.currentgamemode as usize) as u8));
        g.mode().goal = pkg.gamemodeGoal as i16;

        let gm = g.gamemodes[g.currentgamemode as usize].ptr();
        println!("  Game mode #{}: {} with {}: {}", g.currentgamemode, gm.get_mode_name(), gm.get_goal_name(), g.mode().goal);

        g.netplay.theHostIsMe = false;
        g.netplay.currentRoom.hostPlayerNumber = pkg.hostPlayerNumber;
        g.netplay.remotePlayerNumber = pkg.remotePlayerNumber;
        g.netplay.hostPlayerNumber = pkg.hostPlayerNumber;
        g.netplay.theHostIsMe = g.netplay.remotePlayerNumber == pkg.hostPlayerNumber;

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
        const KEEP: usize = 50;
        let pkg = pkgs::RoomChatMsg::from_bytes(data);
        let Some(sender) = g.netplay.currentRoom.playerNames.get(pkg.senderNum as usize).cloned() else { return };
        let message = cstr_field(&pkg.message);
        println!("[net] chat {}: {}", sender, message);
        g.netplay.chat.push((sender, message));
        if g.netplay.chat.len() > KEEP {
            g.netplay.chat.remove(0);
        }
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

        gp.rm.forget_menu_skin(playerID as i16);
        if !gp.rm.load_menu_skin_path(g, playerID as i16, Path::new(&path), playerID as i16, false) {
            println!("[warning] Could not load netplay skin of player {}, using default", playerID);
            gp.get().rm.load_menu_skin(g, playerID as i16, gp.get().game_values.skinids[playerID as usize], playerID as i16, false);
        }
    }

    /****************************
        Phase 2.5: Pre-game
    ****************************/

    /// The lobby starts the room's match. The host decides it here and hands it to the joiners when it enters gameplay
    /// (`net_v2::match_starting`); joiners wait there for it.
    fn handle_match_start(&mut self, g: &mut Game) {
        if g.netplay.theHostIsMe {
            prepare_match(g);
        }
        println!("[net] Game start!");
        g.netplay.gameRunning = true;
        self.set_as_last_received_message(g, NET_L2P_MATCH_START);
    }

    /****************************
        Phase 3: Play
    ****************************/

    pub fn send_leave_game_message(&mut self, g: &mut Game) {
        g.netplay.gameRunning = false;
    }

    /****************************
        Network Communication
    ****************************/

    fn connect_lobby(&mut self, g: &mut Game, hostname: &str, port: u16) -> bool {
        let mut gp = g.ptr();
        g.netplay.connectSuccessful = false;
        net_record::value_bool(g, "connect_lobby", || net_layer(gp.get()).connect_to_lobby_server(gp.get(), hostname, port))
    }

    /// Every client message goes reliably: a lost lobby request or start handshake leaves the client waiting forever.
    fn send_to(&mut self, g: &mut Game, mut peer: Ptr<dyn NetPeer>, data: &[u8]) -> bool {
        if peer.is_null() || !peer.send_reliable(data) {
            return false;
        }

        self.set_as_last_sent_message(g, data[2]);
        true
    }

    fn send_message_to_lobby_server(&mut self, g: &mut Game, data: &[u8]) -> bool {
        let peer = self.foreign_lobbyserver;
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
        if matches!(packageType, NET_RESPONSE_CONNECT_OK | NET_RESPONSE_JOIN_OK | NET_RESPONSE_CREATE_OK | NET_L2P_MATCH_START) {
            self.lastReply = self.lastReceivedMessage;
            self.deadline = 0;
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
            self.lobby_connecting = false;

            let message = pkgs::ClientConnection::with(&g.netplay.myPlayerName);
            self.send_message_to_lobby_server(g, message.as_bytes());
        } else {
            newPeer.disconnect();
        }
    }

    fn on_disconnect(&mut self, g: &mut Game, client: &mut dyn NetPeer) {
        if !self.foreign_lobbyserver.is_null() && client.same_peer(&*self.foreign_lobbyserver) {
            println!("[net] Disconnected from lobby server.");
            self.foreign_lobbyserver = Ptr::null();
            g.netplay.connectSuccessful = false;
            self.fail(LobbyError::ConnectionLost);
            return;
        }

        if self.foreign_lobbyserver.is_null() && self.lobby_connecting {
            self.lobby_connecting = false;
            self.fail(LobbyError::Unreachable);
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
            println!("[net] Protocol version {}.{} differs from ours", protocolMajor, protocolMinor);
            let from_lobby = !self.foreign_lobbyserver.is_null() && client.same_peer(&*self.foreign_lobbyserver);
            if from_lobby {
                let older = (protocolMajor, protocolMinor) < (NET_PROTOCOL_VERSION_MAJOR, NET_PROTOCOL_VERSION_MINOR);
                self.fail(if older { LobbyError::ServerOutdated } else { LobbyError::ClientOutdated });
            }
            return;
        }

        match packageType {
            NET_RESPONSE_BADPROTOCOL => self.fail(LobbyError::VersionMismatch),

            NET_RESPONSE_SERVERINFO => self.handle_serverinfo_and_close(data),

            NET_RESPONSE_SERVER_MOTD => println!("Not implemented: NET_RESPONSE_SERVER_MOTD"),

            NET_RESPONSE_CONNECT_OK => {
                g.netplay.lobby_name = if data.len() >= std::mem::size_of::<pkgs::ConnectOk>() {
                    cstr_field(&pkgs::ConnectOk::from_bytes(data).playerName)
                } else {
                    g.netplay.myPlayerName.clone()
                };
                // Not in the C++: the Servers menu moves on only if the skin goes out no earlier than this receive.
                self.set_as_last_received_message(g, packageType);
                g.netplay.connectSuccessful = true;
                self.send_skin_change(g);
                return;
            }

            NET_RESPONSE_CONNECT_DENIED => {
                g.netplay.connectSuccessful = false;
                self.fail(LobbyError::Denied);
            }

            NET_RESPONSE_CONNECT_SERVERFULL => {
                g.netplay.connectSuccessful = false;
                self.fail(LobbyError::ServerFull);
            }

            NET_RESPONSE_CONNECT_NAMETAKEN => {
                g.netplay.connectSuccessful = false;
                self.fail(LobbyError::NameTaken);
            }

            NET_RESPONSE_ROOM_LIST_ENTRY => self.handle_new_room_list_entry(g, data),

            NET_RESPONSE_NO_ROOMS => {
                self.start_room_list_answer(g);
                self.show_rooms(g, None);
            }

            NET_RESPONSE_JOIN_OK => {
                g.netplay.chat.clear();
                g.netplay.joinSuccessful = true;
                g.netplay.theHostIsMe = false;
            }

            NET_RESPONSE_ROOM_FULL => {
                g.netplay.joinSuccessful = false;
                self.fail(LobbyError::RoomFull);
            }

            NET_RESPONSE_ROOM_NOT_FOUND => {
                g.netplay.joinSuccessful = false;
                self.fail(LobbyError::RoomNotFound);
                self.poll_room_list(g);
            }

            NET_RESPONSE_WRONG_PASSWORD => {
                g.netplay.joinSuccessful = false;
                self.fail(LobbyError::WrongPassword);
            }

            NET_NOTICE_ROOM_CHANGE => self.handle_room_changed_message(g, data),

            NET_NOTICE_MAP_CHANGE => self.handle_map_change_message(g, data),

            NET_NOTICE_SKIN_CHANGE => self.handle_skin_change_message(g, data),

            NET_NOTICE_GAMEMODESETTINGS => self.handle_game_mode_settings_change_message(g, data),

            NET_NOTICE_ROOM_CHAT_MSG => self.handle_room_chat_message(g, data),

            NET_RESPONSE_CREATE_OK => self.handle_room_created_message(g, data),

            NET_RESPONSE_CREATE_ERROR => self.fail(LobbyError::CreateFailed),

            NET_L2P_MATCH_ROOM => {
                if !net_record::replaying(g) {
                    let lobby = g.netplay.savedServers.get(g.netplay.selectedServerIndex as usize).map(|s| s.hostname.clone()).unwrap_or_default();
                    self.peer_probe = Some(PeerProbe::open(&pkgs::MatchRoom::from_bytes(data), &lobby));
                }
            }

            NET_L2P_MATCH_START => self.handle_match_start(g),

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

extern "C" {
    #[cfg_attr(not(windows), link_name = "time")]
    #[cfg_attr(windows, link_name = "_time64")]
    fn libc_time_raw(t: *mut i64) -> i64;
}

unsafe fn libc_time() -> i64 {
    libc_time_raw(std::ptr::null_mut())
}

pub type NetworkState = i32;
pub const INACTIVE: NetworkState = 0;
pub const DISCONNECTED: NetworkState = 1;
pub const CONNECTED: NetworkState = 2;
pub const JOINED: NetworkState = 3;
pub const PLAYING: NetworkState = 4;

#[derive(Clone)]

pub struct Networking {
    pub active: bool,

    pub connectSuccessful: bool,
    pub joinSuccessful: bool,
    pub gameRunning: bool,

    pub currentMenuChanged: bool,
    pub operationInProgress: bool,

    pub client: NetClient,
    pub myPlayerName: String,
    pub v2_match: bool,
    /// The name the lobby server gave this player: `myPlayerName`, numbered if another player has it.
    pub lobby_name: String,

    pub selectedServerIndex: u16,
    pub savedServers: Vec<ServerAddress>,
    /// The entries of `savedServers` that LAN beacons added; they are not saved.
    pub nearby: Vec<String>,

    pub roomFilter: String,
    pub selectedRoomIndex: u16,
    pub currentRooms: Vec<RoomListEntry>,
    pub currentRoom: Room,

    pub newroom_name: String,
    pub newroom_password: String,
    pub mychatmessage: String,
    /// The current room's chat, oldest first, as (sender, message).
    pub chat: Vec<(String, String)>,
    pub mapfilepath: String,

    pub theHostIsMe: bool,
    pub remotePlayerNumber: u8,
    pub hostPlayerNumber: u8,
    pub netPlayerInput: CPlayerInput,

    /// The host's seed for the match (`prepare_match`).
    pub common_random_seed: u32,
    /// The host's "Randomize" room setting: roll the mode settings at the sync instead of playing the menu's.
    pub randomize_settings: bool,

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
            v2_match: false,
            lobby_name: String::new(),
            selectedServerIndex: 0,
            savedServers: Vec::new(),
            nearby: Vec::new(),
            roomFilter: String::new(),
            selectedRoomIndex: 0,
            currentRooms: Vec::new(),
            currentRoom: Room::new(),
            newroom_name: String::new(),
            newroom_password: String::new(),
            mychatmessage: String::new(),
            chat: Vec::new(),
            mapfilepath: String::new(),
            theHostIsMe: false,
            remotePlayerNumber: 0,
            hostPlayerNumber: 0,
            netPlayerInput: CPlayerInput::new(),
            common_random_seed: 0,
            randomize_settings: false,
            _alias: Aliased::new(),
        }
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

