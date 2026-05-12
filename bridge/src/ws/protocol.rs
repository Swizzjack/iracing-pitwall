//! WebSocket Message Schema (Bridge → Dashboard).
//!
//! tagged Enum, serde `type` Discriminator → TypeScript Discriminated Union.

use crate::iracing_sdk::header::HeaderStatus;
use crate::iracing_sdk::types::SessionInfoYaml;
use crate::persistence::queries::{FilterOptions, SessionDetail, SessionSummary};
use crate::telemetry::{StandingsSnapshot, TelemetrySnapshot, TrackMapSnapshot};
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

    // ── Results / History ────────────────────────────────────────────────

    /// Bridge sends the OAuth authorization URL; dashboard opens it in a browser tab.
    #[serde(rename_all = "camelCase")]
    OAuthUrl { url: String },

    /// Current OAuth link status — sent on connect and after successful auth.
    #[serde(rename_all = "camelCase")]
    OAuthStatus {
        linked: bool,
        member_name: Option<String>,
        #[ts(type = "number | null")]
        cust_id: Option<i64>,
    },

    /// Response to `QueryResults`.
    #[serde(rename_all = "camelCase")]
    ResultsList {
        sessions: Vec<SessionSummary>,
        #[ts(type = "number")]
        total: i64,
    },

    /// Response to `QueryResultDetail`.
    #[serde(rename_all = "camelCase")]
    ResultDetail { session: SessionDetail },

    /// Response to `QueryFilterOptions`.
    #[serde(rename_all = "camelCase")]
    FilterOptions { options: FilterOptions },

    /// Push notification — a new result was stored (triggers re-query).
    #[serde(rename_all = "camelCase")]
    ResultsUpdated {
        #[ts(type = "number")]
        sub_session_id: i64,
    },
}
