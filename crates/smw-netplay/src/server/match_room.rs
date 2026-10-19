//! What the lobby hands each player of a starting room for netplay v2 (docs/design/NETPLAY_V2.md): a random
//! signaling room ID, which only admitted players learn, and short-lived TURN credentials.

use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};

/// TURN credentials stay valid this long after the room starts.
pub const TURN_CREDENTIAL_SECONDS: u64 = 6 * 60 * 60;

/// 128 random bits as 32 hex digits.
pub fn new_room_id() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("the OS random source");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// coturn's `use-auth-secret` scheme: the username is `<expiry>:<name>` and the password is
/// base64(HMAC-SHA1(secret, username)).
pub fn turn_credentials(secret: &str, now_unix: u64, name: &str) -> (String, String) {
    let username = format!("{}:{}", now_unix + TURN_CREDENTIAL_SECONDS, name);
    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(secret.as_bytes()).expect("HMAC takes any key length");
    mac.update(username.as_bytes());
    let password = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    (username, password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_credentials_match_coturn_rest_scheme() {
        let (username, password) = turn_credentials("0123abcd", 1_700_000_000 - TURN_CREDENTIAL_SECONDS, "smw");
        assert_eq!(username, "1700000000:smw");
        assert_eq!(password, "raLvRNXAbHqTzoE05oM0nB4fPbg=");
    }

    #[test]
    fn room_ids_are_random_hex() {
        let (a, b) = (new_room_id(), new_room_id());
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
