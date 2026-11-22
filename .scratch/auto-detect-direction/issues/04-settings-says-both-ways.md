# 04: Settings describes the pair as bidirectional

**What to build:** The sentence under the Target select says the pair works both ways — `Translates both ways between Vietnamese and English.` — instead of describing only the partner-to-target direction. It keeps naming a language it does not offer by its own code.

**Blocked by:** 02

**Status:** resolved

- [x] `directionSentence` returns the bidirectional sentence for both offered pairs.
- [x] A pair whose partner is not offered still renders by that partner's code.
- [x] Settings has no other copy that promises a one-way flow.

## Verification

| Command | Result |
| --- | --- |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 14 files, 173 tests pass (incl. three two-way sentence tests) |
| Preview `#settings` | paragraph reads `Translates both ways between English and Vietnamese.` |
