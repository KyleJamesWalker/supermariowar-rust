//! What the lobby hands each player of a starting room for netplay v2 (docs/design/NETPLAY_V2.md): a random
//! signaling room ID, which only admitted players learn, and short-lived TURN credentials.

use base64::Engine;
use hmac::{Hmac, KeyInit, Mac};
use std::net::IpAddr;

/// TURN credentials stay valid this long after the room starts; coturn checks them only when a relay is allocated.
pub const TURN_CREDENTIAL_SECONDS: u64 = 2 * 60 * 60;

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
    let password = base64::engine::general_purpose::STANDARD.encode(hmac(secret, &username));
    (username, password)
}

/// The TURN name of every player from `address`: coturn's `user-quota` counts allocations per name.
pub fn turn_name(secret: &str, address: IpAddr) -> String {
    let digest = hmac(secret, &format!("quota {address}"));
    format!("smw-{}", digest[..8].iter().map(|b| format!("{b:02x}")).collect::<String>())
}

fn hmac(secret: &str, text: &str) -> Vec<u8> {
    let mut mac = Hmac::<sha1::Sha1>::new_from_slice(secret.as_bytes()).expect("HMAC takes any key length");
    mac.update(text.as_bytes());
    mac.finalize().into_bytes().to_vec()
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
    fn one_address_has_one_turn_name() {
        let ip = |s: &str| s.parse::<IpAddr>().unwrap();
        let name = turn_name("00ff", ip("203.0.113.1"));
        assert!(name.starts_with("smw-") && name.len() == 20, "{name}");
        assert_eq!(name, turn_name("00ff", ip("203.0.113.1")));
        assert_ne!(name, turn_name("00ff", ip("203.0.113.2")));
        assert_ne!(name, turn_name("11ee", ip("203.0.113.1")), "the secret keys the name");
    }

    #[test]
    fn room_ids_are_random_hex() {
        let (a, b) = (new_room_id(), new_room_id());
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
