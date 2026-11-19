#!/usr/bin/env bash
#
# Package a real Neris.app and sign it with the stable local identity.
# Packaging is part of the build: an unsigned or ad-hoc build loses the
# Accessibility grant on every rebuild (docs/adr/0004-stable-signing-from-first-commit.md).
set -euo pipefail

cd "$(dirname "$0")/.."

IDENTITY="${TRANSLATOR_SIGNING_IDENTITY:-Translator Local Signing}"
APP_NAME="Neris"
CERTIFICATE_ROOT="0210dfaace23b661886724d8588f09a5ee30882b"
APP="target/bundle/$APP_NAME.app"

if ! security find-identity -v -p codesigning | grep -qF "\"$IDENTITY\""; then
  echo "error: code-signing identity \"$IDENTITY\" is not in the keychain." >&2
  echo "       See docs/adr/0004-stable-signing-from-first-commit.md." >&2
  exit 1
fi

# Identity has one authoring point (packaging/Info.plist for the identifier,
# Cargo.toml for the version). Fail before spending a build on a tree whose
# Tauri config and plist disagree, since the signed identifier would silently
# lose the Accessibility grant.
./scripts/check-identity.sh
BUNDLE_ID="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' packaging/Info.plist)"

if [ ! -d node_modules ]; then
  echo "==> installing workspace dependencies"
  npm ci --no-audit --no-fund
fi

echo "==> building frontend"
npm --prefix ui run build

# The placeholder is the frontend's compile-time stand-in, written by
# crates/app/build.rs. Its marker is authored once in ui/placeholder-marker.txt,
# which the app build also reads, so this check cannot name a different value.
PLACEHOLDER_MARKER="$(cat ui/placeholder-marker.txt)"
if [ -z "$PLACEHOLDER_MARKER" ]; then
  echo "error: ui/placeholder-marker.txt is empty; cannot tell whether the frontend was built." >&2
  exit 1
fi
if grep -qF "$PLACEHOLDER_MARKER" ui/dist/index.html; then
  echo "error: ui/dist still holds the compile-time placeholder (\"$PLACEHOLDER_MARKER\"); refusing to package." >&2
  exit 1
fi

echo "==> building app"
# `custom-protocol` is what makes the shell embed ui/dist; without it the
# release binary points at the `devUrl` vite server instead of the bundle.
cargo build --release -p translator-app --features custom-protocol

echo "==> assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/translator "$APP/Contents/MacOS/translator"
cp packaging/Info.plist "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"

# TCC keys the Accessibility grant to the bundle identifier, and the designated
# requirement pins $BUNDLE_ID. The identifier above was read from this same
# plist, and `check-identity.sh` already held the Tauri config to it, so there is
# no second copy left to compare.
plutil -lint "$APP/Contents/Info.plist" >/dev/null
PLIST_EXECUTABLE="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$APP/Contents/Info.plist")"
if [ "$PLIST_EXECUTABLE" != "translator" ]; then
  echo "error: packaging/Info.plist declares CFBundleExecutable $PLIST_EXECUTABLE, expected translator." >&2
  exit 1
fi

echo "==> building the icon"
ICONSET="$(mktemp -d)/Neris.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" crates/app/icons/icon.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  sips -z "$((size * 2))" "$((size * 2))" crates/app/icons/icon.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/Neris.icns"
rm -rf "$(dirname "$ICONSET")"

echo "==> signing with \"$IDENTITY\""
codesign --force --timestamp=none --sign "$IDENTITY" --identifier "$BUNDLE_ID" "$APP"
codesign --verify --strict "$APP"

SIGNATURE="$(codesign -dv --verbose=4 "$APP" 2>&1)"
if ! printf '%s\n' "$SIGNATURE" | grep -qF "Authority=$IDENTITY"; then
  echo "error: the bundle is not signed by \"$IDENTITY\":" >&2
  printf '%s\n' "$SIGNATURE" >&2
  exit 1
fi
if printf '%s\n' "$SIGNATURE" | grep -q "Signature=adhoc"; then
  echo "error: the bundle is ad-hoc signed." >&2
  exit 1
fi

REQUIREMENT="$(codesign -d -r- "$APP" 2>&1)"
if ! printf '%s\n' "$REQUIREMENT" | grep -qi "certificate root = H\"$CERTIFICATE_ROOT\""; then
  echo "error: the designated requirement does not pin the stable certificate:" >&2
  printf '%s\n' "$REQUIREMENT" >&2
  exit 1
fi
if ! printf '%s\n' "$REQUIREMENT" | grep -q "identifier \"$BUNDLE_ID\""; then
  echo "error: the designated requirement does not pin $BUNDLE_ID:" >&2
  printf '%s\n' "$REQUIREMENT" >&2
  exit 1
fi

if [ "$(/usr/libexec/PlistBuddy -c 'Print :LSUIElement' "$APP/Contents/Info.plist")" != "true" ]; then
  echo "error: LSUIElement is missing from the bundle." >&2
  exit 1
fi

echo
echo "Packaged $APP"
printf '%s\n' "$SIGNATURE" | sed -n '1,6p'
printf '%s\n' "$REQUIREMENT"
