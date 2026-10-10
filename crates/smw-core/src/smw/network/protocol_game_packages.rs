//! Port of src/smw/network/ProtocolGamePackages.h

use crate::common::input::COutputControl;
use crate::common_netplay::protocol_definitions::*;
use crate::common_netplay::protocol_packages::MessageHeader;
use crate::smw::player::CPlayer;

/*
    Pre-game packages
*/

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SyncOK {
    pub header: MessageHeader,
}

impl SyncOK {
    pub fn new() -> Self {
        SyncOK { header: MessageHeader::new(NET_P2G_SYNC_OK) }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StartGame {
    pub header: MessageHeader,
}

impl StartGame {
    pub fn new() -> Self {
        StartGame { header: MessageHeader::new(NET_G2E_GAME_START) }
    }
}

/*
    Gameplay packages
*/

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct LeaveGame {
    pub header: MessageHeader,
}

impl LeaveGame {
    pub fn new() -> Self {
        LeaveGame { header: MessageHeader::new(NET_P2G_LEAVE_GAME) }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RawInput {
    pub flags: u16,
}

impl RawInput {
    pub fn set_player_key(&mut self, keyNum: u8, down: bool, pressed: bool) {
        debug_assert!(keyNum < 8);
        self.flags |= if down { (1i32 << ((keyNum as i32) << 1)) as u16 } else { 0 };
        self.flags |= if pressed { (1i32 << (((keyNum as i32) << 1) + 1)) as u16 } else { 0 };
    }

    pub fn get_player_key(&self, keyNum: u8, down: &mut bool, pressed: &mut bool) {
        debug_assert!(keyNum < 8);
        *down = (self.flags as i32 & (1 << ((keyNum as i32) << 1))) != 0;
        *pressed = (self.flags as i32 & (1 << (((keyNum as i32) << 1) + 1))) != 0;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ClientInput {
    pub header: MessageHeader,
    pub input_id: u8,
    pub input: RawInput,
}

impl ClientInput {
    pub fn new(playerControl: &COutputControl) -> Self {
        let mut s = ClientInput { header: MessageHeader::new(NET_P2G_LOCAL_KEYS), input_id: 0, input: RawInput::default() };
        for k in 0..8u8 {
            s.input.set_player_key(k, playerControl.keys[k as usize].fDown, playerControl.keys[k as usize].fPressed);
        }
        s
    }

    pub fn read_keys(&self, playerControl: &mut COutputControl) {
        for k in 0..8u8 {
            let key = &mut playerControl.keys[k as usize];
            self.input.get_player_key(k, &mut key.fDown, &mut key.fPressed);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RemoteInput {
    pub header: MessageHeader,
    pub playerNumber: u8,
    pub input: RawInput,
}

impl RemoteInput {
    pub fn with_input(player: u8, input: RawInput) -> Self {
        RemoteInput { header: MessageHeader::new(NET_G2P_REMOTE_KEYS), playerNumber: player, input }
    }

    pub fn new(player: u8) -> Self {
        RemoteInput { header: MessageHeader::new(NET_G2P_REMOTE_KEYS), playerNumber: player, input: RawInput::default() }
    }

    pub fn read_keys(&self, playerControl: &mut COutputControl) {
        for k in 0..8u8 {
            let key = &mut playerControl.keys[k as usize];
            self.input.get_player_key(k, &mut key.fDown, &mut key.fPressed);
        }
    }

    pub fn write_keys(&mut self, playerControl: &COutputControl) {
        for k in 0..8u8 {
            self.input.set_player_key(k, playerControl.keys[k as usize].fDown, playerControl.keys[k as usize].fPressed);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct GameState {
    pub header: MessageHeader,
    pub _pad0: u8,
    pub player_x: [f32; 4],
    pub player_y: [f32; 4],
    pub player_xvel: [f32; 4],
    pub player_yvel: [f32; 4],
    pub last_confirmed_local_input_id: u8,
    pub _pad1: [u8; 3],
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            header: MessageHeader::new(NET_G2P_GAME_STATE),
            _pad0: 0,
            player_x: [0.0; 4],
            player_y: [0.0; 4],
            player_xvel: [0.0; 4],
            player_yvel: [0.0; 4],
            last_confirmed_local_input_id: 0,
            _pad1: [0; 3],
        }
    }

    pub fn set_player_coord(&mut self, playerNum: u8, x: f32, y: f32) {
        self.player_x[playerNum as usize] = x;
        self.player_y[playerNum as usize] = y;
    }

    pub fn set_player_vel(&mut self, playerNum: u8, xvel: f32, yvel: f32) {
        self.player_xvel[playerNum as usize] = xvel;
        self.player_yvel[playerNum as usize] = yvel;
    }

    pub fn get_player_coord(&self, playerNum: u8, x: &mut f32, y: &mut f32) {
        *x = self.player_x[playerNum as usize];
        *y = self.player_y[playerNum as usize];
    }

    pub fn get_player_vel(&self, playerNum: u8, xvel: &mut f32, yvel: &mut f32) {
        *xvel = self.player_xvel[playerNum as usize];
        *yvel = self.player_yvel[playerNum as usize];
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RequestPowerup {
    pub header: MessageHeader,
}

impl RequestPowerup {
    pub fn new() -> Self {
        RequestPowerup { header: MessageHeader::new(NET_P2G_REQ_POWERUP) }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StartPowerup {
    pub header: MessageHeader,
    pub player_id: u8,
    pub powerup_id: u8,
    pub _pad0: [u8; 3],
    pub delay: u32,
}

impl StartPowerup {
    pub fn new(playerID: u8, powerupID: u8, delay: u32) -> Self {
        debug_assert!(playerID < 4 || playerID == 0xFF);
        debug_assert!(powerupID < 128);
        StartPowerup { header: MessageHeader::new(NET_G2P_START_POWERUP), player_id: playerID, powerup_id: powerupID, _pad0: [0; 3], delay }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TriggerPowerup {
    pub header: MessageHeader,
    pub player_id: u8,
    pub powerup_id: u8,
    pub _pad0: [u8; 3],
    pub player_x: f32,
    pub player_y: f32,
}

impl TriggerPowerup {
    pub fn new(playerID: u8, powerupID: u8, playerX: f32, playerY: f32) -> Self {
        TriggerPowerup {
            header: MessageHeader::new(NET_G2P_TRIGGER_POWERUP),
            player_id: playerID,
            powerup_id: powerupID,
            _pad0: [0; 3],
            player_x: playerX,
            player_y: playerY,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MapCollision {
    pub header: MessageHeader,
    pub player_id: u8,
    pub player_x: f32,
    pub player_y: f32,
    pub player_xvel: f32,
    pub player_yvel: f32,
}

impl MapCollision {
    /// The default constructor leaves the fields uninitialized in C++.
    pub fn new() -> Self {
        MapCollision { header: MessageHeader::new(NET_G2P_TRIGGER_MAPCOLL), player_id: 0, player_x: 0.0, player_y: 0.0, player_xvel: 0.0, player_yvel: 0.0 }
    }

    pub fn from_player(player: &CPlayer) -> Self {
        let mut s = MapCollision::new();
        s.fill(player);
        s
    }

    pub fn fill(&mut self, player: &CPlayer) {
        self.player_id = player.get_global_id() as u8;
        self.player_x = player.fx;
        self.player_y = player.fy;
        self.player_xvel = player.velx;
        self.player_yvel = player.vely;
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct P2PCollision {
    pub header: MessageHeader,
    pub player_id: [u8; 2],
    pub _pad0: [u8; 3],
    pub player_x: [f32; 2],
    pub player_y: [f32; 2],
    pub player_xvel: [f32; 2],
    pub player_yvel: [f32; 2],
    pub player_oldy: [f32; 2],
}

impl P2PCollision {
    pub fn new() -> Self {
        P2PCollision {
            header: MessageHeader::new(NET_G2P_TRIGGER_P2PCOLL),
            player_id: [0; 2],
            _pad0: [0; 3],
            player_x: [0.0; 2],
            player_y: [0.0; 2],
            player_xvel: [0.0; 2],
            player_yvel: [0.0; 2],
            player_oldy: [0.0; 2],
        }
    }

    pub fn from_players(p1: &CPlayer, p2: &CPlayer) -> Self {
        let mut s = P2PCollision::new();
        s.player_id[0] = p1.get_global_id() as u8;
        s.player_x[0] = p1.fx;
        s.player_y[0] = p1.fy;
        s.player_xvel[0] = p1.velx;
        s.player_yvel[0] = p1.vely;
        s.player_oldy[0] = p1.fOldY;

        s.player_id[1] = p2.get_global_id() as u8;
        s.player_x[1] = p2.fx;
        s.player_y[1] = p2.fy;
        s.player_xvel[1] = p2.velx;
        s.player_yvel[1] = p2.vely;
        s.player_oldy[1] = p2.fOldY;
        s
    }
}

/// Variable length: header, kind u8, context u16, arg count u8, draw count u16, then the i32 args and the u32
/// draws, all little-endian.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RandomEvent {
    pub kind: u8,
    pub context: u16,
    pub args: Vec<i32>,
    pub draws: Vec<u32>,
}

impl RandomEvent {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MessageHeader::new(NET_G2P_RANDOM_EVENT).as_bytes().to_vec();
        out.push(self.kind);
        out.extend(self.context.to_le_bytes());
        out.push(self.args.len() as u8);
        out.extend((self.draws.len() as u16).to_le_bytes());
        for a in &self.args {
            out.extend(a.to_le_bytes());
        }
        for d in &self.draws {
            out.extend(d.to_le_bytes());
        }
        out
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let body = data.get(3..)?;
        let head = body.get(..6)?;
        let (kind, context, nargs, ndraws) = (head[0], u16::from_le_bytes([head[1], head[2]]), head[3] as usize, u16::from_le_bytes([head[4], head[5]]) as usize);
        let words = body.get(6..6 + 4 * (nargs + ndraws))?;
        let word = |i: usize| [words[4 * i], words[4 * i + 1], words[4 * i + 2], words[4 * i + 3]];
        Some(RandomEvent {
            kind,
            context,
            args: (0..nargs).map(|i| i32::from_le_bytes(word(i))).collect(),
            draws: (nargs..nargs + ndraws).map(|i| u32::from_le_bytes(word(i))).collect(),
        })
    }
}

crate::net_package!(
    SyncOK,
    StartGame,
    LeaveGame,
    ClientInput,
    RemoteInput,
    GameState,
    RequestPowerup,
    StartPowerup,
    TriggerPowerup,
    MapCollision,
    P2PCollision
);

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{offset_of, size_of};

    /// Values from tools/ref/net_layout.cpp.
    #[test]
    fn layouts_match_clang() {
        assert_eq!(size_of::<SyncOK>(), 3);
        assert_eq!(size_of::<StartGame>(), 3);
        assert_eq!(size_of::<LeaveGame>(), 3);
        assert_eq!(size_of::<RawInput>(), 2);
        assert_eq!((size_of::<ClientInput>(), offset_of!(ClientInput, input_id), offset_of!(ClientInput, input)), (6, 3, 4));
        assert_eq!((size_of::<RemoteInput>(), offset_of!(RemoteInput, playerNumber), offset_of!(RemoteInput, input)), (6, 3, 4));
        assert_eq!(
            (
                size_of::<GameState>(),
                offset_of!(GameState, player_x),
                offset_of!(GameState, player_y),
                offset_of!(GameState, player_xvel),
                offset_of!(GameState, player_yvel),
                offset_of!(GameState, last_confirmed_local_input_id)
            ),
            (72, 4, 20, 36, 52, 68)
        );
        assert_eq!(size_of::<RequestPowerup>(), 3);
        assert_eq!(
            (size_of::<StartPowerup>(), offset_of!(StartPowerup, player_id), offset_of!(StartPowerup, powerup_id), offset_of!(StartPowerup, delay)),
            (12, 3, 4, 8)
        );
        assert_eq!(
            (
                size_of::<TriggerPowerup>(),
                offset_of!(TriggerPowerup, player_id),
                offset_of!(TriggerPowerup, powerup_id),
                offset_of!(TriggerPowerup, player_x),
                offset_of!(TriggerPowerup, player_y)
            ),
            (16, 3, 4, 8, 12)
        );
        assert_eq!(
            (
                size_of::<MapCollision>(),
                offset_of!(MapCollision, player_id),
                offset_of!(MapCollision, player_x),
                offset_of!(MapCollision, player_y),
                offset_of!(MapCollision, player_xvel),
                offset_of!(MapCollision, player_yvel)
            ),
            (20, 3, 4, 8, 12, 16)
        );
        assert_eq!(
            (
                size_of::<P2PCollision>(),
                offset_of!(P2PCollision, player_id),
                offset_of!(P2PCollision, player_x),
                offset_of!(P2PCollision, player_y),
                offset_of!(P2PCollision, player_xvel),
                offset_of!(P2PCollision, player_yvel),
                offset_of!(P2PCollision, player_oldy)
            ),
            (48, 3, 8, 16, 24, 32, 40)
        );
    }

    #[test]
    fn random_event_round_trip() {
        let ev = RandomEvent { kind: 7, context: 513, args: vec![-1, 65538], draws: vec![0, u32::MAX, 12345] };
        let bytes = ev.to_bytes();
        assert_eq!(bytes.len(), 3 + 6 + 4 * 5);
        assert_eq!(bytes[2], NET_G2P_RANDOM_EVENT);
        assert_eq!(RandomEvent::from_bytes(&bytes), Some(ev));
        assert_eq!(RandomEvent::from_bytes(&bytes[..bytes.len() - 1]), None);
    }

    #[test]
    fn raw_input_bits() {
        let mut r = RawInput::default();
        r.set_player_key(0, true, false);
        r.set_player_key(7, true, true);
        assert_eq!(r.flags, 0b1100_0000_0000_0001);
        let (mut d, mut p) = (false, false);
        r.get_player_key(7, &mut d, &mut p);
        assert!(d && p);
    }
}
