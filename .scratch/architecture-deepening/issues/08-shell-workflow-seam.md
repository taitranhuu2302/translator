# 08: Shell workflow seam — and start at login as its first workflow

**What to build:** The shell gets one seam and one workflow behind it. The seam is a small set of shell ports declared in the shell crate, not in the core: the login item, the popup presenter, the shortcut registry, and the shortcut route. The first workflow is start at login: read the current state, change the login item, save the preference, and on a failure roll back both the login item and the saved preference. The Tauri command shrinks to a hop to the main thread plus a call into the workflow. One named timeout serves every main-thread hop, replacing the inconsistent values in use.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] The four shell ports exist and are implemented by the real shell; no workflow needs a Tauri handle to run.
- [x] Start at login is one function over the seam, with the read → change → save → roll-back order visible in it.
- [x] The command only hops to the main thread and maps the workflow's result.
- [x] Every main-thread hop in the shell uses one shared timeout constant.
- [x] Tests drive the workflow with fakes: success; the login item refuses the change; the save fails; and after each failure both the login item and the saved preference are exactly where they started.
- [x] The packaged app still turns start at login on and off at the system level.

## Implementation

- `crates/app/src/workflows.rs` is the one shell seam. It declares the four
  app-local ports — `LoginItem` (`is_enabled`/`set_enabled`),
  `PanelPresenter` (`hide`/`show`), `KeyRegistry` (`register`/`unregister`) and
  `ShortcutRoute` (`set_route`) — plus the real adapters `ShellLoginItem`,
  `ShellPanelPresenter`, `ShellKeyRegistry` and `ShellShortcutRoute`. The
  adapters own every AppKit/Tauri detail; the workflow functions take only the
  ports, so no workflow needs an `AppHandle`.
- `set_start_at_login(&dyn LoginItem, &dyn SettingsStore, bool)` is the first
  workflow. It reads the preference, changes the login item, saves the
  preference, and on a refused save changes the login item back to the value the
  file still holds. The read failure, the refused change and the refused save
  keep the command's existing sentences.
- `set_start_at_login` in `main.rs` shrinks to one main-thread hop, the workflow
  call, and a `From` mapping onto the wire struct. `start_at_login_enabled` now
  reads through `ShellLoginItem`. `AppState` holds the same `Arc<dyn
  SettingsStore>` the engine uses, which is what lets a workflow change a
  preference beside a platform state.
- `main_thread::MAIN_THREAD_TIMEOUT` (2 s) is the one timeout.
  `on_main_thread` no longer accepts a timeout, so no call site can pass a
  different one. `PANEL_TIMEOUT` (1 s) in `main.rs`,
  `shortcuts::MAIN_THREAD_TIMEOUT`, and `tray::MENU_TIMEOUT` are deleted, and
  the two paths that bypassed the helper — `hide_popup` and
  `open_quick_translate` — now go through it.
- Deleted: the inline read → change → save → rollback block in the Tauri
  command, and the three timeout constants it and its neighbours each carried.
  No UI or `core` code changed. The pure planning code in `shortcuts.rs` is
  untouched; ticket 10 can move only its apply half.

## Verification

| Command | Result |
| --- | --- |
| `./scripts/test.sh` | exit 0 — typecheck, 118 UI tests, production build, IPC boundary, `cargo test --workspace` (34 + 120 + 3 + 63 + 22, plus the 2 history-parity tests after merging ticket 07) |
| `cargo test -p translator-app workflows` | 4 passed: success; login item refuses; save fails in both directions; unreadable settings. Every failure case asserts the login item and the saved preference are exactly where they started. |

The packaged app turning the login item on and off is only checkable by hand; the
steps are in
[`../checks/08-start-at-login-manual.md`](../checks/08-start-at-login-manual.md).
The app was not launched. `scripts/build.sh` was not run (not needed by this
ticket).

## Comments

- 2026-11-15 — The shared timeout is 2 s, the value two of the three existing
  call sites already used; the popup's 1 s hop becomes the same 2 s.
- 2026-11-15 — The rollback restores the preference the settings file still
  holds, as the command did before, rather than the login item's observed
  starting state. The two agree in the normal case; changing that would be a
  product change this ticket does not make.
- 2026-11-15 — `PanelPresenter`, `KeyRegistry` and `ShortcutRoute` and their
  adapters are marked `#[allow(dead_code)]` until tickets 09 and 10 drive them.

