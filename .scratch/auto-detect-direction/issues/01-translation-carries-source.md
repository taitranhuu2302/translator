# 01: A Translation carries the language it was translated from

**What to build:** `Translation` gains `source: Language` beside `target`. Every translation path fills it from the Direction the engine actually used, and a reopened history row restores it, so a surface never has to infer the Direction from the text or from a provider's detection.

**Blocked by:** None

**Status:** resolved

- [x] `Translation` has `source: Language`, documented as the language the text was translated from.
- [x] Quick Translate and Replace fill `source` from the resolved Direction.
- [x] A Translate Window translation fills `source`, with or without an explicit direction.
- [x] `reopen` restores `source` from the stored direction; an older row with no direction falls back to the saved pair's partner.
- [x] `swapDirection(translation)` reverses `{source, target}` and reads neither the Language Pair nor `detected_source`.
- [x] Engine-seam tests assert the recorded direction on each path and on reopened rows.
- [x] UI tests cover the reversal.

## Verification

| Command | Result |
| --- | --- |
| `cargo test --workspace` | 121 / 85 / 49 / 31 / 3 / 2 tests pass, 0 failures |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 14 files, 173 tests pass |
