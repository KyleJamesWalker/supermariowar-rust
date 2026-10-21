//! The relay's settings, from the command line and the environment.

use crate::limits::TrustedProxies;
use std::env;

const USAGE: &str = "usage: smw_relay [--port <n>] [--config <lobby config file>] [--healthcheck]

  --port         TCP port for WebSocket and /healthz (env SMW_RELAY_PORT, default 8080)
  --config       lobby server settings file (default: serverconfig)
  --healthcheck  GET /healthz on 127.0.0.1:<port> and exit 0 if it answers 200

environment:
  SMW_RELAY_ALLOWED_ORIGINS  comma-separated page origins allowed to connect, or * for any; native games send none
                             (default: https://www.kylejameswalker.com,http://localhost:8000)
  SMW_RELAY_MAX_CLIENTS      simultaneous lobby clients, and separately signaling peers (default 64)
  SMW_RELAY_MAX_CONNECTIONS  simultaneous TCP connections of any kind, handshakes included (default 256)
  SMW_RELAY_MAX_PER_ADDRESS  simultaneous connections from one client address, 0 for no limit (default 16)
  SMW_RELAY_CONNECTS_PER_MINUTE  new connections one client address may open per minute, 0 for no limit (default 60)
  SMW_RELAY_TRUSTED_PROXIES  peers whose X-Forwarded-For names the client: none, loopback or private (loopback and
                             private networks, for a proxy on a Docker network) (default loopback)

Loopback clients are exempt from the per-address limits.";

const DEFAULT_ORIGINS: &str = "https://www.kylejameswalker.com,http://localhost:8000";

#[derive(Clone, Debug)]
pub struct Config {
    /// TCP port for lobby clients, signaling and `/healthz`.
    pub port: u16,
    /// The lobby's config file; `<path>.txt` is tried too.
    pub lobby_config: String,
    /// Probe a running relay's `/healthz` and exit instead of serving.
    pub healthcheck: bool,
    /// Normalized page origins browsers may connect from; `*` allows any.
    pub allowed_origins: Vec<String>,
    /// Simultaneous lobby WebSocket clients, and separately simultaneous signaling peers.
    pub max_clients: usize,
    /// Simultaneous TCP connections of any kind; each holds a thread.
    pub max_connections: usize,
    /// Simultaneous connections from one client address; 0 for no limit.
    pub max_per_address: usize,
    /// New connections one client address may open per minute; 0 for no limit.
    pub connects_per_minute: u32,
    pub trusted_proxies: TrustedProxies,
    /// Virtual connections one client may hold open to the lobby.
    pub max_connections_per_client: usize,
    /// Overrides the lobby's `SMW_SIGNAL_URL`; `/signal` means this relay's own signaling.
    pub signal_url: Option<String>,
    /// Whether the lobby writes `serverlog.txt`.
    pub log_file: bool,
    /// The lobby's player limit, overriding its config file.
    pub lobby_max_players: Option<u32>,
    /// The lobby's room limit; `None` for no limit.
    pub lobby_max_rooms: Option<usize>,
}

impl Config {
    /// The environment, then the arguments; the error is the usage text or names the bad value.
    pub fn from_args_and_env() -> Result<Config, String> {
        let mut config = Config {
            port: 8080,
            lobby_config: "serverconfig".to_string(),
            healthcheck: false,
            allowed_origins: parse_origins(&env::var("SMW_RELAY_ALLOWED_ORIGINS").unwrap_or_else(|_| DEFAULT_ORIGINS.to_string())),
            max_clients: 64,
            max_connections: 256,
            max_per_address: 16,
            connects_per_minute: 60,
            trusted_proxies: TrustedProxies::Loopback,
            max_connections_per_client: 8,
            signal_url: env::var_os("SMW_SIGNAL_URL").is_none().then(|| "/signal".to_string()),
            log_file: true,
            lobby_max_players: None,
            lobby_max_rooms: None,
        };
        if let Ok(port) = env::var("SMW_RELAY_PORT") {
            config.port = port.parse().map_err(|_| format!("SMW_RELAY_PORT: not a port: {port}"))?;
        }
        if let Ok(max) = env::var("SMW_RELAY_MAX_CLIENTS") {
            config.max_clients = max.parse().map_err(|_| format!("SMW_RELAY_MAX_CLIENTS: not a number: {max}"))?;
        }
        if let Ok(max) = env::var("SMW_RELAY_MAX_CONNECTIONS") {
            config.max_connections = max.parse().map_err(|_| format!("SMW_RELAY_MAX_CONNECTIONS: not a number: {max}"))?;
        }
        if let Ok(max) = env::var("SMW_RELAY_MAX_PER_ADDRESS") {
            config.max_per_address = max.parse().map_err(|_| format!("SMW_RELAY_MAX_PER_ADDRESS: not a number: {max}"))?;
        }
        if let Ok(rate) = env::var("SMW_RELAY_CONNECTS_PER_MINUTE") {
            config.connects_per_minute = rate.parse().map_err(|_| format!("SMW_RELAY_CONNECTS_PER_MINUTE: not a number: {rate}"))?;
        }
        if let Ok(trusted) = env::var("SMW_RELAY_TRUSTED_PROXIES") {
            config.trusted_proxies =
                TrustedProxies::parse(&trusted).ok_or_else(|| format!("SMW_RELAY_TRUSTED_PROXIES: not none, loopback or private: {trusted}"))?;
        }

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--port" => {
                    let value = args.next().ok_or(USAGE)?;
                    config.port = value.parse().map_err(|_| format!("--port: not a port: {value}"))?;
                }
                "--config" => config.lobby_config = args.next().ok_or(USAGE)?,
                "--healthcheck" => config.healthcheck = true,
                _ => return Err(USAGE.to_string()),
            }
        }
        Ok(config)
    }

    /// The settings of the relay a game runs to host on its own network.
    pub fn embedded(port: u16) -> Config {
        Config {
            port,
            lobby_config: String::new(),
            healthcheck: false,
            allowed_origins: vec!["*".to_string()],
            max_clients: 8,
            max_connections: 32,
            max_per_address: 0,
            connects_per_minute: 0,
            trusted_proxies: TrustedProxies::Loopback,
            max_connections_per_client: 8,
            signal_url: Some("/signal".to_string()),
            log_file: false,
            lobby_max_players: Some(4),
            lobby_max_rooms: Some(1),
        }
    }

    /// Whether a WebSocket handshake with this `Origin` header may proceed.
    pub fn origin_allowed(&self, origin: Option<&str>) -> bool {
        if self.allowed_origins.iter().any(|o| o == "*") {
            return true;
        }
        // Browsers always send Origin; a client without one is a native game, which no web page can impersonate.
        match origin {
            Some(origin) => self.allowed_origins.contains(&normalize_origin(origin)),
            None => true,
        }
    }
}

