//! The lobby protocol's messages. Each goes on the wire as the raw bytes of its `#[repr(C)]` struct, so padding
//! is spelled out as zeroed `_padN` fields and multi-byte fields are little-endian.

use super::protocol_definitions::*;

#[cfg(not(target_endian = "little"))]
compile_error!("lobby messages are sent as their structs' raw bytes, which the protocol defines as little-endian");

/// `as_bytes` / `from_bytes` for a `#[repr(C)]` package made only of integers, floats and byte arrays.
#[macro_export]
macro_rules! net_package {
    ($($t:ty),*) => {$(
        impl $t {
            pub fn as_bytes(&self) -> &[u8] {
                unsafe { ::std::slice::from_raw_parts(self as *const Self as *const u8, ::std::mem::size_of::<Self>()) }
            }

            /// The message in `data`; bytes past the end of `data` read as zero.
            pub fn from_bytes(data: &[u8]) -> Self {
                let mut buf = [0u8; ::std::mem::size_of::<Self>()];
                let n = data.len().min(buf.len());
                buf[..n].copy_from_slice(&data[..n]);
                unsafe { ::std::ptr::read_unaligned(buf.as_ptr() as *const Self) }
            }
        }
    )*};
}

/// Copies `src` up to its first NUL into `dst`, NUL-padded, then puts a NUL at `last`.
pub fn strncpy_terminated(dst: &mut [u8], src: &str, last: usize) {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < dst.len() && i < bytes.len() && bytes[i] != 0 {
        dst[i] = bytes[i];
        i += 1;
    }
    while i < dst.len() {
        dst[i] = 0;
        i += 1;
    }
    dst[last] = 0;
}

/// A NUL-terminated string field as a `String`; a field without a NUL is read whole.
pub fn cstr_field(field: &[u8]) -> String {
    let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    crate::common::file_io::cstr_bytes_to_string(&field[..end])
}

/// The first 3 bytes of every lobby message: the protocol version and the message type.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MessageHeader {
    pub protocolMajorVersion: u8,
    pub protocolMinorVersion: u8,
    pub packageType: u8,
}

impl MessageHeader {
    pub const fn new(packageType: u8) -> Self {
        MessageHeader { protocolMajorVersion: NET_PROTOCOL_VERSION_MAJOR, protocolMinorVersion: NET_PROTOCOL_VERSION_MINOR, packageType }
    }
}

impl Default for MessageHeader {
    fn default() -> Self {
        MessageHeader::new(0)
    }
}

/*
    Connection packages
*/

/// `NET_RESPONSE_SERVERINFO`: the server's name and player counts.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ServerInfo {
    pub header: MessageHeader,
    pub name: [u8; 32],
    pub _pad0: u8,
    pub currentPlayerCount: u32,
    pub maxPlayerCount: u32,
}

impl ServerInfo {
    pub fn new() -> Self {
        ServerInfo { header: MessageHeader::new(NET_RESPONSE_SERVERINFO), name: [0; 32], _pad0: 0, currentPlayerCount: 0, maxPlayerCount: 0 }
    }

    /// Server-side response constructor.
    pub fn with(name: &str, players_current: u32, players_max: u32) -> Self {
        let mut s = ServerInfo::new();
        s.currentPlayerCount = players_current;
        s.maxPlayerCount = players_max;
        strncpy_terminated(&mut s.name, name, 31);
        s
    }
}

/// `NET_REQUEST_CONNECT` with the name the player asks for.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ClientConnection {
    pub header: MessageHeader,
    pub playerName: [u8; NET_MAX_PLAYER_NAME_LENGTH],
}

impl ClientConnection {
    pub fn new() -> Self {
        ClientConnection { header: MessageHeader::new(NET_REQUEST_CONNECT), playerName: [0; NET_MAX_PLAYER_NAME_LENGTH] }
    }

    pub fn with(playerName: &str) -> Self {
        let mut s = ClientConnection::new();
        strncpy_terminated(&mut s.playerName, playerName, NET_MAX_PLAYER_NAME_LENGTH - 1);
        s
    }
}

/// `NET_RESPONSE_CONNECT_OK` with the name the server gave the player, which differs from the requested one when taken.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ConnectOk {
    pub header: MessageHeader,
    pub playerName: [u8; NET_MAX_PLAYER_NAME_LENGTH],
}

impl ConnectOk {
    pub fn with(playerName: &[u8]) -> Self {
        let mut s = ConnectOk { header: MessageHeader::new(NET_RESPONSE_CONNECT_OK), playerName: [0; NET_MAX_PLAYER_NAME_LENGTH] };
        let n = playerName.len().min(NET_MAX_PLAYER_NAME_LENGTH - 1);
        s.playerName[..n].copy_from_slice(&playerName[..n]);
        s
    }
}

