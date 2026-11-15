# 07: Test support — keep the fakes, stop shipping them

**What to build:** The fakes stay in the same module with the same behaviour, but they stop being part of what the core ships. They are exposed only under test configuration, so the library's interface is the seven ports and the engine, and a consumer cannot accidentally build on a second implementation of everything. As part of the same move, the fake history store is corrected to match the real one: it trims the query and treats `%`, `_`, and `\` literally, so a test that passes is evidence about the app.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] The fakes compile only for tests (a test-support feature enabled by dev-dependencies); nothing in the shipped interface exposes them.
- [x] Every current user of the fakes still builds: the engine seam test, the app's tests, and the core's unit tests.
- [x] The in-memory history store and the macOS history store agree on trimming and on treating `%`, `_`, and `\` literally.
- [x] A parity test drives both stores through the history port with the same queries — empty, whitespace, `100%`, `a_b`, a backslash — and asserts the same results.
- [x] The fakes keep one home; no fake is duplicated into a consuming crate.
- [x] The test entry point still runs everything with one command, and a build with the feature off does not reference test support.

## Implementation

- `crates/core/src/testing.rs` is still the one home for every fake. Its module
  is now `#[cfg(any(test, feature = "test-support"))]` in `lib.rs`, and
  `translator-core` declares `test-support = []`. With the feature off the
  library exports the ports, the engine and the model, and no fakes.
- Core's own `tests/*.rs` compile against the library rather than `cfg(test)`,
  so `crates/core/Cargo.toml` gains a self dev-dependency:
  `translator-core = { path = ".", features = ["test-support"] }`. That is the
  same door an external consumer uses; no special-casing.
- The parity test lives in `crates/platform-macos/tests/history_parity.rs`
  because that is the only crate where both stores are reachable. Its
  `[dev-dependencies]` add `translator-core` with `features = ["test-support"]`;
  the normal dependency is unchanged, so a non-test build of
  `translator-platform-macos` — and of the app — never enables the feature.
  Resolver 2 does not unify dev-dependency features into those normal builds.
- `InMemoryHistory::search` now trims the query before lower-casing, matching
  the real store's `query.trim().to_lowercase()`. `%`, `_` and `\` were already
  literal because the fake uses `contains`; the doc comment now states the
  contract and the parity test holds the fake to it. Nothing else about the
  fakes changed.
- No fake is duplicated: the parity test imports `InMemoryHistory` from
  `translator_core::testing`, and the app has no fakes of its own.

## Verification

| Command | Result |
| --- | --- |
| `./scripts/test.sh` | exit 0 — typecheck, 9 UI files / 118 tests, production build, IPC boundary, `cargo test --workspace` (30 + 120 + 3 + 63 + 22 + 2) |
| `cargo test -p translator-core` | 63 unit tests + `engine_seam` pass with the feature enabled through the self dev-dependency |
| `cargo test -p translator-platform-macos --test history_parity` | 2 passed — the same queries return the same entries from both stores, and the expected literal/trim answers |
| `cargo check -p translator-core` | exit 0 with no features |
| `cargo check --workspace` | exit 0 — the app and platform crate build without test support |
| `cargo tree -p translator-core -e features,no-dev` | no `test-support` in a normal build; only the regular dependency features appear |
| `cargo doc -p translator-core --no-deps` | no `testing` module in the generated docs; with `--features test-support` it appears |

The parity test exercises the real SQLite store against a temporary database;
no app launch, Accessibility grant or network is involved. The app was not
launched.

## Comments

- 2026-11-15 — Chose a self dev-dependency in core (`path = "."`) for the
  integration tests, and a plain dev-dependency in `platform-macos` for the
  parity test. Feature unification is confined to test builds: `cargo check
  --workspace` and `cargo doc` show no `testing` module. Later tickets that add
  tests using the fakes need that same dev-dependency line
  (`features = ["test-support"]`); the fakes stay only in
  `crates/core/src/testing.rs`.
