# 10: Settings

**What to build:** The user can shape the app to their own machine: change the Language Pair, rebind any of the four shortcuts, and see which providers are ready and which are exhausted. A shortcut that another app already owns is reported instead of quietly doing nothing.

**Blocked by:** 06

**Status:** resolved

- [x] The Language Pair can be changed and survives relaunch
- [x] Each of the four shortcuts can be rebound and the new binding survives relaunch
- [x] A binding another app already owns surfaces a conflict at the moment it is set
- [x] Provider status shows which providers have a key and which have been skipped after a quota or auth failure
- [x] Resetting to defaults restores the four shipped shortcuts and the default pair

## Verification

The app was **never launched** while building this ticket. The boxes are proven
by the automated suites below; the parts that need a real window, a real
registration conflict and a real settings file are written up in
[`../checks/10-settings-manual.md`](../checks/10-settings-manual.md).
| Command | Result |
| --- | --- |
| `npm --prefix ui run test` | 75 passed — 14 new: 7 `captureBinding` tests (modifier-only ignored, letter/digit/function mapping, modifier order, the allowed modifier-less capture, unsupported keys), 1 `withShortcut`, 5 `settingsView` tests (direction sentence, a language named by its code, the target/partner rule and the unchanged case), and 1 MyMemory status line. |
| `cargo test -p translator-app` | 22 passed — 13 new in `shortcuts`: shipped bindings distinct, a duplicate names the other action, synonyms compare by combination, a re-spelling is a no-op, blank and unparseable bindings, modifier-less refusal, apply keeps the previous binding, the strings the capture emits parse back, the refusal wording, routing, an unregistered shortcut routes nowhere, and reset keeps region/onboarding. The two routing tests moved here from `main.rs`. |
| `cargo test -p translator-core --test engine_seam` | 62 passed — 1 new: a changed pair and all four bindings are written once and read back through the engine. |
| `cargo test -p translator-core` | 110 passed (and 3 in the second target) — unchanged |
| `cargo test -p translator-platform-macos` | 22 passed — unchanged |
| `cargo clippy --workspace --all-targets` | clean |
| `./scripts/test.sh` | passed (typecheck, vitest, IPC boundary, build, cargo, doctests) |

After the code-review pass (2026-11-14) the same gate passes with **75** UI
tests, **30** app tests, **120** core unit tests, **3** dependency-boundary
tests, **63** engine-seam tests and **22** platform tests; `cargo clippy
--workspace --all-targets -- -D warnings` is clean. The nine new app tests are
six `tray_entries` cases (four action rows with live accelerators, the
permission row in both states, Quit last) plus `launch_problem` and
`forget_conflict`.

Proofs for the ticked boxes:

- **The Language Pair can be changed and survives relaunch:** the Settings
  surface's Target select computes the pair with `withTarget` and writes it
  through `ipc.saveSettings` → `save_settings` → the engine's `SettingsStore`.
  `saving_settings_writes_the_pair_and_every_shortcut_and_reads_them_back`
  proves the whole document round trips (pair, four bindings, region), and the
  existing `MacSettingsStore` round-trip test proves the file itself. Manual
  case 3 proves relaunch against the real file.
- **Each shortcut can be rebound and survives relaunch:** each row's pressed
  combination goes to `rebind_shortcut`, which registers on the main thread,
  unregisters what it replaces, updates the live routes and saves. At launch
  `setup` registers exactly the saved bindings into `ShortcutRoutes`, so what
  survived is what works. `a_new_combination_plans_registration_and_replaces_the_old_one`
  and `bindings_the_capture_ui_generates_parse_back_through_the_real_parser`
  cover the two ends; manual cases 5–9 cover macOS and relaunch.
- **A combination another app owns is reported when it is set:** `plan` is pure
  and answers `Invalid`, `InUse { other }`, `Unchanged` or `Apply`; the command
  turns `InUse` into "That shortcut is already used by …" and a macOS refusal
  into "macOS would not give Translator …". `a_combination_another_action_owns_names_that_action`
  and `a_refused_registration_never_names_an_owner` pin both wordings, and the
  row shows the returned problem without changing the saved binding. Manual
  cases 10–11 prove the real Carbon conflict.
- **Provider status shows keys and skipped providers:** the rows read
  `provider_status`; Azure and Groq show **Key saved** or **No key** and the
  skipped sentence, and MyMemory shows that it needs no key. Saving or clearing
  a key returns the refreshed status (the engine clears the session skip), so
  the row changes without a relaunch. `myMemoryStatusLine` and the existing
  status-line tests pin the words; manual cases 19–20 prove the Keychain.
- **Reset restores the four shipped shortcuts and the default pair:**
  `reset_shortcuts_and_language_pair` unregisters every live shortcut, registers
  each `ShortcutBindings::default()` binding, reports every refusal, and writes
  `settings_after_reset` — shipped pair and shortcuts, everything else kept.
  `reset_restores_the_shipped_pair_and_shortcuts_and_keeps_everything_else`
  proves the document; manual cases 15–18 prove the live registrations.

## Decisions

- **A fourth surface, not a modal.** Settings is another tab in the existing
  switch. The working panes' state lives in `App` and only their subtree is
  replaced, so a trip to Settings and back changes neither the input, the
  result, the Improve parts, nor the loaded history. Choosing Settings does not
  touch `mode`, which is why returning to Translate returns to the mode that was
  showing.
- **One Target, an implied partner, and `Language` stays open.** The select
  offers English and Vietnamese; `withTarget` makes the other one the partner.
  Choosing the Target already saved returns the same pair untouched, so a pair
  saved by another tool is not quietly rewritten; a Target that is not offered
  still renders as an option instead of a blank select. The sentence under the
  select is the only explanation of direction and names an unknown language by
  its own code.
