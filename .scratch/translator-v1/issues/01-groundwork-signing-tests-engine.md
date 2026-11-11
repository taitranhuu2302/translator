# 01: Groundwork — Rust workspace, stable signing, engine seam

**What to build:** The ground the rest of v1 stands on. The repository becomes a Cargo workspace with the three crates the spec names — `core`, `platform-macos`, `app` — the packaged app is signed with a stable identity so the Accessibility permission it is granted survives every rebuild, and all non-UI behaviour sits behind a single engine with its collaborators injected. Nothing user-visible ships here; this is the prefactor that keeps every later ticket testable.

The popup spike has already passed (`spike/panel`, recorded in ADR-0005); this ticket does not re-prove it and may delete the spike once the real panel exists.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] The workspace builds from the command line and packages a `.app` whose signature reports the stable identity rather than ad-hoc
- [x] The bundle carries the menu-bar-only plist key and launches with no Dock icon
- [ ] Accessibility permission, granted once, still applies after a clean rebuild and after a rebuild that changes source — **owed, manual** (see Verification; it cannot be exercised until ticket 02 makes the first Accessibility call)
- [x] `cargo test` runs from the command line and passes
- [x] `core` has no AppKit, Tauri or other macOS dependency; `platform-macos` is the only crate that has one — the platform crate currently holds stubs and so declares no macOS crate *yet*; the boundary is enforced in the other direction by `crates/core/tests/dependency_boundary.rs`
- [x] All non-UI behaviour is reachable through the engine; no view, panel, or menu item calls a provider or the Accessibility layer directly
- [x] A test drives the engine with fake ports and asserts on an observable outcome, proving the seam works before any feature depends on it
- [x] The frontend has one IPC client module and no component calls the Tauri API directly

## Verification

All commands run from the repository root, on macOS 26.1 with rustc/cargo 1.96.0, node 25.1.0, npm 11.6.2.

| Command | Result |
| --- | --- |
| `./scripts/test.sh` | exit 0 — `tsc --noEmit`, `vite build`, the IPC boundary check, `cargo test --workspace` (3 dependency-boundary tests, 8 engine-seam tests, doc-tests) |
| `cargo test --workspace` with no `ui/dist` present | exit 0 — `crates/app/build.rs` writes a marked placeholder so `tauri::generate_context!` compiles; `scripts/build.sh` refuses to package while that marker remains |
| `cargo clippy --workspace --all-targets` | no warnings |
| `./scripts/build.sh` | packages `target/bundle/Translator.app` and signs it with `Translator Local Signing` |
| `codesign -dv --verbose=4 target/bundle/Translator.app` | `Authority=Translator Local Signing`; no `Signature=adhoc` |
| `codesign -d -r- target/bundle/Translator.app` | `designated => identifier "com.taitran.translator" and certificate root = H"0210dfaace23b661886724d8588f09a5ee30882b"` |
| `/usr/libexec/PlistBuddy -c 'Print :LSUIElement' …/Contents/Info.plist` | `true` |
| `open target/bundle/Translator.app` then `lsappinfo info -only ApplicationType <pid>` | `"ApplicationType"="UIElement"`; the process is not listed among processes with `background only = false` — no Dock icon, no ⌘Tab entry |
| UI freshness | appending a marker to `ui/dist/index.html` and rebuilding changed the release binary's sha256; restoring the frontend restored the original hash |

Runtime proof that the shell hosts the UI and that the one IPC module works: a temporary `eprintln!` in the `provider_status` command printed when the packaged webview loaded and `App.tsx` called `ipc.providerStatus()`. The probe was removed afterwards.

The IPC boundary check was tested both ways: it passes on the real tree, and it exits 1 naming the file when a probe module imports `@tauri-apps/api` outside `ui/src/ipc.ts`.

### Still owed — Accessibility grant survives a rebuild (manual)

The grant can only be observed once something calls the Accessibility API, which ticket 02 adds. When it does:

1. `./scripts/build.sh`, launch the app, trigger capture, and grant Accessibility in System Settings → Privacy & Security → Accessibility. Confirm the app appears as **Translator** (`com.taitran.translator`).
2. `rm -rf target/bundle && ./scripts/build.sh` (clean rebuild), relaunch, trigger capture. Expected: no prompt, the toggle stays on, the capture works.
3. Touch a source file (`touch crates/core/src/engine.rs`), `./scripts/build.sh` (rebuild that changes source), relaunch, trigger capture. Expected: the same — the grant is pinned to the designated requirement above, not to the binary hash.
4. If any step prompts again, the signature is the suspect: re-check the two `codesign` rows in the table above before anything else.

### Decisions carried forward

- The engine resolves every direction as `text → Target` with `source: None`; the Language Pair's same-language rule (Target text → partner) needs detection and lands with the provider ladder (ticket 06).
- A history write failure currently fails the whole operation (spec: every outcome is a typed success or a typed failure). Whether history may fail non-fatally — and whether Improve results are recorded — is ticket 09's call; the seam test pins today's behaviour.
- `platform-macos` declares no macOS crate yet; ticket 02 adds the first one and with it the "only crate that has one" half of the boundary.
