//! The lobby protocol's version, limits and message types (the third byte of every message).

/// Client and lobby must match in both parts; the lobby answers any other version with `NET_RESPONSE_BADPROTOCOL`.
pub const NET_PROTOCOL_VERSION_MAJOR: u8 = 0;
pub const NET_PROTOCOL_VERSION_MINOR: u8 = 10;
/// The port a lobby connection targets; the relay's switch refuses connections to any other.
pub const NET_LOBBYSERVER_PORT: u16 = 12521;
/// Field sizes in bytes, the terminating NUL included.
pub const NET_MAX_PLAYER_NAME_LENGTH: usize = 16;
pub const NET_MAX_ROOM_NAME_LENGTH: usize = 32;
pub const NET_MAX_ROOM_PASSWORD_LENGTH: usize = 16;
pub const NET_MAX_CHAT_MSG_LENGTH: usize = 100;
/// Players one client brings to a room, each on its own controller.
pub const NET_MAX_LOCAL_PLAYERS: usize = 3;
pub const NET_REQUEST_SERVERINFO: u8 = 1;
pub const NET_RESPONSE_BADPROTOCOL: u8 = 2;
pub const NET_RESPONSE_SERVERINFO: u8 = 3;
pub const NET_RESPONSE_SERVER_MOTD: u8 = 4;
pub const NET_REQUEST_CONNECT: u8 = 10;
pub const NET_REQUEST_LEAVE_SERVER: u8 = 11;
pub const NET_RESPONSE_CONNECT_OK: u8 = 12;
pub const NET_RESPONSE_CONNECT_DENIED: u8 = 13;
pub const NET_RESPONSE_CONNECT_SERVERFULL: u8 = 14;
pub const NET_RESPONSE_CONNECT_NAMETAKEN: u8 = 15;
pub const NET_REQUEST_ROOM_LIST: u8 = 20;
pub const NET_RESPONSE_NO_ROOMS: u8 = 21;
pub const NET_RESPONSE_ROOM_LIST_ENTRY: u8 = 22;
pub const NET_REQUEST_JOIN_ROOM: u8 = 30;
pub const NET_REQUEST_LEAVE_ROOM: u8 = 31;
pub const NET_RESPONSE_JOIN_OK: u8 = 32;
pub const NET_RESPONSE_ROOM_FULL: u8 = 33;
pub const NET_NOTICE_ROOM_CHANGE: u8 = 34;
pub const NET_NOTICE_MAP_CHANGE: u8 = 35;
pub const NET_NOTICE_GAMEMODESETTINGS: u8 = 36;
pub const NET_NOTICE_SKIN_CHANGE: u8 = 37;
pub const NET_NOTICE_ROOM_CHAT_MSG: u8 = 38;
pub const NET_RESPONSE_ROOM_NOT_FOUND: u8 = 39;
pub const NET_REQUEST_CREATE_ROOM: u8 = 40;
pub const NET_RESPONSE_CREATE_OK: u8 = 41;
pub const NET_RESPONSE_CREATE_ERROR: u8 = 42;
pub const NET_RESPONSE_WRONG_PASSWORD: u8 = 43;
/// `LocalPlayers`: how many players this client brings to its room; the lobby answers with `NET_NOTICE_ROOM_CHANGE`.
pub const NET_REQUEST_LOCAL_PLAYERS: u8 = 44;
/// `TeamChange`: moves one of this client's players to another team; the lobby answers with `NET_NOTICE_ROOM_CHANGE`.
pub const NET_REQUEST_TEAM: u8 = 45;
// L: lobby server
// G: game host player
// P: regular players
/// The host asks the lobby to start the room's match.
pub const NET_G2L_START_ROOM: u8 = 50;
/// The match's signaling room and TURN server (`MatchRoom`), sent before `NET_L2P_MATCH_START`.
pub const NET_L2P_MATCH_ROOM: u8 = 53;
/// The room's match starts.
pub const NET_L2P_MATCH_START: u8 = 54;
/// The host is back in the room after a match; the lobby lists the room again.
pub const NET_G2L_GAME_RESULTS: u8 = 58;
