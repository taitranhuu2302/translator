# Manual checks — 05: Replace

Everything below needs a human at the machine. Which apps accept an
Accessibility write, whether a browser takes the paste fallback, secure input,
and what the document does when a write is refused cannot be driven from the
test suite. `./scripts/build.sh` must have produced a clean, signed
`target/bundle/Translator.app` first.

Replace has two entry points and one behaviour: Enter in the Quick Translate
popup writes the translation already on screen, and `⌃⌥R` captures, translates
and writes in one step with no popup. Both must leave the document untouched
when they fail, and both must say so.

## Reset between runs

Quit the app from the menu bar (**Quit Translator**), then clear the settings so
the default shortcuts (`⌃⌥D` Quick Translate, `⌃⌥R` Replace) and the default
Language Pair return:

```bash
rm -f "$HOME/Library/Application Support/com.taitran.translator/settings.json"
```

To re-test the no-permission path, drop the Accessibility grant too:

```bash
tccutil reset Accessibility com.taitran.translator
```

For the clipboard cases, put a known item on the clipboard first (copy a
sentence in TextEdit) and check it is still there afterwards; for the non-text
case, copy an image in Preview or a file in Finder instead.

## Cases

| # | Case | Steps | Expected |
| --- | --- | --- | --- |
| 1 | Enter replaces and closes | Select a Vietnamese sentence in Notes, press ⌃⌥D, wait for the English translation, press Enter. | The popup disappears. Notes now contains the English translation in place of the selection, the surrounding sentence is unchanged, and the menu bar shows Notes as frontmost throughout. |
| 2 | `⌃⌥R` replaces with no popup | Select a Vietnamese sentence in Notes and press ⌃⌥R. | No popup appears at any point. After a moment the selection is replaced with the English translation in place, the window keeps its scroll position, and Notes stays frontmost. |
| 3 | Native text view is written through Accessibility | Repeat case 1 in a rich-text note that has bold text after the selection. | The translation is inserted as plain text with the surrounding formatting intact; one ⌘Z undoes the whole replace and brings the selection back. No clipboard write is visible in a clipboard manager. |
| 4 | Chromium editable field | In Chrome or Edge, focus a text field or a contenteditable area, select the text, press ⌃⌥R. | The selection is replaced with the translation. If the browser does not confirm the Accessibility write within its settle window, the clipboard path is used instead; either way the old text is gone, not duplicated. |
| 5 | Safari / WebKit field | Select text in a Safari text field or contenteditable area, press ⌃⌥R. | Safari is expected to ignore the Accessibility write (WebKit's setter is an empty method), so the paste fallback must run: the selection is replaced exactly once, with no leftover clipboard and no duplicated text. Record what actually happens — the shipping behaviour is unverified. |
| 6 | An editor that silently discards the write | Select text in Google Docs, VS Code's editor or another web editor that owns its document model, press ⌃⌥R. | Replace either succeeds through the paste fallback or reports **This app did not accept the replacement.** It must never leave the popup closed with the document unchanged and no message. |
| 7 | An app that reports success and does nothing | Select text in Microsoft Word, press ⌃⌥R. | Word is reported to return success and ignore the write. Replace must not claim success on that basis: it either pastes over the selection or shows the visible refusal, and the document is unchanged when refused. |
| 8 | Secure password field is refused | Focus a password field (e.g. `sudo -k; sudo ls` in Terminal), press ⌃⌥R, then repeat with the popup route: select nothing, press ⌃⌥D in a normal field, and press Enter. | `⌃⌥R` shows a dialog titled **Replace** saying the field is secure; the password is not read, not written, and never reaches the clipboard. The popup route shows **This is a secure field, so nothing was read.** No password character lands anywhere. |
| 9 | No Accessibility is refused | Reset the grant, launch, select text, press ⌃⌥R. | A **Replace** error dialog says Translator needs Accessibility and points to System Settings → Privacy & Security → Accessibility. Nothing is written and the document is unchanged. Grant it, come back, press ⌃⌥R again: it works without relaunching. |
| 10 | Clipboard is restored — text | Copy a known sentence, select other text in a native app that needs the paste fallback (a browser field is the reliable one), press ⌃⌥R, then press ⌘V somewhere. | The known sentence, not the translation, is pasted. The clipboard manager shows at most the translation as a transient entry; the previous contents survive. |
| 11 | Clipboard is restored — non-text item | Copy an image in Preview (or a file in Finder), then select text in a browser field and press ⌃⌥R, then paste into Preview/Finder. | The image/file is still on the clipboard afterwards, not the translation text. Nothing the user copied is lost. |
| 12 | Clipboard changed during Replace | Copy a sentence, start a Replace in a browser, and copy something else while it runs (or immediately after). | The second copy wins: the user's newer clipboard item is not overwritten by the restore. |
| 13 | Nothing selected | Click an empty area in Notes so nothing is selected, press ⌃⌥R. | A **Replace** error dialog says there is nothing to replace. Nothing is written, and no empty translation is produced. |
| 14 | Enter with nothing retained | Open the popup from a selection, dismiss it with Escape, then reopen it from a different app and press Enter before the translation appears. | Enter does nothing while the popup is still translating. Once the answer is on screen, Enter writes exactly that answer — it never re-reads the Selection or translates again. |
| 15 | Selection covers exactly the new text | Replace a short Vietnamese phrase (e.g. `Xin chào`) in Notes, then type a letter immediately. | The new text is selected exactly — typing replaces the whole translation, not one character of it and not the translation plus the following space. |
| 16 | UTF-16 lengths | Replace a selection with a translation containing Vietnamese diacritics and one containing an emoji (e.g. `👋 Hello`), then type. | The selection covers the whole inserted text in both cases — no character left outside the selection, no following character selected. |
| 17 | History is best-effort | Make a successful Replace, then force a history failure if one can be induced (none is expected until ticket 09). | A failed history write is logged and Replace still succeeds and writes the document. The popup never shows a history error for a Replace that worked. |
| 18 | Target app is not frontmost | Open the popup from a selection in Notes, click another app while the popup is open (which dismisses it), then press ⌃⌥R. | Replace writes into the newly focused app's selection, never into Notes behind it, and never into a field the user did not select. If there is no selection there, it says nothing was selected. |
| 19 | Hung target does not beachball | Select text in an app that is not responding (or busy in a long operation), press ⌃⌥R. | The failure arrives within about half a second per attempt rather than after the default six-second Accessibility timeout, and the app shows a visible refusal. Note the observed delay: the exact default timeout is unverified. |
| 20 | Secure input held by another app | Run `sudo -k; sudo ls` in Terminal without typing, then switch to Notes, select text and press ⌃⌥R. | Whatever happens must be honest: either Replace works, or it reports the secure-input refusal. No silent no-op. Record the observed behaviour — whether Accessibility insertion works while another app holds secure input is unverified. |
| 21 | Offline | Turn Wi-Fi off, select text, press ⌃⌥R. | A **Replace** error dialog names the offline failure. The document is unchanged and the selection is still there. Turn Wi-Fi on and repeat: the translation replaces in place. |
| 22 | Long translation | Replace a long paragraph. | The whole translation lands in place, in order, with diacritics intact, and the selection covers all of it. |

Cases 4–7 and 20 are the ones whose outcome is app-specific. Record what each
app actually does in the results table rather than assuming the fallback.

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
