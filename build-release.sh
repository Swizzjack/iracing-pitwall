#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CARGO_TOML="$ROOT/bridge/Cargo.toml"

# ── 1) Patch-Version in bridge/Cargo.toml bumpen ──────────────────────────────
CUR_VER=$(awk '/^\[package\]/{p=1} p && /^version =/{gsub(/"/,"",$3); print $3; exit}' "$CARGO_TOML")
IFS='.' read -r MAJ MIN PATCH <<< "$CUR_VER"
NEW_PATCH=$((PATCH + 1))
NEW_VER="$MAJ.$MIN.$NEW_PATCH"

awk -v new="$NEW_VER" '
  BEGIN{p=0; done=0}
  /^\[package\]/{p=1}
  /^\[/ && !/^\[package\]/{p=0}
  p && !done && /^version =/ {sub(/"[^"]*"/, "\"" new "\""); done=1}
  {print}
' "$CARGO_TOML" > "$CARGO_TOML.tmp" && mv "$CARGO_TOML.tmp" "$CARGO_TOML"

echo "→ Version bumped: $CUR_VER → $NEW_VER"

# ── 2) Generate shared TypeScript types (ts-rs, must run before dashboard build) ──
echo "→ Generating shared TypeScript types (cargo test)…"
( cd "$ROOT/bridge" && \
  CARGO_HOME=/work/.cache/cargo \
  RUSTUP_HOME=/work/.cache/rustup-overlay \
  cargo test )

# ── 3) Dashboard build ─────────────────────────────────────────────────────────
echo "→ Building dashboard…"
( cd "$ROOT/dashboard" && npm run build )

# ── 4) Bridge build (Windows cross-compile, MSVC target) ──────────────────────
echo "→ Building bridge (iracing-pitwall.exe)…"
( cd "$ROOT/bridge" && \
  PATH="/work/.local/bin:$PATH" \
  CARGO_HOME=/work/.cache/cargo \
  RUSTUP_HOME=/work/.cache/rustup-overlay \
  XWIN_CACHE_DIR=/work/.cache/xwin \
  RUSTFLAGS="--sysroot /work/.cache/rustup-overlay/toolchains/stable-x86_64-unknown-linux-gnu" \
  cargo xwin build --release --target x86_64-pc-windows-msvc )

# ── 5) Ins dist-Verzeichnis kopieren ──────────────────────────────────────────
mkdir -p "$ROOT/dist"
rm -f "$ROOT/dist/bridge.exe"
cp "$ROOT/bridge/target/x86_64-pc-windows-msvc/release/iracing-pitwall.exe" \
   "$ROOT/dist/iracing-pitwall.exe"

SIZE=$(du -h "$ROOT/dist/iracing-pitwall.exe" | cut -f1)
echo "✓ /work/dist/iracing-pitwall.exe — v$NEW_VER ($SIZE)"
