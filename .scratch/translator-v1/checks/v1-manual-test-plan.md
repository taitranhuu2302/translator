# Translator v1 — manual test plan

Status: ready to run. Written 2026-11-14 against `impl/v1-rust`. The app was
**never launched** while the spec was implemented; every runtime behaviour below
is waiting for a human at the machine.

This is the run book for the whole spec. The per-ticket checklists hold the
wording and the edge cases; this document orders them into passes, adds the
scenarios that cross ticket boundaries, maps each user story to where it is
checked, and holds the sign-off table. Run the stages in order: each one assumes
the earlier ones passed.

## What only a human can check

Accessibility and secure event input, the synthesized copy/paste round trip,
TCC prompts, panel focus and Space/full-screen behaviour, real provider
responses, the real Keychain, the SQLite database under Application Support, the
login item, and the tray menu's rendering. Everything else — engine, ladder,
chunking, cache, history rules, settings rules, the IPC seam — is covered by the
suites and is not repeated here.

## Before you start

- macOS 14 or later on the target Mac.
- A clean, signed bundle. The signature is what pins the Accessibility grant, so
  an ad-hoc build will fail stage 0.
- Test apps: Notes, Chrome with Gmail open, Safari, VS Code, Terminal,
  Preview and Numbers (for the rich clipboard), Google Docs and Word (for the
  Replace refusal paths), and one login form or `sudo` prompt for secure input.
- Two clipboard probes: a distinctive word (`banana-42`) and an image copied in
  Preview.

Build and check the identity:

```bash
./scripts/build.sh
codesign --verify --deep --strict target/bundle/Translator.app
codesign -d -r- target/bundle/Translator.app
plutil -extract LSUIElement raw target/bundle/Translator.app/Contents/Info.plist
```

`codesign -d -r-` must name `com.taitran.translator` and the
`Translator Local Signing` certificate root; `plutil` must print `true` (menu-bar
only, no Dock icon, no ⌘Tab entry).

Providers: run the first pass with no keys at all, so the keyless MyMemory path
is what you are testing. Stages 6 and 8 then add an Azure Translator F0 key and a
Groq key. Keep an obviously invalid key handy for the failure cases. Free tiers
are rate-limited, so leave time between long runs.

