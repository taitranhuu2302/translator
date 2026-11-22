# 05: Release artifacts and the update manifest

**What to build:** `scripts/release.sh`, run after a signed universal bundle
exists, produces everything a publish needs in `dist/`: `Neris.app.tar.gz` (the
updater's payload, containing the `.app` at its root), its minisign signature
from `tauri signer sign`, `Neris_<version>_universal.dmg` with the app beside an
`/Applications` link for the first install, `latest.json` with one entry per
platform (`darwin-aarch64` and `darwin-x86_64` pointing at the same universal
payload, `windows-x86_64` added by the port), and release notes generated from
the commits since the previous tag. The script refuses to run without the
update private key and refuses a placeholder public key in `tauri.conf.json`.

**Blocked by:** 02, 03

**Status:** in-progress

- [ ] `./scripts/release.sh` on a signed universal bundle writes the five files
      and no others.
- [ ] The tarball's first path component is `Neris.app/`, and it carries no
      `._*` resource-fork entries.
- [ ] `latest.json`'s signature is the exact content of the `.sig` file and its
      URLs point at the release asset names the workflow uploads.
- [x] A missing `TAURI_SIGNING_PRIVATE_KEY` fails before any artifact is written.
- [ ] The DMG opens to a window with `Neris.app` and an `Applications` link.

## Implementation

`release.sh` checks the bundle and the identity, refuses a placeholder public
key, tars the `.app` with `--no-xattrs`, signs it with `npx tauri signer sign`,
builds the DMG with the app beside an `/Applications` link, writes `latest.json`
with `darwin-aarch64` and `darwin-x86_64` both pointing at the one universal
tarball, and generates notes from the commits since the previous tag.

Verified on 2026-11-22: with `TAURI_SIGNING_PRIVATE_KEY` unset the script exits
before writing anything, and with the placeholder key still in
`tauri.conf.json` it refuses before building. The five-file output, the tarball
layout, the `latest.json` signature and the DMG window are unverified — they
need a real universal bundle.
