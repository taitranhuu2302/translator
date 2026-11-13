# Manual checks — 09: History

Everything below needs a human at the machine. The suites prove the SQLite
round trip, Unicode case folding, wildcard escaping, the 30-day boundary, the
one-row-per-translation rule, the engine's reopen shapes and the local
filtering in `historyView.ts`; only a real window can show that the History
surface renders, that the application reaches its real database under
Application Support, that entries survive a restart, and that browsing and
reopening history opens no connection. `./scripts/build.sh` must have produced
a clean, signed `target/bundle/Translator.app` first. The app was **never
launched** while building this ticket.

Open the window with ⌃⌥E, from the menu bar item (**Open Translate Window**),
then click **History**. History is also where the Quick Translate popup's
translations and the Translate Window's Improve results land.

## Reset between runs

Quit the app from the menu bar (**Quit Translator**). Then remove the state you
want to start fresh:

```bash
# Delete all local history (irreversible). It is recreated empty on next use.
rm -f "$HOME/Library/Application Support/com.taitran.translator/history.sqlite3"

# Or start from default settings as well (also forgets the Language Pair).
rm -f "$HOME/Library/Application Support/com.taitran.translator/settings.json"

# Forget the Groq key (Improve needs one).
security delete-generic-password -s com.taitran.translator -a groq 2>/dev/null || true
```

Inspect the database while the app is running (SQLite allows a second reader):

```bash
DB="$HOME/Library/Application Support/com.taitran.translator/history.sqlite3"

sqlite3 -header -column "$DB" \
  "SELECT id, mode, provider, source, result, corrected, created_at FROM history ORDER BY created_at DESC;"

sqlite3 "$DB" "SELECT COUNT(*) FROM history;"
```

When the app starts it purges rows older than 30 days. To age the newest row
past the cutoff while the app is **quit**:

```bash
DB="$HOME/Library/Application Support/com.taitran.translator/history.sqlite3"

# Newest row -> 31 days old.
sqlite3 "$DB" \
  "UPDATE history SET created_at = strftime('%s','now') - 31*24*60*60 WHERE id = (SELECT MAX(id) FROM history);"

# Another row -> 29 days old (must survive).
sqlite3 "$DB" \
  "UPDATE history SET created_at = strftime('%s','now') - 29*24*60*60 WHERE id = (SELECT MAX(id) FROM history);"
```

To simulate an Improve row written before the corrected part existed:

```bash
sqlite3 "$DB" \
  "UPDATE history SET corrected = NULL WHERE id = (SELECT id FROM history WHERE mode = 'improve' ORDER BY created_at DESC LIMIT 1);"
```

To watch what the app logs while testing:

```bash
log stream --style compact --predicate 'process == "Translator"'
```

To prove that browsing and reopening history sends nothing to a provider:

```bash
sudo tcpdump -n -i any '(host api.groq.com or host api.cognitive.microsofttranslator.com or host api.mymemory.translated.net)'
```

## Cases

