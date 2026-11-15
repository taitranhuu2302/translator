# Settings write — manual checks

Written 2026-11-15 for ticket 05. Run these on the signed bundle; the agent did
not launch the app. The settings file is
`~/Library/Application Support/com.taitran.translator/settings.json`; making it
read-only is how the failure cases are forced. Restore it with
`chmod u+w` after each failure case.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Settings → Language Pair, choose **Vietnamese**. Quit and relaunch. | The select shows Vietnamese at once, the sentence says "Translates from English into Vietnamese.", and **Language pair saved.** appears. After relaunch the Target is still Vietnamese. |
| 2 | `chmod a-w "$HOME/Library/Application Support/com.taitran.translator/settings.json"`. Choose **English**, then restore the file's permissions. | The select snaps back to Vietnamese — the saved value never changed — and the only sentence shown is **The setting could not be saved.** |
| 3 | Provider keys → Azure, type `eastus` in **Region (optional)**, click **Save region**. Quit and relaunch. | The field shows `eastus` after trimming and **Region saved: eastus.** appears. After relaunch the region is still `eastus`. |
| 4 | With the settings file read-only, type `westeurope` and click **Save region**. Restore permissions. | The field keeps the typed `westeurope` (nothing was mirrored) and the sentence is the same **The setting could not be saved.** as case 2, never a region-specific wording. |
| 5 | Clear **Region (optional)** and click **Save region**. | The notice says **Region cleared: a global Azure resource sends no region header.** and the field stays empty. |
| 6 | Settings → Shortcuts, click **Change** on Replace and press `⌘⇧W`. Quit and relaunch. | The Replace row shows `⌘⇧W` at once. After relaunch the row still shows `⌘⇧W`. |
| 7 | Press a combination with no modifier (for example `K`) while recording Replace. | The row keeps its saved binding and shows the engine's refusal sentence (starting "A shortcut needs…"); nothing is mirrored. |
| 8 | With the settings file read-only, rebind Replace to `⌘⇧L`. Restore permissions. | The Replace row keeps the binding it had before the attempt and the engine's save-failure sentence appears; nothing that was not saved enters the view. |
| 9 | Customise the Target and one shortcut, then Settings → Reset → Reset shortcuts and language pair. Quit and relaunch. | The pair flips back to English ⇄ Vietnamese, every shortcut shows its shipped binding, the notice says **Shortcuts and language pair put back to the shipped settings.**, and a relaunch keeps them. History and provider keys are untouched. |
| 10 | Customise a shortcut, make the settings file read-only, then reset. Restore permissions. | The rows keep the customised (saved) values — the module does not mirror the live-but-unsaved defaults — and the notice says the defaults are live but the settings file could not be written, so relaunching restores the old settings. |
| 11 | Choose the Target that is already selected. Repeat with a region that is already saved, typed with surrounding spaces. | Nothing changes, no success sentence appears, and nothing is written to the file (its modification time does not move). |
