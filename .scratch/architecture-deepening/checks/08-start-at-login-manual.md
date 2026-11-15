# Start at login — manual checks

Written 2026-11-15 for ticket 08. Run these on the signed bundle; the agent did
not launch the app. The settings file is
`~/Library/Application Support/com.taitran.translator/settings.json`; restore its
permissions after cases 4 and 5.

| # | Do | Expect |
| --- | --- | --- |
| 1 | Open Translator → Settings. While **Start at login** is off, turn it on. Then open System Settings → General → Login Items. | The checkbox stays on with no problem line, and **Translator** appears in the Login Items list. |
| 2 | Quit Translator and open it again, then reopen Settings. | The checkbox reads on, because it shows the login item's actual state rather than a remembered value. |
| 3 | Turn **Start at login** off. | The checkbox stays off with no problem line, and **Translator** disappears from the Login Items list. |
| 4 | With the checkbox off, run `chmod 444` on the settings file, then turn **Start at login** on. | The checkbox returns to off, **Translator** does not appear in the Login Items list, and the pane says **The login item was changed back because the settings file could not be written.** Run `chmod 644` on the file afterwards. |
| 5 | With the file still at `chmod 000`, turn **Start at login** on. | **Translator** does not appear in the Login Items list, the checkbox keeps the state it had, and the pane says **The login item was not changed: the settings file could not be read.** Run `chmod 644` on the file afterwards. |
| 6 | After restoring permissions, turn **Start at login** on and then off again. | Both changes stick, with no problem line, and the Login Items list follows each change. |
