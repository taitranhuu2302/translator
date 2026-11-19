#!/usr/bin/env bash
#
# The repository's one identity check. The bundle identifier is authored in
# packaging/Info.plist — the artifact macOS reads — and the version in the
# workspace; everything else derives from those two places or is asserted equal
# to them here. It runs in scripts/test.sh and before packaging, so a drift is a
# failing check rather than a lost Accessibility grant
# (docs/adr/0004-stable-signing-from-first-commit.md).
set -euo pipefail

cd "$(dirname "$0")/.."

PLIST="packaging/Info.plist"
LEGACY="packaging/legacy-identifier.txt"
CONFIG="crates/app/tauri.conf.json"

fail() {
  echo "error: $1" >&2
  exit 1
}

plist_value() {
  /usr/libexec/PlistBuddy -c "Print :$1" "$PLIST"
}

config_value() {
  node -e '
    const config = require(require("path").resolve(process.argv[1]));
    const value = config[process.argv[2]];
    if (value === undefined) process.exit(1);
    process.stdout.write(String(value));
  ' "$CONFIG" "$1"
}

workspace_version() {
  awk '
    /^\[workspace\.package\]/ { section = 1; next }
    /^\[/ { section = 0 }
    section && /^version[[:space:]]*=/ {
      sub(/^version[[:space:]]*=[[:space:]]*"/, "")
      sub(/".*$/, "")
      print
      exit
    }
  ' Cargo.toml
}

PLIST_ID="$(plist_value CFBundleIdentifier)"
LEGACY_ID="$(tr -d '[:space:]' < "$LEGACY")"
CONFIG_ID="$(config_value identifier)"
WORKSPACE_VERSION="$(workspace_version)"

[ -n "$PLIST_ID" ] || fail "$PLIST declares no CFBundleIdentifier."
[ -n "$WORKSPACE_VERSION" ] || fail "Cargo.toml declares no [workspace.package] version."

# The identifier the app ran under before it was renamed to Neris, which the
# one-time migration reads. It lives here for the same reason the live one does:
# a bundle identifier in the Rust source would be a second copy that could
# drift. An old identifier equal to the new one means the migration has nothing
# to carry over and the file was never updated.
[ -n "$LEGACY_ID" ] || fail "$LEGACY names no identifier."
if [ "$LEGACY_ID" = "$PLIST_ID" ]; then
  fail "$LEGACY still names $PLIST_ID, the identifier the app runs under."
fi

if [ "$CONFIG_ID" != "$PLIST_ID" ]; then
  fail "$CONFIG declares identifier $CONFIG_ID, but $PLIST (the single source) declares $PLIST_ID."
fi

for key in CFBundleShortVersionString CFBundleVersion; do
  version="$(plist_value "$key")"
  if [ "$version" != "$WORKSPACE_VERSION" ]; then
    fail "$PLIST declares $key $version, but the workspace version is $WORKSPACE_VERSION."
  fi
done

CONFIG_VERSION="$(config_value version)"
if [ "$CONFIG_VERSION" != "$WORKSPACE_VERSION" ]; then
  fail "$CONFIG declares version $CONFIG_VERSION, but the workspace version is $WORKSPACE_VERSION."
fi

# The runtime copies were deleted, not re-pointed: no Rust or shell source may
# name either identifier a second time. If this fires, derive from
# translator_platform_macos::bundle or read the packaging file; do not add a
# constant.
guard_identifier() {
  local identifier="$1" authored="$2"
  local status=0 offenders
  offenders="$(grep -rn --include='*.rs' --include='*.sh' -F "$identifier" crates scripts)" || status=$?
  if [ "$status" -gt 1 ]; then
    fail "the copy guard could not scan crates/ and scripts/ (grep exited $status)."
  fi
  if [ -n "$offenders" ]; then
    echo "error: $identifier is authored in $authored; these files still copy it:" >&2
    printf '%s\n' "$offenders" >&2
    exit 1
  fi
}

guard_identifier "$PLIST_ID" "$PLIST"
guard_identifier "$LEGACY_ID" "$LEGACY"

echo "identity ok: $PLIST_ID, version $WORKSPACE_VERSION"