/// `NET_REQUEST_LEAVE_SERVER`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ClientDisconnection {
    pub header: MessageHeader,
}

impl ClientDisconnection {
    pub fn new() -> Self {
        ClientDisconnection { header: MessageHeader::new(NET_REQUEST_LEAVE_SERVER) }
    }
}

/*
    Room packages
*/

/// `NET_REQUEST_ROOM_LIST`; the lobby answers with one `RoomInfo` per listed room, or `NET_RESPONSE_NO_ROOMS`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RoomList {
    pub header: MessageHeader,
}

impl RoomList {
    pub fn new() -> Self {
        RoomList { header: MessageHeader::new(NET_REQUEST_ROOM_LIST) }
    }
}

/// `NET_RESPONSE_ROOM_LIST_ENTRY`: one room of the room list.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RoomInfo {
    pub header: MessageHeader,
    pub _pad0: u8,
    pub roomID: u32,
    pub name: [u8; NET_MAX_ROOM_NAME_LENGTH],
    pub currentPlayerCount: u8,
    pub passwordRequired: u8,
    pub gamemodeID: u8,
    pub _pad1: u8,
}

impl RoomInfo {
    pub fn new() -> Self {
        RoomInfo {
            header: MessageHeader::new(NET_RESPONSE_ROOM_LIST_ENTRY),
            _pad0: 0,
            roomID: 0,
            name: [0; NET_MAX_ROOM_NAME_LENGTH],
            currentPlayerCount: 0,
            passwordRequired: 0,
            gamemodeID: 0,
            _pad1: 0,
        }
    }
}

/// `NET_REQUEST_CREATE_ROOM` with the room's name, password and game mode.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NewRoom {
    pub header: MessageHeader,
    pub name: [u8; NET_MAX_ROOM_NAME_LENGTH],
    pub password: [u8; NET_MAX_ROOM_PASSWORD_LENGTH],
    pub gamemodeID: u8,
    pub gamemodeGoal: u16,
}

impl NewRoom {
    pub fn new() -> Self {
        NewRoom {
            header: MessageHeader::new(NET_REQUEST_CREATE_ROOM),
            name: [0; NET_MAX_ROOM_NAME_LENGTH],
            password: [0; NET_MAX_ROOM_PASSWORD_LENGTH],
            gamemodeID: 0,
            gamemodeGoal: 10,
        }
    }

    /// Cuts the name to `NET_MAX_PLAYER_NAME_LENGTH - 1` bytes, shorter than the room name field holds.
    pub fn with(name: &str, password: &str) -> Self {
        let mut s = NewRoom::new();
        strncpy_terminated(&mut s.name, name, NET_MAX_PLAYER_NAME_LENGTH - 1);
        strncpy_terminated(&mut s.password, password, NET_MAX_ROOM_PASSWORD_LENGTH - 1);
        s
    }
}

/// `NET_RESPONSE_CREATE_OK` with the new room's ID.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct NewRoomCreated {
    pub header: MessageHeader,
    pub _pad0: u8,
    pub roomID: u32,
}

impl NewRoomCreated {
    pub fn new() -> Self {
        NewRoomCreated { header: MessageHeader::new(NET_RESPONSE_CREATE_OK), _pad0: 0, roomID: 0 }
    }
}

/// `NET_REQUEST_JOIN_ROOM` with the room's ID and the password typed.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct JoinRoom {
    pub header: MessageHeader,
    pub _pad0: u8,
    pub roomID: u32,
    pub password: [u8; NET_MAX_ROOM_PASSWORD_LENGTH],
}

impl JoinRoom {
    pub fn new() -> Self {
        JoinRoom { header: MessageHeader::new(NET_REQUEST_JOIN_ROOM), _pad0: 0, roomID: 0, password: [0; NET_MAX_ROOM_PASSWORD_LENGTH] }
    }

    pub fn with(roomID: u32, password: &str) -> Self {
        let mut s = JoinRoom::new();
        s.roomID = roomID;
        strncpy_terminated(&mut s.password, password, NET_MAX_ROOM_PASSWORD_LENGTH - 1);
        s
    }
}

/// `NET_REQUEST_LEAVE_ROOM`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LeaveRoom {
    pub header: MessageHeader,
}

impl LeaveRoom {
    pub fn new() -> Self {
        LeaveRoom { header: MessageHeader::new(NET_REQUEST_LEAVE_ROOM) }
    }
}

