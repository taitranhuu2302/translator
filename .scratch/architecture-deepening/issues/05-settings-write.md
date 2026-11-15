# 05: Settings write — build next, save, mirror

**What to build:** One module owns the settings write protocol. Its interface is a single `change(mutation)`: it reads the current settings, builds the next whole object, saves it, and returns what the view may mirror into its shown state. The rule for mirroring is stated once, replacing the three different rules in use today (mirror only when there is no problem, mirror only when the value is live and saved, mirror unconditionally). Failure comes back with one sentence per failure class instead of a sentence per call site.

Target, shortcut rebind, reset, and Azure region all go through it in this ticket; the provider key state follows in ticket 06, which is blocked by this one.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] A settings change cannot be written without going through `change(mutation)`; no view builds a whole settings object inline to save it.
- [x] The module saves a complete object every time, never a partial one.
- [x] The mirror rule lives in the module: a view shows the new value only when the module says it may, with reset expressed as a mutation that is unambiguously saved.
- [x] Failure returns one sentence per failure class, and the view shows it without adding wording.
- [x] Tests cover each of the four mutations: a successful save mirrors, a failed save does not mirror, reset mirrors, and a mutation that changes nothing still leaves shown and saved equal.
- [x] No settings view keeps its own copy of the mirror condition or of a failure sentence.

## Implementation

- `ui/src/settingsWrite.ts` owns the protocol. `change(mutation)` takes one of
  four mutations — `target`, `rebind`, `reset`, `azure_region` — reads the saved
  `Settings` through injected effects, builds the next whole object, saves it,
  and answers a `ChangeOutcome`. `createSettingsWriter(effects)` is the testable
  seam; the exported `settingsWriter` binds the same interface to `ipc` once, so
  the views share one wiring.
- The one mirror rule is now in the union: the success branch carries
  `mirror: Settings`, the failure branch carries `mirror: null`. A view never
  asks "did it work?"; it draws `mirror` or the sentence and nothing else.
  A no-op mutation (same Target, or a region that trims to the saved one)
  saves nothing and returns the current settings, so shown and saved stay equal.
- Reset is mirrored only when the shell reports no `save_problem`. The old view
  mirrored `ResetOutcome.settings` unconditionally, including when the file was
  not written; that copy is gone.
- Failure wording: `READ_FAILED`, `SAVE_FAILED`, `REBIND_FAILED` and
  `RESET_FAILED` are the module's one sentence per class. Target and the Azure
  region both fail a write, so they share `SAVE_FAILED` instead of naming their
  own field. A rebind refused by the engine and a reset whose file write failed
  carry the shell's existing one-sentence wording through; the view adds none.
  `RESET_FAILED` covers a command that could not run at all.
- Deleted from the views: `withTarget`/`withShortcut`/`withAzureRegion` call
  sites, `REBIND_FAILED`, the four per-site failure sentences, and the local
  `refusals()` helper. `SettingsPane` still owns its own non-settings state
  (recording, launch conflicts, login item), and `ProviderSettings` still owns
  its key drafts.
- Scope kept for ticket 06: the provider key save/clear handlers remain direct
  `ipc` calls, and the key sentences in `ProviderSettings` are untouched. The
  extension points are `SettingsMutation` (add key kinds),
  `SettingsWriteEffects` (add save/clear effects) and `ChangeOutcome` (ticket 06
  will need a channel for the refreshed `ProviderStatus` as well as the
  `Settings` mirror). Start-at-login (B2) and onboarding are also outside this
  ticket's four mutations.

## Verification

| Command | Result |
| --- | --- |
| `./scripts/test.sh` | exit 0 — typecheck, 10 UI files / 132 tests, production build, IPC boundary, `cargo test --workspace` (30 + 120 + 3 + 63 + 22) |
| `npm --prefix ui run test -- src/settingsWrite.test.ts` | 14 tests: whole-object saves for Target and region, save/read failures not mirrored with the shared sentence, rebind mirrored without a second save, engine refusal passed through, reset mirrored only when saved, macOS refusals reported, and no-op Target and region leaving shown equal to saved |
| `npm --prefix ui run typecheck` | exit 0 |

The runtime behaviour — which control shows which value after a failed write —
is manual; the steps are in
[`../checks/05-settings-write-manual.md`](../checks/05-settings-write-manual.md).
The app was not launched.

## Comments

- 2026-11-15 — Reset does not read the settings through the UI first: the shell
  command reads the current settings itself, writes the whole shipped object,
  and returns it, so an extra UI read would only add a way to fail before the
  reset is attempted. `change` reads first for Target, the Azure region and a
  rebind, where the next object is built from the saved value.
- 2026-11-15 — A failed reset leaves the confirmation buttons in place so the
  user can retry; the previous code closed them on a `save_problem` response.
  The visible rule is unchanged: the rows keep the saved values.
