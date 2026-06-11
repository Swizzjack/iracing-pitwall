//! Runtime configuration (env-driven, no config file yet).

#[derive(Debug, Clone)]
pub struct Config {
    pub ws_port: u16,
    /// Disables auto-shutdown (BRIDGE_KEEP_ALIVE=1).
    pub keep_alive: bool,
    /// Seconds without a client after the first connect before shutdown (BRIDGE_SHUTDOWN_GRACE_SEC).
    pub shutdown_grace_sec: u64,
    /// Startup-timeout seconds if no client ever connects (BRIDGE_STARTUP_GRACE_SEC).
    pub startup_grace_sec: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            ws_port: std::env::var("BRIDGE_WS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8765),
            keep_alive: std::env::var("BRIDGE_KEEP_ALIVE")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            shutdown_grace_sec: std::env::var("BRIDGE_SHUTDOWN_GRACE_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            startup_grace_sec: std::env::var("BRIDGE_STARTUP_GRACE_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
        }
    }
}
