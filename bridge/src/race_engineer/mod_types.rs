//! Shared types used across race_engineer sub-modules.

/// Install/download progress event, sent from blocking install tasks
/// back to the async engineer task via an unbounded channel.
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub stage: String,       // "downloading" | "extracting" | "validating"
    pub target: String,      // "piper" | "voice"
    pub target_id: Option<String>,
}
