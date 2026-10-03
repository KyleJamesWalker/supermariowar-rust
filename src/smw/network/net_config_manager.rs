//! Port of src/smw/network/NetConfigManager.cpp

use crate::common::path::get_home_directory;
use crate::common_netplay::protocol_definitions::NET_MAX_PLAYER_NAME_LENGTH;
use crate::smw::net::{netplay, ServerAddress};
use yaml_rust2::{Yaml, YamlLoader};

/// yaml-cpp's plain-scalar test for the strings this file writes; other strings get double quotes.
fn needs_quotes(s: &str) -> bool {
    if s.is_empty() || matches!(s, "~" | "null" | "Null" | "NULL") {
        return true;
    }
    let b = s.as_bytes();
    let first = b[0];
    let next_blank = b.get(1).is_none_or(|c| *c == b' ' || *c == b'\t');
    if b",[]{}#&*!|>'\"%@`".contains(&first) || (b"-?:".contains(&first) && next_blank) {
        return true;
    }
    if first == b' ' || b[b.len() - 1] == b' ' || s.contains(": ") || s.contains(" #") || s.ends_with(':') {
        return true;
    }
    s.chars().any(|c| c.is_control())
}

fn scalar(s: &str) -> String {
    if !needs_quotes(s) {
        return s.to_string();
    }
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out += "\\\"",
            '\\' => out += "\\\\",
            '\n' => out += "\\n",
            '\t' => out += "\\t",
            c if c.is_control() => out += &format!("\\x{:02x}", c as u32),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[derive(Default)]
pub struct NetConfigManager;

impl NetConfigManager {
    pub fn save(&mut self) {
        unsafe {
            debug_assert!(!netplay.myPlayerName.is_empty());

            let path = get_home_directory() + "servers.yml";

            if netplay.savedServers.len() == 1 && netplay.savedServers[0].hostname == "(none)" {
                netplay.savedServers.clear();
            }

            let mut content = format!("player_name: {}\nservers:\n", scalar(&netplay.myPlayerName));
            if netplay.savedServers.is_empty() {
                content += "  []";
            } else {
                let lines: Vec<String> = netplay.savedServers.iter().map(|s| format!("  - {}", scalar(&s.hostname))).collect();
                content += &lines.join("\n");
            }

            if std::fs::write(&path, content).is_err() {
                println!("[net][error] Could not save network settings");
            }
        }
    }

    pub fn load(&mut self) {
        let Some(config) = self.load_file() else {
            return;
        };

        self.read_playername(&config);
        self.read_servers(&config);
    }

    fn load_file(&mut self) -> Option<Yaml> {
        let Ok(text) = std::fs::read_to_string(get_home_directory() + "servers.yml") else {
            println!("[net][warning] Could not open servers.yml, using default values.");
            return None;
        };

        match YamlLoader::load_from_str(&text) {
            Ok(mut docs) => Some(if docs.is_empty() { Yaml::Null } else { docs.remove(0) }),
            Err(e) => {
                print!("[net][warning] servers.yml: {}", e);
                None
            }
        }
    }

    fn read_playername(&mut self, config: &Yaml) {
        let result = (|| -> Result<(), String> {
            let config_playername = &config["player_name"];
            if config_playername.is_null() || config_playername.is_badvalue() {
                return Ok(());
            }

            let net_player_name = match config_playername {
                Yaml::String(s) | Yaml::Real(s) => s.clone(),
                Yaml::Integer(i) => i.to_string(),
                Yaml::Boolean(b) => b.to_string(),
                _ => return Err("player name must be a simple string".to_string()),
            };

            if net_player_name.len() < 3 {
                return Err("player name too short".to_string());
            }

            if net_player_name.len() >= NET_MAX_PLAYER_NAME_LENGTH {
                return Err(format!("player name must be less than {} letters", NET_MAX_PLAYER_NAME_LENGTH));
            }

            unsafe { netplay.myPlayerName = net_player_name };
            Ok(())
        })();

        if let Err(e) = result {
            println!("[net][warning] servers.yml: {}", e);
        }
    }

    fn read_servers(&mut self, config: &Yaml) {
        let result = (|| -> Result<(), String> {
            let config_servers = &config["servers"];
            if config_servers.is_null() || config_servers.is_badvalue() {
                return Ok(());
            }

            let Yaml::Array(list) = config_servers else {
                return Err("`servers` is in wrong format".to_string());
            };

            for (i, entry) in list.iter().enumerate() {
                let address_str = match entry {
                    Yaml::String(s) | Yaml::Real(s) => s.clone(),
                    Yaml::Integer(n) => n.to_string(),
                    Yaml::Boolean(b) => b.to_string(),
                    _ => return Err("bad conversion".to_string()),
                };

                if address_str.len() < 8 || address_str.len() > 250 {
                    println!("[net][warning] servers.yml: server #{} is invalid", i + 1);
                    continue;
                }

                unsafe { netplay.savedServers.push(ServerAddress { hostname: address_str }) };
            }
            Ok(())
        })();

        if let Err(e) = result {
            println!("[net][warning] servers.yml: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::scalar;

    /// Expected strings from yaml-cpp's emitter.
    #[test]
    fn quoting_matches_yaml_cpp() {
        assert_eq!(scalar("smw.example.org"), "smw.example.org");
        assert_eq!(scalar("127.0.0.1:12521"), "127.0.0.1:12521");
        assert_eq!(scalar("(none)"), "(none)");
        assert_eq!(scalar("yes"), "yes");
        assert_eq!(scalar("a: b"), "\"a: b\"");
        assert_eq!(scalar("#x"), "\"#x\"");
        assert_eq!(scalar("-dash"), "-dash");
        assert_eq!(scalar(""), "\"\"");
    }
}
