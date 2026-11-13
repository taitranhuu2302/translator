# Manual checks — 04: Quick Translate with the keyless provider

Everything below needs a human at the machine: Accessibility grants, secure
input, Spaces, full-screen windows and a second display cannot be driven from
the test suite, and the popup's focus behaviour is the whole point of the
ticket. `./scripts/build.sh` must have produced a clean, signed
`target/bundle/Translator.app` first.

## Reset between runs

Quit the app from the menu bar (**Quit Translator**), then clear the settings so
the default Language Pair (Target English, partner Vietnamese) and the first
launch return:

```bash
rm -f "$HOME/Library/Application Support/com.taitran.translator/settings.json"
```

The translation cache is in memory only, so quitting also makes the cache cases
repeatable. To re-test the no-permission path, drop the grant too:

```bash
tccutil reset Accessibility com.taitran.translator
```

`open target/bundle/Translator.app` starts it. The app is menu-bar only: no Dock
icon, no ⌘Tab entry. The popup must never change that.

## Cases

| # | Case | Steps | Expected |
| --- | --- | --- | --- |
| 1 | Opens beside the cursor | With the default pair, select a Vietnamese sentence in Notes and press ⌃⌥D. | A borderless popup appears below and to the right of the cursor, about 420×220 points. It first says **Reading the selection…** then **Translating…**, then the English translation with **MyMemory** at the top right. Notes keeps its menu bar and remains the frontmost app. |
| 2 | Translating state precedes the answer | Repeat case 1 on a slow connection (or a long paragraph). | The popup is visible with **Translating…** before any text arrives; the translation later replaces that state in the same popup, without a second window or a flash of empty content. |
| 3 | Vietnamese to English | Repeat case 1. | The result is English. |
| 4 | English to Vietnamese | Select an English sentence in any app, press ⌃⌥D. | The result is Vietnamese — text already in the Target goes to the partner. |
| 5 | Long text, diacritics intact | Select more than 500 characters of Vietnamese prose (four or five sentences), press ⌃⌥D. | The whole text is translated, in order, with no dropped or duplicated sentences and Vietnamese diacritics intact in the source line. |
| 6 | Cache avoids a second request | Press ⌃⌥D twice in a row with the same selection, without changing the selection. | The second answer appears without a visible translating pause; the text is identical. (Failures are never cached, so this only holds for a successful translation.) |
| 7 | Focus and caret | Put the caret mid-document in Notes, note its position, press ⌃⌥D, wait for the answer, press Escape, then type a word. | The popup had the keyboard while open (Escape reached it). After Escape the word is typed into Notes at the same caret position, and the document was not modified by Translator. |
| 8 | Escape dismisses | Open the popup, press Escape. | The popup disappears and the source app is usable immediately. No translation text is inserted anywhere. |
| 9 | Losing key dismisses | Open the popup, then click another app or window without pressing Escape. | The popup hides; the clicked app receives the click normally. |
| 10 | Active Space | Switch to a different Space (⌃→), select text there, press ⌃⌥D. | The popup appears on that Space beside the cursor; it does not follow back to the first Space. |
| 11 | Full-screen app | Enter full screen in Notes or Safari, select text in it, press ⌃⌥D. | The popup appears over the full-screen app and takes Escape; the full-screen space is not left. |
| 12 | Second display | Move the pointer to the second display, select text in an app there, press ⌃⌥D. | The popup appears on that display next to the cursor, fully visible at that display's scale — not at the primary display's origin and not doubled or halved in size. |
| 13 | Bottom edge flips above | Move the cursor near the bottom edge of the screen, select text, press ⌃⌥D. | The popup opens above the cursor rather than falling off the screen. |
| 14 | Right edge is clamped | Move the cursor near the right edge, select text, press ⌃⌥D. | The popup is pulled left so its whole width stays on screen. |
| 15 | Nothing selected | Click an empty area in Notes so nothing is selected, press ⌃⌥D. | The popup opens and says **Select something first, then press the Quick Translate shortcut again.** No request is made and no empty translation is shown. Escape closes it. |
| 16 | No Accessibility | Reset the grant, launch, select text, press ⌃⌥D. | The popup says **Translator needs Accessibility to read your selection.** and points to System Settings → Privacy & Security → Accessibility. Grant it, come back, press ⌃⌥D again: the translation works without relaunching. |
| 17 | Secure input | Focus a password field (e.g. `sudo -k; sudo ls` in Terminal), press ⌃⌥D. | The popup says **This is a secure field, so nothing was read.** The password is not read and does not land on the clipboard. |
| 18 | Offline is named as offline | Turn Wi-Fi off, select text, press ⌃⌥D. | The popup says **You appear to be offline.** with a network-connection suggestion — not a generic provider error, and not a spinner that never ends. Turn Wi-Fi back on and press ⌃⌥D again: it translates. |
| 19 | Copy button | Wait for a translation, click **Copy**. | The button reads **Copied**; pasting (⌘V) in Notes inserts exactly the translation text, without the source line or provider label. |
| 20 | Quota or provider failure | If the free MyMemory quota is exhausted, press ⌃⌥D. | The popup says the free quota is used up and names MyMemory; it is a finished popup, not an endless translating state. |
| 21 | Long translation scrolls | Translate a long paragraph. | The translation scrolls inside the popup; the Copy button and **Esc dismisses** stay reachable, and the window never grows past the screen. |
| 22 | Repeat opens reuse the popup | Open, Escape, then select another sentence and press ⌃⌥D. | The same popup returns in the new place with the new source and answer; there is never a second popup window. |

Cases 3, 5 and 6 need the network; case 18 needs it off. Everything else works
offline because the provider still answers errors.

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
| 18 | | |
| 19 | | |
| 20 | | |
| 21 | | |
| 22 | | |
