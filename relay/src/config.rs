use std::env;

const USAGE: &str = "usage: smw_relay [--port <n>] [--config <lobby config file>] [--healthcheck]

  --port         TCP port for WebSocket and /healthz (env SMW_RELAY_PORT, default 8080)
  --config       lobby server settings, as smw_server's serverconfig (default: serverconfig)
  --healthcheck  GET /healthz on 127.0.0.1:<port> and exit 0 if it answers 200

environment:
  SMW_RELAY_ALLOWED_ORIGINS  comma-separated page origins allowed to connect, or * for any
                             (default: https://www.kylejameswalker.com,http://localhost:8000)
  SMW_RELAY_MAX_CLIENTS      simultaneous WebSocket clients (default 64)";

const DEFAULT_ORIGINS: &str = "https://www.kylejameswalker.com,http://localhost:8000";

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub lobby_config: String,
    pub healthcheck: bool,
    pub allowed_origins: Vec<String>,
    pub max_clients: usize,
    pub max_connections_per_client: usize,
}

impl Config {
    pub fn from_args_and_env() -> Result<Config, String> {
        let mut config = Config {
            port: 8080,
            lobby_config: "serverconfig".to_string(),
            healthcheck: false,
            allowed_origins: parse_origins(&env::var("SMW_RELAY_ALLOWED_ORIGINS").unwrap_or_else(|_| DEFAULT_ORIGINS.to_string())),
            max_clients: 64,
            max_connections_per_client: 8,
        };
        if let Ok(port) = env::var("SMW_RELAY_PORT") {
            config.port = port.parse().map_err(|_| format!("SMW_RELAY_PORT: not a port: {port}"))?;
        }
        if let Ok(max) = env::var("SMW_RELAY_MAX_CLIENTS") {
            config.max_clients = max.parse().map_err(|_| format!("SMW_RELAY_MAX_CLIENTS: not a number: {max}"))?;
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

    pub fn origin_allowed(&self, origin: Option<&str>) -> bool {
        if self.allowed_origins.iter().any(|o| o == "*") {
            return true;
        }
        match origin {
            Some(origin) => self.allowed_origins.contains(&normalize_origin(origin)),
            None => false,
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
            max_connections_per_client: 1,
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
        assert!(!c.origin_allowed(None));

        let any = config(" * ");
        assert!(any.origin_allowed(None));
        assert!(any.origin_allowed(Some("https://evil.example")));
    }
}
