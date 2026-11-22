# Auto-detected direction, made visible

Status: done

## Problem Statement

Neris already translates both ways for the English ⇄ Vietnamese Language Pair. Text in the Target is translated into the partner; everything else is translated into the Target. Nothing on screen says which way a result went.

That silence costs the user in three places:

- The Translate Window shows a translation with a `via Azure` badge but no Direction. After Swap, the window keeps translating in the reversed direction until it is reopened, even if the user edits the text — so a later translation can go somewhere the user did not ask for, with no sign that it did.
- The Quick Translate popup shows the provider but not the Direction, so a wrong guess looks like a wrong translation.
- Settings says "Translates from Vietnamese into English.", which reads as one-way even though the pair works both ways.

The detection itself is not missing. `LanguagePair::direction_for` already resolves a Direction locally before any request, and every surface uses it. The work is to make that Direction a first-class part of the answer, show it where a result is shown, and make the behaviour predictable when the user edits what was translated.

## Solution

A `Translation` carries the source language it was translated from, alongside the target it was translated into. Every surface that shows a translation also shows its Direction, in the same words: `Vietnamese → English`.

The Translate Window's Swap becomes a reversal of the recorded direction rather than a second guess: the new source is the old target, the new target is the old source. Editing the Source text clears the window's explicit direction, so the next Translate detects again from the edited text.

Settings describes the pair as bidirectional: both languages are translated into each other.

Detection itself stays exactly as it is: Vietnamese letters and tone marks mean Vietnamese; any other Latin letter means English; text with no recognisable letter makes no guess and falls back to the pair's partner as the source. Unaccented Vietnamese reads as English. That boundary is accepted for v1 and documented rather than fixed.

## User Stories

- As someone typing Vietnamese into the Translate Window, I see `Vietnamese → English` beside the result, so I know the app understood me.
- As someone typing English into the same window, I see `English → Vietnamese`, so a result in Vietnamese is expected rather than surprising.
- As someone who presses Swap, I see the direction reverse, and further Translations keep working unless I edit the text — at which point the app detects again.
- As someone using Quick Translate, the popup names the direction and the provider, so I can tell a wrong guess from a bad translation.
- As someone looking at Settings, I understand that the pair works both ways.

## Implementation Decisions

- `Translation` gains `source: Language`. The engine fills it from the Direction it actually used; `reopen` restores it from the stored `HistoryEntry.direction`, and falls back to the saved pair's partner for rows written before the Direction was stored.
- `swapDirection(translation)` reverses `{source, target}`; it no longer reads the Language Pair or the provider's `detected_source`. The provider's detection stays in the payload as a fact about the answer, not as an input to the rule.
- `directionLabel(direction)` in `ui/src/languageLabel.ts` is the one spelling: full language names joined by `→`. The Translate Window and the popup both use it.
- The Translate Window's result header shows the direction pill beside the existing `via …` pill. The Quick Translate popup shows `Direction · Provider` on the right of its header.
- The window's explicit `direction` state is cleared by `changeText`. Swap still sets it and still runs its translation with it.
- `directionSentence` in Settings becomes `Translates both ways between Vietnamese and English.`
- The `guess_language` doc comment names it as Detection for Direction and Read Aloud, and states that unaccented Vietnamese is read as English by design.

## Consequences

- A translation answer now says which way it ran without the user inferring it from the text.
- Swap is exact: reversing a translation can no longer land in a different language because a provider reported a third language.
- Every UI fixture and Rust test that constructs a `Translation` gains one field. There is no persisted format for `Translation`, so no migration is needed.
- The unaccented-Vietnamese limitation is now written down. Fixing it is a separate feature.

## Out of Scope

- Detecting unaccented Vietnamese, or any language identification beyond the current heuristic.
- Adding a third language to the Language Pair.
- Re-detecting as the user types, before they press Translate.
- Changing what `detected_source` means or removing it from the payload.
