//! WebSocket-Server + Message-Protokoll.

pub mod handler;
pub mod protocol;

// Re-export für öffentliche API. `allow(unused_imports)`, weil
// main.rs das Protokoll erst nach WS-Server-Verdrahtung nutzt.
#[allow(unused_imports)]
pub use protocol::ServerMessage;
