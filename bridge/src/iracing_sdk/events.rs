//! Windows-Event-Primitive für DataValidEvent.
//!
//! Kapselt `WaitForSingleObject` + Timeout-Semantik, damit `reader.rs`
//! plattform-agnostische Signatur hat.

use crate::error::Result;

#[cfg(windows)]
pub fn wait_signaled(_event_handle: isize, _timeout_ms: u32) -> Result<bool> {
    todo!("WaitForSingleObject, map WAIT_OBJECT_0 / WAIT_TIMEOUT / WAIT_FAILED")
}

#[cfg(not(windows))]
pub fn wait_signaled(_event_handle: isize, _timeout_ms: u32) -> Result<bool> {
    use crate::error::BridgeError;
    Err(BridgeError::SdkNotConnected(
        "Windows events unavailable on this platform".into(),
    ))
}
