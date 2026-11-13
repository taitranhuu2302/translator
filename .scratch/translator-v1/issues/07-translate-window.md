# 07: Translate Window

**What to build:** The Translate Window becomes real. Type or paste text, press Enter, and the translation fills the other pane. The result can be copied or selected in part, and one translation can be reversed without changing the saved Target. This is the surface for text too long for a popup.

**Blocked by:** 04

**Status:** resolved

- [x] Typing or pasting text and pressing Enter translates it into the result pane
- [x] The result is selectable, and copying the result copies only the translation
- [x] A swap control reverses direction for that one translation and leaves the saved Target as it was
- [x] Translating from the window records the same history and cache behaviour as Quick Translate
- [x] The window remembers its frame between launches
- [x] A failure shows in the result pane with the provider named, and does not discard the input

## Verification

The app was **never launched** while building this ticket. The boxes are proven
by the automated suites below; the parts that need a real keyboard, clipboard,
second display and the signed bundle are written up in
[`../checks/07-translate-window-manual.md`](../checks/07-translate-window-manual.md).

| Command | Result |
| --- | --- |
| `npm --prefix ui run test` | 34 passed — 12 new for this ticket: 5 swap-direction rules and 7 result-pane transitions. The 22 popup tests still pass through the re-export. |
| `cargo test -p translator-core` | 69 library tests passed — unchanged engine/provider behaviour |
| `cargo test -p translator-core --test engine_seam` | 44 passed — 2 new: a given direction reaches the provider without saving settings; `translate_text` records history |
| `cargo test -p translator-platform-macos` | 12 passed — settings file and pointer geometry unchanged |
| `cargo test -p translator-app` | 11 passed — 9 new frame geometry/store tests, 2 shortcut routing |
| `cargo clippy --workspace --all-targets` | clean |
| `./scripts/test.sh` | passed (typecheck, vitest, IPC boundary, build, cargo, doctests) |

Proofs for the ticked boxes:

- **Enter translates into the result pane:** `the Translate Window result pane`
  tests pin every transition: started → translating, an answer filling the pane,
  a cleared pane, and a retry after a failure. The input staying editable is a
  property of the component (the textarea is never disabled); manual cases 1 and
  6 confirm the key handling and the in-flight state.
- **The result is selectable and Copy copies only the translation:**
  `ui/src/App.tsx` renders the result as selectable text and `copy()` passes
  only `translation.text` to `ipc.copyText` (AppKit). The state machine makes
  Copy unavailable outside the translated phase. Manual cases 7–9.
- **Swap never writes the saved Target:**
  `translating_in_a_given_direction_uses_it_and_never_saves_the_settings`
  asserts the provider received the passed source/target, the settings pair is
  unchanged, and `InMemorySettings::saves() == 0`; the engine's
  `translate_text_in` does not even load settings when a direction is given.
  `swapDirection` is pure and covered by five tests (reversal, provider
  detection, no self-translation, a third language staying in the pair, and a
  saved target that is the partner). Manual cases 10–12.
- **History and cache parity:** `translate_text` delegates to
  `translate_text_in(text, mode, None)`, so both surfaces share the ladder and
  `CachedProvider`. The existing
  `translating_the_same_text_twice_through_the_engine_asks_the_provider_once`
  proves one provider request; the new
  `translate_text_records_what_it_translated_in_history` proves the window path
  writes the same `HistoryEntry`. Recording remains best-effort. Manual cases 18–19.
- **The frame is remembered, in physical pixels, and off-screen frames are
  dropped:** `crate::frame` has unit tests for on-screen, a negative-origin
  display, a disconnected display, partial overlap, an edge-only touch, zero
  size, no displays, round trip, and a missing/corrupt file. The frame lives in
  `window-frame.json` under `app_data_dir`, separate from `settings.json`.
  Manual cases 13–15.
- **A failure keeps the input:** `reduceResult` turns any `EngineError` into the
  shared typed failure (offline named as offline, provider named where there is
  one) and never touches the component's text; a new Enter starts a fresh
  request from the same input. Manual case 17.

## Decisions

- **Direction is an engine argument, not a second translation path.**
  `TranslatorEngine::translate_text_in(text, mode, Option<Direction>)` joins
  `translate_text`, which delegates with `None`. An explicit direction is
  deterministic even for text the local guess cannot read (digits, emoji), and
  it lets the seam test prove the settings store is never written. Everything
  else — ladder, cache, history — is the same code.
- **Swap reverses the translation on screen.** `swapDirection(pair,
  translation)` takes `translation.target` as the new source and returns to the
  language the text came from: the provider's `detected_source` when it names a
  member of the saved pair, otherwise the pair's other member, never the source
  itself. Clicking Swap moves the result into the source pane and translates it
  back; the reversed direction lives in the window's state and is reset when a
  new Selection opens the window. `Translation` gained no `source` field.
- **Failure wording moved to one module.** `ui/src/failures.ts` now owns
  `formatFailure`, `formatCaptureFailure` and `providerLabel`; `popupState.ts`
  re-exports them so the popup's public shape and its 22 tests are unchanged.
  Two generic sentences became surface-neutral ("Reconnect, then try again." and
  "Try again.") because they now serve both the popup and the window; every
  provider-specific message is untouched.
- **The frame has its own file and a pure rule.** `Frame::is_on_any_screen`
  works on physical rectangles and treats a zero-sized frame and an edge-only
  touch as unusable. `FrameStore` writes atomically and treats a missing or
  corrupt file as "no remembered frame". A dropped frame simply means the window
  keeps the default from `tauri.conf.json`.
- **Saving is debounced; closing hides.** A single worker coalesces a drag's
  move/resize burst into one write after 400 ms. `CloseRequested` is prevented,
  the frame is saved immediately, and the window is hidden, so the same window
  (and its typed text) comes back on the next shortcut instead of being
  destroyed.
- **No Improve mode, streaming or mode switch here.** The ticket is about the
  translate surface; Improve and its stream land with their own ticket.

## Comments

- 2026-11-14 — Implemented in `b6ff368`. The window's keyboard handling,
  clipboard, live frame restore and provider attribution still need a run on the
  signed bundle: [`../checks/07-translate-window-manual.md`](../checks/07-translate-window-manual.md)
  covers Enter/Shift+Enter, the button, empty input, in-flight editing,
  part-selection and Copy, Swap and the untouched settings document, the
  remembered frame across launches and a disconnected display, closing and
  reopening, failures keeping the input, and cache/history parity.
- 2026-11-14 — Review pass added manual case 22: the shortcuts act on the
  window's own Selection. The spec's story 34 was the only one without a case;
  it is now checked and mapped in
  [`../checks/v1-manual-test-plan.md`](../checks/v1-manual-test-plan.md).
