# 06: The tag workflow and the release runbook

**What to build:** `.github/workflows/release.yml` on a `v*` tag: it verifies
the tag against `scripts/version.sh`, installs Node and both Rust targets, runs
`scripts/test.sh`, imports the self-signed certificate from secrets into a
temporary keychain, runs `scripts/package.sh --universal` and
`scripts/release.sh`, and creates a **draft** GitHub Release with the DMG,
tarball, `.sig` and `latest.json` attached and the generated notes as its body.
Publishing is the human step after the release checks pass. `docs/install.md`
tells a first-time user exactly what to download and the one quarantine command
they must run; `docs/release.md` is the maintainer runbook — the one-time
secrets bootstrap, the certificate export, the update key, cutting a version,
publishing, and what to do when an update cannot be installed.

**Blocked by:** 01, 03, 04, 05

**Status:** in-progress

- [ ] A `v0.3.0` tag on a `0.3.0` tree produces a draft release with five
      assets and notes naming the commits since `v0.2.0` (or the first commit).
- [x] A tag that disagrees with the workspace version fails before building.
- [x] The workflow needs five secrets and no others: the certificate and its
      password, the keychain password, and the update key and its password.
- [ ] The draft's app installs on a clean Mac, takes one Accessibility grant,
      and reports `0.3.0` in Settings.
- [ ] `docs/install.md` and `docs/release.md` match what the workflow actually
      does; the README no longer claims the app is neither distributed nor
      updated.

## Implementation

`release.yml` runs on `push` of a `v*` tag with `contents: write`, checks the
tag with `./scripts/check-tag.sh "$GITHUB_REF_NAME"` as its first step, then
installs Node 22 and both Rust targets, imports `APPLE_CERTIFICATE_BASE64` into
a temporary keychain, runs `cargo fmt --check` and `./scripts/test.sh`, packages
`--universal --updater`, runs `./scripts/release.sh` with the update key, and
opens the release with `gh release create --draft`. The workflow itself is what
hands over; publishing stays a human step.

Verified on 2026-11-22: `check-tag.sh` accepts the matching tag and refuses a
mismatch, and the workflow calls it before any build step; both workflow files
parse as YAML; the workflow reads exactly the five secrets the runbook
documents; the README, `docs/install.md` and `docs/release.md` are rewritten to
describe the shipped pipeline. Unverified: an actual tag run, the draft's five
assets, and the clean-Mac install.
