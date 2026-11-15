# 02: Blank text — one rule at the engine's door

**What to build:** The engine, not its callers, decides what an empty Selection means. Text that is empty after trimming never becomes a provider request: every text entry point on the engine returns one explicit outcome — "there is nothing to translate" — with no provider call and no history row. The same rule covers Translate, Improve, and streaming Improve, which today returns an empty improvement instead. The new outcome is worded once by the notice module from ticket 01.

This closes an opening in the engine's interface, not a visible bug: the UI already trims before calling in three places, so nothing user-visible changes on the existing surfaces. Those checks stay as input affordances (a disabled button is still right) but stop being the enforcement point.

**Blocked by:** 01 (Capture notice — one mapper from a failure to words)

**Status:** resolved

- [x] Every engine entry point that accepts text refuses text that is empty after trimming, and none of them consults a provider or records history for it.
- [x] Streaming Improve returns the same outcome instead of an empty improvement, so all modes share one rule.
- [x] The outcome is a wire value the UI maps through ticket 01's notice module, with one message.
- [x] The engine seam proves it with `""`, `"   "`, and `"\n\t "` across every entry point: zero provider calls, zero history rows, one outcome each.
- [x] A test asserts that a non-blank Selection containing leading or trailing whitespace is still translated (the rule trims to decide, it does not rewrite the text).
- [x] The existing UI trim checks remain and carry a comment saying the engine enforces the rule and they only improve the affordance.

## Implementation

- `crates/core/src/error.rs` gains the one new public value:
  `EngineError::NothingToTranslate` (`Display`: "there is nothing to translate").
  Its serde wire value is the bare string `"nothing_to_translate"`; a unit test
  pins that shape.
- The rule lives in `TranslatorEngine::translate_text_in`, as the first statement
  before the mode match, the Settings read, or any provider. `translate_text`
  inherits it by delegation, so both modes of both window entry points are
  covered. The check is `text.trim().is_empty()`; the original `text` is what
  goes to the provider and to history, so surrounding whitespace is preserved.
- `TranslatorEngine::improve_streaming` now returns the same error instead of
  `Ok(Improvement { corrected: "", better: "" })`. That empty-success path is
  deleted, along with the test that asserted it.
- `reopen` is deliberately unchanged. It takes a stored `HistoryEntry`, not
  text, and it makes no provider call and writes no history row; refusing a
  stored row would misdescribe it as a translation request. Rows are only
  written after a successful translation, so new blank rows cannot exist.
- The Selection-capture entry points (`quick_translate`, `replace_selection`)
  take no text argument and are outside this ticket's named text entry points.
  The popup path already classifies a whitespace-only Selection as
  `PopupPrefill::NothingSelected` in `popup_prefill`, so no blank Selection
  reaches them from the UI.
- `ui/src/ipc.ts` adds `"nothing_to_translate"` to the `EngineError` union. It is
  the only string-shaped engine error, so `ui/src/notice.ts`'s `noticeFor` checks
  `typeof error === "string"` before its `in` checks, and returns the one notice:
  kind `capture`, title "There is nothing to translate.", detail "Select or type
  some text, then try again.", no action. No component words it.
- The three trim checks stay as affordances and each now says so in a comment:
  `ui/src/popupState.ts` (popup prefill) and `ui/src/App.tsx` (Translate and
  Improve panes). `App.tsx`'s trim in `changeText` is unrelated (it clears the
  panes when input is emptied) and was left alone.
- No provider failure mapping was touched; ticket 03 owns it.

## Verification

| Command | Result |
| --- | --- |
| `cargo test -p translator-core --test engine_seam` | 78 passed, 0 failed (after the merge; 67 before it — ticket 03 added seam tests) |
| `npm --prefix ui run test` | 10 files, 134 tests passed, including the blank outcome's single wording and its place in the every-variant sweep |
| `./scripts/test.sh` | exit 0 — typecheck, 134 UI tests, production build, IPC boundary, `cargo test --workspace` (34 + 121 + 3 + 78 + 22 + 2) |
| `git merge spec/architecture-deepening --no-edit` | clean; brought in tickets 03, 05 and 08 without conflicts |

The app was not launched and `scripts/build.sh` was not run: this ticket changes
no packaged file, and the UI trims before calling the engine on every existing
surface, so no user-visible change is expected. The wire value is pinned by the
Rust serde test and the TS mapper test rather than by a manual run.

## Comments

- 2026-11-15 — The new outcome is a unit variant, so it serialises as a bare
  string unlike every other `EngineError` (which is an externally tagged
  object). `noticeFor` therefore checks for a string before using `in`; without
  that, an incoming blank outcome threw instead of mapping.
- 2026-11-15 — The rule was not extended to `quick_translate` and
  `replace_selection`: they accept no text, and giving them a different
  blank-Selection behaviour would be a change beyond this ticket's entry points.
  Worth a follow-up only if a caller beyond the popup ever captures a Selection
  directly.
- 2026-11-15 — Code-review finding, resolved here: the earlier comment above was
  adjudicated as the leak story 10 forbids. The platform bridge happens to
  refuse a whitespace-only Selection, but the engine's own interface still
  permitted one, so a future caller could reach a provider with a space. One
  private `is_blank` predicate in `engine.rs` is now shared by
  `translate_text_in`, `improve_streaming`, `read_aloud`, `quick_translate` and
  `replace_selection`; a blank captured Selection answers
  `EngineError::Capture(CaptureError::NothingSelected)` — the classification
  story 1 and `popup_prefill` already use — before any provider call, history
  row or `replace_selection` write. `read_aloud` keeps its empty-success
  behaviour, and `reopen` is untouched. New seam tests drive a fake capture
  returning `""`, `"   "` and `"\n\t "` through both capture entry points
  (zero provider calls, zero history rows, no write) and prove a non-blank
  Selection with surrounding whitespace still translates and replaces
  unchanged. No user-visible string changed.
