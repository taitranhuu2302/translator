#!/usr/bin/env bash
#
# A release tag is the boundary between "merged" and "released", so the tag and
# the workspace version must be the same fact. The release workflow runs this
# before it builds anything.
set -euo pipefail

cd "$(dirname "$0")/.."

TAG="${1:-${GITHUB_REF_NAME:-}}"
if [ -z "$TAG" ]; then
  echo "usage: ./scripts/check-tag.sh vX.Y.Z" >&2
  exit 2
fi

VERSION="$(./scripts/version.sh)"
if [ "$TAG" != "v$VERSION" ]; then
  echo "error: tag $TAG does not match the workspace version $VERSION." >&2
  echo "       Either cut $TAG with ./scripts/set-version.sh ${TAG#v} and re-tag," >&2
  echo "       or move the tag onto a tree that is already $VERSION." >&2
  exit 1
fi

echo "tag ok: $TAG (version $VERSION)"
