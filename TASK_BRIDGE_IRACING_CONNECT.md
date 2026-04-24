# Task: Implement `IRacingClient::connect()` + `Drop`

## Goal

Implement the Windows-only connection logic for the iRacing shared-memory
reader in `bridge/src/iracing_sdk/reader.rs`. After this task, calling
`IRacingClient::connect()` while iRacing is running must return `Ok(client)`
and populate the internal state; when iRacing is not running, it must return
`Err(BridgeError::SdkNotConnected(...))` without panicking.

## Scope — what IS in this task

1. `IRacingClient` struct: real fields (MMF handle, view pointer, view size,
   event handle, parsed `Header`, placeholder for `VarIndex`).
2. `IRacingClient::connect()`:
   - Open the memory-mapped file `Local\IRSDKMemMapFileName` via
     `OpenFileMappingW` (read-only: `FILE_MAP_READ`).
   - Map the view via `MapViewOfFile`. Size: use 0 for the length parameter
     to map the entire file (iRacing sets this internally; do NOT hardcode).
   - Copy the top-level header from the start of the view into a local
     `Header` struct using `std::ptr::read_unaligned` (the struct is
     `#[repr(C)]` — alignment is not guaranteed through a mapped view).
   - Verify `header.ver == IRSDK_VER_EXPECTED` (= 2). On mismatch, return
     `SdkNotConnected` with a clear message like "unexpected SDK version N,
     expected 2".
   - Open the event `Local\IRSDKDataValidEvent` via `OpenEventW`
     (access: `SYNCHRONIZE`, inherit: false).
   - Store all handles + parsed header + view pointer + view size in the
     struct fields.
   - Leave `VarIndex` as an empty `HashMap` for now — the next task will
     parse the variable header array.
3. `Drop for IRacingClient`:
   - `UnmapViewOfFile` on the view pointer.
   - `CloseHandle` on MMF handle and event handle.
   - Ignore errors from these (standard Drop pattern) but log via
     `log::warn!` if a handle was null when it shouldn't be.
4. Non-Windows targets: keep the existing `cfg(not(windows))` branch
   returning `SdkNotConnected("iRacing SDK only available on Windows")`.
5. `main.rs` verdrahten:
   - Replace the `todo!("wire up reader + ws server")` at the end of `main()`
     with: call `IRacingClient::connect()`, log success/failure, and
     for now just `log::info!` the parsed header fields (ver, tick_rate,
     num_vars, buf_len, status connected-flag).
   - Do NOT start the WS server yet (that's a later task).
   - If connect fails on non-Windows: log a warning and exit 0 (normal in dev).

## Scope — what is NOT in this task

- `wait_for_frame()` — leave as `todo!()`
- `var_index()` and `get_f32()` — leave as `todo!()`
- Parsing the variable-header array — that's the next task
- YAML parsing — that's a later task
- Starting the WebSocket server

## Constraints

- All Windows API calls must use `windows-sys::Win32::*`. No other Windows
  binding crates.
- Every `unsafe` block must have a `// SAFETY:` comment directly above it
  explaining the invariant (see AGENTS.md).
- Convert Rust `&str` paths to wide-char (UTF-16) for W-suffix APIs. Helper
  pattern: `let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();`
- The MMF name and event name are raw strings: `r"Local\IRSDKMemMapFileName"`
  and `r"Local\IRSDKDataValidEvent"`.
- No `.unwrap()`, no `.expect()`, no `panic!` in the connect path. Use `?`
  with `BridgeError` conversions.
- Handles from `OpenFileMappingW` / `OpenEventW` return 0 on failure; check
  for that before using them.

## Required `windows-sys` features

The `Cargo.toml` already enables:
- `Win32_Foundation` — `HANDLE`, `CloseHandle`, `GetLastError`
- `Win32_System_Memory` — `OpenFileMappingW`, `MapViewOfFile`,
  `UnmapViewOfFile`, `FILE_MAP_READ`
- `Win32_System_Threading` — `OpenEventW`, `SYNCHRONIZE`

If you discover a function that needs another feature, STOP and report —
do not modify `Cargo.toml` without confirmation.

## Files to modify

- `bridge/src/iracing_sdk/reader.rs` — most of the work
- `bridge/src/main.rs` — remove the `todo!()`, wire up the connect call
- No other files

If you feel you need to touch another file, STOP and explain why.

## Verification before declaring done

Run ALL of these in order. Every one must pass:
cd /work/bridge
cargo check
cargo fmt --check
cargo clippy -- -D warnings

Additionally:
- Confirm that `cargo check --target x86_64-pc-windows-gnu` reports no
  `windows-sys` API errors (it may fail at link time — that's fine for
  `check`, but compilation errors are not acceptable).

## Notes

- Testing this end-to-end requires iRacing running on Windows, which is not
  possible in this container. Correctness is verified by: compile gates pass,
  code review by the user, then a Windows build + live test by the user.
- If you are unsure about a specific API signature, read the windows-sys
  docs rather than guessing. The crate is fully documented on docs.rs.
- The original C header with canonical struct layouts is at:
  https://github.com/vipoo/irsdk/blob/master/irsdk_defines.h
  but you should already have enough context from `header.rs` — do not
  fetch external URLs.
