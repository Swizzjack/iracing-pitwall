# Task: Parse variable-header array, build var_index

After Task 1, `IRacingClient::connect()` opens the MMF, parses the
top-level header, and opens the DataValidEvent. The `var_index` field
is currently an empty `HashMap`. This task populates it.

## IMPORTANT — work efficiently

The API is rate-limited (40 RPM). Minimize tool calls. Do NOT use
`sed`-based partial reads. If you need to see a file, read it once in
full. Batch your edits; run verification gates at the end, not between
changes.

## Goal

1. Implement `var_header::parse_var_index()`: given the bytes at
   `header.var_header_offset` (length `num_vars * 144`), return a
   `HashMap<String, VarDescriptor>` keyed by variable name.

2. Call `parse_var_index()` from `IRacingClient::connect()` and store
   the result in `self.var_index`.

3. Implement `IRacingClient::var_index()` getter (currently `todo!()`).

4. In `main.rs`, after successful connect, check for three known
   telemetry variables (`Speed`, `Throttle`, `SessionTime`). For each,
   log whether it was found and, if yes, its `VarType`, `offset`, and
   `count`. This is the acceptance test.

## Binary layout of `irsdk_varHeader` (144 bytes per record)

Source: iRacing's `irsdk_defines.h` — canonical, do not guess.

| Offset | Size | Type                | Field           | Notes |
|--------|------|---------------------|-----------------|-------|
| 0      | 4    | i32                 | type            | VarType enum (0..=5) |
| 4      | 4    | i32                 | offset          | byte offset into frame buffer |
| 8      | 4    | i32                 | count           | 1 = scalar, N = array |
| 12     | 1    | bool                | countAsTime     | (not stored in VarDescriptor — ignore) |
| 13     | 3    | u8[3]               | pad             | alignment padding, ignore |
| 16     | 32   | u8[32]              | name            | null-terminated ASCII |
| 48     | 64   | u8[64]              | desc            | null-terminated ASCII |
| 112    | 32   | u8[32]              | unit            | null-terminated ASCII |
| 144    |      |                     | (end of record) | |

The `VAR_HEADER_SIZE`, `IRSDK_MAX_STRING`, `IRSDK_MAX_DESC` constants
are already defined in `var_header.rs` — use them.

## Implementation guidance

### `parse_var_index(raw: &[u8], num_vars: usize) -> Result<VarIndex>`

Current signature returns `VarIndex` (not `Result<VarIndex>`). Change
it to `Result<VarIndex>` so we can surface parse errors. Update
`BridgeError` if needed: a new variant like `VarHeaderParse(String)`
is appropriate, but `SdkRead(String)` also fits — use `SdkRead` to
avoid adding a new variant unless necessary.

Validate:
- `raw.len() >= num_vars * VAR_HEADER_SIZE` — else error
- For each record: `VarType::from_i32(type_bytes)` must return `Some` — else error

For each record:
1. Read 4 bytes as i32 for type, offset, count (use `i32::from_le_bytes`
   on arrays of 4 bytes — little-endian is correct on Windows x86_64).
2. Extract name: bytes[16..48], trim at first null byte, convert via
   `std::str::from_utf8(...).map_err(...)`. If empty after trim: skip
   this record (iRacing sometimes emits padding records). Duplicate
   names: keep the first, log a `warn!` for the second. This matches
   how C-based SDK clients behave.
3. Same for desc (bytes[48..112]) and unit (bytes[112..144]).
4. Construct `VarDescriptor` and insert into the HashMap.

Helper you may want: a small private `fn cstr_from_bytes(b: &[u8]) -> Result<String>`
that finds the first null and UTF-8-decodes the prefix.

### `connect()` integration

Right after the version check passes (before opening the event) or
right after the event is opened — either order works — read the
variable-header slice:

```rust
// SAFETY: view_ptr + var_header_offset is within the mapped view;
// num_vars * VAR_HEADER_SIZE bytes are guaranteed by the SDK layout.
let var_header_slice = unsafe {
    std::slice::from_raw_parts(
        (view_ptr.Value as *const u8).add(header.var_header_offset as usize),
        (header.num_vars as usize) * VAR_HEADER_SIZE,
    )
};
let var_index = parse_var_index(var_header_slice, header.num_vars as usize)?;
```

On error, clean up the view + mmf handle + (if already opened) event
handle before returning, same pattern as the existing partial-cleanup.

### `var_index()` getter

Currently:
```rust
pub fn var_index(&self) -> &VarIndex {
    todo!("return cached var index")
}
```

Replace with one-line body: `&self.var_index`.

### `main.rs` acceptance test

After the existing `log::info!("Header: ...")` block, add:

```rust
let var_index = client.var_index();
log::info!("var_index contains {} entries", var_index.len());

for name in ["Speed", "Throttle", "SessionTime"] {
    match var_index.get(name) {
        Some(v) => log::info!(
            "  {}: type={:?} offset={} count={} unit={:?}",
            name, v.var_type, v.offset, v.count, v.unit
        ),
        None => log::warn!("  {}: NOT FOUND", name),
    }
}
```

Do NOT add imports if `log::info!` and `log::warn!` are already usable
in `main.rs` (they should be via the existing `use` statements).

## What is NOT in this task

- `wait_for_frame()` — remains `todo!()`
- `get_f32()` — remains `todo!()`
- Unit tests — not required
- Changes to `Cargo.toml` — if you think you need to, STOP and report
- Changes to any file outside `var_header.rs`, `reader.rs`, `main.rs`,
  and `error.rs` (only if adding a new error variant — prefer using
  existing `SdkRead` instead)

## Constraints

- Every `unsafe` block must have a `// SAFETY:` comment
- No `.unwrap()`, no `.expect()`, no `panic!()` in non-test code
- No `#[allow(...)]` without justification comment
- Prefer `?` for error propagation
- Names like `IRSDK_MAX_STRING` are constants in `var_header.rs` — use them
- `VAR_HEADER_SIZE` is 144 — use the constant, not a literal

## Verification gates

Run ALL of these after ALL your changes are complete:
cd /work/bridge
cargo check
cargo fmt --check
cargo clippy -- -D warnings
cargo check --target x86_64-pc-windows-gnu

All must pass with zero warnings.

## When finished

Summarize per file what changed. Do not commit — the user reviews and
commits. The user will then build the Windows .exe and live-test
against a running iRacing session.
