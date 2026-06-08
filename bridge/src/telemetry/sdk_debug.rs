//! Admin/debug snapshot: full live dump of all SDK data.
//!
//! Unlike `TelemetrySnapshot` (curated whitelist), this snapshot delivers
//! EVERYTHING the SDK provides — every variable, the complete raw session
//! YAML, and the header diagnostics. Intended only for the hidden admin
//! view (see `dashboard/src/features/sdk-debug`); never persisted and only
//! built/sent while the panel is open.

use serde::Serialize;
use ts_rs::TS;

use crate::iracing_sdk::header::HeaderStatus;
use crate::iracing_sdk::yaml::decode_raw;
use crate::iracing_sdk::IRacingClient;

/// A single SDK variable with its current value, formatted for display.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct VarDump {
    pub name: String,
    pub desc: String,
    pub unit: String,
    /// "Float", "Int", "Bool", … (Debug representation of `VarType`).
    pub var_type: String,
    pub count: usize,
    /// One entry per array element; empty if the variable currently lies
    /// outside the frame buffer (e.g. right after connecting).
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct SdkDebugSnapshot {
    pub header: HeaderStatus,
    pub vars: Vec<VarDump>,
    /// Complete, unfiltered session YAML as text (display-only,
    /// not parsed again — see `iracing_sdk::yaml::decode_raw`).
    pub session_yaml_raw: String,
}

impl SdkDebugSnapshot {
    pub fn build(client: &IRacingClient) -> Self {
        Self {
            header: HeaderStatus::from_header(client.header()),
            vars: client.dump_all_vars(),
            session_yaml_raw: decode_raw(client.session_info_bytes()),
        }
    }
}
