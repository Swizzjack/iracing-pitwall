//! Central error types for bridge components.
//!
//! Convention: every module boundary takes `BridgeError` or a more specific
//! subtype; use `?` in the hot path, never `unwrap()`/`panic!()`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("iRacing SDK not connected: {0}")]
    SdkNotConnected(String),

    #[error("iRacing SDK read failed: {0}")]
    SdkRead(String),

    #[error("YAML parse error: {0}")]
    YamlParse(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl From<axum::Error> for BridgeError {
    fn from(e: axum::Error) -> Self {
        BridgeError::WebSocket(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, BridgeError>;
