# Accessibility first, clipboard as the fallback

No single method reads and replaces a selection in every macOS app. Terminals expose nothing useful, Electron apps need `AXManualAccessibility` forced and report broken selection offsets, web areas are usually read-only, and secure input fields block event taps.

Read: try the focused element's selected-text attribute, then fall back to a synthesized ⌘C with the clipboard saved, read and restored. Write: set the selected-text attribute only when `AXUIElementIsAttributeSettable` says it is settable, then fall back to writing the clipboard and synthesizing ⌘V, restoring the previous contents afterwards. Surface failures to the user instead of pasting into the wrong place.

This is the ladder every shipping selection tool converges on (PopClip, Easydict), and treating Accessibility as the fast path rather than the product avoids building the app around what only AppKit-native apps support.

In Rust the attribute names are the sharp edge: `objc2-application-services` does not generate the `kAXSelectedTextAttribute`-style constants, so they are built from their string values (`CFString("AXSelectedText")`). A typo there fails as a silent empty read, not a compile error.

**Consequences:** the clipboard path must compare `NSPasteboard.changeCount` before overwriting and restore reliably — PopClip has shipped clipboard-pollution bugs from exactly this. Terminal, VS Code, Slack and browser page content will always take the slower path, so the popup must tolerate ~100–200 ms before text appears.
