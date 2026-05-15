//! Messages from Dashboard → Bridge.

use crate::persistence::queries::ResultsFilter;
use serde::Deserialize;
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    /// Start the OAuth2 PKCE flow — bridge opens a loopback listener and
    /// returns the authorization URL in `OAuthUrl`.
    StartOAuth,

    /// Query the stored results list with optional filters.
    #[serde(rename_all = "camelCase")]
    QueryResults { filter: ResultsFilter },

    /// Query full detail for one session.
    #[serde(rename_all = "camelCase")]
    QueryResultDetail {
        #[ts(type = "number")]
        sub_session_id: i64,
    },

    /// Query available filter options (distinct tracks / cars / series).
    QueryFilterOptions,

    /// Manually trigger a fetch for a known SubSessionID (dev / testing).
    #[serde(rename_all = "camelCase")]
    TriggerFetch {
        #[ts(type = "number")]
        sub_session_id: i64,
    },

    /// Query lap-by-lap data for a session. If car_idx is omitted, returns all cars.
    #[serde(rename_all = "camelCase")]
    QueryLaps {
        #[ts(type = "number")]
        sub_session_id: i64,
        car_idx: Option<i32>,
    },
}
