# Accessibility first, clipboard as the fallback

No single method reads and replaces a selection in every macOS app. Terminals expose nothing useful, Electron apps need `AXManualAccessibility` forced and report broken selection offsets, web areas are usually read-only, and secure input fields block event taps.

Read: try `kAXSelectedTextAttribute` on the focused element, then fall back to synthesized ⌘C with the clipboard saved, read, and restored. Write: try `AXUIElementSetAttributeValue` on `kAXSelectedTextAttribute`, gated on `AXUIElementIsAttributeSettable`, then fall back to writing the clipboard and synthesizing ⌘V, restoring the previous clipboard contents afterwards. Surface failures to the user instead of pasting into the wrong place.

This is the ladder every shipping selection tool converges on (PopClip, Easydict), and treating AX as the fast path rather than the product avoids building the app around what only AppKit-native apps support.

**Consequences:** the clipboard path must handle `NSPasteboard.changeCount` and restore reliably — PopClip has shipped clipboard-pollution bugs from exactly this. Terminal, VS Code, Slack and browser page content will always take the slower path, so the popup must tolerate ~100–200 ms before text appears.
