#!/usr/bin/env bash
#
# Turn a signed target/bundle/Neris.app into everything a GitHub Release
# carries: the updater payload and its signature, the first-install DMG,
# latest.json, and notes generated from the commits since the previous tag.
#
# Run ./scripts/package.sh --universal first. The update private key must be in
# TAURI_SIGNING_PRIVATE_KEY (see docs/release.md).
set -euo pipefail

cd "$(dirname "$0")/.."

APP="target/bundle/Neris.app"
DIST="dist"
VERSION="$(./scripts/version.sh)"
NAME="Neris_${VERSION}_universal"
REPOSITORY="taitranhuu2302/translator"
BASE_URL="https://github.com/$REPOSITORY/releases/download/v$VERSION"

if [ ! -d "$APP" ]; then
  echo "error: $APP is missing; run ./scripts/package.sh --universal first." >&2
  exit 1
fi

if [ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]; then
  echo "error: TAURI_SIGNING_PRIVATE_KEY is not set; the update payload cannot be signed." >&2
  echo "       See docs/release.md." >&2
  exit 1
fi

PUBLIC_KEY="$(node -e 'const c = require("./crates/app/tauri.conf.json"); process.stdout.write(c.plugins?.updater?.pubkey ?? "")')"
case "$PUBLIC_KEY" in
  ""|PLACEHOLDER*)
    echo "error: crates/app/tauri.conf.json still holds a placeholder update public key." >&2
    echo "       Run ./scripts/update-key.sh and paste the public key; see docs/release.md." >&2
    exit 1
    ;;
esac

rm -rf "$DIST"
mkdir -p "$DIST"

TARBALL="$DIST/$NAME.tar.gz"
DMG="$DIST/$NAME.dmg"

# The updater payload: the .app at the root of the archive, without the
# resource-fork entries macOS would otherwise add.
echo "==> tarball"
STAGING="$(mktemp -d)"
cp -R "$APP" "$STAGING/Neris.app"
tar -C "$STAGING" --exclude='._*' -czf "$TARBALL" Neris.app
rm -rf "$STAGING"

# The first install: the app beside an Applications link, so dragging is one
# gesture. A DMG is never updated; the tarball above is.
echo "==> dmg"
DMG_STAGING="$(mktemp -d)"
cp -R "$APP" "$DMG_STAGING/Neris.app"
ln -s /Applications "$DMG_STAGING/Applications"
hdiutil create -quiet -volname "Neris" -srcfolder "$DMG_STAGING" -ov -format UDZO "$DMG"
rm -rf "$DMG_STAGING"

echo "==> signing the update payload"
npx tauri signer sign "$TARBALL"

SIGNATURE_FILE="$TARBALL.sig"
if [ ! -f "$SIGNATURE_FILE" ]; then
  echo "error: the signer wrote no $SIGNATURE_FILE." >&2
  exit 1
fi

echo "==> latest.json"
node - "$DIST/latest.json" "$VERSION" "$(cat "$SIGNATURE_FILE")" "$BASE_URL/$NAME.tar.gz" <<'JS'
const fs = require("fs");
const [path, version, signature, url] = process.argv.slice(2);

// One universal payload answers both Apple architectures; Windows adds its own
// key when the port lands (.scratch/windows-port/spec.md).
const platforms = {
  "darwin-aarch64": { signature, url },
  "darwin-x86_64": { signature, url },
};

fs.writeFileSync(
  path,
  JSON.stringify(
    {
      version,
      pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
      platforms,
    },
    null,
    2,
  ) + "\n",
);
JS

echo "==> release notes"
PREVIOUS="$(git tag --list 'v*' --sort=-v:refname | grep -v "^v$VERSION$" | head -1 || true)"
if [ -n "$PREVIOUS" ]; then
  RANGE="$PREVIOUS..HEAD"
else
  RANGE="HEAD"
fi

{
  echo "## What's changed"
  echo
  git log --no-merges --pretty='- %s' "$RANGE"
  echo
  echo "## Install"
  echo
  echo "Download \`$NAME.dmg\`, drag **Neris** into **Applications**, then remove"
  echo "the quarantine flag once, because this build is signed by the maintainer"
  echo "rather than notarised by Apple:"
  echo
  echo '```bash'
  echo "xattr -dr com.apple.quarantine /Applications/Neris.app"
  echo '```'
  echo
  echo "Existing installs update themselves from the menu bar; this file is only"
  echo "needed for a first install or a manual reinstall."
} > "$DIST/notes.md"

echo
echo "Artifacts in $DIST:"
ls -1 "$DIST"
