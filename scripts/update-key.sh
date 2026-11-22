#!/usr/bin/env bash
#
# Generate the update key pair once (docs/adr/0009-self-signed-distribution-and-in-app-updates.md).
# The private half never enters the repository; it is uploaded to GitHub Secrets
# and kept offline. Losing it means existing installs can no longer update
# in-app, so this refuses to overwrite an existing key.
set -euo pipefail

cd "$(dirname "$0")/.."

KEY="${NERIS_UPDATE_KEY:-$HOME/.tauri/neris-updater.key}"

if [ -e "$KEY" ]; then
  echo "error: $KEY already exists; refusing to overwrite the update key." >&2
  echo "       Set NERIS_UPDATE_KEY to write somewhere else." >&2
  exit 1
fi

mkdir -p "$(dirname "$KEY")"

echo "==> generating $KEY"
npx tauri signer generate -w "$KEY"

echo
echo "Next, both once:"
echo "  1. Put the public key printed above into plugins.updater.pubkey in"
echo "     crates/app/tauri.conf.json, then commit it."
echo "  2. Upload the private key and its password as the repository secrets"
echo "     TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD."
echo "     See docs/release.md."
