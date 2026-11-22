# 03: One reader and one writer for the version

**What to build:** `scripts/version.sh` prints the workspace version and
nothing else; `scripts/check-identity.sh` calls it instead of holding its own
`awk`. `scripts/set-version.sh X.Y.Z` writes that version into the three places
`check-identity.sh` already holds equal — `Cargo.toml`, both plist keys and
`crates/app/tauri.conf.json` — refreshes `Cargo.lock`, and then runs the
identity check, so a bad edit fails immediately. A tag check (used by the
release workflow) refuses a `vX.Y.Z` tag whose version is not the workspace
version. The first version cut with this tool is `0.3.0`.

**Blocked by:** None (can start immediately)

**Status:** in-progress

- [ ] `./scripts/version.sh` prints `0.3.0` after the first bump and nothing else.
- [ ] `./scripts/set-version.sh 0.3.0` leaves `check-identity.sh` green and
      `Cargo.lock` consistent.
- [x] `set-version.sh` refuses a malformed version and a tree it cannot write.
- [x] The tag check refuses `v0.3.1` against a `0.3.0` tree and accepts
      `v0.3.0`.
- [x] `check-identity.sh` has no second copy of the version-extraction rule.

## Implementation

`version.sh` reads `[workspace.package].version` and prints only it;
`check-identity.sh` calls it instead of holding its own `awk`. `set-version.sh`
writes that value into `Cargo.toml`, both plist keys and `tauri.conf.json`,
refreshes `Cargo.lock` through `cargo metadata`, then runs the identity check.
`check-tag.sh` is what the release workflow runs first.

Verified on 2026-11-22 against the current `0.2.0` tree: `check-tag.sh v0.2.0`
accepts, `v0.3.0` refuses with the tag and the workspace version named, and
`set-version.sh 0.3` exits 2 without writing. The `0.3.0` bump itself is the
first release step in `docs/release.md`, not part of this change.