| # | Case | Steps | Expected |
| --- | --- | --- | --- |
| 1 | Empty history shows an empty state | Remove `history.sqlite3`, open the window, click **History**. | The surface shows a search field, the line "Kept on this Mac for 30 days", and exactly "Nothing here yet — translations and improvements you make will appear here." There is no error and no spinner. |
| 2 | A translation appears in History | In **Translate**, type `Xin chào` and press **Enter**. Open **History**. | One row shows `Xin chào` as the source, the English result beneath it, the mode **Translate**, and an age ("just now" or a few seconds ago). |
| 3 | A Replace appears exactly once | Select text in another app, press ⌃⌥E for Quick Translate, then press **Enter** to replace the selection. Open **History**. | Exactly one row exists for that translation — not two — with the original Selection as its source and the replaced text as its result. Running another Replace on a new selection adds exactly one more row. |
| 4 | An Improve shows both parts and its mode | In **Improve**, improve `me and him goes home`. Open **History**. | One row is labelled **Improve**; its source is the typed sentence and its result is the better version. |
| 5 | Reopening restores a translation | Click a **Translate** row. | The view switches to **Translate**, the input holds the stored source, and the pane immediately shows the stored result with its provider line. There is no "Translating…" step. |
| 6 | Reopening restores the direction | Swap so the window translates English into Vietnamese, translate `Hello`, then open **History** and click that new entry. Press **Enter** again without editing. | The reopened pane shows the stored Vietnamese result and the input holds `Hello`. Pressing Enter translates English into Vietnamese again (it does not fall back to the saved Vietnamese-to-English pair), so the new result is Vietnamese. |
| 7 | Reopening spends no quota | Turn Wi-Fi off, open **History**, click a row. | The stored result appears instantly with no network error and no provider call; the pane is complete even offline. |
| 8 | Reopening writes no new row | Note `SELECT COUNT(*)` from the SQLite recipe, click a few rows, then count again. | The count is unchanged. Only pressing Enter on a new translation or improvement adds a row. |
| 9 | Reopening an Improve restores both parts | Click an **Improve** row. | The view switches to **Improve**, the input holds the stored source, and the **Corrected** and **Better** sections show the stored pair. Each section's **Copy** still copies only its own text. |
| 10 | Search finds the Vietnamese side | Type `chào` in the History search field. | Rows whose Vietnamese source or result contains that word remain; unrelated rows disappear. The list narrows as characters are typed, with no perceptible delay. |
| 11 | Search finds the English side | Type `hello` in the History search field. | A row whose source was Vietnamese and whose result was `Hello` is still found, proving both sides are searched. |
| 12 | Search ignores case, not accents | Type `XIN CHÀO`, then `HELLO`, then `chao`. | The first two find the same rows as their lower-case forms. `chao` without the accent does **not** match `chào` (accent-insensitive matching is not promised). |
| 13 | Wildcards are literal | Translate text containing `100% sure` and `a_b`, then search `%`, then `_`. | Searching `%` returns only the row containing a literal percent sign; searching `_` returns only the row containing a literal underscore. Neither acts as a wildcard. |
| 14 | A search with no match has the empty state | Type `zzzzzz`. | The same empty-state sentence appears, with no error and no stale rows. Clearing the field brings every entry back. |
| 15 | A row older than 30 days is purged at launch | Quit. Age one row to 31 days with the SQL recipe. Relaunch and open **History**. | That row is gone; the log may say "purged … history entries older than 30 days". The rest of the list is intact. |
| 16 | A row exactly under 30 days survives | Quit. Age another row to 29 days. Relaunch and open **History**. | The 29-day row is still listed. Only rows strictly older than 30 days disappear. |
| 17 | History survives a restart | Translate or improve something, quit the app, relaunch, open **History**. | The entry is still there, in the same newest-first order, with its original age. Reopening it still restores the result. |
| 18 | Browsing and reopening stay local | Run the `tcpdump` capture, open **History**, type several searches, clear them, and click translate and improve entries. | tcpdump shows no connection to `api.groq.com`, `api.cognitive.microsofttranslator.com` or `api.mymemory.translated.net`. Searching and reopening never contact a provider. |
| 19 | An old Improve row without a corrected part still opens | With an Improve row showing, quit, run the "corrected = NULL" SQL recipe, relaunch, click that row. | The view opens in **Improve** with **Better** restored and **Corrected** empty. Nothing crashes and the input is intact. |
| 20 | Newest entries come first | Make three entries a few seconds apart, open **History**, clear the search. | The newest is at the top and the oldest at the bottom. Rows from different modes and both sides are interleaved by time, not grouped by mode. |

The daily repeat of the purge is not practical to observe by hand (it runs 24
hours after launch, on the background thread). Case 15 proves the launch purge
against the real database; the automated cutoff boundary test and the
`maintain_history` loop cover the interval itself.

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
