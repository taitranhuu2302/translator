# 11: Bundle identity — one authoring point

**What to build:** The bundle identifier and the version are each authored once. The identifier is authored in the packaged plist, which is the artifact macOS reads; the runtime app-support path is derived from the running bundle instead of being rebuilt from three hard-coded literals, with a compile-time fallback for test binaries that have no bundle; the build script reads the identifier from the plist instead of carrying its own copy; and the Tauri config is asserted against the plist rather than being a second source. The version is authored in the workspace and both plist keys and the config are asserted against it. The assertions run in the test entry point as well as during packaging, so a drift fails the suite before anyone signs a bundle.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Changing the identifier in one place changes the packaged plist, the runtime app-support path, and the signed identifier together.
- [x] The three runtime copies of the identifier are gone; the path is derived from the bundle, with a test-binary fallback.
- [x] The build script holds no identifier of its own and fails packaging when the config disagrees with the plist.
- [x] The two plist version keys and the config version are asserted against the workspace version, and a mismatch fails before signing.
- [x] The identity check runs as part of the test entry point, not only inside packaging.
- [x] Packaging still produces a signed bundle that reports the stable identity, the designated requirement still pins the identifier, and the menu-bar-only key is still present.
- [x] A one-line note about the rename consequence (a new identifier moves history and settings to a new directory) is recorded in ADR-0004 when this lands.

## Implementation

- `packaging/Info.plist` is the one authoring point for the bundle identifier;
  `Cargo.toml`'s `[workspace.package] version` is the one for the version.
- `crates/platform-macos/src/bundle.rs` owns the derivation.
  `identifier()` asks `NSBundle mainBundle` through a new Objective-C bridge
  (`src/macos/bundle.{h,m}`) and falls back to `env!("TRANSLATOR_BUNDLE_ID")`,
  which `crates/platform-macos/build.rs` reads out of the plist at compile time —
  the fallback for test binaries, which have no bundle.
  `application_support_dir()` is the one builder for
  `~/Library/Application Support/<identifier>`.
- The three literals are deleted rather than re-pointed:
  `settings_file.rs::default_path` and `history.rs::default_path` ask
  `application_support_dir()`, and `credentials.rs` uses the identifier as the
  Keychain service (the `SERVICE` const and its literal-asserting test are
  gone). `docs/adr/0004` records that a rename therefore moves history,
  settings and saved keys with the identifier.
- `scripts/check-identity.sh` is the one identity check, shared by the suite and
  packaging. It asserts the config identifier equals the plist's, both plist
  version keys and the config version equal the workspace version, and greps
  `crates/` and `scripts/` for a second copy of the identifier (the needle is
  read from the plist, never written into the script). `scripts/test.sh` runs it
  first; `scripts/build.sh` runs it before the build and then reads `BUNDLE_ID`
  from the plist, so the old plist-vs-copy comparison is deleted and no
  identifier literal is left in the build script.
- The runtime behaviour is unchanged: the identifier did not change, so the
  settings, history and Keychain locations are the same paths and service as
  before. Nothing else in the app was touched.
- The Tauri config keeps its identifier as the build-time input the shell reads;
  it is now asserted against the plist instead of being treated as a source.

## Verification

| Command | Result |
| --- | --- |
| `./scripts/check-identity.sh` (before the change) | exit 1 — named the five copies (`credentials.rs` ×2, `history.rs`, `settings_file.rs`, `build.sh`) |
| `./scripts/check-identity.sh` | exit 0 — `identity ok: com.taitran.translator, version 0.1.0` |
| deliberate drift: config identifier changed | exit 1 — `crates/app/tauri.conf.json declares identifier com.taitran.translator.drift, but packaging/Info.plist (the single source) declares com.taitran.translator.`; reverted |
| deliberate drift: workspace version 0.2.0 | exit 1 — `packaging/Info.plist declares CFBundleShortVersionString 0.1.0, but the workspace version is 0.2.0.`; reverted |
| deliberate drift: one plist version changed | exit 1 — the same assertion, naming the plist key; reverted |
| deliberate drift: config version changed | exit 1 — `crates/app/tauri.conf.json declares version 0.2.0, but the workspace version is 0.1.0.`; reverted |
| deliberate drift: a literal re-added to `bundle.rs` | exit 1 — the copy guard named the file and line; reverted |
| `cargo test -p translator-platform-macos` | exit 0 — 24 unit + 2 parity tests, including `a_test_binary_without_a_bundle_falls_back_to_the_plist_identifier` and `application_support_lives_under_the_bundle_identifier` |
| `./scripts/test.sh` | exit 0 — identity check, typecheck, UI tests, production build, IPC boundary, `cargo test --workspace` |
| `./scripts/build.sh` | exit 0 — packages and signs `target/bundle/Translator.app`; build asserts the identifier, the designated requirement and `LSUIElement` |
| `PlistBuddy` on the packaged `Contents/Info.plist` | `CFBundleIdentifier = com.taitran.translator`, both versions `0.1.0`, `CFBundleExecutable = translator`, `LSUIElement = true`; `codesign -d -r-` gives `identifier "com.taitran.translator" and certificate root = H"0210dfaa…"` |
| `./scripts/test.sh` after merging `spec/architecture-deepening` | see Comments |

Running from the packaged bundle is the only way to exercise the `NSBundle`
half of the derivation, and the app is not launched by the agent. The steps are
in [`../checks/11-bundle-identity-manual.md`](../checks/11-bundle-identity-manual.md);
steps 1–3 there are also asserted by `scripts/build.sh`.

## Comments

- 2026-11-15 — The check lives in `scripts/check-identity.sh` rather than being
  copied into `test.sh` and `build.sh`: the point of the ticket is one home for
  the identity rule, and two inline copies of the assertions would recreate the
  drift this change deletes. `test.sh` and `build.sh` each call it once.
- 2026-11-15 — The Keychain service moves with the identifier because it was one
  of the three runtime copies. The identifier itself is unchanged, so this
  release reads the same service and paths as before.

