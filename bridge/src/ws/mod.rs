//! WebSocket-Server + Message-Protokoll.

pub mod handler;
pub mod protocol;

pub use handler::{serve, BridgeState};
#[allow(unused_imports)]
pub use protocol::ServerMessage;
