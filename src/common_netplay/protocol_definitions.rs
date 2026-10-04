//! Port of src/common_netplay/ProtocolDefinitions.h

pub const NET_PROTOCOL_VERSION_MAJOR: u8 = 0;
pub const NET_PROTOCOL_VERSION_MINOR: u8 = 5;
pub const NET_MAX_MESSAGE_SIZE: usize = 128;
pub const NET_LOBBYSERVER_PORT: u16 = 12521;
pub const NET_GAMEHOST_PORT: u16 = 12522;
pub const NET_MAX_PLAYER_NAME_LENGTH: usize = 16;
pub const NET_MAX_ROOM_NAME_LENGTH: usize = 32;
pub const NET_MAX_ROOM_PASSWORD_LENGTH: usize = 16;
pub const NET_MAX_CHAT_MSG_LENGTH: usize = 100;
pub const NET_GAMESTATE_FRAMES_TO_SEND: u32 = 3;
pub const NET_REQUEST_SERVERINFO: u8 = 1;
pub const NET_RESPONSE_BADPROTOCOL: u8 = 2;
pub const NET_RESPONSE_SERVERINFO: u8 = 3;
pub const NET_RESPONSE_SERVER_MOTD: u8 = 4;
pub const NET_NOTICE_SERVER_STOPPED: u8 = 5;
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
pub const NET_REQUEST_CREATE_ROOM: u8 = 40;
pub const NET_RESPONSE_CREATE_OK: u8 = 41;
pub const NET_RESPONSE_CREATE_ERROR: u8 = 42;
// L: lobby server
// G: game host player
// P: regular players
pub const NET_G2L_START_ROOM: u8 = 50;
pub const NET_L2P_GAMEHOST_INFO: u8 = 51;
pub const NET_L2G_CLIENTS_INFO: u8 = 52;
pub const NET_G2P_SYNC: u8 = 55;
pub const NET_P2G_SYNC_OK: u8 = 56;
pub const NET_G2E_GAME_START: u8 = 57;
pub const NET_G2L_GAME_RESULTS: u8 = 58;
pub const NET_P2G_LEAVE_GAME: u8 = 70;
pub const NET_P2G_LOCAL_KEYS: u8 = 71;
pub const NET_G2P_REMOTE_KEYS: u8 = 72;
pub const NET_G2P_GAME_STATE: u8 = 73;
pub const NET_P2G_REQ_POWERUP: u8 = 74;
pub const NET_G2P_START_POWERUP: u8 = 75;
pub const NET_G2P_TRIGGER_POWERUP: u8 = 76;
pub const NET_G2P_TRIGGER_MAPCOLL: u8 = 77;
pub const NET_G2P_TRIGGER_P2PCOLL: u8 = 78;
// Not in the C++ protocol: the game host decides random outcomes (PROGRESS.md, Netplay deviations).
pub const NET_G2P_HOST_DECIDES_RANDOM: u8 = 92;
pub const NET_G2P_RANDOM_EVENT: u8 = 93;
