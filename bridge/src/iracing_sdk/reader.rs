//! MMF-basierter Shared-Memory-Reader.
//!
//! Lifecycle:
//! 1. `IRacingClient::connect()` öffnet MMF + DataValidEvent
//! 2. `wait_for_frame()` blockiert auf Event, kopiert aktuellen varBuf
//! 3. `get_*()` liefert Werte aus dem lokalen Puffer
//!
//! Die Triple-Buffer-Strategie (tickCount lesen → kopieren → tickCount
//! erneut prüfen) lebt in `wait_for_frame()`.

use crate::error::{BridgeError, Result};
use crate::iracing_sdk::header::Header;
use crate::iracing_sdk::var_header::VarIndex;
#[cfg(windows)]
use crate::iracing_sdk::var_header::VAR_HEADER_SIZE;

#[cfg(windows)]
use crate::iracing_sdk::header::IRSDK_VER_EXPECTED;

#[cfg(windows)]
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
#[cfg(windows)]
use windows_sys::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingW, UnmapViewOfFile, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS,
};
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{OpenEventW, SYNCHRONIZATION_SYNCHRONIZE};

/// Platform-agnostic handle type
#[cfg(windows)]
type Handle = HANDLE;
#[cfg(not(windows))]
type Handle = *mut ();

/// Platform-agnostic view pointer
#[cfg(windows)]
type ViewPtr = *const u8;
#[cfg(not(windows))]
type ViewPtr = *const u8;

pub struct IRacingClient {
    /// Handle to the memory-mapped file (MMF)
    mmf_handle: Handle,
    /// Pointer to the mapped view (immutable)
    view_ptr: ViewPtr,
    /// Handle to the DataValidEvent
    event_handle: Handle,
    /// Parsed header from the MMF
    header: Header,
    /// Variable name → descriptor index
    var_index: VarIndex,
}

impl IRacingClient {
    /// Öffnet das MMF `Local\IRSDKMemMapFileName` und das DataValidEvent.
    /// Gibt Fehler zurück, wenn iRacing nicht läuft oder MMF noch nicht erstellt.
    pub fn connect() -> Result<Self> {
        #[cfg(windows)]
        {
            // Open memory-mapped file
            let mmf_name: Vec<u16> = r"Local\IRSDKMemMapFileName"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            // SAFETY: Calling OpenFileMappingW with valid UTF-16 string that is
            // null-terminated. FILE_MAP_READ is the correct access mode.
            let mmf_handle = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, mmf_name.as_ptr()) };
            if mmf_handle.is_null() {
                return Err(BridgeError::SdkNotConnected(
                    "iRacing not running or MMF not available".into(),
                ));
            }

            // Map the view with size 0 to map the entire file
            // SAFETY: mmf_handle is valid and non-null. Size 0 maps the entire file
            // as per iRacing's internal mapping.
            let view_ptr: MEMORY_MAPPED_VIEW_ADDRESS =
                unsafe { MapViewOfFile(mmf_handle, FILE_MAP_READ, 0, 0, 0) };
            if view_ptr.Value.is_null() {
                // SAFETY: CloseHandle is safe to call on a valid handle
                unsafe {
                    CloseHandle(mmf_handle);
                }
                return Err(BridgeError::SdkNotConnected(
                    "failed to map view of MMF".into(),
                ));
            }

            // Read the header from the start of the mapped view
            // SAFETY: The view is valid and iRacing places the Header at offset 0.
            // We use read_unaligned because the mapped view may not have proper alignment.
            let header: Header =
                unsafe { std::ptr::read_unaligned(view_ptr.Value as *const Header) };

            // Verify SDK version
            if header.ver != IRSDK_VER_EXPECTED {
                // Unmap view before returning error
                // SAFETY: view_ptr is valid
                unsafe {
                    UnmapViewOfFile(view_ptr);
                    CloseHandle(mmf_handle);
                }
                return Err(BridgeError::SdkNotConnected(format!(
                    "unexpected SDK version {}, expected {}",
                    header.ver, IRSDK_VER_EXPECTED
                )));
            }

