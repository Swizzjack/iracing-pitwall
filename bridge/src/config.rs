//! Runtime configuration (env-driven, no config file yet).

#[derive(Debug, Clone)]
pub struct Config {
    pub ws_port: u16,
    pub telemetry_hz: u32,
    pub standings_hz: u32,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            ws_port: std::env::var("BRIDGE_WS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8765),
            telemetry_hz: 60,
            standings_hz: 4,
        }
    }
}
