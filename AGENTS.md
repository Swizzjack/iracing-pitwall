# iRacing Pitwall v2 — Agent Rules

## Projekt-Layout
- `bridge/` — Rust (iRacing SDK Reader + WebSocket-Server)
  - Custom Shared Memory Reader via `windows-sys`
  - MMF: `Local\IRSDKMemMapFileName`, 60 Hz Telemetry
  - YAML SessionInfo Parser (ISO-8859-1 Encoding!)
- `dashboard/` — React + TypeScript + Vite
- `shared/` — Shared TypeScript Types
- Target: Windows x86_64, Cross-Compile via cargo-zigbuild

## Verifikation vor jedem Commit (PFLICHT)
- Rust-Änderungen: `cargo check --manifest-path bridge/Cargo.toml`
- TypeScript-Änderungen: `cd dashboard && npx tsc --noEmit`

## Build
- `cargo zigbuild --release --target x86_64-pc-windows-gnu --manifest-path bridge/Cargo.toml`

## VERBOTEN
- Änderungen an AGENTS.md, .pi/
- `cargo clean`, `rm -rf target/`, `rm -rf node_modules/`
- `git push`, `git remote` — niemals. Nur lokale Commits auf aktuellem Branch.
- SDK-Annahmen raten:
  - IRSDKSharper C# Property-Namen weichen von YAML-Keys ab — immer verifizieren
  - YAML SessionInfo kommt in ISO-8859-1, nicht UTF-8
- Secrets oder `.env`-Files committen

## Stil
- Rust: snake_case, `log::info!/warn!/error!` statt `println!`
- TypeScript: camelCase, keine `any` ohne Kommentar warum
- Error-Handling: `?` + `thiserror` im Bridge, keine Panics in Hot-Paths

## iRacing SDK Spezifika
- Telemetry: ~200+ vars, 60 Hz, CarIdx-Arrays
- YAML ResultsPositions Felder:
  - Position, ClassPosition, CarIdx, Lap
  - Time = cumulative elapsed (NICHT lap time!)
  - FastestTime = best lap secs, -1 = none
  - LastTime = last lap secs, -1 = none
  - LapsLed, LapsComplete, LapsDriven, Incidents, ReasonOutId, ReasonOutStr
- Practice Gap: `driver.FastestTime - leader.FastestTime`
- Active Session Matching: via `SessionNum` Telemetry-Var
- Broadcast Msgs: Pit, Camera, Replay Commands

## Architektur-Referenz
- Analog zu lmu-pitwall v1.0.84+: Bridge ↔ Dashboard via WebSocket
- Single-exe Distribution via rust-embed (Dashboard gebundelt in Bridge)
