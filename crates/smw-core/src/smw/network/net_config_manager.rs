//! Port of src/smw/network/NetConfigManager.cpp

use crate::common::path::get_home_directory;
use crate::common_netplay::protocol_definitions::NET_MAX_PLAYER_NAME_LENGTH;
use crate::smw::net::{netplay, ServerAddress};

const CONFIG_FILENAME: &str = "servers.toml";

/// toml11 v4 `escape_basic_string`, byte by byte with a signed `char`, so bytes from 0x80 pass through.
fn toml_basic_string(s: &str) -> String {
    let mut out = vec![b'"'];
    for &c in s.as_bytes() {
        match c {
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'"' => out.extend_from_slice(b"\\\""),
            0x08 => out.extend_from_slice(b"\\b"),
            b'\t' => out.extend_from_slice(b"\\t"),
            0x0C => out.extend_from_slice(b"\\f"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            0x00..=0x1F | 0x7F => out.extend_from_slice(format!("\\u00{:X}{:X}", c / 16, c % 16).as_bytes()),
            _ => out.push(c),
        }
    }
    out.push(b'"');
    String::from_utf8(out).expect("escaping keeps UTF-8 intact")
}

/// toml11 v4 `toml::format` of the `{player_name, servers}` table. libc++'s unordered_map puts
/// `servers` first; an array goes multiline once its approximate inline length passes 60.
fn format_config(player_name: &str, servers: &[String]) -> String {
    let mut approx_len = 0;
    let mut multiline = false;
    for server in servers {
        approx_len += 2 + toml_basic_string(server).len();
        if approx_len > 60 {
            multiline = true;
            break;
        }
        approx_len += 2;
    }

    let array = if multiline {
        let mut a = String::from("[\n");
        for server in servers {
            a += &format!("    {},\n", toml_basic_string(server));
        }
        a + "]"
    } else {
        format!("[{}]", servers.iter().map(|s| toml_basic_string(s)).collect::<Vec<_>>().join(", "))
    };

    format!("servers = {}\nplayer_name = {}\n\n", array, toml_basic_string(player_name))
}

fn load_file() -> Option<toml::Table> {
    let Ok(text) = std::fs::read_to_string(get_home_directory() + CONFIG_FILENAME) else {
        println!("[net][warning] Could not open {}, using default values.", CONFIG_FILENAME);
        return None;
    };

    match text.parse::<toml::Table>() {
        Ok(config) => Some(config),
        Err(error) => {
            println!("[net][warning] {}: {}", CONFIG_FILENAME, error);
            None
        }
    }
}

fn read_playername(config: &toml::Table) {
    let result = (|| -> Result<(), String> {
        let Some(config_playername) = config.get("player_name") else {
            return Ok(());
        };

        let Some(net_player_name) = config_playername.as_str() else {
            return Err("player name must be a simple string".to_string());
        };

        if net_player_name.len() < 3 {
            return Err("player name too short".to_string());
        }

        if net_player_name.len() >= NET_MAX_PLAYER_NAME_LENGTH {
            return Err(format!("player name must be less than {} letters", NET_MAX_PLAYER_NAME_LENGTH));
        }

        unsafe { netplay.myPlayerName = net_player_name.to_string() };
        Ok(())
    })();

    if let Err(error) = result {
        println!("[net][warning] {}: {}", CONFIG_FILENAME, error);
    }
}

fn read_servers(config: &toml::Table) {
    let result = (|| -> Result<(), String> {
        let Some(config_servers) = config.get("servers") else {
            return Ok(());
        };

        let Some(servers) = config_servers.as_array() else {
            return Err("`servers` is in wrong format".to_string());
        };

        for (i, server) in servers.iter().enumerate() {
            let Some(address_str) = server.as_str() else {
                println!("[net][warning] {}: server #{} is invalid", CONFIG_FILENAME, i + 1);
                continue;
            };

            if address_str.len() < 8 || address_str.len() > 250 {
                println!("[net][warning] {}: server #{} is invalid", CONFIG_FILENAME, i + 1);
                continue;
            }

            unsafe { netplay.savedServers.push(ServerAddress { hostname: address_str.to_string() }) };
        }
        Ok(())
    })();

    if let Err(error) = result {
        println!("[net][warning] {}: {}", CONFIG_FILENAME, error);
    }
}

#[derive(Default)]
pub struct NetConfigManager;

impl NetConfigManager {
    pub fn save(&mut self) {
        unsafe {
            debug_assert!(!netplay.myPlayerName.is_empty());

            // Remove `(none)`
            if netplay.savedServers.len() == 1 && netplay.savedServers[0].hostname == "(none)" {
                netplay.savedServers.clear();
            }

            let servers: Vec<String> = netplay.savedServers.iter().map(|s| s.hostname.clone()).collect();
            let content = format_config(&netplay.myPlayerName, &servers);

            if std::fs::write(get_home_directory() + CONFIG_FILENAME, content).is_err() {
                println!("[net][error] Could not save network settings");
            }
        }
    }

    pub fn load(&mut self) {
        let Some(config) = load_file() else {
            return;
        };

        read_playername(&config);
        read_servers(&config);
    }
}

#[cfg(test)]
mod tests {
    use super::format_config;

    /// Expected bytes from toml11 v4.4.0 (`toml::format`) built with the C++ reference's libc++.
    #[test]
    fn format_matches_toml11() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(format_config("Player", &[]), "servers = []\nplayer_name = \"Player\"\n\n");
        assert_eq!(
            format_config("Player", &s(&["127.0.0.1", "smw.example.org"])),
            "servers = [\"127.0.0.1\", \"smw.example.org\"]\nplayer_name = \"Player\"\n\n"
        );
        assert_eq!(format_config("Pl\"a\\y\ter", &[]), "servers = []\nplayer_name = \"Pl\\\"a\\\\y\\ter\"\n\n");
        assert_eq!(format_config("h\u{e9}\u{7f}\u{1}", &[]), "servers = []\nplayer_name = \"h\u{e9}\\u007F\\u0001\"\n\n");
        assert_eq!(
            format_config("P", &s(&["123456789012", "123456789012", "123456789012", "123456789012", "12345678901"])),
            "servers = [\n    \"123456789012\",\n    \"123456789012\",\n    \"123456789012\",\n    \"123456789012\",\n    \"12345678901\",\n]\nplayer_name = \"P\"\n\n"
        );
    }
}
