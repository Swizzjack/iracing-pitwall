# iRacing Pitwall

[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![Platform: Windows](https://img.shields.io/badge/Platform-Windows-blue.svg)](#installation)
[![Built with Rust](https://img.shields.io/badge/Built%20with-Rust-orange.svg)](bridge/)
[![Built with React](https://img.shields.io/badge/Built%20with-React-61DAFB.svg)](dashboard/)

**Real-time telemetry dashboard for iRacing.**

A single self-contained `.exe` (~3.3 MB) that reads the iRacing SDK and opens a live
overlay dashboard in your browser. No installer, no background service — just run it
while iRacing is open.

---

## Screenshots

<!-- Drop screenshots/GIFs here before the public release. -->
> Screenshots will be added before the first public release.

---

## Features

**Widgets** (all draggable, resizable, individually configurable):
- **Standings** — live race/session order, gaps, incidents, pit state
- **Strength of Field** — iRating-based SoF display
- **Track Map** — animated car positions at 60 Hz with sector labels and wind compass
- **Telemetry Inputs** — throttle, brake, steering, gear
- **Lap History** — lap times, gap to leader, incident dots per lap
- **Fuel** — fuel level, laps remaining, projected usage
- **Tires** — tyre temperatures and wear per corner
- **Electronics** — brake bias, TC, ABS, engine map
- **Weather** — air/track temperature, rubber state, humidity, wind
- **Wind** — dedicated wind compass with vehicle silhouette and relative wind direction

**Dashboard UX:**
- Drag-and-drop grid layout (saved per session), resize any widget
- Per-widget settings drawers (columns, units, display options)
- Global UI scale slider — works at any screen resolution / overlay size
- **LAN access** — open the dashboard on a phone or tablet on the same network
- **Single-instance** — launching a second copy opens the browser instead
- **Auto-shutdown** — the bridge exits when all browser tabs close (configurable)
- No data leaves your machine: all traffic stays on `127.0.0.1`

---

## Installation

### Requirements

- Windows 10 / 11 (64-bit)
- iRacing client (the bridge reads its shared memory — iRacing must be running for live
  data, but the dashboard opens regardless)
- A modern browser (Chrome, Firefox, Edge)

### Steps

1. Download `iracing-pitwall.exe` from the [Releases](../../releases) page.
2. Place it anywhere (e.g. your Desktop or a `Tools\` folder).
3. Double-click to run. Your browser opens automatically at `http://127.0.0.1:8765`.
4. Start an iRacing session — telemetry appears in the dashboard automatically.
5. Close all browser tabs to shut down the bridge (or press Ctrl+C in the console).

> **Antivirus false positives:** see the [dedicated section below](#antivirus--false-positives).

### LAN access (phone / second screen)

The bridge also announces a LAN URL in its log (`http://192.168.x.x:8765`).
Open that address on any device on the same Wi-Fi network to follow the session from
a phone, tablet, or a second PC.

---

## Configuration

All settings are optional environment variables. The defaults work for most setups.

| Variable | Default | Description |
|---|---|---|
| `BRIDGE_WS_PORT` | `8765` | HTTP + WebSocket port |
| `BRIDGE_KEEP_ALIVE` | `0` | Set to `1` to prevent auto-shutdown when all tabs close |
| `BRIDGE_SHUTDOWN_GRACE_SEC` | `5` | Seconds to wait after last tab disconnects before exiting |
| `BRIDGE_STARTUP_GRACE_SEC` | `30` | Seconds to wait for the first browser connection before giving up |
| `BRIDGE_NO_BROWSER` | *(unset)* | Set to any value to suppress the automatic browser launch |
| `BRIDGE_LOG` | `info` | Log verbosity: `trace`, `debug`, `info`, `warn`, `error` |

The log file is written next to the executable as `bridge.log`.

---

## Architecture

```
iRacing sim
    │
    │  Named Shared Memory (MMF)
    │  "Local\IRSDKMemMapFileName"
    │  60 Hz, triple-buffered, event-synced
    ▼
┌─────────────────────────────────────────┐
│  Rust bridge  (iracing-pitwall.exe)      │
│                                          │
│  iracing_sdk/   — MMF reader, YAML       │
│                   session info parser    │
│                   (ISO-8859-1 encoded)   │
│  telemetry/     — snapshots, standings,  │
│                   track recorder,        │
│                   pit/sector/finish      │
│                   trackers               │
│  ws/            — axum HTTP + WebSocket  │
│                   server                 │
└───────────────┬─────────────────────────┘
                │  HTTP + WebSocket
                │  127.0.0.1:8765
                ▼
┌─────────────────────────────────────────┐
│  React dashboard  (embedded in .exe)     │
│                                          │
│  Built with Vite, served via rust-embed  │
│  Types shared with bridge via ts-rs      │
│  Layout: react-grid-layout + @dnd-kit   │
│  Charts: uplot                           │
└─────────────────────────────────────────┘
```

**Key design points:**
- The entire React app is compiled into the `.exe` via
  [`rust-embed`](https://crates.io/crates/rust-embed) — no separate files to distribute.
- Shared types between Rust and TypeScript are generated automatically by
  [`ts-rs`](https://crates.io/crates/ts-rs) (`cargo test` writes to `shared/*.ts`).
- The SDK reader uses `WaitForSingleObject` on `IRSDKDataValidEvent` for
  frame-accurate, zero-CPU-waste synchronisation.
- Session info YAML from iRacing is encoded in ISO-8859-1 and decoded before parsing.

---

## Build from Source

### Prerequisites

- **Rust** 1.80 or later (`rustup`)
- **Node.js** 18+ and `npm`

### Windows (native)

```sh
# 1. Generate shared TypeScript types from Rust structs
cargo test --manifest-path bridge/Cargo.toml

# 2. Build the React dashboard
cd dashboard
npm install
npm run build
cd ..

# 3. Build the bridge (embeds the dashboard at compile time)
cargo build --release --manifest-path bridge/Cargo.toml
# Output: bridge/target/release/iracing-pitwall.exe
```

> **Important:** step 1 must come before step 2. `shared/*.ts` is gitignored (generated
> output). If you skip `cargo test`, the TypeScript compiler will fail to resolve
> `@shared/*` imports.

### Linux → Windows cross-compile

The repo ships a `build-release.sh` script that uses
[cargo-xwin](https://github.com/rust-cross/cargo-xwin) with a Zig-backed MSVC toolchain
to produce a native `x86_64-pc-windows-msvc` binary from Linux. Additional prerequisites:

- `cargo-xwin` installed (`cargo install cargo-xwin`)
- A Zig 0.13+ installation with `zig cc`, `zig lld-link`, and `zig ar` on `PATH` (or
  wrapper scripts pointing at them)
- MSVC CRT headers/libs (downloaded once automatically by `cargo-xwin`)
- Rust target `x86_64-pc-windows-msvc` added (`rustup target add x86_64-pc-windows-msvc`)

```sh
bash build-release.sh
# Output: dist/iracing-pitwall.exe
```

The script auto-bumps the patch version in `bridge/Cargo.toml`, builds the dashboard,
cross-compiles the bridge, and copies the binary to `dist/`.

---

## Project Layout

```
iracing-pitwall/
├── bridge/                  Rust bridge (iRacing SDK reader + HTTP/WS server)
│   ├── src/
│   │   ├── iracing_sdk/     MMF reader, YAML parser, shared-memory types
│   │   ├── telemetry/       Snapshot builders, pit/sector/track trackers
│   │   └── ws/              axum HTTP + WebSocket server, lifecycle watcher
│   ├── assets/              App icon (.ico, .o COFF resource)
│   └── Cargo.toml
├── dashboard/               React + TypeScript + Vite frontend
│   └── src/
│       ├── widgets/         One file per widget component
│       ├── components/      Shared UI components (SettingsDrawer, WindCompass …)
│       ├── layout/          Grid layout, widget registry, storage helpers
│       └── ws/              WebSocket client
├── shared/                  Generated TypeScript types (do not edit by hand)
└── build-release.sh         Linux → Windows cross-compile script
```

---

## Antivirus / False Positives

Some antivirus engines may flag `iracing-pitwall.exe`. **This is a false positive.**

As of **v0.1.85**, [VirusTotal reports 2 / 71 engines](https://www.virustotal.com) flag
the binary. Both are pure ML/heuristic detections (`*!ml`, `HighConfidence`) with no
identified malicious behaviour.

### Why AV scanners flag it

The binary combines several patterns that individually trigger heuristics, even though
each has a legitimate purpose:

| Trigger | Reason | Actual behaviour |
|---|---|---|
| No Authenticode signature | Unsigned binary | No cert yet — planned post-release |
| Low download reputation | New/unknown file | Resolves naturally as downloads grow |
| Named Shared Memory access | `OpenFileMappingW` | Required by the iRacing SDK protocol |
| No console window (`windows_subsystem = "windows"`) | Common in malware | Dashboard opens a browser instead |
| Browser launch (`webbrowser::open`) | Common in downloaders | Opens `http://127.0.0.1:8765` only |

### Mitigations already in place

- **v0.1.84:** Replaced `8.8.8.8` (used for LAN-IP detection via routing table) with
  `192.0.2.1` (RFC 5737 TEST-NET, never routed on the internet) to eliminate the
  "hardcoded external IP" heuristic.
- **v0.1.85:** Switched from MinGW (`x86_64-pc-windows-gnu`) to the MSVC target
  (`x86_64-pc-windows-msvc`), eliminating MinGW-specific PE-format patterns. These two
  changes brought the count from 5 down to 2.

### What the binary does NOT do

- No network traffic outside `127.0.0.1` (plus the one-shot UDP connect to RFC 5737
  TEST-NET to read the local routing table — **no packet is sent**, the socket
  connects instantly and is closed).
- No registry writes, no autostart, no scheduled tasks, no services installed.
- No file writes other than `bridge.log` next to the `.exe`.
- All Windows API calls are read-only access to the iRacing SDK shared memory.

### Building from source

If you prefer to verify the binary yourself, build it from source (see above). The
entire codebase is in this repository.

---

## License

This project is licensed under the [MIT License](LICENSE).

---

## Acknowledgements

- This project is an independent, community-built tool.
- **iRacing** is a trademark of iRacing.com Motorsport Simulations LLC. This project
  is not affiliated with, endorsed by, or in any way associated with iRacing.com.
- The bridge reads iRacing's telemetry via the official iRacing SDK shared-memory
  interface (read-only).
