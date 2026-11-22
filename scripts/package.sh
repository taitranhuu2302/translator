#!/usr/bin/env bash
#
# Package a real Neris.app and sign it with a stable identity.
# Packaging is part of the build: an unsigned or ad-hoc build loses the
# Accessibility grant on every rebuild (docs/adr/0004-stable-signing-from-first-commit.md).
#
# The local build and the release workflow both call this script, so a released
# bundle and a local one cannot be assembled differently
# (docs/adr/0008-tag-driven-release-from-github.md).
#
#   ./scripts/package.sh                     native build, the local ritual
#   ./scripts/package.sh --universal         both Apple targets, lipo'd
#   ./scripts/package.sh --universal --updater
#                                            what a release ships: universal, and
#                                            able to replace itself with a newer
#                                            Release (docs/adr/0009-…). Without
#                                            the flag the bundle reports updates
#                                            as disabled and can never update.
set -euo pipefail

cd "$(dirname "$0")/.."

UNIVERSAL=0
UPDATER=0
for argument in "$@"; do
  case "$argument" in
    --universal) UNIVERSAL=1 ;;
    --updater) UPDATER=1 ;;
    *)
      echo "error: unknown argument $argument (expected --universal or --updater)" >&2
      exit 2
      ;;
  esac
done

FEATURES="custom-protocol"
if [ "$UPDATER" = 1 ]; then
  FEATURES="custom-protocol,updater"
fi

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

# `custom-protocol` is what makes the shell embed ui/dist; without it the
# release binary points at the `devUrl` vite server instead of the bundle.
# `updater` is only on for the bundle that ships, since it is what lets the app
# replace itself.
if [ "$UPDATER" = 1 ]; then
  PUBLIC_KEY="$(node -e 'const c = require("./crates/app/tauri.conf.json"); process.stdout.write(c.plugins?.updater?.pubkey ?? "")')"
  case "$PUBLIC_KEY" in
    ""|PLACEHOLDER*)
      echo "error: --updater needs the update public key in crates/app/tauri.conf.json." >&2
      echo "       Run ./scripts/update-key.sh and paste it; see docs/release.md." >&2
      exit 1
      ;;
  esac
fi

if [ "$UNIVERSAL" = 1 ]; then
  for target in aarch64-apple-darwin x86_64-apple-darwin; do
    echo "==> building app ($target)"
    cargo build --release -p translator-app --features "$FEATURES" --target "$target"
  done
  echo "==> merging the two binaries"
  lipo -create \
    target/aarch64-apple-darwin/release/translator \
    target/x86_64-apple-darwin/release/translator \
    -output target/release/translator
else
  echo "==> building app"
  cargo build --release -p translator-app --features "$FEATURES"
fi

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

if [ "$UNIVERSAL" = 1 ]; then
  echo "==> checking the binary is fat"
  ARCHITECTURES="$(lipo -archs "$APP/Contents/MacOS/translator")"
  for architecture in arm64 x86_64; do
    case " $ARCHITECTURES " in
      *" $architecture "*) ;;
      *)
        echo "error: the universal bundle is missing $architecture (has: $ARCHITECTURES)." >&2
        exit 1
        ;;
    esac
  done
  echo "    architectures: $ARCHITECTURES"
fi

echo
echo "Packaged $APP"
printf '%s\n' "$SIGNATURE" | sed -n '1,6p'
printf '%s\n' "$REQUIREMENT"
