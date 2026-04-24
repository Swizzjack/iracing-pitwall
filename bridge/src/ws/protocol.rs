//! WebSocket Message Schema (Bridge → Dashboard).
//!
//! tagged Enum, serde `type` Discriminator → TypeScript Discriminated Union.

use crate::iracing_sdk::header::HeaderStatus;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::telemetry::{StandingsSnapshot, TelemetrySnapshot};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerMessage {
    #[serde(rename_all = "camelCase")]
    Hello { bridge_version: String },

    #[serde(rename_all = "camelCase")]
    SdkStatus { status: HeaderStatus },

    #[serde(rename_all = "camelCase")]
    Telemetry { snapshot: TelemetrySnapshot },

    #[serde(rename_all = "camelCase")]
    Standings { snapshot: StandingsSnapshot },

    #[serde(rename_all = "camelCase")]
    SessionInfo { info: SessionInfoYaml },

    #[serde(rename_all = "camelCase")]
    Disconnected { reason: String },
}