- **`crates/app/src/shortcuts.rs` holds the rules, `main.rs` holds the wiring.**
  `Route` and `ShortcutRoutes` moved here, and the four `DEFAULT_*` constants
  were replaced by `ShortcutBindings::default()` (the same four strings), so a
  settings file that cannot be read still boots with the shipped shortcuts.
  `plan` is pure; comparison is on `mods` and `key` after parsing, so
  Ctrl/Control, Alt/Option and Cmd/Super/Cmd are the same combination. The
  command's shape is `rebind_shortcut(app, action, binding) -> Result<RebindResult, ()>`;
  `RebindResult.problem: None` means the binding is both live and saved.
- **The settings file stores one canonical spelling.** `binding_string` emits
  `Cmd+Ctrl+Alt+Shift+<key>` with the key stripped of `Key`/`Digit` (`D`, `1`,
  `F5`), and that is what the surface and the tray read. Equality never uses the
  string, only the parsed combination, so the stored spelling cannot create a
  duplicate the engine misses.
- **Registration and the tray accelerator run on the main thread.** Carbon
  registration goes through `run_on_main_thread` and a channel with a bounded
  wait, so a shell that never answers reports a failure instead of hanging the
  surface. `MenuItem` is an AppKit object, so `set_accelerator` is posted the
  same way — a deliberate addition to the ticket's plan, which allowed for it —
  and it is only updated after the settings write succeeds.
- **Save failure rolls the registration back.** Order is: register the new
  combination, unregister the old, update the routes, save. If the save fails
  the new registration is removed and the old one restored (or the route cleared
  when it cannot be), and the result carries "The shortcut was not changed: the
  settings file could not be written." A shortcut therefore never works now and
  vanishes at relaunch.
- **Press-to-record captures `event.code`.** That is the physical key, so the
  combination is the same on every keyboard layout. Modifier-only presses and
  keys the parser cannot store return `null` and keep listening; Escape cancels.
  The capture deliberately lets a modifier-less press through, so the one
  refusal wording ("A shortcut needs at least one of Ctrl, Alt, Cmd or Shift.")
  lives only in the engine instead of being duplicated in TypeScript.
- **The provider forms moved rather than changed.** Ticket 08's Azure and Groq
  key fields, region and status lines now live inside the Settings surface.
  MyMemory joins them as a keyless row. Saving or clearing a key still uses the
  status returned by the command, which is why a replaced key stops showing as
  skipped immediately.
- **Reset is explicit and reversible.** The label names both things it restores,
  a second click confirms, and the sentence says history and API keys are kept.
  Every live shortcut is unregistered before the defaults are registered, so a
  shipped combination that a rebind had taken is free again. A refused binding
  is listed one by one; a failed settings write is surfaced rather than hidden,
  because relaunching would then restore the previous file.
- **Start at login uses the LaunchAgent the plugin installs.** A review pass
  added the spec's missing preference: `tauri-plugin-autostart` 2.7 with
  `MacosLauncher::LaunchAgent`, wrapped by `start_at_login_enabled` (the actual
  state, never the stored field) and `set_start_at_login`. The command enables
  or disables the login item, then saves `settings.start_at_login`; a failed
  save puts the login item back, so the file and macOS never disagree after a
  relaunch. The checkbox starts from the plugin's answer, so a login item the
  user removed by hand reads as off. The app never enables itself at launch.
- **The tray lists the four actions and the permission.** The menu-bar item is
  no longer just “Open Translate Window”: `tray_entries` yields Quick Translate,
  Replace Selection, Read Aloud and Open Translate Window with their live
  combinations, then the Accessibility row (clicking the missing-permission row
  opens the pane), then Quit. Clicking an action runs the same handler as its
  global shortcut. The menu is rebuilt on the main thread after a rebind or
  reset, and when `accessibility_granted` observes a change, so the tray never
  keeps a stale combination. The pure entry list is tested without AppKit.
- **Launch-time refusals are not silent.** macOS can refuse a saved binding at
  startup; `setup` now collects each refusal into `launch_conflicts` with the
  same wording reset uses, and `shortcut_conflicts` feeds a warning above the
  Shortcuts rows. A successful rebind drops that action's warning, and reset
  clears the list. A broken binding can therefore be seen and fixed in the
  surface instead of only in the log.
- **Not verified here:** the rendered four-tab surface, real Carbon conflicts
  with another app, relaunch persistence on the real settings path, the tray
  label updating, the Keychain rows refreshing, and the read-only settings
  directory. Those are the manual checks, and the app was not launched.

## Comments

- 2026-11-14 — Review pass in `9539a77` and `8bf53fd` closed four gaps the
  two-axis review found: start at login, the tray content, launch-time shortcut
  conflicts, and the onboarding's Accessibility actions (ticket 02). The manual
  checks grew cases 24–29 for the login item, the tray menu, the permission row
  and a refused-at-launch binding; the app was still not launched.
- 2026-11-14 — Implemented in `819a725`. A real window, real Carbon
  registrations and the real settings file still need a run on the signed
  bundle: [`../checks/10-settings-manual.md`](../checks/10-settings-manual.md)
  covers the fourth surface, the direction sentence, a changed pair and each of
  the four rebinds surviving relaunch, a duplicate naming its action, a
  combination another app owns, modifier-only and modifier-less presses, Escape,
  reset restoring all four and the pair while keeping history and keys, a reset
  that reports a refusal, the provider rows and an immediately refreshed key,
  history and Improve surviving a trip to Settings, and both failed-write paths.
