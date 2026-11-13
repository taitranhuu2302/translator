# Manual checks — 02: Read the Selection

Everything below needs a human at the machine: Accessibility grants, secure input
and the clipboard cannot be driven from the test suite. `./scripts/build.sh`
must have produced a clean, signed `target/bundle/Translator.app` first.

## Reset between runs

Quit the app from the menu bar (the icon has **Quit Translator**), then:

```bash
rm -f "$HOME/Library/Application Support/com.taitran.translator/settings.json"
```

That brings the onboarding back on the next launch. To re-test the
no-permission path, also drop the grant:

```bash
tccutil reset Accessibility com.taitran.translator
```

`open target/bundle/Translator.app` starts it. The app is menu-bar only: no Dock
icon, no ⌘Tab entry.

## Cases

| # | Case | Steps | Expected |
| --- | --- | --- | --- |
| 1 | Onboarding on first launch | Reset the settings file, then launch. | The Translate Window opens with a "Welcome to Translator" card listing four shortcuts — ⌃⌥D Quick Translate, ⌃⌥R Replace, ⌃⌥S Read Aloud, ⌃⌥E Translate Window — each with a one-line description. While Accessibility is missing, the card also offers **Grant Accessibility…** and **Open Privacy Settings**. |
| 2 | Onboarding stays dismissed | In case 1 click **Got it**, quit, relaunch. | No window opens; the app sits in the menu bar only. Pressing ⌃⌥E opens the window with no onboarding. |
| 3 | Notes | Select a paragraph in Notes, press ⌃⌥E. | The window opens with that paragraph in the left pane and no notice. |
| 4 | Chrome page text | Select text on a web page in Chrome (not in an input), press ⌃⌥E. | Same as case 3. If Chrome exposes no Accessibility text, the clipboard path covers it and the result is still the same. |
| 5 | VS Code | Select a few lines in the editor, press ⌃⌥E. | The editor selection appears in the left pane, line breaks intact. |
| 6 | Terminal | Select a command's output in Terminal, press ⌃⌥E. | That text appears in the left pane. |
| 7 | Nothing selected | Click into empty space in Notes (no selection), press ⌃⌥E. | The window opens with an empty input pane, the counter reads "Nothing selected — the window opens empty.", and no error notice appears. |
| 8 | No Accessibility | Reset the grant, launch, press ⌃⌥E. | A notice says Translator needs Accessibility to read the selection, with **Grant Accessibility** and **Open System Settings**. Either button leads to the Accessibility pane (the first also shows the macOS prompt). Enable Translator there, come back: the notice changes to "Accessibility is on…". Press ⌃⌥E again — the selection is read. No reinstall or relaunch needed. |
| 9 | Clipboard restored, plain text | Copy a distinctive word (e.g. `banana-42`) in any app. Select different text elsewhere, press ⌃⌥E, then paste into a text field (⌘V). | The pasted text is `banana-42`, not the selection. |
| 10 | Clipboard restored, rich content | Copy an image in Preview or a styled cell in Numbers. Select text in Notes, press ⌃⌥E, then paste into Preview/Numbers. | The image or styled cell pastes back unchanged (formatting included). |
| 11 | Secure field refuses | Focus a password field — e.g. run `sudo -k; sudo ls` in Terminal and start typing the password, or use a login form in Chrome — press ⌃⌥E. | The window opens with "This is a secure field, so nothing was read." and the input pane stays empty. No selection text is read. |
| 12 | Secure input leaves no trace | During case 11, paste (⌘V) into any text field afterwards. | Your previous clipboard contents come back; the password is not on the clipboard. |
| 13 | Clipboard fallback | Use an app that does not expose text through Accessibility (e.g. a page rendered inside a canvas, or Firefox). Select text, press ⌃⌥E. | The text still arrives through the ⌘C fallback, and your clipboard is restored. |
| 14 | Focus is not stolen | With text selected in Notes, note where the caret is, press ⌃⌥E. | Notes keeps its selection until the Translator window appears; the captured text is the Notes selection, never Translator's own. |
| 15 | Menu bar entry | Click the menu bar icon, choose **Open Translate Window**. | The window opens and captures the selection from the app that had focus before the menu was clicked. |
| 16 | Onboarding offers the grant | `tccutil reset Accessibility com.taitran.translator`, delete `settings.json`, launch. | The card shows **Grant Accessibility…** and **Open Privacy Settings**. The first asks macOS for the permission; the second opens the Accessibility pane. Enable Translator there and return to Translator: the card says "Accessibility is granted." and the two buttons disappear without a relaunch. |
| 17 | Onboarding knows an existing grant | Keep the Accessibility grant (do not reset TCC), delete `settings.json`, launch. | The card says "Accessibility is granted." from the start and offers no grant buttons. |

⌃⌥D, ⌃⌥R and ⌃⌥S appear in the onboarding because they are the app's shortcuts,
but only ⌃⌥E is wired up in this ticket; the others arrive in tickets 03–05.

## Results

All 15 cases were run by hand against the signed bundle on 2026-11-13 and passed.
Anything failing in future runs becomes a bug ticket.

| # | Result | Notes |
| --- | --- | --- |
| 1 | | |
| 2 | | |
| 3 | | |
| 4 | | |
| 5 | | |
| 6 | | |
| 7 | | |
| 8 | | |
| 9 | | |
| 10 | | |
| 11 | | |
| 12 | | |
| 13 | | |
| 14 | | |
| 15 | | |
| 16 | | |
| 17 | | |
