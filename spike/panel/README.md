# Panel spike

Proves the one thing the architecture depends on and that no document can settle: a Tauri v2
window can become a non-activating `NSPanel` that receives keyboard events from a WKWebView while
the owning app stays inactive, and floats above full-screen apps and every Space.

This is throwaway code. It deliberately does no Accessibility work, no translation and no
synthesis — those are not in doubt.

## Result: passes (2026-11-11)

The design holds. The part that took work is the last mile of keyboard focus, and the cause is
worth remembering when the real app is written:

> `window.ns_view()` returns Wry's `WryWebViewParent`, a plain `NSView` that wraps the real
> `WryWebView` (a `WKWebView` subclass). A plain `NSView` forwards `keyDown:` **up** the responder
> chain, not down into its subviews. `show_and_make_key()` sets the panel's content view as first
> responder, so the panel was key, the app stayed inactive, and every keystroke died in the
> container: no `SPIKE_KEY`, no text in the host app.

The fix is `focus_webview()` in `main.rs`: walk the subviews, find the `WryWebView`, and make that
the panel's first responder after showing.

Evidence gathered while the app was inactive and another app was frontmost:

| Check | How it was verified | Result |
| --- | --- | --- |
| Panel appears without activating the app | frontmost process stays `Finder`/`TextEdit` | pass |
| Webview receives keys while app inactive | `SPIKE_KEY z` after a synthesized keystroke | pass |
| Panel is really key | `isKeyWindow` = true, first responder = `WryWebView` | pass |
| Enter hides, no newline in the host app | panel windows 1 → 0, host text unchanged | pass |
| Escape hides | `SPIKE_KEY Escape`, panel windows 1 → 0 | pass |
| Typing after close goes to the host app | host received `qq` | pass |
| Reopens after hiding | hotkey again → panel windows = 1, keys still arrive | pass |
| Floats over a full-screen app | `isVisible` = true, `isOnActiveSpace` = true, host stays frontmost | pass |
| Appears on the Space you are on | after `⌃→`, same three flags true | pass |

Not verified by automation: that the panel *visually* sits above a full-screen app rather than
merely reporting itself as on the active space. Screen Recording permission is not granted here, so
this needs one look from a person.

## Run

```bash
cargo run --manifest-path spike/panel/Cargo.toml
```

No bundle, no signing and no permission are needed: the spike never reads a Selection and never
posts a key event. The app sets itself to the accessory activation policy at launch, so it has no
Dock icon. Press `⌃⌥D` to show the panel; `Enter` or `Escape` hides it. Quit with `⌃C`.

## What to check by eye

With the panel open over a full-screen app (Safari, Finder): the panel should be visible on top,
the full-screen app should keep its menu bar, and the panel should have no title bar.

## What failure would have looked like

- **The counter never moves** — the webview is not first responder inside the non-activating panel.
  This is the bug that was actually hit; see the result section.
- **The host app loses its active appearance or the panel shows a title bar** — the style mask is
  not being applied; check that `nonactivating_panel()` reaches `setStyleMask:` and that
  `_setPreventsActivation:` is called with `true`.
- **The panel does not appear over full screen** — `FullScreenAuxiliary` is not enough on its own;
  the level may need to be `PanelLevel::PopUpMenu` or higher.

## Notes

- `tauri-nspanel` 2.1.1 does not hardcode `canBecomeKeyWindow`, unlike the version the earlier
  Swift-era ADR was written against; it configures it per panel class.
- The hotkey is registered through `global-hotkey`, which uses Carbon `RegisterEventHotKey` and
  therefore needs no Input Monitoring permission.
- A local `NSEvent` monitor confirmed the keystroke reached the app's event loop even when the
  webview ignored it. That is what separated "the WindowServer is not routing keys here" from
  "the webview is dropping them" — worth reusing as a diagnostic.
