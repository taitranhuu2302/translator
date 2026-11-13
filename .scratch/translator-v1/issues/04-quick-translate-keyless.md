# 04: Quick Translate with the keyless provider

**What to build:** Selecting Vietnamese text and pressing `⌃⌥D` opens a popup beside the cursor showing the English translation, with the source app still focused and the caret still where the user left it. It works on first launch, before any key exists, because it uses the keyless provider. This is the slice that proves the whole Quick Translate shape: popup placement, focus behaviour, direction, chunking, and caching.

**Blocked by:** 02

**Status:** resolved

- [x] `⌃⌥D` with a Selection opens the popup next to the cursor and shows a translating state immediately, before any network response
- [x] The translation replaces the translating state in the same popup
- [ ] The popup becomes key so Enter and Escape reach it, without activating the app: the source app keeps its menu bar and stays frontmost
- [ ] When the popup closes, typing goes back to the source app with the caret where it was
- [x] With Target set to English, a Vietnamese Selection translates to English and an English Selection translates to Vietnamese
- [x] Text longer than the keyless provider's per-request byte cap is split and rejoined in order, with Vietnamese diacritics intact
- [x] Translating the same Selection twice returns the second result without a network request
- [ ] Escape dismisses the popup and leaves the document untouched
- [ ] The popup appears on the active Space and over a full-screen app
- [x] With no network, the popup reports an offline failure specifically rather than a generic error

## Verification

The app was **never launched** while building this ticket. The ticked boxes are
proven by the automated suites only; the unticked ones need a human at the
machine and are written up in
[`../checks/04-quick-translate-manual.md`](../checks/04-quick-translate-manual.md).

| Command | Result |
| --- | --- |
| `npm --prefix ui run test` | 8 passed — popup transitions, reopen, and typed-error formatting |
| `cargo test -p translator-core` | 38 library tests passed — chunking, MyMemory parsing, ladder, cache |
| `cargo test -p translator-core --test engine_seam` | 33 passed — popup prefill, direction, caching through the engine |
| `cargo test -p translator-platform-macos` | 11 passed — placement, bottom-edge flip, clamping, DPI conversion |
| `cargo test --workspace` | all passed |
| `cargo clippy --workspace --all-targets` | clean |
| `./scripts/test.sh` | passed |

Proofs for the ticked boxes:

- Opening state before the network: `shows the translating state before any answer exists` (vitest) and `the_popup_starts_with_the_selection_captured_before_it_was_shown` (seam).
- Translation replaces it: `replaces the translating state with the translation in the same popup` (vitest).
- Direction: `a_vietnamese_selection_is_translated_into_the_english_target`, `with_vietnamese_as_the_target_an_english_selection_goes_to_vietnamese`, `an_english_selection_is_translated_into_the_partner_when_english_is_the_target` (seam).
- Long text: `a_five_hundred_character_vietnamese_text_stays_one_chunk`, `a_long_text_is_split_and_rejoins_losslessly_with_diacritics_intact` (core), `long_text_is_chunked_by_characters_and_rejoined_in_order` (MyMemory).
- Cache: `the_same_text_and_direction_is_answered_without_a_second_request` (core) and `translating_the_same_text_twice_through_the_engine_asks_the_provider_once` (seam).
- Offline: `an_unreachable_network_is_reported_as_offline` (MyMemory) and `names an offline failure as offline, not as a generic error` (vitest).

## Decisions

- **Characters, not bytes.** The ticket calls the cap a byte cap; MyMemory's
  limit is 500 Unicode scalar values. Verified against the live API: 500
  Vietnamese characters (630 bytes) succeeds, 501 characters fails. Chunking
  therefore cuts at 500 characters, prefers a sentence end, then a space, and
  never splits inside a scalar value. `a_five_hundred_character_vietnamese_text_stays_one_chunk`
  pins the distinction.
- **Best-effort history.** Until ticket 09 provides a real store, a rejected
  history write is logged and the translation still succeeds. A translation is
  never lost because the history stub is not configured.
- **HTTP behind a port.** `core` owns an `HttpClient` trait; the blocking
  `ureq` adapter lives with the provider and tests inject a scripted fake, so no
  test touches the network. A transport failure before any answer is `offline`;
  a request that happened and failed is a typed network error.
- **Placement is AppKit's cursor, not Tauri's.** `Window::cursor_position()` is
  wrong on Retina and mixed-DPI desktops in the tao version Tauri ships. The
  cursor and the visible frame come from `NSEvent`/`NSScreen` through
  `platform-macos`, and a pure function turns cursor + popup size + screen into
  a top-left point, so the flip and clamp rules are tested without a window.
- **Size first, then place.** The popup's size is set and read back before
  positioning, so the bottom-edge flip and the clamp use real dimensions rather
  than a guess.
- **`panel.to_window()` is destructive.** In `tauri-nspanel` 2.1.1 it converts
  the panel back into a regular window and deregisters it. Sizing and moving go
  through the Tauri `WebviewWindow` handle, which wraps the same `NSWindow`, so
  the panel survives.
- **Copy through AppKit.** The popup's Copy button writes the pasteboard
  through `platform-macos` rather than the web view's clipboard API.
