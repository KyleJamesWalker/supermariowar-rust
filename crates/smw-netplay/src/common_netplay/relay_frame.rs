//! The frames between a game's WebSocket and `smw_relay`'s switch, specified in docs/guides/RELAY.md.

/// Game to relay: open virtual connection `conn` to the address in the payload.
pub const CONNECT: u8 = 1;
/// Relay to game: `conn` is open.
pub const ACCEPTED: u8 = 2;
/// Relay to game: `conn` was refused.
pub const REFUSED: u8 = 3;
/// Either way: one message on `conn`.
pub const DATA: u8 = 4;
/// Either way: `conn` is closed.
pub const DISCONNECT: u8 = 5;
/// Relay to game, first: the game's own virtual address, on `conn` 0.
pub const WELCOME: u8 = 9;

/// `[kind u8][conn u16 big-endian]` before the payload.
pub const HEADER_LEN: usize = 3;
/// An IPv4 address and a big-endian port.
pub const ADDRESS_LEN: usize = 6;
/// The largest WebSocket message the relay accepts, in bytes.
pub const MAX_MESSAGE_LEN: usize = 64 * 1024;

pub fn encode(kind: u8, conn: u16, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(HEADER_LEN + payload.len());
    frame.push(kind);
    frame.extend_from_slice(&conn.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// `(kind, conn, payload)`; `None` for a frame shorter than the header.
pub fn decode(frame: &[u8]) -> Option<(u8, u16, &[u8])> {
    if frame.len() < HEADER_LEN {
        return None;
    }
    Some((frame[0], u16::from_be_bytes([frame[1], frame[2]]), &frame[HEADER_LEN..]))
}

/// `a.b.c.d` is `[a, b, c, d]`.
pub fn encode_address(host: [u8; 4], port: u16) -> [u8; ADDRESS_LEN] {
    let p = port.to_be_bytes();
    [host[0], host[1], host[2], host[3], p[0], p[1]]
}

pub fn decode_address(payload: &[u8]) -> Option<([u8; 4], u16)> {
    if payload.len() != ADDRESS_LEN {
        return None;
    }
    Some(([payload[0], payload[1], payload[2], payload[3]], u16::from_be_bytes([payload[4], payload[5]])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let frame = encode(DATA, 0x8001, &[0, 5, 71]);
        assert_eq!(frame, [DATA, 0x80, 0x01, 0, 5, 71]);
        assert_eq!(decode(&frame), Some((DATA, 0x8001, &[0u8, 5, 71][..])));
        assert_eq!(decode(&[DATA, 0]), None);

        let address = encode_address([10, 0, 0, 2], 12522);
        assert_eq!(address, [10, 0, 0, 2, 0x30, 0xEA]);
        assert_eq!(decode_address(&address), Some(([10, 0, 0, 2], 12522)));
        assert_eq!(decode_address(&address[..5]), None);
    }
}
