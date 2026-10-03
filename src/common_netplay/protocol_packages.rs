//! Port of src/common_netplay/ProtocolPackages.h
//!
//! Packages go on the wire as their raw C++ memory image, so each struct is `#[repr(C)]` with the
//! clang layout; padding is spelled out as zeroed `_padN` fields (C++ sends whatever bytes were there).

use super::protocol_definitions::*;

/// `as_bytes` / `from_bytes` for a `#[repr(C)]` package made only of integers, floats and byte arrays.
#[macro_export]
macro_rules! net_package {
    ($($t:ty),*) => {$(
        impl $t {
            pub fn as_bytes(&self) -> &[u8] {
                unsafe { ::std::slice::from_raw_parts(self as *const Self as *const u8, ::std::mem::size_of::<Self>()) }
            }

            /// `memcpy(&pkg, data, sizeof(pkg))`; bytes past the end of `data` read as zero.
            pub fn from_bytes(data: &[u8]) -> Self {
                let mut buf = [0u8; ::std::mem::size_of::<Self>()];
                let n = data.len().min(buf.len());
                buf[..n].copy_from_slice(&data[..n]);
                unsafe { ::std::ptr::read_unaligned(buf.as_ptr() as *const Self) }
            }
        }
    )*};
}

/// `strncpy(dst, src, N); dst[last] = '\0';`
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

/// A NUL-terminated C string field as a Rust string (reads past the array like C would stop at the array end).
pub fn cstr_field(field: &[u8]) -> String {
    let end = field.iter().position(|&c| c == 0).unwrap_or(field.len());
    crate::common::file_io::cstr_bytes_to_string(&field[..end])
}

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

    /// Terminates the name at `NET_MAX_PLAYER_NAME_LENGTH - 1`, not the room-name length (C++ bug).
    pub fn with(name: &str, password: &str) -> Self {
        let mut s = NewRoom::new();
        strncpy_terminated(&mut s.name, name, NET_MAX_PLAYER_NAME_LENGTH - 1);
        strncpy_terminated(&mut s.password, password, NET_MAX_ROOM_PASSWORD_LENGTH - 1);
        s
    }
}

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

    /// Values from tools/ref/net_layout.cpp compiled with clang against the C++ headers.
    #[test]
    fn layouts_match_clang() {
        assert_eq!(size_of::<MessageHeader>(), 3);
        assert_eq!((size_of::<ServerInfo>(), offset_of!(ServerInfo, name), offset_of!(ServerInfo, currentPlayerCount), offset_of!(ServerInfo, maxPlayerCount)), (44, 3, 36, 40));
        assert_eq!((size_of::<ClientConnection>(), offset_of!(ClientConnection, playerName)), (19, 3));
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
