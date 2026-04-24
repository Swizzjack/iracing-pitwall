//! MMF-basierter Shared-Memory-Reader.
//!
//! Lifecycle:
//!   1. `IRacingClient::connect()` öffnet MMF + DataValidEvent
//!   2. `wait_for_frame()` blockiert auf Event, kopiert aktuellen varBuf
//!   3. `get_*()` liefert Werte aus dem lokalen Puffer
//!
//! Die Triple-Buffer-Strategie (tickCount lesen → kopieren → tickCount
//! erneut prüfen) lebt in `wait_for_frame()`.

use crate::error::{BridgeError, Result};
use crate::iracing_sdk::var_header::VarIndex;

pub struct IRacingClient {
    // TODO: HANDLE to file mapping, pointer to mapped view, Event-Handle,
    // zuletzt geparster Header, VarIndex, lokaler Frame-Puffer.
    _private: (),
}

impl IRacingClient {
    /// Öffnet das MMF `Local\IRSDKMemMapFileName` und das DataValidEvent.
    /// Gibt Fehler zurück, wenn iRacing nicht läuft oder MMF noch nicht erstellt.
    pub fn connect() -> Result<Self> {
        #[cfg(windows)]
        {
            // TODO: OpenFileMappingW + MapViewOfFile, OpenEventW für DataValidEvent
            Err(BridgeError::SdkNotConnected(
                "connect() not yet implemented".into(),
            ))
        }
        #[cfg(not(windows))]
        {
            Err(BridgeError::SdkNotConnected(
                "iRacing SDK only available on Windows".into(),
            ))
        }
    }

    /// Wartet auf neues Frame (bis `timeout_ms`), kopiert den aktuellen
    /// Variable-Buffer in einen lokalen Puffer und verifiziert tickCount.
    pub fn wait_for_frame(&mut self, _timeout_ms: u32) -> Result<i32> {
        todo!("WaitForSingleObject + triple-buffer copy + tickCount re-check")
    }

    /// Gibt den nach `connect()` aufgebauten Variable-Index zurück.
    pub fn var_index(&self) -> &VarIndex {
        todo!("return cached var index")
    }

    /// Liest einen f32-Scalar anhand des Variable-Namens.
    pub fn get_f32(&self, _name: &str) -> Result<f32> {
        todo!("lookup in var_index, read f32 at offset from local frame buffer")
    }
}

impl Drop for IRacingClient {
    fn drop(&mut self) {
        // TODO: UnmapViewOfFile, CloseHandle für MMF + Event.
    }
}
