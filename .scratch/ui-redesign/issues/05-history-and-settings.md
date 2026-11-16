# 05: History panel and Settings sheet

**What to build:** History and Settings stop being destinations that replace the work. History becomes a `Sheet` sliding from the right with its search field, list and empty state; Settings becomes a sheet holding the same five sections in the same order.

**Blocked by:** 02, 03

**Status:** resolved

- [x] History is a `Sheet` from the right; the panes stay visible beside it.
- [x] Search, the list, the empty state and the failure state all work as they do today.
- [x] Selecting a History entry still restores its text into the panes, exactly as today.
- [x] Settings is a sheet with Language Pair, Startup, Shortcuts, Providers and Reset, in that order, scrolling independently of the window.
- [x] Every control in Settings uses the installed primitives; no raw `<select>` or unstyled `<input>` remains.
- [x] Provider key rows keep their consolidated behaviour and wording from the architecture effort.
- [x] Both sheets close on Escape and return focus to the control that opened them.
- [x] The harness renders both, including the empty History state and the settings failure state.
- [x] Typecheck, tests and build pass.

## Implementation

- `ui/src/components/HistorySheet.tsx` is `Sheet` + `SheetContent side="right"`
  at a fixed 400px. It is an overlay, not a destination: `App.tsx` keeps the
  panes mounted underneath, and the sheet closes back onto them.
- The sheet owns no data. `App.tsx` reads History once when the sheet opens
  (`openHistory`), holds `history`, `historyQuery` and `historyError`, and passes
  them in; the field filters locally through the existing `filterHistory`, which
  is where "search behaves as it does today" still lives, unchanged and still
  covered by `historyView.test.ts`.
- Rows are buttons: choosing one calls the existing `openHistoryEntry`, which
  restores the entry into the panes exactly as before, then closes the sheet.
  Empty History draws `EmptyState` with `EMPTY_HISTORY`; a read failure draws
  `NoticeView` in place of the list.
- `ui/src/components/SettingsSheet.tsx` is the other `Sheet` (right, 400px). It
  takes `settings`, `error` and `onSettingsSaved` and renders
  `ui/src/SettingsPane.tsx`, which keeps its five `Section`s in the existing
  order — Language Pair, Startup, Shortcuts, Providers, Reset — inside its own
  scroll region.
- The sheet is tolerant of a settings read that has not finished or has failed:
  `settings` may be `null`, in which case the body is either the loading line or
  the failure `NoticeView` — never a crash and never an empty panel.
- Every control in `SettingsPane.tsx` is an installed primitive: `Select` for the
  language pair, `Switch` for startup, `Input`s for text and key fields,
  `Button`s and `Kbd`s for shortcuts. `ProviderKeyRow.tsx` and
  `ProviderSettings.tsx` keep the consolidated rows from the architecture effort
  word for word; only their markup changed.
- Escape and focus return come from the primitive (Base UI's dialog): the sheet
  traps focus while open and hands it back to the header button that opened it.
  That was checked in the harness rather than assumed.

## Verification

| Command | Result |
| --- | --- |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 13 files, 172 tests passed (`historyView`, `settingsView`, `settingsWrite`, `providerKey`, `providerStatus` among them) |
| `npm --prefix ui run build` | clean |
| `bash scripts/test.sh` | exit 0 |
| Design preview: `history-empty`, `history-list`, `history-failure`, `settings`, `settings-failure` | all five render; the panes stay visible beside each sheet |
| Design preview, Escape / focus | the sheet closes on Escape and focus returns to the opener |

## Comments

- 2026-11-16 — History and Settings became overlays, so the old `surface` state
  in `App.tsx` is gone; two booleans replace it. Nothing else about reading,
  filtering or restoring History changed.
- 2026-11-16 — The settings failure state was missing from the first version of
  the harness (the sheet was always handed mock settings). Added as
  `#settings-failure`, which is what proves the sheet's null-settings branch.
