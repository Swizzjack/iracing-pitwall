//! WebSocket-Server + Message-Protokoll.

pub mod client;
pub mod handler;
pub mod lifecycle;
pub mod protocol;

pub use handler::{bind, serve, BridgeState};
pub use lifecycle::{ClientTracker, LifecycleConfig};
#[allow(unused_imports)]
pub use protocol::ServerMessage;
