# 01: Groundwork — Rust workspace, stable signing, engine seam

**What to build:** The ground the rest of v1 stands on. The repository becomes a Cargo workspace with the three crates the spec names — `core`, `platform-macos`, `app` — the packaged app is signed with a stable identity so the Accessibility permission it is granted survives every rebuild, and all non-UI behaviour sits behind a single engine with its collaborators injected. Nothing user-visible ships here; this is the prefactor that keeps every later ticket testable.

The popup spike has already passed (`spike/panel`, recorded in ADR-0005); this ticket does not re-prove it and may delete the spike once the real panel exists.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] The workspace builds from the command line and packages a `.app` whose signature reports the stable identity rather than ad-hoc
- [ ] The bundle carries the menu-bar-only plist key and launches with no Dock icon
- [ ] Accessibility permission, granted once, still applies after a clean rebuild and after a rebuild that changes source
- [ ] `cargo test` runs from the command line and passes
- [ ] `core` has no AppKit, Tauri or other macOS dependency; `platform-macos` is the only crate that has one
- [ ] All non-UI behaviour is reachable through the engine; no view, panel, or menu item calls a provider or the Accessibility layer directly
- [ ] A test drives the engine with fake ports and asserts on an observable outcome, proving the seam works before any feature depends on it
- [ ] The frontend has one IPC client module and no component calls the Tauri API directly
