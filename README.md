# iRacing Pitwall v2

Real-time telemetry dashboard for iRacing.

## Stack
- **bridge/** — Rust, reads iRacing SDK shared memory (MMF: `Local\IRSDKMemMapFileName`), serves WebSocket
- **dashboard/** — React + TypeScript + Vite frontend
- **shared/** — Shared TypeScript types

## Build
Cross-compile bridge for Windows from Linux:
