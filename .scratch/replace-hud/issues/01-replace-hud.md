# 01: Replace progress HUD

**What to build:** `⌃⌥R` shows what it is doing, the way Quick Translate does, without opening a popup. A small non-activating capsule appears near the cursor when a Replace is still running after 120 ms, names the step the engine is on — *Reading the selection…*, *Translating…*, *Replacing…* — and disappears the moment the Replace ends. A Replace that finishes inside 120 ms shows nothing, and a second Replace pressed while one runs is ignored.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] `ReplacePhase { Capturing, Translating, Replacing }` lives in `core`; `replace_selection` reports each step through the sink it is given, in order, and only when that step is reached.
- [x] The shell owns one Replace at a time: a second `⌃⌥R`, or the popup's Enter, while one runs does nothing.
- [x] `hud` is a second `NSPanel` (`ReplaceHudPanel`): non-activating, never key, ignores the mouse, floats above other apps and joins every Space.
- [x] The HUD is shown only when the Replace is still running after 120 ms, at the cursor, and hidden on completion or failure.
- [x] The popup's Enter runs under the same HUD, so a slow write shows *Replacing…* rather than nothing.
- [x] The three labels have one home (`ui/src/statusCopy.ts`), used by both `PopupView` and the HUD.
- [x] The popup's native failure dialog is unchanged; the HUD never explains a failure.
- [x] `hud.html` is a build input beside `index.html` and `popup.html`; `preview.html` stays out.
- [x] Tests, typecheck, build and `scripts/test.sh` pass.

## Implementation

- `ReplacePhase { Capturing, Translating, Replacing }` in `crates/core/src/engine.rs`
  is `Serialize` with `snake_case`, and is exported from `translator_core`.
  `TranslatorEngine::replace_selection` takes one sink, `on_phase: impl
  FnMut(ReplacePhase)`, and reports each step immediately before the work it
  names: before the Selection is read, before the provider is asked, before the
  translation is written. A Replace refused before translating (nothing
  selected, blank text) reports `Capturing` and nothing more; one whose
  translation fails never reports `Replacing`. There is no second entry point
  that returns the steps — the callback is the only source. `replace_with_translation`
  is unchanged, because its only step is the write.
- `crates/core/tests/engine_seam.rs` asserts the step sequences against the
  fakes: every step in order, the early refusal, and a failing provider.
- `crates/app/src/hud.rs` is the shell's half. `ReplaceHudPanel` (declared with
  `tauri_panel!`) can never become key or main; `prepare` adds the
  non-activating style mask, sets the Floating level, joins every Space
  including full-screen auxiliary windows, keeps the panel visible when the app
  deactivates, ignores mouse events, and leaves it hidden. `ReplaceHud::start`
  spawns a thread that waits `SHOW_DELAY` (120 ms) and, on the main thread,
  shows the capsule at the cursor only if the Replace is still running;
  `phase` records the step and emits `replace-hud-phase`; `finish` cancels a
  pending show and hides. `finish` waits on the main thread, so both callers
  invoke it from the worker that ran the Replace.
- One Replace at a time: `AppState.replace: Mutex<Option<hud::ReplaceHud>>`
  with `begin_replace` / `advance_replace` / `end_replace`. The `⌃⌥R` worker
  drops a press that cannot claim the slot; the IPC command and the popup's
  Enter answer `CaptureError::Unavailable` — "a Replace is already running, so
  nothing was replaced" — so a user who pressed the shortcut twice sees one
  Replace happen, not an error about nothing.
- `hud` window in `tauri.conf.json` (260 × 52, transparent, undecorated, never
  focused, always on top) and in the capability list; `hud.html` is a third
  Vite input. `ui/src/HudView.tsx` draws nothing until it knows a step, follows
  the event, and claims the current step through `replace_hud_phase` on mount
  for the first opening, when the page may still have been loading as the
  capsule appeared. `ReplaceHudCard` is the capsule on its own, so the design
  preview can draw it without a shell.
- The three labels live in `ui/src/statusCopy.ts`; `PopupView` and the HUD both
  read that map, so the popup and the capsule cannot drift apart.
- The failure dialog is untouched: `end_replace` (and so the hide) runs before
  `show_replace_failure`, and the HUD never carries the bad news.
- `preview.html` gains a `#hud` scene that draws all three steps in the real
  260 × 52 frame; it stays out of the production build.

## Verification

| Command | Result |
| --- | --- |
| `cargo test -p translator-core --test engine_seam` | pass — the three new Replace-step tests plus the existing seam tests |
| `cargo test --workspace` | pass — every suite, 0 failures |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 13 files, 172 tests passed |
| `npm --prefix ui run build` | `dist/index.html`, `dist/popup.html`, `dist/hud.html` |
| `./scripts/test.sh` | pass — identity check, typecheck, UI tests, production build, IPC boundary, `cargo test --workspace` |
| `./scripts/build.sh` | release bundle built and signed; not launched |
| design preview `#hud` | all three capsules draw with the expected copy and card styling; no errors from the preview page |

## Comments

- 2026-11-19 — A second Replace is dropped, not merged: the `⌃⌥R` path
  discards the press (a shortcut must not raise a dialog), while the IPC path
  and the popup's Enter answer with `CaptureError::Unavailable`, because the
  user is looking at a surface that can explain. The slot in `AppState` is the
  guard; there is no queue.
- 2026-11-19 — `ReplaceHud::finish` hides through `main_thread::on_main_thread`
  (2 s timeout) and must be called off the main thread. Both paths call it from
  the worker that ran the Replace, after their own main-thread hops.
