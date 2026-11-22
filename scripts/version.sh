#!/usr/bin/env bash
#
# The one reader of the workspace version. Everything that needs the version —
# the identity check, the tag check, the release artifacts — asks here; nothing
# re-implements the extraction (docs/adr/0004-stable-signing-from-first-commit.md).
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="$(awk '
  /^\[workspace\.package\]/ { section = 1; next }
  /^\[/ { section = 0 }
  section && /^version[[:space:]]*=/ {
    sub(/^version[[:space:]]*=[[:space:]]*"/, "")
    sub(/".*$/, "")
    print
    exit
  }
' Cargo.toml)"

if [ -z "$VERSION" ]; then
  echo "error: Cargo.toml declares no [workspace.package] version." >&2
  exit 1
fi

printf '%s\n' "$VERSION"
