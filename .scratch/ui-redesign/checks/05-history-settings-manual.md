# History and Settings panels — manual checks

Written 2026-11-16 for ticket 05. Run these on the signed bundle. The agent did
not launch the app.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Type a sentence, then open History. | The panel slides in from the right and the two panes stay visible beside it. Your text is still in the source pane. |
| 2 | Type in History's search field. | The list filters as it does today; clearing it restores the full list. |
| 3 | Select an entry. | Its text returns to the panes exactly as it does today. |
| 4 | Close History. | The panel closes and the panes are exactly as you left them. |
| 5 | Open Settings. | A sheet with Language Pair, Startup, Shortcuts, Providers and Reset, in that order, scrolling on its own. |
| 6 | Change the Target and close the sheet. | The saved value is what you see; nothing reverts. |
| 7 | With Settings open, press Escape. | The sheet closes and focus returns to the button that opened it. |
| 8 | Open History with no entries. | A composed empty state, not a lone sentence in an empty box. |
