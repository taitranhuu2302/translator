# 05: Write down what Detection does and does not see

**What to build:** The doc comments on `LanguagePair::direction_for` and `guess_language` say that Detection is the app's own, local, and runs before the provider; that it reads Vietnamese letters and tone marks as Vietnamese, other Latin letters as English, and no letter as no guess; and that unaccented Vietnamese is therefore treated as English in v1.

**Blocked by:** None

**Status:** resolved

- [x] `direction_for` says unknown text falls back to the pair's partner as the source.
- [x] `guess_language` names its two callers (Direction and Read Aloud) and the unaccented-Vietnamese boundary.
- [x] The wording matches `GLOSSARY.md` and [ADR-0008](../../../docs/adr/0008-local-direction-resolution.md).
- [x] The stale sentence in `.scratch/translator-v1/spec.md` points at the ADR.

## Verification

| Command | Result |
| --- | --- |
| `cargo test --workspace` | 121 / 85 / 49 / 31 / 3 / 2 tests pass, 0 failures |
| `./scripts/test.sh` | passes end to end |
