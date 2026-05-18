//! Messages from Dashboard → Bridge (currently none).

use serde::Deserialize;
use ts_rs::TS;

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {}
