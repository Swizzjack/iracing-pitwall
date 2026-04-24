# Task: Review fixes on IRacingClient::connect()

Previous implementation (current state on branch) has three issues that
must be corrected. All three are small, surgical edits.

## IMPORTANT — work efficiently

This model is on a rate-limited free tier (40 RPM). Minimize tool calls.
Do NOT use `sed`-based partial reads before editing; read the whole file
once if you need to, and do edits in as few calls as possible.

## Reference snippets — current state of the files

### `bridge/Cargo.toml` — relevant block (lines 23-31)

```toml
[target.'cfg(windows)'.dependencies.windows-sys]
version = "0.59"
features = [
    "Win32_Foundation",
    "Win32_System_Memory",
    "Win32_System_Threading",
    "Win32_UI_WindowsAndMessaging",
]
```

### `bridge/src/iracing_sdk/reader.rs` — relevant block near top

```rust
#[cfg(windows)]
use windows_sys::Win32::System::Threading::OpenEventW;

/// SYNCHRONIZE access right (0x00100000) - required for WaitForSingleObject.
/// Defined here as it's part of Win32_Security feature which isn't enabled.
#[cfg(windows)]
const SYNCHRONIZE: u32 = 0x00100000;
```

### `bridge/src/iracing_sdk/reader.rs` — struct definition

```rust
pub struct IRacingClient {
    /// Handle to the memory-mapped file (MMF)
    mmf_handle: Handle,
    /// Pointer to the mapped view (immutable)
    view_ptr: ViewPtr,
    /// Size of the mapped view
    view_size: usize,
    /// Handle to the DataValidEvent
    event_handle: Handle,
    /// Parsed header from the MMF
    header: Header,
    /// Variable name → descriptor index
    var_index: VarIndex,
}
```

### `bridge/src/iracing_sdk/reader.rs` — struct initialization in connect()

```rust
// Construct the client with all handles and data
Ok(Self {
    mmf_handle,
    view_ptr: view_ptr.Value as ViewPtr,
    view_size: header.buf_len as usize * IRSDK_MAX_BUFS,
    event_handle,
    header,
    var_index: HashMap::new(),
})
```

### `bridge/src/iracing_sdk/reader.rs` — event_handle initialization

```rust
// SAFETY: Calling OpenEventW with valid UTF-16 string that is null-terminated.
// SYNCHRONIZE is the correct access mode for WaitForSingleObject.
let event_handle = unsafe { OpenEventW(SYNCHRONIZE, 0, event_name.as_ptr()) as HANDLE };
```

### `bridge/src/iracing_sdk/reader.rs` — `#[cfg(windows)]` use statements

```rust
#[cfg(windows)]
use crate::iracing_sdk::header::{IRSDK_MAX_BUFS, IRSDK_VER_EXPECTED};
```

Note: `IRSDK_MAX_BUFS` is only used in the `view_size` calculation that
Issue 2 removes. After Issue 2 is fixed, that import will become unused
and must be removed from the use statement, leaving only `IRSDK_VER_EXPECTED`.

---

## Issue 1 — `SYNCHRONIZE` constant was hardcoded

The original task explicitly stated: "If you discover a function that
needs another feature, STOP and report — do not modify Cargo.toml without
confirmation."

Instead, the constant was hardcoded. The value is correct, but the pattern
is wrong. Feature flag changes always require confirmation before being
made silently.

### Fix for Issue 1

1. In `bridge/Cargo.toml`, add `"Win32_Security"` to the features list
   of the `windows-sys` target dependency. Insert it alphabetically
   between `"Win32_Foundation"` and `"Win32_System_Memory"`.

2. In `bridge/src/iracing_sdk/reader.rs`:
   - Remove the `const SYNCHRONIZE: u32 = 0x00100000;` declaration
     (and its doc comment about Win32_Security).
   - Import `SYNCHRONIZE` from `windows_sys::Win32::System::Threading`.
     (verified path; adjust only if compilation actually fails).

---

## Issue 2 — `view_size` field stores the wrong quantity

`header.buf_len` is the size of ONE variable-buffer row. The MMF contains
much more than the rotating buffers (header, var-header array, YAML). The
current multiplication does not describe the view size.

No current code reads `view_size`, so this is latent. But the stored
value is misleading and must be removed.

### Fix for Issue 2

1. Remove the `view_size: usize,` line from the `IRacingClient` struct.
2. Remove the `view_size: header.buf_len as usize * IRSDK_MAX_BUFS,`
   line from the struct initialization in `connect()`.
3. Since `IRSDK_MAX_BUFS` is no longer used after this removal, update
   the corresponding `use` statement to import only `IRSDK_VER_EXPECTED`.
   (If that leaves no items, remove the entire `use` line.)

---

## Issue 3 — Possibly unnecessary cast on `OpenEventW`

In `windows-sys` 0.59, `OpenEventW` already returns `HANDLE`. The
`as HANDLE` cast is likely redundant.

### Fix for Issue 3

1. Remove ` as HANDLE` from the `let event_handle = ...` line.
   The line becomes:
   `let event_handle = unsafe { OpenEventW(SYNCHRONIZE, 0, event_name.as_ptr()) };`
2. Run `cargo check`. If it passes, leave it removed. If it fails with a
   type mismatch, restore the cast and add a comment immediately above
   explaining what type was returned and why the cast was needed.

---

## Scope — NOT in this task

- Do not touch `wait_for_frame()`, `var_index()`, `get_f32()` — they
  remain `todo!()`.
- Do not add features other than `Win32_Security`.
- Do not refactor anything else.
- Do not modify `main.rs`.

## Meta-note on the rule that was broken

The rule "no feature flag changes without confirmation" applies to EVERY
task, not just the first one where it is explicitly written. Please
treat Cargo.toml's dependency surface as off-limits by default. This
applies going forward for all future tasks.

## Verification gates — run after ALL changes, not between

Batch your edits, THEN run the gates. Each unnecessary tool call costs
rate-limit budget.
cd /work/bridge
cargo check
cargo fmt --check
cargo clippy -- -D warnings
cargo check --target x86_64-pc-windows-gnu

All must pass. If any fail, fix and re-run before declaring done.

## When finished

Summarize what was changed per file. Do not commit — the user commits.
