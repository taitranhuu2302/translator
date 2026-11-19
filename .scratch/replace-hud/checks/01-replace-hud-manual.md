# Replace HUD — manual checks

Written 2026-11-19 for ticket 01. Run these on the signed bundle. The agent did
not launch the app.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Select a sentence in TextEdit and press `⌃⌥R` with the network on. | If the translation takes longer than a blink, a small dark capsule appears near the cursor, says **Reading the selection…** then **Translating…**, and disappears once the selection has been replaced. No popup, no dialog. |
| 2 | Press `⌃⌥R`, then immediately type into TextEdit. | The keystrokes land in TextEdit, not in the capsule. The menu bar never switches to Translator, and no window takes the keyboard. |
| 3 | Select the same sentence again and press `⌃⌥R` twice in a row. | The second press is ignored while the first still runs. No second capsule, no second replacement, and nothing is left on screen. |
| 4 | Replace the same sentence a second time, once the first has finished, so the cached translation answers instantly. | No capsule appears — the Replace finishes inside the 120 ms delay. |
| 5 | Press `⌃⌥R` with nothing selected. | No capsule. The native Replace dialog explains that nothing was selected, exactly as before. |
| 6 | Turn the network off (or clear the Azure key and block MyMemory), select a sentence and press `⌃⌥R`. | The capsule disappears and the native Replace dialog explains the failure. No capsule stays behind. |
| 7 | Open a full-screen app, select text there and press `⌃⌥R`. | The capsule appears above the full-screen app without switching Spaces. |
| 8 | Click exactly where the capsule is showing. | The click reaches the window underneath the capsule; the capsule does not swallow it. |
| 9 | Open Quick Translate (`⌃⌥D`) on a long selection and press Enter. | Normally no capsule is visible — the write is immediate. The popup's own **Replacing…** state and its failure notice are unchanged when it fails. |
| 10 | Look at the capsule while it shows. | Its card is the Neris popover surface with Inter text and the same spinner as the popup: no light surface, no system font, no default Aqua chrome. |
