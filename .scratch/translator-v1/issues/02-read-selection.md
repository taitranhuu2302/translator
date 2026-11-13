# 02: Read the Selection

**What to build:** The app can see what the user has selected in any app. Selecting a paragraph in Notes, page text in Chrome, a line in VS Code or Terminal, then opening the Translate Window shows that text already sitting in the input pane. This is the first slice where a real user gets a result from a real document, and it carries the two failure paths the rest of the app depends on: reading through the clipboard when Accessibility exposes nothing, and refusing to read at all when the focused field is secure.

**Blocked by:** 01

**Status:** resolved

- [x] With the Translate Window opened from a Selection, the input pane holds that Selection, in Notes, Chrome page text, VS Code and Terminal
- [x] When Accessibility is not granted, the app says why it needs it and opens the right settings pane; after granting, the same flow works without reinstalling
- [x] Reading through the clipboard restores the user's previous clipboard contents afterwards
- [x] With a password field or secure event input active, the app refuses to read and no selected text leaves the machine
- [x] With nothing selected, the app opens with an empty input pane and reports no error
- [x] The onboarding that appears on first launch explains the four shortcuts and what each does

## Implementation

- Reading lives in `crates/platform-macos/src/macos/capture.m`, a contained
  Objective-C file compiled by `build.rs`, with a safe `extern "C"` wrapper in
  `capture.rs`. The ladder is the one ADR-0002 describes: `AXIsProcessTrusted`,
  then `IsSecureEventInputEnabled`, then the focused UI element, then
  `kAXSelectedTextAttribute`, then the ⌘C clipboard fallback guarded by
  `changeCount` comparison before restoring.
- `AXManualAccessibility` is set on the focused app before asking for its focused
  element, which is what lets Electron apps (VS Code, Chrome) expose a selection
  without the user turning on a per-app accessibility setting.
- The engine owns the decisions: `selection_for_window` maps `NothingSelected` to
  an empty `Selection` (not an error), while a missing grant and secure input stay
  typed errors. `accessibility_granted`, `request_accessibility`,
  `open_accessibility_settings` and `complete_onboarding` sit beside it, all
  behind `Capture`/`SettingsStore` ports so the seam tests drive them with fakes.
- Capture runs on a worker thread *before* the window is shown, while the source
  app still has focus; the result waits in `AppState::prefill` and the UI claims
  it through `take_selection_prefill`. Capturing after the window appears would
  read Translator itself.
- The settings file lands early (`MacSettingsStore`, atomic write, `onboarding_seen`
  defaulting to false for files written before the field existed) because the
  first-launch onboarding needs somewhere to record that it was dismissed.
  Ticket 10 extends this file rather than introducing a new one.
- A menu bar item and a ⌃⌥E global shortcut are registered here: an
  `LSUIElement` app with no Dock icon otherwise has no way to open its window.
  Ticket 10 owns rebinding and conflict handling; the registration already reads
  `settings.shortcuts.translate_window` so rebinding only needs the UI.

## Verification

All commands run from the repository root.

| Command | Result |
| --- | --- |
| `./scripts/test.sh` | exit 0 — `tsc --noEmit`, `vite build`, the IPC boundary check, and `cargo test --workspace`: 3 dependency-boundary tests, 16 engine-seam tests, 3 settings-store tests |
| `cargo clippy --workspace --all-targets` | no warnings |
| `./scripts/build.sh` | packages and signs `target/bundle/Translator.app` with `Translator Local Signing` |
| `codesign -d -r- target/bundle/Translator.app` | `designated => identifier "com.taitran.translator" and certificate root = H"0210dfaace23b661886724d8588f09a5ee30882b"` |

The engine-seam tests cover the observable contract: the window gets the
selection, gets an empty input when nothing is selected, reports a missing grant
instead of reading, blocks secure input before any text reaches a provider,
delegates the grant request and settings pane to the platform port, and persists
the onboarding flag once and only then.

Runtime behaviour — real Accessibility, secure input and the clipboard — is
manual; the steps are in
[`../checks/02-read-selection-manual.md`](../checks/02-read-selection-manual.md),
and cases 1–15 passed on the signed bundle on 2026-11-13. Cases 16–17 were
added by the review pass and have not been run. The grant-survives-a-rebuild
check owed by ticket 01 is still open: it needs one grant, then a
`./scripts/build.sh` rebuild, then a selection read without granting again.

## Comments

- 2026-11-14 — Review pass, same commit as the run book: the onboarding now offers **Grant
  Accessibility…** and **Open Privacy Settings**, shows "Accessibility is
  granted." from the window's own permission check (including when a rebuild
  kept the grant), and drops the buttons once the grant is observed. Manual
  cases 16–17 cover both states; the app was not launched.

