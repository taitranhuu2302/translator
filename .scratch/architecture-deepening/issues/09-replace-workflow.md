# 09: Replace — the hide, replace, restore order in one place

**What to build:** Replace becomes a workflow over ticket 08's seam: hide the Quick Translate popup, replace the Selection, and on failure show the popup again so the user can retry or copy. The two failure paths stay distinguishable — a failed Replace, and a popup that cannot be brought back — so the shell can still report the second one without confusing it with the first.

**Blocked by:** 08 (Shell workflow seam — and start at login as its first workflow)

**Status:** resolved

- [x] The hide → Replace → restore ordering lives in one workflow, not in a command.
- [x] The command only hops to the main thread and maps the result.
- [x] A failed Replace restores the popup; a failure to restore it is reported separately and logged, never swallowed.
- [x] Tests with a fake presenter assert: success hides once and shows never; a failed Replace hides once and shows once; a presenter that refuses to show is reported as its own outcome.
- [x] Replacing still writes the translation into the app that owns the Selection, and still refuses when no Selection was captured.
- [x] The popup's own Replace path and any other path that replaces behave the same way, because they share the workflow.

## Implementation

- `workflows::replace(panel: Option<&dyn PanelPresenter>, replace: impl FnOnce()
  -> Result<Replacement, EngineError>) -> ReplaceOutcome` in
  `crates/app/src/workflows.rs` is the one ordering: hide, run the Replace, and
  on failure show the popup again. `ReplaceOutcome::{Replaced, Failed}` carries
  the result, and `Failed { error, restore }` keeps the two failure paths apart:
  `PopupRestore::Restored` means the user can retry, `PopupRestore::Failed(
  problem)` means the popup is gone as well, and `PopupRestore::NoPopup` says
  the caller had no popup in its flow.
- `replace_with_translation` in `main.rs` shrank to one
  `main_thread::on_main_thread` hop that runs the workflow with
  `ShellPanelPresenter`, plus a `replace_result` mapping. The two hops and the
  inline hide/match/show block the command carried are deleted. Two log lines
  stay, in the workflow and in `replace_result`: a popup that would not hide
  (the clipboard fallback would paste into it) and a popup that could not come
  back after a failed Replace. A hop that never runs now answers with a typed
  `CaptureError::Unavailable` instead of an untyped message.
- The other two callers replace through the same function with `None`: the
  `replace_selection` command and `replace_selection_directly` (the `⌃⌥R`
  shortcut and the tray's Replace row). Neither has a popup — that path
  translates and writes without one — and both must stay off the main thread
  because the Replace includes a provider request. Passing `None` keeps the
  operation and its typed failure on the one workflow without changing either
  behaviour: no popup is shown, and direct failures still reach
  `show_replace_failure`'s dialog.
- Removed the `#[allow(dead_code)]` markers from `PanelPresenter`,
  `ShellPanelPresenter` and their impls; the other two port pairs stay marked
  until ticket 10 drives them. No `core`, UI, or shortcut-planning code changed.

## Verification

| Command | Result |
| --- | --- |
| `./scripts/test.sh` (after merging `spec/architecture-deepening`) | exit 0 — identity check, typecheck, 134 UI tests in 10 files, production build, IPC boundary, `cargo test --workspace` (39 app + 121 core + 3 dependency-boundary + 78 engine-seam + 24 platform + 2 history-parity, 0 doctests) |
| `cargo test -p translator-app workflows` | 9 passed: the 5 Replace cases below plus the 4 start-at-login cases. |
| `rustfmt --edition 2021 --check crates/app/src/workflows.rs crates/app/src/main.rs` | clean. |

The fake presenter drives all five Replace cases — success (hide only), failed
Replace (hide then show), a presenter that refuses to show (reported as its own
`PopupRestore::Failed` beside the Replace error), and both no-popup cases — so
the ordering is asserted, not assumed. What the real AppKit panel does when it
is hidden and shown again is only checkable by hand; the steps are in
[`../checks/09-replace-manual.md`](../checks/09-replace-manual.md). The app was
not launched.

## Comments

- 2026-11-15 — The `⌃⌥R` paths pass `None` for the presenter rather than
  running inside the hop. They have no popup to hide, and the Replace includes a
  network request that must not run on the main thread (it would freeze the app
  and could exceed the shared 2 s timeout). The ticket's "the command only hops"
  describes the popup command; the no-popup paths still share the workflow and
  its result type.
- 2026-11-15 — A popup that fails to hide does not stop the Replace: it is
  logged and the operation proceeds, because a Replace refused on that basis
  would be a regression. The log exists because the clipboard fallback's
  synthesized ⌘V would otherwise land in our own page.

