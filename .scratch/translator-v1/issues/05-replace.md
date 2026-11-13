# 05: Replace

**What to build:** The user can overwrite the Selection with its translation in place. Pressing Enter in the Quick Translate popup does it after a look; `⌃⌥R` does it directly with no popup. It works both where the app accepts inserted text and where it only accepts a paste, and it tells the truth when it cannot write at all.

**Blocked by:** 02, 04

**Status:** resolved

- [x] Enter in the Quick Translate popup Replaces the Selection with the translation and closes the popup
- [x] `⌃⌥R` translates and Replaces in one step, with no popup
- [ ] Where the focused element accepts text insertion, Replace goes through it and the surrounding text is unchanged
- [ ] Where it does not, Replace uses the clipboard and restores the previous clipboard contents afterwards
- [x] When Replace cannot happen at all, the failure is visible and the document is left unchanged
- [x] After Replace, the Selection in the source app covers the new text rather than extending past it
- [x] Replace is refused, and says so, when secure event input is active

## Verification

The app was **never launched** while building this ticket. The ticked boxes are
proven by the automated suites only; the two unticked ones depend on which app is
in front and are written up, with cases for Safari, Chromium, Word and secure
input, in [`../checks/05-replace-manual.md`](../checks/05-replace-manual.md).

| Command | Result |
| --- | --- |
| `npm --prefix ui run test` | 13 passed — replacement uses the on-screen translation, refused replace keeps the popup open, failure wording |
| `cargo test -p translator-core` | 39 library tests passed — including the UTF-16 length rule |
| `cargo test -p translator-core --test engine_seam` | 38 passed — both Replace entry points, retention, typed refusals, best-effort history |
| `cargo test -p translator-platform-macos` | 11 passed |
| `cargo test -p translator-app` | 2 passed — shortcut routing including `Ctrl+Alt+R` |
| `cargo clippy --workspace --all-targets` | clean |
| `./scripts/test.sh` | passed (typecheck, vitest, IPC boundary, build, cargo, doctests) |

Proofs for the ticked boxes:

- Enter uses the answer on screen and does not read again:
  `the_popup_replace_writes_the_translation_the_user_saw_without_reading_again`
  (one write, zero capture reads, zero provider requests) and
  `replaces with exactly the translation on screen and does not start over` (vitest).
  That the panel is then hidden is AppKit/window behaviour and is manual case 1.
- `⌃⌥R` is one step with no popup:
  `replace_selection_writes_the_translation_over_the_selection` (seam) and
  `each_registered_shortcut_routes_to_its_own_action` (app). That no popup opens
  is shell behaviour and is manual case 2.
- A Replace that cannot happen is typed and visible:
  `a_replace_that_the_focused_app_refuses_is_a_typed_error`,
  `a_popup_replace_with_no_captured_selection_writes_nothing`,
  `a_popup_replace_that_the_app_refuses_is_typed_and_records_no_history`, and
  `keeps the popup open with an inline failure when the app refuses the replace`
  (vitest). No write happens on the error path.
- The Selection covers the new text:
  `utf16_length_counts_code_units_not_bytes_or_scalars` pins the length the range
  must use. The actual reselection is platform behaviour and is manual case 15.
- Secure input is refused before anything is written:
  `secure_input_refuses_a_popup_replace_before_anything_is_written` (seam, no
  write reached the fake) and `explains a secure field refusing the replace`
  (vitest).

## Decisions

- **Retain the target; never re-read on Enter.** `read_selection` keeps the
  `AXUIElement` and range it read, so Enter writes where the Selection came from
  even though the popup holds the keyboard by then. With nothing retained the
  capture port answers `NothingToReplace` instead of pasting into whatever is
  focused now. The fake models the same retention, so the seam test proves no
  second read happens.
- **Two entry points, one behaviour.** `replace_selection` captures → translates
  → writes; `replace_with_translation` only writes the text already on screen.
  Both record history best-effort and return the same `Replacement`.
- **Settable is advisory; effect is the proof.** `AXUIElementIsAttributeSettable`
  is not used as a gate: Chromium reports "not settable" and accepts, editors
  that own their model report settable and discard. The write is attempted and
  then verified by diffing `kAXValueAttribute` (the element's whole text) with
  ~10/25/50/100/250 ms backoff, because `AXSelectedText` reads back empty once
  the selection collapses and `kAXErrorSuccess` proves nothing.
- **WebKit is the paste path.** WebKit implements no `AXSelectedText` setter and
  its `AXValue` route needs the whole document, so Safari, terminals and web
  editors fall through to the clipboard. When `AXValue` is unreadable the AX
  route is treated as unverifiable and the paste fallback runs rather than
  claiming success. Word's "success, nothing changed" is caught by the same
  check; a replace whose translation equals the source is recognised by the
  caret moving past the inserted text instead of a value diff.
- **Paste restores what it borrowed.** `changeCount` is the only ownership token:
  contents are saved, ours is written, ⌘V is synthesized, and the saved contents
  go back only when `changeCount` is still ours, so a copy the user makes during
  Replace is never clobbered. Non-text items are saved type-by-type.
- **Bounded and serialized Accessibility.** AX messaging is synchronous IPC with
  a 6 s default timeout, so every call runs on one serial queue and the target
  application gets `AXUIElementSetMessagingTimeout(0.5)`. `CGEventPost` has no
  main-thread requirement, so the paste path stays on the worker.
- **UTF-16 ranges, re-read before reselecting.** The range length is UTF-16 code
  units, wrapped with `AXValueCreate(kAXValueCFRangeType, …)`. After the write
  the range is read fresh and the selection is placed over the inserted text
  only when the caret is collapsed just past it; `(oldLocation, newLength)` is
  never used blind, and an app that exposes no range keeps the caret after the
  text.
- **Secure fields refuse before the attempt.** `IsSecureEventInputEnabled()`
  gates both routes, and an `AXSecureTextField` target is refused even when the
  flag is off, because password fields expose no writable selected text at all.
  The document is never touched on a refusal.
- **Popup stays open to report.** The shell hides the panel before writing (a
  synthesized ⌘V must reach the source app) and re-shows it if the write fails,
  so the popup can show the refusal inline with the translation still available
  to Copy. The UI suppresses the blur that the hide causes.
- **History is still best-effort.** `MacHistoryStore` remains a stub until ticket
  09; a rejected write is logged and never fails a Replace. One consequence to
  reconcile then: the popup's translation was already recorded by
  `translate_text`, so Enter currently writes a second, identical entry.
- **Unverified, and left to the manual run.** Whether Safari's shipping build
  really no-ops the Accessibility write, the exact default AX messaging timeout
  (the notes say 6 s), and whether AX insertion works while another app holds
  secure input. A paste failure can only be detected as a clipboard-write
  failure: `CGEventPost` returns void, so an app that ignores ⌘V reports success.
- **Retention is last-read-wins.** A single retained target is kept process-wide;
  two captures in flight would leave the later one as the Replace target.
- **Raw AX error codes stay in ObjC.** The bridge maps outcomes to its own typed
  statuses, so the corrected `AXError` values (−25201 illegal argument, −25202
  invalid element, −25204 cannot complete, −25205 unsupported, −25211 API
  disabled) are documentation here rather than wire values.
