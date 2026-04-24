//! Per-Client WebSocket-Handler.
//!
//! Pattern: tokio broadcast channel verteilt `ServerMessage` an N Clients.

use crate::error::Result;

pub async fn serve(_port: u16) -> Result<()> {
    todo!("TcpListener::bind → accept loop → tokio-tungstenite handshake → per-client task")
}
