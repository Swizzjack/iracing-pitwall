# iRacing Pitwall v2 — Agent Rules

## Project plan

The full implementation roadmap, struct definitions, and task status live in
`@IRACING_PITWALL_PROJECT_PLAN.md`. Read it before starting any task.

## Projekt-Layout

- `bridge/` — Rust (iRacing SDK Reader + WebSocket-Server)
  - Custom Shared Memory Reader via `windows-sys`
  - MMF: `Local\IRSDKMemMapFileName`, 60 Hz Telemetry
  - YAML SessionInfo Parser (ISO-8859-1 Encoding!)
- `dashboard/` — React + TypeScript + Vite
- `shared/` — Generierte TypeScript Types (NICHT handpflegen!)
- Target: Windows x86_64, Cross-Compile via cargo-zigbuild

## Verifikation vor jedem Commit (PFLICHT)

Alle drei Rust-Gates, nicht nur `cargo check`:

- `cargo check --manifest-path bridge/Cargo.toml`
- `cargo fmt --manifest-path bridge/Cargo.toml --check`
- `cargo clippy --manifest-path bridge/Cargo.toml -- -D warnings`

TypeScript:

- `cd dashboard && npx tsc --noEmit`

## Build

- `cargo zigbuild --release --target x86_64-pc-windows-gnu --manifest-path bridge/Cargo.toml`

## Shared Types (bridge ↔ dashboard)

- TypeScript-Typen in `shared/` werden aus Rust via `ts-rs` generiert.
- Export-Trigger: `cargo test --manifest-path bridge/Cargo.toml`
- NIEMALS `.ts`-Dateien in `shared/` handpflegen oder direkt editieren.
- Wenn WS-Protokoll oder Snapshot-Struktur sich ändert:
  1. Rust-Struct in `bridge/src/...` anpassen (`#[derive(TS)]`)
  2. `cargo test` laufen lassen — `shared/*.ts` wird regeneriert
  3. TypeScript-Compiler im `dashboard/` fängt Breaking-Changes

## VERBOTEN

- Änderungen an AGENTS.md, `.pi/`
- `cargo clean`, `rm -rf target/`, `rm -rf node_modules/`
- `git push`, `git remote` — niemals. Nur lokale Commits auf aktuellem Branch.
- Clippy-Warnings mit `#[allow(...)]` unterdrücken ohne Kommentar, warum.
- `unsafe`-Blöcke ohne `// SAFETY:`-Kommentar direkt darüber.
- SDK-Annahmen raten:
  - IRSDKSharper C# Property-Namen weichen von YAML-Keys ab — immer verifizieren
  - YAML SessionInfo kommt in ISO-8859-1, nicht UTF-8
- Secrets oder `.env`-Files committen

## Stil

- Rust: snake_case, `log::info!/warn!/error!` statt `println!`
- TypeScript: camelCase, keine `any` ohne Kommentar warum
- Error-Handling: `?` + `thiserror` im Bridge, keine Panics in Hot-Paths

## iRacing SDK Spezifika

### MMF & Shared Memory

- MMF-Name als Rust Raw-String: `r"Local\IRSDKMemMapFileName"`
  (normaler String bricht: `\I` ist keine gültige Escape-Sequenz)
- Wakeup-Event: `r"Local\IRSDKDataValidEvent"` mit `WaitForSingleObject`
  statt Polling (spart CPU, frame-genaue Sync)
- Triple-Buffer-Lesung: höchsten `tickCount` unter `varBuf[]` finden,
  Buffer in lokalen Puffer kopieren, dann `tickCount` erneut prüfen.
  Bei Änderung: Kopie verwerfen, nochmal. Direkt aus MMF parsen ist
  eine Race Condition mit dem Sim-Writer.

### YAML SessionInfo

- Encoding ist ISO-8859-1, NICHT UTF-8. Vor `serde_yaml::from_str`
  mit `encoding_rs::WINDOWS_1252` decoden. Sonst Crash bei non-ASCII
  Fahrernamen (Umlaute, skandinavische Zeichen).
- Bekannte iRacing-YAML-Bugs: unquoted Strings mit `:`, inkonsistente
  Bool-Formatierung. Wenn Parser crasht: nicht raten, Input-Sample
  loggen und reporten.

### Active-Session-Matching

- Live-Telemetry-Var `SessionNum` (i32) gegen
  `SessionInfo.Sessions[].SessionNum` im YAML matchen.
- NICHT das erste oder letzte Session-Element nehmen — zeigt falsche
  Ergebnisse (z.B. Qualifying-Standings während Race-Session).

### Telemetry-Variablen

- ~200+ vars, 60 Hz Update-Rate, CarIdx-Arrays sind immer 64 Slots
  (auch wenn weniger Autos), freie Slots haben Marker-Werte (-1 o.ä.)
- Whitelist-Approach: nur kuratierte Variablen ins Snapshot-Struct,
  nicht alle 200+ durchtunneln (siehe `bridge/src/telemetry/snapshot.rs`)

### YAML ResultsPositions-Felder

- `Position`, `ClassPosition`, `CarIdx`, `Lap`
- `Time` = cumulative elapsed (NICHT lap time!)
- `FastestTime` = best lap secs, -1 = none
- `LastTime` = last lap secs, -1 = none
- `LapsLed`, `LapsComplete`, `LapsDriven`, `Incidents`, `ReasonOutId`, `ReasonOutStr`
- Practice-Gap: `driver.FastestTime - leader.FastestTime` (beide != -1)

### Broadcast-Messages

- Pit, Camera, Replay Commands via `PostMessageW(HWND_BROADCAST, ...)`
- Message-ID via `RegisterWindowMessageW(r"IRSDK_BROADCASTMSG")`

## Architektur-Referenz

- Analog zu lmu-pitwall v1.0.84+: Bridge ↔ Dashboard via WebSocket
- Single-exe Distribution via rust-embed (Dashboard gebundelt in Bridge)

## Meta-Regeln für den Agent

- **Keine Self-Updates.** Wenn pi oder andere System-Tools Update-Banner zeigen,
  IGNORIEREN. Updates macht der User außerhalb des Agents.
- Keine `npm install -g`, `cargo install`, `pip install`, `dnf install` etc.
- Alle Dev-Tools sind bereits im Container-Image. Wenn etwas fehlt, sagen —
  nicht installieren.
