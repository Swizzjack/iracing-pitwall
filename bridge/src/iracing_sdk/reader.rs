//! MMF-basierter Shared-Memory-Reader.
//!
//! Lifecycle:
//! 1. `IRacingClient::connect()` öffnet MMF + DataValidEvent, baut var_index auf
//! 2. `wait_for_frame()` blockiert auf Event, kopiert aktuellen varBuf (Triple-Buffer)
//! 3. `get_*()` liefert Werte aus dem lokalen Puffer

use crate::error::{BridgeError, Result};
use crate::iracing_sdk::header::Header;
use crate::iracing_sdk::var_header::{VarIndex, VarType};

#[cfg(windows)]
use crate::iracing_sdk::header::IRSDK_VER_EXPECTED;
#[cfg(windows)]
use crate::iracing_sdk::var_header::VAR_HEADER_SIZE;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
#[cfg(windows)]
use windows_sys::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{
    OpenEventW, WaitForSingleObject, INFINITE, SYNCHRONIZATION_SYNCHRONIZE,
};

#[cfg(windows)]
type Handle = HANDLE;
#[cfg(not(windows))]
type Handle = *mut ();

pub struct IRacingClient {
    mmf_handle: Handle,
    /// Base pointer of the mapped view (cast from MEMORY_MAPPED_VIEW_ADDRESS.Value)
    view_ptr: *const u8,
    event_handle: Handle,
    header: Header,
    var_index: VarIndex,
    /// Local copy of the current frame buffer (triple-buffer snapshot)
    frame: Vec<u8>,
}

// SAFETY: IRacingClient holds Windows handles and a raw pointer. These are only used
// from the thread that owns the client and are never shared concurrently.
unsafe impl Send for IRacingClient {}

impl IRacingClient {
    /// Öffnet `Local\IRSDKMemMapFileName` und `Local\IRSDKDataValidEvent`.
    /// Parst sofort den var_index (kein separater Aufruf nötig).
    pub fn connect() -> Result<Self> {
        #[cfg(windows)]
        {
            let mmf_name: Vec<u16> = r"Local\IRSDKMemMapFileName"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            // SAFETY: mmf_name is a valid null-terminated UTF-16 string.
            let mmf_handle = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, mmf_name.as_ptr()) };
            if mmf_handle.is_null() {
                return Err(BridgeError::SdkNotConnected(
                    "iRacing not running or MMF not available".into(),
                ));
            }

            // SAFETY: mmf_handle is valid. Size 0 maps the full file.
            let view: MEMORY_MAPPED_VIEW_ADDRESS =
                unsafe { MapViewOfFile(mmf_handle, FILE_MAP_READ, 0, 0, 0) };
            if view.Value.is_null() {
                // SAFETY: mmf_handle is valid.
                unsafe { CloseHandle(mmf_handle) };
                return Err(BridgeError::SdkNotConnected(
                    "failed to map view of MMF".into(),
                ));
            }
            let view_ptr = view.Value as *const u8;

            // SAFETY: view is valid; Header is at offset 0 per iRacing SDK layout.
            let header: Header = unsafe { std::ptr::read_unaligned(view_ptr as *const Header) };

            if header.ver != IRSDK_VER_EXPECTED {
                // SAFETY: both handles are valid.
                unsafe {
                    UnmapViewOfFile(view);
                    CloseHandle(mmf_handle);
                }
                return Err(BridgeError::SdkNotConnected(format!(
                    "unexpected SDK version {}, expected {}",
                    header.ver, IRSDK_VER_EXPECTED
                )));
            }

            let event_name: Vec<u16> = r"Local\IRSDKDataValidEvent"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            // SAFETY: event_name is a valid null-terminated UTF-16 string.
            let event_handle =
                unsafe { OpenEventW(SYNCHRONIZATION_SYNCHRONIZE, 0, event_name.as_ptr()) };
            if event_handle.is_null() {
                // SAFETY: both handles are valid.
                unsafe {
                    UnmapViewOfFile(view);
                    CloseHandle(mmf_handle);
                }
                return Err(BridgeError::SdkNotConnected(
                    "failed to open DataValidEvent".into(),
                ));
            }

            // SAFETY: view_ptr is valid; var_header_offset and num_vars come from
            // the SDK header and point to valid memory within the mapped view.
            let var_header_slice = unsafe {
                std::slice::from_raw_parts(
                    view_ptr.add(header.var_header_offset as usize),
                    (header.num_vars as usize) * VAR_HEADER_SIZE,
                )
            };
            let var_index = match crate::iracing_sdk::var_header::parse_var_index(
                var_header_slice,
                header.num_vars as usize,
            ) {
                Ok(vi) => vi,
                Err(e) => {
                    // SAFETY: all three handles are valid at this point.
                    unsafe {
                        UnmapViewOfFile(view);
                        CloseHandle(event_handle);
                        CloseHandle(mmf_handle);
                    }
                    return Err(e);
                }
            };

