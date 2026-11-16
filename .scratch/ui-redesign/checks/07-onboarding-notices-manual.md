# Onboarding and notices — manual checks

Written 2026-11-16 for ticket 07. Run these on the signed bundle. The agent did
not launch the app. Reset the one-time flag between cases by quitting, then
removing `onboarding_seen` from the app's settings file (or from Settings →
Reset, if that is easier for you).

| # | Do | Expect |
| --- | --- | --- |
| 1 | Launch with onboarding not yet seen. | A dialog centred over the panes, with a muted backdrop. The four shortcuts are keycaps, each followed by its name and what it does. |
| 2 | Press Tab repeatedly without clicking. | Focus moves only among the dialog's controls; it never reaches the panes behind the backdrop. |
| 3 | Press Escape. | The dialog closes for this session. It has **not** been marked seen: relaunch and it is back. |
| 4 | Click the backdrop (outside the card). | Same as Escape: closes, does not write the flag. |
| 5 | Relaunch, then press **Got it**. | The dialog closes and does not return. The pane behind is editable straight away. |
| 6 | With Accessibility not granted, click **Grant Accessibility…**. | macOS shows its own prompt. If you allow it, the card replaces the two buttons with a green tick and "Accessibility is granted." |
| 7 | Revoke Accessibility in System Settings while on the onboarding card, then return to Translator. | The buttons come back without you clicking anything — the window re-checks when it comes to the front. |
| 8 | Click **Open Privacy Settings**. | System Settings opens on Privacy & Security → Accessibility. |
| 9 | With Accessibility revoked, press a translate shortcut. | A notice with a warning triangle and a neutral (not red) tint, naming the missing permission and carrying its one action button. |
| 10 | Grant Accessibility, then click that notice's action (or reopen the window). | The card swaps to the green tick and "Accessibility is on. Select text in any app and press … again." — the app's own resolved wording, not a second message. |
| 11 | Translate with the network off. | A notice with a wifi-off icon; same neutral tint. |
| 12 | Remove the provider's key in Settings and translate. | A notice with a cloud-off icon, naming the provider's key. |
| 13 | Make a quote a real failure (for example, a malformed response — the error path), or translate with every provider failing. | The error notice is the only one with the red border and wash and an alert-circle icon. |
| 14 | Trigger a failure in Quick Translate. | The popup draws the same alert card as the window — same icon, tint and words — with nothing clipped at the card's width. |

Cases 9–13 are checking one thing: the icon names the cause, the tint names the
severity. Three kinds share the neutral tint on purpose; only `error` is red and
only the resolved `ok` is blue. If every failure is red, the mapping regressed.