fn normalize_origin(origin: &str) -> String {
    origin.trim().trim_end_matches('/').to_ascii_lowercase()
}

fn parse_origins(list: &str) -> Vec<String> {
    list.split(',').map(normalize_origin).filter(|o| !o.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(origins: &str) -> Config {
        Config {
            port: 8080,
            lobby_config: String::new(),
            healthcheck: false,
            allowed_origins: parse_origins(origins),
            max_clients: 1,
            max_connections: 1,
            max_per_address: 0,
            connects_per_minute: 0,
            trusted_proxies: TrustedProxies::Loopback,
            max_connections_per_client: 1,
            signal_url: None,
            log_file: true,
            lobby_max_players: None,
            lobby_max_rooms: None,
        }
    }

    #[test]
    fn origins() {
        let c = config(DEFAULT_ORIGINS);
        assert!(c.origin_allowed(Some("https://www.kylejameswalker.com")));
        assert!(c.origin_allowed(Some("HTTPS://www.KyleJamesWalker.com/")));
        assert!(c.origin_allowed(Some("http://localhost:8000")));
        assert!(!c.origin_allowed(Some("http://localhost:8001")));
        assert!(!c.origin_allowed(Some("https://evil.example")));
        assert!(c.origin_allowed(None));

        let any = config(" * ");
        assert!(any.origin_allowed(None));
        assert!(any.origin_allowed(Some("https://evil.example")));
    }
}
