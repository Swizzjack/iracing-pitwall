//! Top-level irsdk_header layout (offset 0 in the MMF).
//!
//! Source: irsdk_defines.h (iRacing.com Motorsport Simulations, 2013).
//! Padding and alignment are kept via `#[repr(C)]` + packed to match
//! the C layout bit-for-bit.

use serde::Serialize;
use ts_rs::TS;

pub const IRSDK_MAX_BUFS: usize = 4;
pub const IRSDK_VER_EXPECTED: i32 = 2;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VarBuf {
    pub tick_count: i32,
    pub buf_offset: i32,
    pub _pad: [i32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub ver: i32,
    pub status: i32,
    pub tick_rate: i32,
    pub session_info_update: i32,
    pub session_info_len: i32,
    pub session_info_offset: i32,
    pub num_vars: i32,
    pub var_header_offset: i32,
    pub num_buf: i32,
    pub buf_len: i32,
    pub _pad1: [i32; 2],
    pub var_buf: [VarBuf; IRSDK_MAX_BUFS],
}

impl Header {
    pub fn is_connected(&self) -> bool {
        (self.status & 0x01) != 0
    }
}

/// Flattened diagnostic view for the dashboard (debug overlay).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../shared/")]
#[serde(rename_all = "camelCase")]
pub struct HeaderStatus {
    pub connected: bool,
    pub ver: i32,
    pub tick_rate: i32,
    pub num_vars: i32,
    pub buf_len: i32,
    pub num_buf: i32,
    pub session_info_update: i32,
}

impl HeaderStatus {
    pub fn from_header(h: &Header) -> Self {
        Self {
            connected: h.is_connected(),
            ver: h.ver,
            tick_rate: h.tick_rate,
            num_vars: h.num_vars,
            buf_len: h.buf_len,
            num_buf: h.num_buf,
            session_info_update: h.session_info_update,
        }
    }
}
