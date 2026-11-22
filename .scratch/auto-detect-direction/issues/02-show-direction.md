# 02: Every result names its Direction

**What to build:** Where a translation is shown, say which way it went. The Translate Window's result header gains a `Vietnamese → English` pill beside `via Azure`; the Quick Translate popup shows `Vietnamese → English · Azure` where it showed only the provider.

**Blocked by:** 01

**Status:** resolved

- [x] `ui/src/languageLabel.ts` owns `OFFERED_LANGUAGES`, `languageName` and `directionLabel`, which joins full language names with `→`.
- [x] `SettingsPane` and `settingsView` read the language names from that module.
- [x] The Translate Window result header shows the direction pill for a translated result; Improve is unchanged.
- [x] The Quick Translate popup shows the direction and provider once an answer exists.
- [x] `directionLabel` has tests; the long popup header fits its 420 px window.

## Verification

| Command | Result |
| --- | --- |
| `cargo test --workspace` | 121 / 85 / 49 / 31 / 3 / 2 tests pass, 0 failures |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 14 files, 173 tests pass (incl. `languageLabel.test.ts`) |
| Preview `#translate-done` | header pill reads `English → Vietnamese · Azure` |
| Preview `preview-popup.html` | 420 px header reads `Vietnamese → English · Azure` on one line |