            // Open the DataValidEvent
            let event_name: Vec<u16> = r"Local\IRSDKDataValidEvent"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            // SAFETY: Calling OpenEventW with valid UTF-16 string that is null-terminated.
            // SYNCHRONIZE is the correct access mode for WaitForSingleObject.
            let event_handle =
                unsafe { OpenEventW(SYNCHRONIZATION_SYNCHRONIZE, 0, event_name.as_ptr()) };

            if event_handle.is_null() {
                // Cleanup on failure
                // SAFETY: view_ptr and mmf_handle are valid
                unsafe {
                    UnmapViewOfFile(view_ptr);
                    CloseHandle(mmf_handle);
                }
                return Err(BridgeError::SdkNotConnected(
                    "failed to open DataValidEvent".into(),
                ));
            }

            // Parse variable header array
            // SAFETY: view_ptr is valid and header values are from SDK layout.
            // var_header_offset and num_vars are guaranteed by the SDK to point
            // to valid memory within the MMF view.
            let var_header_slice = unsafe {
                std::slice::from_raw_parts(
                    (view_ptr.Value as *const u8).add(header.var_header_offset as usize),
                    (header.num_vars as usize) * VAR_HEADER_SIZE,
                )
            };
            let var_index = match crate::iracing_sdk::var_header::parse_var_index(
                var_header_slice,
                header.num_vars as usize,
            ) {
                Ok(vi) => vi,
                Err(e) => {
                    // Cleanup on parse failure: mmf_handle, view, event_handle all open
                    // SAFETY: all three handles are valid at this point
                    unsafe {
                        UnmapViewOfFile(view_ptr);
                        CloseHandle(mmf_handle);
                        CloseHandle(event_handle);
                    }
                    return Err(e);
                }
            };

            // Construct the client with all handles and data
            Ok(Self {
                mmf_handle,
                view_ptr: view_ptr.Value as ViewPtr,
                event_handle,
                header,
                var_index,
            })
        }
        #[cfg(not(windows))]
        {
            Err(BridgeError::SdkNotConnected(
                "iRacing SDK only available on Windows".into(),
            ))
        }
    }

    /// Returns a reference to the parsed header.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Wartet auf neues Frame (bis `timeout_ms`), kopiert den aktuellen
    /// Variable-Buffer in einen lokalen Puffer und verifiziert tickCount.
    pub fn wait_for_frame(&mut self, _timeout_ms: u32) -> Result<i32> {
        todo!("WaitForSingleObject + triple-buffer copy + tickCount re-check")
    }

    /// Gibt den nach `connect()` aufgebauten Variable-Index zurück.
    pub fn var_index(&self) -> &VarIndex {
        &self.var_index
    }

    /// Liest einen f32-Scalar anhand des Variable-Namens.
    pub fn get_f32(&self, _name: &str) -> Result<f32> {
        todo!("lookup in var_index, read f32 at offset from local frame buffer")
    }
}

impl Drop for IRacingClient {
    #[cfg(windows)]
    fn drop(&mut self) {
        // SAFETY: view_ptr is valid if non-null
        if !self.view_ptr.is_null() {
            let view_addr = MEMORY_MAPPED_VIEW_ADDRESS {
                Value: self.view_ptr as *mut _,
            };
            unsafe {
                UnmapViewOfFile(view_addr);
            }
        } else {
            log::warn!("IRacingClient::drop: view_ptr was null, cannot unmap");
        }
        // SAFETY: mmf_handle is valid if non-null
        if !self.mmf_handle.is_null() {
            unsafe {
                CloseHandle(self.mmf_handle);
            }
        } else {
            log::warn!("IRacingClient::drop: mmf_handle was null, cannot close");
        }
        // SAFETY: event_handle is valid if non-null
        if !self.event_handle.is_null() {
            unsafe {
                CloseHandle(self.event_handle);
            }
        } else {
            log::warn!("IRacingClient::drop: event_handle was null, cannot close");
        }
    }

    #[cfg(not(windows))]
    fn drop(&mut self) {
        // No-op on non-Windows
    }
}
