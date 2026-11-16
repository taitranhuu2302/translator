# 06: Quick Translate as a floating card

**What to build:** The popup stops being an opaque fixed rectangle and becomes a rounded card floating on a transparent window, sized to its content between 120 and 320 logical pixels, re-placed after resizing so it never drifts off the cursor or off the screen.

**Blocked by:** 01, 02

**Status:** resolved

- [x] `transparent: true` on the popup window; the page body is transparent and the card carries the design-system shadow and a `lg` radius.
- [x] Width stays 420; `alwaysOnTop`, `skipTaskbar`, `decorations: false` and `focus: false` are unchanged.
- [x] Height follows content, clamped to 120–320 logical pixels by a **pure function** with unit tests at both boundaries and inside the range.
- [x] The panel is re-placed after resizing, clamped to the visible screen frame; a popup near a screen edge stays fully visible.
- [x] The popup is composed from native elements styled with Tailwind — **no Radix component**, because a portal dropdown would clip inside 420px.
- [x] ADR-0005's behaviour is intact: the panel does not activate the app, still becomes key, and still hands first responder to the web view.
- [x] Enter still Replaces, Escape still dismisses, and the failure state still re-shows the popup.
- [x] Long answers scroll inside the clamp instead of overflowing the window.
- [x] Typecheck, tests, build and `cargo test --workspace` pass.

## Implementation

- `crates/app/tauri.conf.json`: the `popup` window gains `"transparent": true`.
  `macOSPrivateApi` was already `true`, which a transparent window needs. Width
  stays 420, and `visible: false`, `focus: false`, `decorations: false`,
  `resizable: false`, `alwaysOnTop: true`, `skipTaskbar: true` are untouched.
- `ui/src/index.css` gives `body.popup-page` a transparent background and
  `overflow: hidden`, so the window shows the card and nothing else.
- The clamp is a pure function, `clamp_popup_height` in
  `crates/platform-macos/src/pointer.rs`, beside `MIN_POPUP_HEIGHT = 120` and
  `MAX_POPUP_HEIGHT = 320`. A height that is not finite resolves to the minimum
  rather than being coerced — `NaN` is not a size. Four unit tests cover the
  floor, the middle, the ceiling and `NaN`; the module's suite is 12 tests.
- The page measures, the shell decides. `ui/src/PopupView.tsx` adds up the outer
  padding (10px a side), the card's border and the three measured regions —
  header, content, footer — and reports the total through
  `ipc.setPopupHeight`. A `ResizeObserver` on all three re-reports whenever the
  answer, the failure or the footer changes; identical numbers are not re-sent.
  The measurement effect deliberately has **no dependency list**: a dismissed
  popup draws no card, so the next opening has to attach to elements that did not
  exist at mount.
- The measured box is the padded one: the scroll region keeps no padding of its
  own and the inner content div carries `px-3.5 py-3`. Without that the reported
  height omitted the padding and the first card clipped its last line — found by
  looking at the harness, not by reasoning.
- `crates/app/src/panel.rs`: `show` anchors (one `screen_under_cursor` per
  opening, kept in a `static ANCHOR`) and places; `set_height` resizes to the
  clamped content height and re-places **from that stored anchor**, never from
  the live cursor, so the card cannot crawl while the mouse travels to Copy. A
  hidden panel is resized but not moved. `size_and_place` is gone: resetting to
  the 220 default on show would have left a re-opened popup in a window it had
  already been fitted to, with dead transparent space around it.
- `crates/app/src/main.rs` adds `set_popup_height`, a thin async command that
  hops to the main thread through `main_thread::on_main_thread`, like every other
  panel operation. `ui/src/ipc.ts` exposes it as `setPopupHeight`.
- The card is native elements + Tailwind only: a `<main>` with the outer padding,
  a `bg-card` div with `shadow-lg`, an `<header>`, one scroll region and a
  `<footer>`. No portal component appears, so nothing can be clipped by a 420px
  window. `NoticeView` — which is `Alert` markup, portal-free — is what a failure
  draws, so the popup and the window word their failures the same way.
- ADR-0005 is untouched: `prepare` still converts the window to a
  non-activating `NSPanel`, `show` still calls `show_and_make_key`, and
  `focus_webview` still hands first responder to the `WKWebView`.
- The keyboard and dismissal behaviour is unchanged — Enter replaces, Escape
  dismisses, a blur that is not Replace's own dismisses, and a refused replace
  keeps the answer and its Copy button on screen with the reason inline.

## Verification

| Command | Result |
| --- | --- |
| `npm --prefix ui run typecheck` | clean |
| `npm --prefix ui run test` | 13 files, 172 tests passed |
| `npm --prefix ui run build` | clean |
| `cargo test -p translator-platform-macos` | 28 passed, 0 failed — 12 in `pointer::tests`, four of them the clamp |
| `cargo test --workspace` | 285 passed, 0 failed |
| Popup harness (`/preview-popup.html`): Translated | card 168px, nothing clipped |
| Popup harness: Long answer | page reports 432, window clamps to 320, the answer scrolls inside |
| Popup harness: No key | 195px, `Alert` for the missing Groq key |
| Popup harness: Translating | 164px, spinner in a `role="status"` line |

The app itself was not launched: the clamp is pinned by Rust tests, and the card
was judged in the harness at the exact width and both height bounds the shell
would impose.

## Comments

- 2026-11-16 — `set_height` exists because the alternative (Rust measuring the
  web content) would have needed a second source of truth for the card's size.
  The page knows what it drew; the shell knows the bounds and the screen.
- 2026-11-16 — Two bugs were caught by looking rather than by tests: the padding
  left out of the reported height, and `show` resetting a size the page had
  already reported. Both are described above so the next change does not
  reintroduce them.
