//! Messages from Dashboard → Bridge.

use serde::Deserialize;
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    #[serde(rename_all = "camelCase")]
    DeleteTrackMap { track_key: String },

    /// Hidden admin/debug view: enable/disable the live full-SDK-dump feed.
    /// Gated so the bridge only builds `SdkDebugSnapshot` while someone is
    /// actually looking at it (it walks every SDK variable every tick).
    #[serde(rename_all = "camelCase")]
    SetSdkDebug { enabled: bool },

    // --- Race Engineer ---

    EngineerGetStatus,

    EngineerInstallPiper,

    #[serde(rename_all = "camelCase")]
    EngineerInstallVoice { voice_id: String },

    #[serde(rename_all = "camelCase")]
    EngineerUninstallVoice { voice_id: String },

    #[serde(rename_all = "camelCase")]
    EngineerSynthesize {
        voice_id: String,
        text: String,
        request_id: String,
    },

    #[serde(rename_all = "camelCase")]
    EngineerUpdateBehavior {
        enabled: bool,
        frequency: String,
        mute_in_qualifying: bool,
        debug_all_rules_in_practice: bool,
        active_voice_id: Option<String>,
        pilot_name: Option<String>,
        mute_name: bool,
    },
}