Useful while testing (full recipes in each checklist's reset section):

```bash
log stream --style compact --predicate 'process == "Translator"'
```

```bash
sudo tcpdump -n -i any '(host api.groq.com or host api.cognitive.microsofttranslator.com or host api.mymemory.translated.net or host *.openai.com)'
```

```bash
DB="$HOME/Library/Application Support/com.taitran.translator/history.sqlite3"
sqlite3 -header -column "$DB" "SELECT * FROM history ORDER BY id DESC LIMIT 5;"
```

## Reset between passes

Quit from the menu bar (**Quit Translator**) first; the database and the settings
file are written by the running app.

```bash
APP_DIR="$HOME/Library/Application Support/com.taitran.translator"

# Full reset: onboarding, pair, shortcuts, history.
rm -f "$APP_DIR/settings.json" "$APP_DIR/history.sqlite3"

# Keep settings, drop history.
rm -f "$APP_DIR/history.sqlite3"

# Drop the Accessibility grant (to re-test the permission paths).
tccutil reset Accessibility com.taitran.translator

# Forget provider keys.
security delete-generic-password -s com.taitran.translator -a azure 2>/dev/null || true
security delete-generic-password -s com.taitran.translator -a groq 2>/dev/null || true

# Forget the login item (after quitting the app).
rm -f "$HOME/Library/LaunchAgents"/*ranslator*.plist
```

## How to run

Work stage by stage. For each case in the linked checklist, record pass or fail
in that file's own Results table — the failure notes belong there, next to the
steps. Then record the stage's overall result in the sign-off table at the end of
this document. Anything that fails becomes a bug ticket; do not retest later
stages against a broken earlier one.

Stage 0 is the gate. If the grant does not survive a rebuild, stop: every later
stage's capture results are meaningless.

## Stages

| Stage | What it proves | Checklist |
| --- | --- | --- |
| 0. Build, identity, grant survives a rebuild | The bundle is menu-bar only, signed with the stable identity, and one grant covers clean rebuilds and source-changing rebuilds | [`01`](../issues/01-groundwork-signing-tests-engine.md#still-owed--accessibility-grant-survives-a-rebuild-manual) steps 1–3 |
| 1. First launch and onboarding | The card lists the four shortcuts, offers the grant, notices an existing grant, and stays dismissed | [`02`](02-read-selection-manual.md) cases 1, 2, 16, 17 |
| 2. Reading the Selection | Notes, Chrome, VS Code, Terminal; nothing selected; no grant; clipboard restored; secure input refused; menu bar entry | [`02`](02-read-selection-manual.md) cases 3–15 |
| 3. Read Aloud | Speaks the Selection and never a translation; no window; interrupts cleanly; voices and failures | [`03`](03-read-aloud-manual.md) cases 1–12 |
| 4. Quick Translate | Popup beside the cursor without focus, translating state, copy, Escape, Spaces, full screen, second display, offline and quota errors | [`04`](04-quick-translate-manual.md) cases 1–22 |
| 5. Replace | Enter and ⌃⌥R, the Accessibility and clipboard write paths, refused apps, secure fields, clipboard restoration, long text | [`05`](05-replace-manual.md) cases 1–22 |
| 6. Providers and the ladder | Keyless first run, Azure answers, Keychain storage, bad-key and quota skips, offline wording, region, clearing a key | [`06`](06-azure-manual.md) cases 1–13 |
| 7. Translate Window | The two panes, Enter, Swap, focus, frame persistence, cache sharing, hidden-not-destroyed, the window's own Selection | [`07`](07-translate-window-manual.md) cases 1–22 |
| 8. Improve | The mode switch, streaming, the two labelled parts, copy per part, malformed and truncated answers, Groq wording, Groq as the translation fallback | [`08`](08-improve-manual.md) cases 1–21 |
| 9. History | Recording, one row per translation, reopening without a provider call, search, purge, restart, stored direction | [`09`](09-history-manual.md) cases 1–21 |
| 10. Settings, shortcuts, startup, tray | The fourth surface, language pair, rebinding, conflicts, reset, key rows, start at login, tray contents and permission row, launch-time refusal | [`10`](10-settings-manual.md) cases 1–29 |

## End-to-end scenarios

The stages check features. These scenarios check the product: each one is a
single sitting that crosses ticket boundaries, and each maps to the stories in
the spec. Run them after the stages pass, on a fresh state.

### A. The email (stories 1–5, 9, 18, 43)

1. In Gmail in Chrome, select a Vietnamese sentence. Press ⌃⌥D.
2. Expected: the popup appears beside the cursor within a moment, the compose
   field keeps its caret and focus, and the popup first says it is translating.
3. Expected: the English translation arrives; the provider line names MyMemory.
4. Expected: pressing Escape closes the popup and changes nothing in Gmail;
   pressing ⌃⌥D again reuses the popup.
5. Select the sentence again and press ⌃⌥D, wait, press Enter.
6. Expected: the sentence in Gmail is replaced by its English translation, the
   popup closes, focus stays in Gmail, and the clipboard still holds
   `banana-42`.
7. Expected: **History** shows exactly one row for that translation.

### B. Read, never translate (stories 6, 7, 38)

1. Select an English word in an article and press ⌃⌥S.
2. Expected: the word is spoken in an English voice; no window or popup appears.
3. With Target still English, select a Vietnamese sentence and press ⌃⌥S.
4. Expected: the Vietnamese original is spoken (Linh, if installed) — never its
   English translation — and nothing is translated.
5. Press ⌃⌥S again mid-speech.
6. Expected: speech stops; a new selection speaks with a single press.

### C. Half the apps (stories 8, 10, 13, 41)

1. Select text on a canvas-drawn page in Firefox and press ⌃⌥E.
2. Expected: the text arrives through the clipboard fallback, the popup/window
   opens immediately, and the clipboard is restored.
3. In Google Docs, select text and press ⌃⌥R.
4. Expected: a **Replace** error says the app did not accept the text, so the
   user knows to paste manually; the Selection is unchanged.
5. In Word, repeat.
6. Expected: the same visible failure — never a silent success.
7. With nothing selected in Notes, press each of ⌃⌥D, ⌃⌥R, ⌃⌥S, ⌃⌥E.
8. Expected: each says to select something (or opens empty, for the window) and
   nothing is sent to a provider.

### D. A secret never leaves (stories 11, 47)

1. Start `sudo -k; sudo ls` in Terminal and focus the password prompt.
2. Press ⌃⌥D, ⌃⌥R, ⌃⌥S, ⌃⌥E in turn.
3. Expected: each refuses with the secure-field wording, nothing is spoken,
   nothing is replaced, and no popup content appears.
4. Paste into any field.
5. Expected: the clipboard holds what it held before, never the password.
6. With the tcpdump capture running through all of the above, and with the log
   stream open.
7. Expected: no request to any provider, and no typed text in the log.

### E. Long text and repeats (stories 16, 17, 42)

1. Select a long Vietnamese paragraph (over 500 bytes of UTF-8) and press ⌃⌥D.
2. Expected: it translates in order, with no lost sentence, diacritics intact,
   and the popup scrolls.
3. Press ⌃⌥D again on the same selection, then ⌃⌥E and translate the same text.
4. Expected: the second and third answers are instant, and the provider log or
   tcpdump shows a single upstream request for that text and direction.
5. With Azure keyed and MyMemory rate-limited (or Azure keyed wrongly), translate
   a fresh sentence.
6. Expected: the result arrives from the fallback and says which provider
   answered, so the result can be trusted.

### F. The window round trip (stories 19–24, 36, 37)

1. Select text in Notes and press ⌃⌥E.
2. Expected: the window opens with the Selection in the left pane, two panes side
   by side, and a **Translate | Improve** switch.
3. Press Enter, then click **Swap**, then Enter again.
4. Expected: the first result is one direction; the Swap reverses only this
   window's direction and the second result reverses back on the next Enter.
5. Drag-select a phrase in the result and press ⌘C, then paste elsewhere; then
   press the **Copy** button.
6. Expected: the selection copies only the phrase; the button copies only the
   translation with no labels.
7. Switch to **Improve** on a long sentence and press Enter.
8. Expected: text streams in visibly, and the pane shows **Corrected** and
   **Better** as two separate parts with their own **Copy** buttons.
9. Close the window, reopen it from the tray.
10. Expected: the window reappears where it was, with the same content.
11. Open **History**, click the earliest translation and then the improvement.
12. Expected: each reopens in its own mode with both parts restored and no
    network call.

### G. What stays on this Mac (stories 27, 28, 29, 47)

1. Translate, improve and replace a few things, then open **History**.
2. Expected: the entries appear newest first and survive a quit and relaunch.
3. Search a Vietnamese word, an English word, `%` and `_`.
4. Expected: both sides are searched, case is ignored, and wildcards are
   literal.
5. Quit, age the newest row past 30 days with the SQL in the history checklist,
   relaunch.
6. Expected: that row is gone and the rest remain.
7. With no keys, open **Settings → Providers**.
8. Expected: Azure and Groq say **No key**, MyMemory says **Always available**.
9. Save an Azure key, then run `grep` for its first six characters in
   `settings.json` and in the log stream.
10. Expected: neither contains the key; it is in the Keychain only.

### H. Control (stories 30, 31, 32, 33, 35, 40, 45, 46)

1. Quit and relaunch with a grant already given.
2. Expected: the grant still works without a prompt (stage 0's rebuild check,
   repeated here on a normal relaunch), and the app shows no Dock icon.
3. Tick **Start Translator at login**, quit, remove the plist, relaunch, open
   **Settings**.
4. Expected: the checkbox is unticked even though the file said true; unticking a
   real login item removes the plist.
5. Open the menu bar item.
6. Expected: four actions each with their live combination, an Accessibility row,
   and Quit.
7. Rebind Quick Translate to a combination another app owns, then to one macOS
   accepts.
8. Expected: the refusal is reported at once and the old shortcut still works;
   the accepted one works immediately and appears in the tray.
9. Open the popup on a second Space and inside a full-screen app.
10. Expected: in both places it appears where the work is, over the full-screen
    app, without leaving the Space and without activating Translator.

## Coverage map

Every user story in the spec, and where it is checked. "Stage" is the stage
above; "case" numbers are per-checklist.

| Story | What it asks | Where |
| --- | --- | --- |
| 1 | Vietnamese selection, ⌃⌥D, English beside the cursor | 04 c1–c5; scenario A |
| 2 | The popup does not take focus | 04 c7; scenario A |
| 3 | Enter Replaces the Selection | 05 c1; scenario A |
| 4 | ⌃⌥R translates and Replaces in one step | 05 c2 |
| 5 | English while Target is English goes into the partner | 04 c4, 07 c10, 09 c6/c19 |
| 6 | Read Aloud speaks a word | 03 c1 |
| 7 | Read Aloud never speaks the translation | 03 c2; scenario B |
| 8 | Chrome, VS Code and Terminal still work | 02 c13, 05 c4/c5; scenario C |
| 9 | The previous clipboard is restored | 02 c9/c10, 05 c10/c11; scenario A |
| 10 | Replace fails visibly where insertion is refused | 05 c6/c7; scenario C |
| 11 | A password field is refused | 02 c11/c12, 03 c10, 04 c17, 05 c8, 07 c20; scenario D |
| 12 | No Accessibility is explained and fixable in one step | 02 c8, 03 c9, 04 c16, 05 c9, 10 c28 |
| 13 | The popup opens immediately and fills in | 04 c2 |
| 14 | It works on first launch with no key | 06 c1 |
| 15 | Quota errors name the provider and the next step | 04 c20, 06 c5/c6/c7, 05 c21 |
| 16 | Long text is split transparently | 04 c5, 05 c22, 07 c21; scenario E |
| 17 | A repeated translation is instant | 04 c6, 07 c18; scenario E |
| 18 | Escape dismisses without changing anything | 04 c8; scenario A |
| 19 | ⌃⌥E opens the Translate Window | 07 c1, 02 c3–c6 |
| 20 | Two panes side by side | 07 c1/c7 |
| 21 | Improve returns corrected and better versions | 08 c1/c5/c6 |
| 22 | The two parts are separate and copyable | 08 c7/c8/c9 |
| 23 | Enter translates, ⌘C copies | 07 c1/c7/c8 |
| 24 | Improve streams | 08 c3; scenario F |
| 25 | The Language Pair is remembered | 10 c2/c3 |
| 26 | One configured Target implies the partner | 10 c2 |
| 27 | Search history for 30 days | 09 c1–c14, c17, c20/c21 |
| 28 | History stays local and ages out | 09 c15/c16/c18; scenario G |
| 29 | Keys live in the Keychain | 06 c3/c4; scenario G |
| 30 | Opens fast and can start at login | 10 c24–c26, 04 c2 |
| 31 | Menu bar only | Stage 0 `plutil`; 02 c15, 10 c27 |
| 32 | The popup is on the active Space | 04 c10; scenario H |
| 33 | Full-screen apps work | 04 c11; scenario H |
| 34 | Shortcuts act on the window's own Selection | 07 c22 |
| 35 | Shortcut conflicts can be resolved by rebinding | 10 c5–c18; scenario H |
| 36 | Part of the result can be selected and copied | 07 c7; scenario F |
| 37 | Swap reverses one translation | 07 c10/c11/c12 |
| 38 | Offline has its own error | 04 c18, 05 c21, 07 c17; scenario B |
| 39 | Onboarding grants Accessibility and lists the shortcuts | 02 c1/c16/c17 |
| 40 | The Accessibility grant survives a rebuild | Stage 0 (ticket 01's still-owed steps) |
| 41 | Nothing selected is explained | 02 c7, 03 c3, 04 c15, 05 c13; scenario C |
| 42 | The fallback answers and says so | 04 c20, 06 c2/c5; scenario E |
| 43 | The result can be copied from the popup | 04 c19 |
| 44 | Another pair can be chosen | 10 c2, 07 c10 |
| 45 | The menu bar shows shortcuts and permission state | 10 c27/c28; scenario H |
| 46 | Start at login can be turned off again | 10 c25/c26; scenario H |
| 47 | Only the text being translated leaves the Mac | 02 c11/c12, 06 c4, 07 c20, 09 c18; scenarios D and G |

## Out of scope for this pass

Windows and Linux, App Store distribution and notarization, auto-update,
on-device models, OCR, syncing, glossaries, and the unwired Gemini and Google
Cloud providers. The spec lists them as out of scope for v1; nothing below should
test them.

## Sign-off

| # | Item | Result | Date | Notes |
| --- | --- | --- | --- | --- |
| 0 | Build, identity, grant survives a rebuild | | | |
| 1 | First launch and onboarding | | | |
| 2 | Reading the Selection | | | |
| 3 | Read Aloud | | | |
| 4 | Quick Translate | | | |
| 5 | Replace | | | |
| 6 | Providers and the ladder | | | |
| 7 | Translate Window | | | |
| 8 | Improve | | | |
| 9 | History | | | |
| 10 | Settings, shortcuts, startup, tray | | | |
| A | Scenario: the email | | | |
| B | Scenario: read, never translate | | | |
| C | Scenario: half the apps | | | |
| D | Scenario: a secret never leaves | | | |
| E | Scenario: long text and repeats | | | |
| F | Scenario: the window round trip | | | |
| G | Scenario: what stays on this Mac | | | |
| H | Scenario: control | | | |

A failed item becomes a bug ticket naming the stage or scenario and the case
number; the spec is only signed off once every row has a date and a pass.
