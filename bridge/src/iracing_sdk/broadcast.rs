//! Broadcast-Messages: Pit, Camera, Replay Commands.
//!
//! Implementation deferred. Wenn Phase-2 kommt:
//! - RegisterWindowMessageW(r"IRSDK_BROADCASTMSG") → message id
//! - PostMessageW(HWND_BROADCAST, msg_id, wParam, lParam)
//!   wParam = (BroadcastType & 0xFFFF) | ((var1 as u32) << 16)
//!   lParam = (var2 as u32) | ((var3 as u32) << 16)

#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum BroadcastMsg {
    CamSwitchPos = 0,
    CamSwitchNum = 1,
    PitCommand = 9,
    ReplaySearchSessionTime = 12,
}
