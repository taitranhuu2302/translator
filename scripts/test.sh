#!/usr/bin/env bash
#
# The command-line test entry point: frontend typecheck and build, the IPC
# boundary check, then the Rust suite (engine seam + dependency boundary).
set -euo pipefail

cd "$(dirname "$0")/.."

if [ ! -d ui/node_modules ]; then
  echo "==> installing frontend dependencies"
  npm --prefix ui ci --no-audit --no-fund
fi

echo "==> frontend typecheck"
npm --prefix ui run typecheck

# `cargo test` compiles the Tauri shell, which embeds ui/dist at compile time.
echo "==> frontend build"
npm --prefix ui run build

echo "==> IPC boundary"
matches="$(mktemp)"
status=0
grep -rn "@tauri-apps/" ui/src --include='*.ts' --include='*.tsx' >"$matches" || status=$?
if [ "$status" -gt 1 ]; then
  echo "error: the IPC boundary check could not scan ui/src (grep exited $status)." >&2
  exit 1
fi
violations="$(grep -v '^ui/src/ipc\.ts:' "$matches" || true)"
rm -f "$matches"
if [ -n "$violations" ]; then
  echo "error: only ui/src/ipc.ts may import @tauri-apps. Found:" >&2
  echo "$violations" >&2
  exit 1
fi

echo "==> rust tests"
cargo test --workspace
