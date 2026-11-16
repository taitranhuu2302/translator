# Quick Translate popup — manual checks

Written 2026-11-16 for ticket 06. Run these on the signed bundle. The agent did
not launch the app. Default shortcut: `⌃⌥D`.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Select a short phrase in Notes and press `⌃⌥D`. | The popup is a rounded card with a shadow on a transparent background. Its height fits the content; there is no empty area beneath a short answer. |
| 2 | Select a long paragraph and press `⌃⌥D`. | The card grows to its cap and the answer scrolls inside it. It never exceeds the cap and never runs off the screen. |
| 3 | Put the cursor in the bottom-right corner of the screen and repeat. | The card is re-placed so it is fully visible; it does not hang off the edge. |
| 4 | Press Enter. | The Selection is replaced in the source app and the popup hides. |
| 5 | Press Escape. | The popup hides and nothing is replaced. |
| 6 | While the popup is open, watch the Dock and the menu bar. | Translator does not activate. The app you came from keeps its menu bar and its frontmost state. |
| 7 | Type in the source app while the popup is open. | Keystrokes go to the popup, not the source app — Enter must not reach the document underneath. |
| 8 | Trigger a provider failure. | The popup still appears as a card, with the failure notice and no truncated text. |
