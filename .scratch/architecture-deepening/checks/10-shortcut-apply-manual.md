# Shortcut apply — manual checks

Written 2026-11-15 for ticket 10. Run these on the signed bundle; the agent did
not launch the app. The settings file is
`~/Library/Application Support/com.taitran.translator/settings.json`; restore its
permissions after cases 5 and 6.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Open Translator → Settings and rebind **Quick Translate** to `Cmd+Shift+K`. | No problem line appears, the menu bar item shows `⌘⇧K` beside Quick Translate, pressing it opens the popup, and the old `Ctrl+Alt+D` does nothing. |
| 2 | Quit Translator and open it again, then press `Cmd+Shift+K`. | The popup opens, so the rebind was saved, and the menu bar still shows `⌘⇧K`. |
| 3 | Rebind **Quick Translate** to `Cmd+Space` (Spotlight's combination). | The pane shows **macOS would not give Translator Cmd+Space. Another app may already be using it.**; `Cmd+Shift+K` still opens the popup, `Cmd+Space` opens Spotlight, and the menu bar still shows `⌘⇧K`. |
| 4 | With two bindings changed, press **Reset** in Settings. | All four shipped combinations are live and shown in the menu bar, each fires its own action, the Language Pair is back to its default, and a relaunch keeps all of it. |
| 5 | With a changed binding, run `chmod 444` on the settings file, then rebind **Quick Translate** to `Cmd+Shift+J`. | The pane says **The shortcut was not changed: the settings file could not be written.**; the old binding still works, `Cmd+Shift+J` does nothing, and the file still holds the old binding. Run `chmod 644` afterwards. |
| 6 | With the file at `chmod 444`, press **Reset** in Settings. | The shipped combinations are live now, but the pane says **The defaults are live, but the settings file could not be written, so relaunching restores the old settings: …**; a relaunch restores the previous bindings. Run `chmod 644` afterwards. |
