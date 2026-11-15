# 01: Capture notice — one mapper from a failure to words

**What to build:** Every capture failure speaks with one voice. A single module turns a capture failure or an engine error into a notice model — a kind, a title, a detail, and at most one action — and both the Quick Translate popup and the Translate Window draw that model. Neither surface keeps its own wording. The drift that already exists between them (Accessibility, secure input, and the fallback notice each say something different on the two surfaces for the same cause) is resolved by choosing one text per case.

**Blocked by:** None (can start immediately)

**Status:** resolved

- [x] Every `EngineError` variant and every capture failure kind produces exactly one notice model; the model is the only source of title, detail, and action.
- [x] Quick Translate and the Translate Window show the same title and the same detail for the same cause; no surface adds, trims, or rewords a notice.
- [x] The styling kind is derived from the same model, so a wire value is mapped once rather than once per surface.
- [x] The existing drift is settled: one surviving text for Accessibility, one for secure input, one for the fallback.
- [x] Tests cover every variant and kind against the model, plus one test asserting that a surface renders the model without adding wording of its own.
- [x] No capture failure reaches the user as a silently empty surface; every path still reports something, as it does today.

## Implementation

- `ui/src/notice.ts` is the one mapper. `noticeFor(EngineError)` routes capture and
  provider failures to `noticeForCapture` and `noticeForProvider`; the model is
  `Notice { kind, title, detail, action? }`. Every capture kind and every provider
  variant has its own wording, and an unknown wire value still gets a notice
  rather than nothing. The resolved Accessibility state
  (`accessibilityGrantedNotice`) lives in the same module, so no surface invents
  a sentence.
- Only the Accessibility cause carries an action —
  `open_accessibility_settings`, labelled "Open System Settings" —
  and `ui/src/noticeActions.ts` is the one place it becomes an IPC call.
- `ui/src/NoticeView.tsx` is the one renderer. `CaptureNoticeView` only selects
  the resolved notice when a grant lands while the window stays open. The
  popup's failure states, the window's capture notice, and both window result
  panes draw through it. `ui/src/failures.ts` and `format.ts::captureFailure` are
  deleted; `popupState.ts`, `translateWindow.ts` and `improveWindow.ts` carry the
  notice fields in their failed states.
- The styling kind lives in the model (`capture`, `offline`, `provider`,
  `error`, `ok`), and the surfaces pass only their layout class. Wording drift
  settled to one text per cause; `providerLabel` moved to `format.ts` beside the
  other display formatting.

## Verification

| Command | Result |
| --- | --- |
| `./scripts/test.sh` | exit 0 — typecheck, 9 UI files / 118 tests, production build, IPC boundary, `cargo test --workspace` (30 + 120 + 3 + 63 + 22) |
| `npm --prefix ui run test` | 9 files, 118 tests: every `EngineError` variant, every capture kind, both doors into the capture mapper, the model's one action, the view adding no wording, and the resolved-grant notice |
| `./scripts/build.sh` | exit 0 — packages and signs `target/bundle/Translator.app` with `Translator Local Signing` |

Runtime wording and the resolved-grant state are manual; the steps are in
[`../checks/01-capture-notice-manual.md`](../checks/01-capture-notice-manual.md).
The app was not launched.

## Comments

- 2026-11-14 — Review pass: the Translate Window's result and Improve panes now
  draw `NoticeView` instead of hand-rendered markup, so a model action is never
  dropped; the resolved Accessibility sentence became
  `accessibilityGrantedNotice` in the model, with `CaptureNoticeView` only
  choosing which notice applies; `nothing_selected` no longer names the Quick
  Translate shortcut, since the same text shows in the window; non-provider
  engine failures use an `error` kind instead of posing as provider failures.
- 2026-11-14 — The full suite was flaky in `translator-platform-macos`. The
  cause was test-only: each `temp_path` helper used `SystemTime::now()` alone,
  so two parallel tests could share a directory and one would write
  `this is not sqlite` into the database another was using. The history,
  settings and window-frame helpers now include the process id and an atomic
  counter. No production code changed.

