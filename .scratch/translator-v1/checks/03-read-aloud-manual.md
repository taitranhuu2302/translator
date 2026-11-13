# Manual checks — 03: Read Aloud

Everything below needs a human at the machine: a voice has to be heard, and
whether speech blocks the next shortcut can only be felt. `./scripts/build.sh`
must have produced a clean, signed `target/bundle/Translator.app` first.

## Reset between runs

Quit the app from the menu bar (the icon has **Quit Translator**), then:

```bash
rm -f "$HOME/Library/Application Support/com.taitran.translator/settings.json"
```

That brings the onboarding back on the next launch; dismiss it with **Got it**.
To re-test the no-permission path, also drop the grant:

```bash
tccutil reset Accessibility com.taitran.translator
```

`open target/bundle/Translator.app` starts it. The app is menu-bar only: no Dock
icon, no ⌘Tab entry.

For case 8, remove the Vietnamese voice (or download it again for case 7) in
System Settings → Accessibility → Spoken Content → System Voice → **Manage
Voices…**. On a fresh Mac the only Vietnamese voice is **Linh (vi_VN)**; this
machine already has it.

## Cases

| # | Case | Steps | Expected |
| --- | --- | --- | --- |
| 1 | Speaks an English Selection | Select a sentence in Notes, press ⌃⌥S. | The sentence is spoken in an English system voice. No window, popup or notice appears, and the Notes selection is unchanged. |
| 2 | Never speaks the translation | With Target English, select Vietnamese text (e.g. `Xin chào, tôi là Linh`), press ⌃⌥S. | The Vietnamese original is spoken, not its English translation. Nothing is replaced and no translation popup or window opens. |
| 3 | Nothing selected | Click into an empty part of Notes with nothing selected, press ⌃⌥S. | No sound, no error dialog, no window. The app looks exactly as it did before the shortcut. |
| 4 | Speech does not block shortcuts | Select a long paragraph in Notes, press ⌃⌥S, then press ⌃⌥E while it is still speaking. | The Translate Window opens immediately with that paragraph in the source pane, while the speech plays on. |
| 5 | A second press interrupts | Select a long paragraph, press ⌃⌥S, then select a single word and press ⌃⌥S again while the paragraph is still being spoken. | The paragraph stops and the word is spoken. The word is not dropped silently. |
| 6 | The shortcut works after speech ends | After a Selection has finished being spoken, select another word and press ⌃⌥S. | The new word is spoken normally. |
| 7 | Vietnamese voice available | With **Linh (vi_VN)** installed, select Vietnamese text, press ⌃⌥S. | It is spoken with Vietnamese pronunciation in the Linh voice, not read as English letters. |
| 8 | No Vietnamese voice fails visibly | Remove the Vietnamese voice in **Manage Voices…**, select Vietnamese text, press ⌃⌥S. | A **Read Aloud** error dialog says no voice is installed for `vi` and points at System Settings → Accessibility → Spoken Content → System Voice → Manage Voices. Nothing is spoken. Dismissing it leaves the app usable and the other shortcuts working. |
| 9 | No Accessibility grant | `tccutil reset Accessibility com.taitran.translator`, launch, select text in Notes, press ⌃⌥S. | A **Read Aloud** error dialog says Accessibility has not been granted. Nothing is read and nothing is spoken. After granting in System Settings, ⌃⌥S works without a reinstall. |
| 10 | Secure field refuses | Focus a password field — e.g. run `sudo -k; sudo ls` in Terminal and start typing the password — and press ⌃⌥S. | A **Read Aloud** error dialog says secure input is active. Nothing is read and nothing is spoken. |
| 11 | Read Aloud leaves the clipboard alone | Copy a distinctive word (e.g. `banana-42`), select different text in Chrome, press ⌃⌥S, then paste (⌘V) into a text field. | The pasted text is `banana-42`, not the selection: Read Aloud speaks through Accessibility and does not use the clipboard. |
| 12 | No window on any Read Aloud outcome | Repeat cases 1, 3 and 8. | No window opens and no Dock icon or ⌘Tab entry appears; the menu bar icon stays the only presence. |

⌃⌥D and ⌃⌥R appear in the onboarding because they are the app's shortcuts, but
they are not wired up in this ticket (tickets 04 and 05).

## Results

Not yet run. The signed bundle has been built, but the app has not been launched
for this ticket; fill the table in when it is exercised by hand. Anything failing
in a future run becomes a bug ticket.

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
