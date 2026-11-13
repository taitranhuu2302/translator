# Manual checks — 10: Settings

Everything below needs a human at the machine. The suites prove the shortcut
rules (canonical comparison, duplicates, no-ops, modifier-less refusal, parser
round trips), the settings round trip through the engine, the reset's effect on
the saved document, and the capture and Language Pair rules in TypeScript. Only a
real window can show that the fourth surface renders, that a pressed combination
is actually registered with macOS and outlives a relaunch, that a conflict with
another app is reported at the moment it is set, that the tray menu lists all
four shortcuts with their live combinations and the Accessibility state, that
the login item is really created and removed, and that the Keychain rows
refresh. `./scripts/build.sh`
must have produced a clean, signed `target/bundle/Translator.app` first. The app
was **never launched** while building this ticket.

Open the window with ⌃⌥E, from the menu bar item (**Open Translate Window**),
then click **Settings**. The four surfaces are Translate, Improve, History and
Settings; leaving and returning must never change what the working panes hold.

## Reset between runs

Quit the app from the menu bar (**Quit Translator**). Then remove the state you
want to start fresh:

```bash
# Start from the shipped Language Pair and shortcuts.
rm -f "$HOME/Library/Application Support/com.taitran.translator/settings.json"

# Read what the app actually saved.
SETTINGS="$HOME/Library/Application Support/com.taitran.translator/settings.json"
cat "$SETTINGS"
python3 -m json.tool "$SETTINGS"   # if it is easier to read pretty
```

To make a settings write fail (case 22):

```bash
DIR="$HOME/Library/Application Support/com.taitran.translator"

# Remove the directory's write bit: reading still works, creating the temporary
# file the atomic write uses does not.
chmod 500 "$DIR"

# After the case, put it back or the app cannot save anything.
chmod 700 "$DIR"

# If the app was quit with the directory read-only, restore it before relaunch.
chmod 700 "$DIR"
```

Forget a provider key:

```bash
security delete-generic-password -s com.taitran.translator -a azure 2>/dev/null || true
security delete-generic-password -s com.taitran.translator -a groq 2>/dev/null || true
```

Forget the login item without opening the app:

```bash
ls "$HOME/Library/LaunchAgents" | grep -i translator
rm -f "$HOME/Library/LaunchAgents"/*ranslator*.plist   # only after quitting the app
```

To watch what the app logs while testing (a refused registration is logged, the
conflict words come back to the surface):

```bash
log stream --style compact --predicate 'process == "Translator"'
```

## Cases

