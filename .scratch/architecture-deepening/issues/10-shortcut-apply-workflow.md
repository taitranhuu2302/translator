# 10: Shortcut apply — one sequence for rebind and reset

**What to build:** The apply half of shortcut rebinding moves behind ticket 08's seam as one sequence: register the new binding, unregister the old one, set the route, save the settings, and on any failure after the old binding is gone, undo both — unregister the new binding, re-register the old one, restore the old route. Resetting shortcuts reuses the same sequence instead of repeating it. The pure planning half is unchanged.

**Blocked by:** 08 (Shell workflow seam — and start at login as its first workflow)

**Status:** resolved

- [x] The apply sequence is one function over the registry and route ports, in the stated order, with the rollback stated once.
- [x] Reset reuses that sequence; there is no second copy of the ordering.
- [x] The pure planning function and its behaviour are untouched.
- [x] Tests assert the order through fakes, and assert that a forced save failure leaves the old binding registered, the new one unregistered, and the route back where it was.
- [x] A successful rebind leaves exactly one binding registered for the action.
- [x] A shortcut that the system refuses to register is reported with the existing wording and changes nothing.

## Implementation

- `crates/app/src/workflows.rs` now holds the apply half. `apply_shortcut(&dyn
  KeyRegistry, &dyn ShortcutRoute, &dyn SettingsStore, Settings,
  ShortcutChange) -> ShortcutApplyOutcome` is the one sequence: register the new
  combination (a refusal returns the existing `refused_message` before anything
  changes), unregister what it replaces, point the route at the new one, save the
  settings, and on a refused save unregister the new one, re-register the old
  one and restore the old route. The rollback is written once, in this function.
- `rebind_shortcut(registry, route, settings_store, action, requested) ->
  Result<RebindOutcome, ()>` is the whole rebind: load the settings, decide with
  the untouched pure `shortcuts::plan`, then call `apply_shortcut`. Invalid,
  in-use and unchanged plans never touch a port.
- `ShortcutRoute` gained `live(action)`; `apply_shortcut` releases the actually
  registered old binding (falling back to the stored `previous`), and
  `reset_shortcuts` uses it to free every live registration first.
- `reset_shortcuts(registry, route, settings_store) -> Result<ResetOutcome, ()>`
  releases each action's live route, then runs every shipped default through
  `apply_shortcut` against a private `CollectedSave` sink, and writes the
  collected settings once at the end. A reset is still one write by contract; a
  refused final write keeps the existing "defaults are live, but …" sentence.
- `shortcuts.rs` keeps only the pure planning half and the launch-time
  registration. `rebind_shortcut`, `reset_shortcuts_and_language_pair` and
  `unregister_on_main_thread` are deleted. `with_binding` and `is_the_same` are
  now `pub(crate)`.
- The two Tauri commands in `main.rs` each perform one `main_thread::on_main_thread`
  hop and call the workflow with `ShellKeyRegistry`/`ShellShortcutRoute`. After
  the hop returns (never inside it) the command clears the launch conflict and
  refreshes the tray, preserving the previous ordering.
- No `core` or UI code changed; ticket 09's Replace workflow is untouched.

## Verification

| Command | Result |
| --- | --- |
| `./scripts/test.sh` | exit 0 — typecheck, UI tests, production build, IPC boundary, `cargo test --workspace` (43 app + 121 core + 3 dependency-boundary + 78 engine-seam + 24 platform + 2 history-parity, after merging ticket 06) |
| `cargo test -p translator-app workflows` | 13 passed: the 4 start-at-login tests plus 9 shortcut-apply tests — successful order, refused registration, forced save failure with rollback, unchanged plan, in-use plan, unreadable settings (rebind and reset), reset order with one write, and a shipped default the system refuses |

Carbon registration, the menu bar refresh and the settings-permission cases are
only checkable by hand; the steps are in
[`../checks/10-shortcut-apply-manual.md`](../checks/10-shortcut-apply-manual.md).
The app was not launched and `scripts/build.sh` was not run.

## Comments

- 2026-11-15 — Reset drives each shipped default through `apply_shortcut` with a
  private sink that records nothing, then writes the collected settings once.
  That keeps ticket 05's documented reset behaviour (defaults live, one write,
  `save_problem` when the write fails) while removing the second copy of the
  register → route ordering.
- 2026-11-15 — `ShortcutApplyOutcome.applied` means the registration, route and
  settings object all took the new binding. For a reset the real write is still
  pending, which is why reset's statuses come from the same outcome but its
  single save reports separately.
- 2026-11-15 — The rollback restores the stored `previous` binding, matching the
  implementation it replaces, even when the live route had drifted from the
  stored value.
