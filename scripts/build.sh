#!/usr/bin/env bash
#
# The local entry point: build the frontend and the app, then sign
# target/bundle/Neris.app with the stable local identity.
#
# Everything lives in package.sh, which the release workflow calls too, so the
# bundle you develop against and the bundle that ships are assembled by the
# same script. Pass --universal to build both Apple architectures locally; a
# release additionally passes --updater, which arms the in-app updater and is
# deliberately not part of a local build.
set -euo pipefail

cd "$(dirname "$0")/.."

exec ./scripts/package.sh "$@"
