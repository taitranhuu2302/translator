# 01: CI on every push and pull request

**What to build:** A GitHub Actions workflow that runs the repository's own
suite on macOS and the portable half on Windows. The macOS job runs
`cargo fmt --check` and `scripts/test.sh` — the identity check, the UI
typecheck, the UI tests, the production frontend build, the IPC boundary check
and `cargo test --workspace` — so the rules that only a script enforces now run
without a human. The Windows job installs the JS workspace and runs the UI
typecheck, the UI tests and `cargo test -p translator-core`, because the
workspace's other two crates do not compile off macOS until the port lands.
Nothing is gated: `main` keeps its direct-merge flow, and the release workflow
re-runs the suite before it packages anything.

**Blocked by:** None (can start immediately)

**Status:** in-progress

- [x] `.github/workflows/ci.yml` exists and runs on `push` to `main` and on
      `pull_request`.
- [x] The macOS job runs `cargo fmt --check` before the suite and fails on drift.
- [ ] The macOS job runs `./scripts/test.sh` with no signing certificate
      present: the identity check reads the plist, so it must not need the
      keychain.
- [x] The Windows job runs the UI typecheck, the UI tests and
      `cargo test -p translator-core`, and does not run `check-identity.sh`
      (it is AppKit/PlistBuddy-bound).
- [ ] Rust and npm caches keep a warm run under a few minutes.
- [ ] The tree is `cargo fmt`-clean, landed as its own formatting-only commit.

## Implementation

`ci.yml` has two jobs. macOS pins `macos-14`, installs the workspace once, runs
`cargo fmt --check` before `./scripts/test.sh`, and caches Cargo through
`Swatinem/rust-cache` and npm through `actions/setup-node`. Windows runs the
typecheck, the UI tests and `cargo test -p translator-core` — deliberately not
`test.sh`, because `check-identity.sh` reads the plist with PlistBuddy.

Verified locally on 2026-11-22: `cargo fmt --check` clean, `npm --prefix ui run
typecheck` clean, 176 UI tests, `cargo test --workspace` 299 tests. The tree is
formatted but the formatting-only commit is not landed; the working tree also
held another session's in-flight shortcut work at the time.
