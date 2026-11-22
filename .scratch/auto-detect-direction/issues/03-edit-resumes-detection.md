# 03: Editing the text hands direction back to Detection

**What to build:** After Swap, the Translate Window holds an explicit direction. Editing the Source text clears it, so the next Translate resolves the Direction from the edited text instead of keeping the swapped one. Swap itself still sets the direction and runs its translation with it.

**Blocked by:** 01

**Status:** resolved

- [x] `changeText` clears the window's explicit direction.
- [x] `swap` still sets the direction and translates in it.
- [x] A new Selection prefill still starts with no explicit direction.
- [x] Reopening a history row restores that row's direction unchanged.

## Verification

| Command | Result |
| --- | --- |
| `cargo test --workspace` | 121 / 85 / 49 / 31 / 3 / 2 tests pass, 0 failures |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 14 files, 173 tests pass |
