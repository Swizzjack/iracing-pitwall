//! Persistent data directory for iRacing Pitwall.

use std::path::PathBuf;

/// Returns `%APPDATA%\iracing-pitwall\` (Windows) or a home-dir fallback on other platforms.
/// Creates the directory if it does not exist.
pub fn data_dir() -> PathBuf {
    let base = directories::ProjectDirs::from("com", "iracing", "iracing-pitwall")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
                .join("data")
        });
    if let Err(e) = std::fs::create_dir_all(&base) {
        log::warn!("paths: failed to create data_dir {}: {e}", base.display());
    }
    base
}
