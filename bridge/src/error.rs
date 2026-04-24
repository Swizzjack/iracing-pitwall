//! Zentrale Fehlertypen für Bridge-Komponenten.
//!
//! Konvention: jede Modulgrenze nimmt `BridgeError` oder einen spezifischeren
//! Subtyp an; `?` im Hot-Path, keine `unwrap()`/`panic!()`.

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

pub type Result<T> = std::result::Result<T, BridgeError>;
