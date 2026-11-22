#!/usr/bin/env bash
#
# The one writer of the version. Cargo.toml authors it; the plist keys and the
# Tauri config are copies that check-identity.sh already holds equal, so writing
# all three here is the only way the copies can move together.
set -euo pipefail

cd "$(dirname "$0")/.."

VERSION="${1:-}"
if ! printf '%s' "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "usage: ./scripts/set-version.sh X.Y.Z" >&2
  exit 2
fi

node - "$VERSION" <<'JS'
const fs = require("fs");
const version = process.argv[2];

const cargoPath = "Cargo.toml";
const cargo = fs.readFileSync(cargoPath, "utf8");
const nextCargo = cargo.replace(
  /(\[workspace\.package\][\s\S]*?\nversion\s*=\s*")[^"]+(")/,
  `$1${version}$2`,
);
if (nextCargo === cargo) {
  console.error("error: Cargo.toml has no [workspace.package] version to write.");
  process.exit(1);
}
fs.writeFileSync(cargoPath, nextCargo);

const configPath = "crates/app/tauri.conf.json";
const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
config.version = version;
fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + "\n");
JS

/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" packaging/Info.plist
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION" packaging/Info.plist

echo "==> refreshing Cargo.lock"
cargo metadata --format-version 1 >/dev/null

./scripts/check-identity.sh
echo "version set: $VERSION"
