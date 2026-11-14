# Capture notice — manual checks

Written 2026-11-14 for ticket 01. Run these on the signed bundle; the agent did
not launch the app. Default shortcuts: Quick Translate `⌃⌥D`, Translate Window
`⌃⌥E`, Replace `⌃⌥R`.

| # | Do | Expect |
| --- | --- | --- |
| 1 | In System Settings → Privacy & Security → Accessibility, turn Translator off. Select text in Notes and press `⌃⌥D`. | The popup says **Translator needs Accessibility to read your selection.** with the detail "Open System Settings → Privacy & Security → Accessibility, enable Translator, then press the shortcut again." and exactly one **Open System Settings** button. |
| 2 | With the grant still off, select text and press `⌃⌥E`. | The window shows the same title, the same detail and the same single button as case 1 — no second wording, no second action. |
| 3 | From case 2, click **Open System Settings**, enable Translator, then return to the window. | The failure is replaced by a green **Accessibility is on.** with "Select text in any app and press `⌃⌥E` again." and no button. |
| 4 | Click into a password field (a login form), then press `⌃⌥D`. Repeat with `⌃⌥E`. | Both surfaces say **This is a secure field, so nothing was read.** with "Passwords stay on your Mac. Finish there, then try again elsewhere."; no button. |
| 5 | Focus an app with no selection (or click empty space), then press `⌃⌥D`. | The popup says **Select something first.** with "Select text in any app, then press the shortcut again."; no button. Pressing `⌃⌥E` instead still opens the window empty, with no error. |
| 6 | Settings → Provider keys: give Azure an invalid key and make Azure the active provider. Select text, press `⌃⌥D`, then repeat in the window by pressing Enter. | Both say **The provider rejected its key.** with "Azure needs a new key in Settings." in the popup and in the window's result pane. |
| 7 | With no captured selection, press `⌃⌥R`. | The popup says **There is no captured selection to replace.** with "Select text in the app you were working in, then press the shortcut again."; no button. |
| 8 | After each case above, look at the whole surface. | There is never a silently empty box: every failure shows a title and a detail, and the styling colour follows the cause (accent for capture, amber for offline, red for provider). |
