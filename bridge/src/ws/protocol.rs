//! WebSocket Message Schema (Bridge → Dashboard).
//!
//! tagged Enum, serde `type` Discriminator → TypeScript Discriminated Union.

use crate::iracing_sdk::header::HeaderStatus;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::race_engineer::voice_manager::VoiceInfo;
use crate::telemetry::{SdkDebugSnapshot, StandingsSnapshot, TelemetrySnapshot, TrackMapSnapshot};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerMessage {
    #[serde(rename_all = "camelCase")]
    Hello { bridge_version: String, lan_url: Option<String> },

    #[serde(rename_all = "camelCase")]
    SdkStatus { status: HeaderStatus },

    /// Full live SDK dump for the hidden admin/debug view — only sent while
    /// a client has it enabled via `ClientMessage::SetSdkDebug`.
    #[serde(rename_all = "camelCase")]
    SdkDebug { snapshot: SdkDebugSnapshot },

    #[serde(rename_all = "camelCase")]
    Telemetry { snapshot: TelemetrySnapshot },

    #[serde(rename_all = "camelCase")]
    Standings { snapshot: StandingsSnapshot },

    #[serde(rename_all = "camelCase")]
    SessionInfo { info: SessionInfoYaml },

    #[serde(rename_all = "camelCase")]
    TrackMap { snapshot: TrackMapSnapshot },

    #[serde(rename_all = "camelCase")]
    Disconnected { reason: String },

    #[serde(rename_all = "camelCase")]
    UpdateAvailable { latest_version: String, release_url: String },

    // --- Race Engineer ---

    #[serde(rename_all = "camelCase")]
    EngineerStatus {
        piper_installed: bool,
        piper_version: Option<String>,
        voices: Vec<VoiceInfo>,
    },

    #[serde(rename_all = "camelCase")]
    EngineerInstallProgress {
        target: String,
        target_id: Option<String>,
        bytes_downloaded: u32,
        bytes_total: Option<u32>,
        stage: String,
    },

    #[serde(rename_all = "camelCase")]
    EngineerInstallComplete {
        target: String,
        target_id: Option<String>,
        success: bool,
        error: Option<String>,
    },

    #[serde(rename_all = "camelCase")]
    EngineerAudio {
        request_id: String,
        priority: String,
        wav_base64: String,
        sample_rate: u32,
        duration_ms: u32,
        text: String,
    },

    #[serde(rename_all = "camelCase")]
    EngineerError { message: String },
}