/// `NET_NOTICE_ROOM_CHANGE`: the room as the receiving player, in slot `remotePlayerNumber`, sees it.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CurrentRoom {
    pub header: MessageHeader,
    pub _pad0: u8,
    pub roomID: u32,
    pub name: [u8; NET_MAX_ROOM_NAME_LENGTH],
    pub playerName: [[u8; NET_MAX_PLAYER_NAME_LENGTH]; 4],
    pub hostPlayerNumber: u8,
    pub remotePlayerNumber: u8,
    pub gamemodeID: u8,
    pub _pad1: u8,
    pub gamemodeGoal: u16,
    pub _pad2: u16,
}

impl CurrentRoom {
    pub fn new() -> Self {
        CurrentRoom {
            header: MessageHeader::new(NET_NOTICE_ROOM_CHANGE),
            _pad0: 0,
            roomID: 0,
            name: [0; NET_MAX_ROOM_NAME_LENGTH],
            playerName: [[0; NET_MAX_PLAYER_NAME_LENGTH]; 4],
            hostPlayerNumber: 0,
            remotePlayerNumber: 0,
            gamemodeID: 0,
            _pad1: 0,
            gamemodeGoal: 10,
            _pad2: 0,
        }
    }
}

/// `NET_NOTICE_ROOM_CHAT_MSG`; `senderNum` is the sender's room slot.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RoomChatMsg {
    pub header: MessageHeader,
    pub senderNum: u8,
    pub message: [u8; NET_MAX_CHAT_MSG_LENGTH],
}

impl RoomChatMsg {
    pub fn new() -> Self {
        RoomChatMsg { header: MessageHeader::new(NET_NOTICE_ROOM_CHAT_MSG), senderNum: 0xFF, message: [0; NET_MAX_CHAT_MSG_LENGTH] }
    }

    pub fn with(playerNum: u8, msg: &str) -> Self {
        debug_assert!(playerNum < 4);
        let mut s = RoomChatMsg::new();
        s.senderNum = playerNum;
        strncpy_terminated(&mut s.message, msg, NET_MAX_CHAT_MSG_LENGTH - 1);
        s
    }
}

/// `NET_G2L_START_ROOM`: the room's host asks the lobby to start the match.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StartRoom {
    pub header: MessageHeader,
}

impl StartRoom {
    pub fn new() -> Self {
        StartRoom { header: MessageHeader::new(NET_G2L_START_ROOM) }
    }
}

/*
    Pre-game packages
*/

/// Where a starting room's players meet for netplay v2: the signaling room URL and the TURN server with credentials.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MatchRoom {
    pub header: MessageHeader,
    /// Players in the match, the receiver included.
    pub playerCount: u8,
    /// The signaling room's URL; one starting with `/` is on the lobby's server.
    pub roomURL: [u8; 160],
    /// Empty when there is no TURN server.
    pub turnURL: [u8; 96],
    pub turnUsername: [u8; 64],
    pub turnCredential: [u8; 32],
}