            Ok(Self {
                mmf_handle,
                view_ptr,
                event_handle,
                header,
                var_index,
                frame: Vec::with_capacity(header.buf_len as usize),
            })
        }
        #[cfg(not(windows))]
        {
            Err(BridgeError::SdkNotConnected(
                "iRacing SDK only available on Windows".into(),
            ))
        }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Raw YAML slice from the MMF (ISO-8859-1, NUL-padded tail).
    /// Valid until the next `wait_for_frame()` call.
    pub fn session_info_bytes(&self) -> &[u8] {
        #[cfg(windows)]
        {
            let off = self.header.session_info_offset as usize;
            let len = self.header.session_info_len as usize;
            // SAFETY: view_ptr + (off..off+len) is within the MMF as guaranteed
            // by the iRacing SDK layout. Lifetied to &self, which keeps the
            // mapping alive (UnmapViewOfFile only in Drop).
            unsafe { std::slice::from_raw_parts(self.view_ptr.add(off), len) }
        }
        #[cfg(not(windows))]
        {
            &[]
        }
    }

    /// Update counter that increments when iRacing rewrites the YAML.
    /// Sourced from the header snapshot; refreshed by each `wait_for_frame()`.
    pub fn session_info_update(&self) -> i32 {
        self.header.session_info_update
    }

    pub fn var_index(&self) -> &VarIndex {
        &self.var_index
    }

    /// No-op: var_index is already built during `connect()`. Kept for API symmetry.
    pub fn parse_var_index(&mut self) -> Result<()> {
        Ok(())
    }

    /// Blocks until iRacing signals a new frame, then snapshots it via the
    /// triple-buffer protocol. Result stored in `self.frame`.
    pub fn wait_for_frame(&mut self) -> Result<()> {
        #[cfg(windows)]
        {
            // Step 1: Block until iRacing signals the data-valid event.
            // SAFETY: event_handle is a valid auto-reset event handle.
            let wait_result = unsafe { WaitForSingleObject(self.event_handle, INFINITE) };
            if wait_result != WAIT_OBJECT_0 {
                return Err(BridgeError::SdkRead(format!(
                    "WaitForSingleObject failed: {wait_result:#010x}"
                )));
            }

            // Steps 2–4: Triple-buffer read with up to 5 retry attempts.
            for attempt in 0..5_u32 {
                // Re-read header from MMF to get current VarBuf state.
                // SAFETY: view_ptr is valid; Header is at offset 0.
                let hdr: Header =
                    unsafe { std::ptr::read_unaligned(self.view_ptr as *const Header) };

                let num_buf = (hdr.num_buf as usize).min(4);
                if num_buf == 0 {
                    return Err(BridgeError::SdkRead("num_buf is 0".into()));
                }

                // Pick the slot with the highest tick_count.
                let best = (0..num_buf)
                    .max_by_key(|&i| hdr.var_buf[i].tick_count)
                    .expect("num_buf > 0");

                let tick_a = hdr.var_buf[best].tick_count;
                let buf_offset = hdr.var_buf[best].buf_offset as usize;
                let buf_len = hdr.buf_len as usize;

                // Copy frame bytes into our local buffer.
                self.frame.clear();
                // SAFETY: view_ptr is valid mapped memory; buf_offset + buf_len
                // is within the MMF as guaranteed by the SDK layout.
                let src =
                    unsafe { std::slice::from_raw_parts(self.view_ptr.add(buf_offset), buf_len) };
                self.frame.extend_from_slice(src);

                // Verify tick_count hasn't changed during the copy.
                // SAFETY: view_ptr is valid.
                let hdr2: Header =
                    unsafe { std::ptr::read_unaligned(self.view_ptr as *const Header) };
                let tick_b = hdr2.var_buf[best].tick_count;

                if tick_b == tick_a {
                    self.header = hdr;
                    return Ok(());
                }
                log::trace!("wait_for_frame: tick changed on attempt {attempt}, retrying");
            }

            Err(BridgeError::SdkRead(
                "triple-buffer: tick unstable after 5 attempts".into(),
            ))
        }
        #[cfg(not(windows))]
        {
            Err(BridgeError::SdkNotConnected(
                "iRacing SDK only available on Windows".into(),
            ))
        }
    }

    // ── Typed getters ────────────────────────────────────────────────────────

    pub fn get_f32(&self, name: &str) -> Result<f32> {
        self.read::<f32>(name, VarType::Float).map(|s| s[0])
    }

    pub fn get_f64(&self, name: &str) -> Result<f64> {
        self.read::<f64>(name, VarType::Double).map(|s| s[0])
    }

    pub fn get_i32(&self, name: &str) -> Result<i32> {
        self.read::<i32>(name, VarType::Int).map(|s| s[0])
    }

    pub fn get_bool(&self, name: &str) -> Result<bool> {
        let desc = self
            .var_index
            .get(name)
            .ok_or_else(|| BridgeError::SdkRead(format!("unknown var '{name}'")))?;
        if desc.var_type != VarType::Bool {
            return Err(BridgeError::SdkRead(format!(
                "var '{name}': expected Bool, got {:?}",
                desc.var_type
            )));
        }
        let end = desc.offset + 1;
        if end > self.frame.len() {
            return Err(BridgeError::SdkRead(format!(
                "var '{name}': out of bounds (offset={} frame={})",
                desc.offset,
                self.frame.len()
            )));
        }
        Ok(self.frame[desc.offset] != 0)
    }

    pub fn get_bitfield(&self, name: &str) -> Result<u32> {
        self.read::<u32>(name, VarType::BitField).map(|s| s[0])
    }

    pub fn get_f32_array(&self, name: &str) -> Result<&[f32]> {
        self.read::<f32>(name, VarType::Float)
    }

    pub fn get_i32_array(&self, name: &str) -> Result<&[i32]> {
        self.read::<i32>(name, VarType::Int)
    }

    pub fn get_bool_array(&self, name: &str) -> Result<Vec<bool>> {
        let desc = self
            .var_index
            .get(name)
            .ok_or_else(|| BridgeError::SdkRead(format!("unknown var '{name}'")))?;
        if desc.var_type != VarType::Bool {
            return Err(BridgeError::SdkRead(format!(
                "var '{name}': expected Bool, got {:?}",
                desc.var_type
            )));
        }
        let end = desc.offset + desc.count;
        if end > self.frame.len() {
            return Err(BridgeError::SdkRead(format!(
                "var '{name}': out of bounds (offset={} count={} frame={})",
                desc.offset,
                desc.count,
                self.frame.len()
            )));
        }
        Ok(self.frame[desc.offset..end]
            .iter()
            .map(|&b| b != 0)
            .collect())
    }

    // ── Private helper ───────────────────────────────────────────────────────

    /// Returns a `&[T]` view into `self.frame` for the named variable.
    /// Verifies the stored `VarType` matches `expected`.
    fn read<T: Copy>(&self, name: &str, expected: VarType) -> Result<&[T]> {
        let desc = self
            .var_index
            .get(name)
            .ok_or_else(|| BridgeError::SdkRead(format!("unknown var '{name}'")))?;
        if desc.var_type != expected {
            return Err(BridgeError::SdkRead(format!(
                "var '{name}': type mismatch (expected {expected:?}, got {:?})",
                desc.var_type
            )));
        }
        let elem_size = std::mem::size_of::<T>();
        let byte_len = desc
            .count
            .checked_mul(elem_size)
            .ok_or_else(|| BridgeError::SdkRead(format!("var '{name}': byte length overflow")))?;
        let end = desc
            .offset
            .checked_add(byte_len)
            .ok_or_else(|| BridgeError::SdkRead(format!("var '{name}': offset overflow")))?;
        if end > self.frame.len() {
            return Err(BridgeError::SdkRead(format!(
                "var '{name}': out of bounds (offset={} len={} frame={})",
                desc.offset,
                byte_len,
                self.frame.len()
            )));
        }
        let bytes = &self.frame[desc.offset..end];
        // SAFETY: The iRacing SDK lays out all variables at naturally aligned offsets
        // (multiples of the type's size). Vec<u8> on Windows x86_64 is allocated with
        // ≥16-byte alignment by the global allocator, so bytes.as_ptr() is properly
        // aligned for any T with align ≤ 8 (all iRacing types). The slice covers
        // exactly `count` elements of size `elem_size`.
        Ok(unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const T, desc.count) })
    }
}

impl Drop for IRacingClient {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            // SAFETY: all three handles were opened successfully in connect().
            unsafe {
                let view_addr = MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.view_ptr as *mut std::ffi::c_void,
                };
                UnmapViewOfFile(view_addr);
                CloseHandle(self.event_handle);
                CloseHandle(self.mmf_handle);
            }
        }
    }
}
