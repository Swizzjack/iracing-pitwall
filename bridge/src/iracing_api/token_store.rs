//! Persistent storage for OAuth2 tokens.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds) when the access token expires.
    pub expires_at: i64,
    pub cust_id: Option<i64>,
    pub member_name: Option<String>,
}

impl StoredTokens {
    pub fn is_access_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        now >= self.expires_at - 60
    }
}

pub fn load(path: &PathBuf) -> Option<StoredTokens> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data)
        .map_err(|e| log::warn!("token_store: parse error: {e}"))
        .ok()
}

pub fn save(path: &PathBuf, tokens: &StoredTokens) -> Result<()> {
    let json = serde_json::to_string_pretty(tokens)?;
    std::fs::write(path, json).with_context(|| format!("write token file {}", path.display()))?;
    Ok(())
}