impl MatchRoom {
    /// None when a value does not fit with its terminating zero.
    pub fn with(playerCount: u8, roomURL: &str, turnURL: &str, turnUsername: &str, turnCredential: &str) -> Option<Self> {
        fn put<const N: usize>(field: &mut [u8; N], value: &str) -> Option<()> {
            (value.len() < N).then(|| field[..value.len()].copy_from_slice(value.as_bytes()))
        }
        let mut s = MatchRoom {
            header: MessageHeader::new(NET_L2P_MATCH_ROOM),
            playerCount,
            roomURL: [0; 160],
            turnURL: [0; 96],
            turnUsername: [0; 64],
            turnCredential: [0; 32],
        };
        put(&mut s.roomURL, roomURL)?;
        put(&mut s.turnURL, turnURL)?;
        put(&mut s.turnUsername, turnUsername)?;
        put(&mut s.turnCredential, turnCredential)?;
        Some(s)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GameHostInfo {
    pub header: MessageHeader,
    pub _pad0: u8,
    pub host: u32,
}

impl GameHostInfo {
    pub fn new(gh_address: u32) -> Self {
        GameHostInfo { header: MessageHeader::new(NET_L2P_GAMEHOST_INFO), _pad0: 0, host: gh_address }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PlayerInfo {
    pub header: MessageHeader,
    pub _pad0: u8,
    pub host: [u32; 3],
    pub port: [u16; 3],
    pub _pad1: u16,
}

impl PlayerInfo {
    pub fn new() -> Self {
        PlayerInfo { header: MessageHeader::new(NET_L2G_CLIENTS_INFO), _pad0: 0, host: [0; 3], port: [0; 3], _pad1: 0 }
    }

    pub fn set_player(&mut self, playerNum: u8, p_host: u32, p_port: u16) {
        self.host[playerNum as usize] = p_host;
        self.port[playerNum as usize] = p_port;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StartSync {
    pub header: MessageHeader,
    pub _pad0: u8,
    pub commonRandomSeed: u32,
}

impl StartSync {
    pub fn new(seed: u32) -> Self {
        StartSync { header: MessageHeader::new(NET_G2P_SYNC), _pad0: 0, commonRandomSeed: seed }
    }
}

crate::net_package!(
    MessageHeader,
    ServerInfo,
    ClientConnection,
    ConnectOk,
    MatchRoom,
    ClientDisconnection,
    RoomList,
    RoomInfo,
    NewRoom,
    NewRoomCreated,
    JoinRoom,
    LeaveRoom,
    CurrentRoom,
    RoomChatMsg,
    StartRoom,
    GameHostInfo,
    PlayerInfo,
    StartSync
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    /// Pins every message's wire size and field offsets.
    #[test]
    fn layouts_match_clang() {
        assert_eq!(size_of::<MessageHeader>(), 3);
        assert_eq!((size_of::<ServerInfo>(), offset_of!(ServerInfo, name), offset_of!(ServerInfo, currentPlayerCount), offset_of!(ServerInfo, maxPlayerCount)), (44, 3, 36, 40));
        assert_eq!((size_of::<ClientConnection>(), offset_of!(ClientConnection, playerName)), (19, 3));
        assert_eq!((size_of::<ConnectOk>(), offset_of!(ConnectOk, playerName)), (19, 3));
        assert_eq!((size_of::<MatchRoom>(), offset_of!(MatchRoom, roomURL), offset_of!(MatchRoom, turnCredential)), (356, 4, 324));
        assert_eq!(size_of::<ClientDisconnection>(), 3);
        assert_eq!(size_of::<RoomList>(), 3);
        assert_eq!(
            (
                size_of::<RoomInfo>(),
                offset_of!(RoomInfo, roomID),
                offset_of!(RoomInfo, name),
                offset_of!(RoomInfo, currentPlayerCount),
                offset_of!(RoomInfo, passwordRequired),
                offset_of!(RoomInfo, gamemodeID)
            ),
            (44, 4, 8, 40, 41, 42)
        );
        assert_eq!(
            (size_of::<NewRoom>(), offset_of!(NewRoom, name), offset_of!(NewRoom, password), offset_of!(NewRoom, gamemodeID), offset_of!(NewRoom, gamemodeGoal)),
            (54, 3, 35, 51, 52)
        );
        assert_eq!((size_of::<NewRoomCreated>(), offset_of!(NewRoomCreated, roomID)), (8, 4));
        assert_eq!((size_of::<JoinRoom>(), offset_of!(JoinRoom, roomID), offset_of!(JoinRoom, password)), (24, 4, 8));
        assert_eq!(size_of::<LeaveRoom>(), 3);
        assert_eq!(
            (
                size_of::<CurrentRoom>(),
                offset_of!(CurrentRoom, roomID),
                offset_of!(CurrentRoom, name),
                offset_of!(CurrentRoom, playerName),
                offset_of!(CurrentRoom, hostPlayerNumber),
                offset_of!(CurrentRoom, remotePlayerNumber),
                offset_of!(CurrentRoom, gamemodeID),
                offset_of!(CurrentRoom, gamemodeGoal)
            ),
            (112, 4, 8, 40, 104, 105, 106, 108)
        );
        assert_eq!((size_of::<RoomChatMsg>(), offset_of!(RoomChatMsg, senderNum), offset_of!(RoomChatMsg, message)), (104, 3, 4));
        assert_eq!(size_of::<StartRoom>(), 3);
        assert_eq!((size_of::<GameHostInfo>(), offset_of!(GameHostInfo, host)), (8, 4));
        assert_eq!((size_of::<PlayerInfo>(), offset_of!(PlayerInfo, host), offset_of!(PlayerInfo, port)), (24, 4, 16));
        assert_eq!((size_of::<StartSync>(), offset_of!(StartSync, commonRandomSeed)), (8, 4));
    }

    #[test]
    fn new_room_name_is_cut_at_player_name_length() {
        let r = NewRoom::with("a very long room name here", "pw");
        assert_eq!(cstr_field(&r.name), "a very long roo");
        let back = NewRoom::from_bytes(r.as_bytes());
        assert_eq!(back.gamemodeGoal, 10);
        assert_eq!(back.header.packageType, NET_REQUEST_CREATE_ROOM);
    }
}