| # | Case | Steps | Expected |
| --- | --- | --- | --- |
| 1 | Settings is the fourth surface | Open the window. | The tab strip reads **Translate · Improve · History · Settings**. Clicking **Settings** shows Language Pair, Shortcuts, Providers and Reset. Clicking **Translate** again shows the panes exactly as they were. |
| 2 | The sentence follows the Target | In **Settings**, read the Language Pair sentence, then choose **Vietnamese** as Target. | The sentence starts as "Translates from Vietnamese into English." Choosing Vietnamese changes it to "Translates from English into Vietnamese." and the partner follows automatically. |
| 3 | A changed pair survives relaunch | Choose **Vietnamese** as Target, quit, relaunch, open **Settings**. | The select still shows Vietnamese and the sentence still says "Translates from English into Vietnamese." `settings.json` has `"target": "vi"` and `"partner": "en"`. |
| 4 | The pair drives a new translation | With Vietnamese as Target, open **Translate**, type `Hello`, press **Enter**. | The result is Vietnamese. |
| 5 | Quick Translate can be rebound | In **Settings**, click **Change** on Quick Translate, press ⌃⌥M. | The row shows ⌃⌥M. Select text elsewhere and press ⌃⌥M: the popup opens. Pressing the old ⌃⌥D does nothing. |
| 6 | Replace can be rebound | Click **Change** on Replace, press ⌘⇧L. Select text elsewhere and press ⌘⇧L. | The Selection is replaced with its translation. The old ⌃⌥R no longer replaces. |
| 7 | Read Aloud can be rebound | Click **Change** on Read Aloud, press ⌃⌥J. Select text elsewhere and press ⌃⌥J. | The Selection is spoken. The old ⌃⌥S does nothing. |
| 8 | Translate Window can be rebound, and the tray follows | Click **Change** on Translate Window, press ⌘⇧O. Open the menu bar item. | The header hint reads ⌘⇧O and **Open Translate Window** shows ⇧⌘O beside it. Pressing ⌘⇧O opens the window; ⌃⌥E no longer does. |
| 9 | Every rebound shortcut survives relaunch | Rebind all four, quit, relaunch, open **Settings**. | All four rows show the new combinations, `settings.json` stores them, and each one still fires its own action. |
| 10 | A combination used by another action is reported | Click **Change** on Replace and press the combination Quick Translate currently shows (e.g. ⌃⌥D). | The row keeps its old binding and shows exactly "That shortcut is already used by Quick Translate." Nothing is registered or saved. |
| 11 | A combination another app owns is reported at once | Click **Change** on Quick Translate and press ⌘Space (Spotlight's default). | The row keeps its old binding and shows exactly "macOS would not give Translator Cmd+Space. Another app may already be using it." The old shortcut still works. If Spotlight's shortcut was changed, try ⌘⇧3 (screenshots) or ⌃↑ (Mission Control). |
| 12 | A modifier pressed on its own is ignored | Click **Change** on Replace, press and release Control alone. | The row keeps saying "Listening…" and nothing is registered. Pressing the rest of the combination (e.g. ⌃⌥M) then finishes the capture. |
| 13 | A combination with no modifier is refused with a reason | Click **Change** on Replace, press `K` alone. | The row keeps its old binding and shows exactly "A shortcut needs at least one of Ctrl, Alt, Cmd or Shift." |
| 14 | Escape cancels capture | Click **Change** on Quick Translate, press Escape. | The row shows its old binding again, the button says **Change**, and no registration changed. |
| 15 | Reset restores all four shipped shortcuts | Rebind two or three actions, click **Reset shortcuts and language pair…**, then confirm. | All four rows read ⌃⌥D, ⌃⌥R, ⌃⌥S and ⌃⌥E, each works immediately, and `settings.json` stores those four. |
| 16 | Reset restores the default pair | With Vietnamese as Target, reset. | The Target select shows English and the sentence reads "Translates from Vietnamese into English." |
| 17 | Reset keeps history and keys | Make a translation and an improvement, save an Azure key, reset. | **History** still lists the entries, and the Azure row still says **Key saved**. Only the pair and shortcuts changed. |
| 18 | Reset reports a binding macOS refuses | Make another app own one shipped combination (e.g. bind ⌃⌥E in another app), then reset. | The reset notice lists "macOS would not give Translator Ctrl+Alt+E. Another app may already be using it." while the other three rows restore and work. |
| 19 | Provider rows show key state | With no keys saved, open **Settings**. | Azure and Groq rows say **No key** with their explanations; the MyMemory row says **Always available** and "No key needed — MyMemory is always available…". |
| 20 | Saving a key refreshes the status immediately | Save an Azure key. Then save an obviously invalid Groq key, run an **Improve** so Groq is skipped, and reopen **Settings**. | Azure immediately reads **Key saved** after saving. After the failed Improve, the Groq row says it was skipped for this session. Entering a new Groq key and saving makes the row read **Key saved** at once, without relaunching. |
| 21 | History and Improve survive a trip to Settings | Translate something, open **History**, note the list; click **Settings**, then back to **History**; also improve a sentence and visit Settings and return to **Improve**. | The history list and the improved parts are exactly as they were; the input text and direction are unchanged. |
| 22 | A failed settings write is refused and rolled back | Quit, `chmod 500` the settings directory, relaunch, try to rebind Quick Translate. | The row keeps its old binding and shows exactly "The shortcut was not changed: the settings file could not be written." The old shortcut still works. Restore `chmod 700` afterwards. |
| 23 | A reset whose settings write fails says so | With the directory still read-only, reset. | The four defaults are live and listed, and the notice says the settings file could not be written, so relaunching restores the old settings. Restore `chmod 700` afterwards. |
| 24 | Start at login can be turned on | In **Settings → Startup**, tick **Start Translator at login**. | The box stays ticked, `settings.json` has `"start_at_login": true`, and `ls ~/Library/LaunchAgents` now lists a Translator plist. Log out and back in: Translator starts with only its menu bar icon (the onboarding is already dismissed). |
| 25 | Start at login can be turned off | Untick **Start Translator at login**. | The box clears, `settings.json` has `"start_at_login": false`, the plist is gone, and logging out and back in does not start Translator. |
| 26 | The checkbox reflects macOS, not just the file | With the box ticked, quit the app, remove its plist with the reset command above, relaunch and open **Settings**. | The box is unticked even though the file still said true. Ticking it again recreates the plist. Removing Translator from System Settings → General → Login Items is reflected the same way on the next visit. |
| 27 | The tray lists every shortcut and follows changes | Rebind one action, then click the menu bar icon. | The menu shows Quick Translate, Replace Selection, Read Aloud and Open Translate Window, each with its current combination, then a separator, the Accessibility row, a separator and Quit Translator. Clicking each action row does what that shortcut does. Rebind another action and open the menu again: the new combination is shown. |
| 28 | The tray shows the permission and opens its pane | `tccutil reset Accessibility com.taitran.translator`, quit and relaunch. Open the menu bar item. | The row reads "Accessibility: Not granted — Open Settings…". Clicking it opens System Settings at Privacy & Security → Accessibility. Grant Translator there, then bring the window to the front once: the row becomes "Accessibility: Granted" without a relaunch. |
| 29 | A shortcut macOS refused at launch is reported | Bind ⌃⌥D in another app, quit and relaunch Translator, open **Settings**. | Above the Shortcuts rows a warning names **Quick Translate** and ⌃⌥D with macOS's refusal; the row still shows ⌃⌥D. Remove the other app's binding, rebind Quick Translate, and the warning disappears; the new shortcut works. |

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
| 23 | | |
| 24 | | |
| 25 | | |
| 26 | | |
| 27 | | |
| 28 | | |
| 29 | | |
