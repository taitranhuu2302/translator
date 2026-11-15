# Replace workflow — manual checks

Written 2026-11-15 for ticket 09. Run these on the signed bundle; the agent did
not launch the app. The ordering itself is unit-tested with a fake presenter;
these cases check that the real panel behaves as the workflow assumes. Case 7
needs the app started from a terminal so its stderr is visible.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Select a Vietnamese sentence in Notes, press ⌃⌥D, wait for the translation, press Enter. | The popup disappears once and does not come back; Notes holds the translation, stays frontmost, and no dialog appears. |
| 2 | Repeat case 1 in an app whose text view refuses the Accessibility write (Google Docs, VS Code's editor, or a Safari field). | The popup comes back once, still showing the translation, with **This app did not accept the replacement.** inline. It never stays hidden with the document unchanged and no message. |
| 3 | In the returned popup from case 2, press **Copy**, then Enter, then Escape. | Copy puts the translation on the clipboard; Enter retries the Replace and restores the popup again on refusal (one hide, one show); Escape dismisses without writing. |
| 4 | Focus a secure password field (e.g. `sudo -k; sudo ls` in Terminal), select text, press ⌃⌥D, then Enter. | The popup returns with **This is a secure field, so nothing was read.** The password is not read, written, or put on the clipboard. |
| 5 | Select a sentence in Notes and press ⌃⌥R. | No popup appears at any point — before, during, or after the Replace. The selection is replaced and Notes stays frontmost. |
| 6 | Click an empty area so nothing is selected, then press ⌃⌥R (and repeat from the tray's **Replace** row). | A **Replace** error dialog appears on both paths and no popup is shown; the document is unchanged. |
| 7 | Quit Translator, start it from Terminal with `target/bundle/Translator.app/Contents/MacOS/Translator`, and repeat case 6. | The terminal shows `Translator: Replace failed: nothing is selected`; no line about the popup is logged because this path has no popup. |

The two log-only outcomes — `the popup was not hidden before Replace` and
`the popup could not be shown after a failed Replace` — are unit-tested with a
fake presenter and cannot be induced reliably by hand. The main-thread hop
failing (`the main thread did not answer`) is likewise unverified by hand: it
needs a wedged run loop, and the shortcut paths deliberately never hop.

## Results

Fill this in only from a real run on the signed bundle. Anything failing becomes
a bug ticket.

| # | Result | Notes |
| --- | --- | --- |
| 1 | | |
| 2 | | |
| 3 | | |
| 4 | | |
| 5 | | |
| 6 | | |
| 7 | | |
