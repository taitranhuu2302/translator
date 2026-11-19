# 01 — Rename and migration: manual run

The unit and shell checks prove the identifier is authored once and that the
migration's pieces behave under test; the steps below prove the *packaged*
Neris 0.2.0 reports the new identity, carries 0.1.0's state across, and draws
the mark in the three places it belongs. They need the app to run, which the
agent does not do.

Build first: `./scripts/build.sh`. Then confirm the old build has state to carry:

```bash
ls "$HOME/Library/Application Support/com.taitran.translator/"
```

`settings.json`, `history.sqlite3` and its `-wal`/`-shm` files should be there,
and at least one provider key should be in the Keychain under the old service.

| # | Check | How | Expected |
| --- | --- | --- | --- |
| 1 | The bundle reports the new identifier | `defaults read "$PWD/target/bundle/Neris.app/Contents/Info.plist" CFBundleIdentifier` | `com.taitran.neris` |
| 2 | The display name is Neris | `defaults read "$PWD/target/bundle/Neris.app/Contents/Info.plist" CFBundleDisplayName` | `Neris` |
| 3 | The version is 0.2.0 | `defaults read "$PWD/target/bundle/Neris.app/Contents/Info.plist" CFBundleShortVersionString` | `0.2.0` |
| 4 | The signature pins the new identifier | `codesign -d -r- target/bundle/Neris.app` | `designated => identifier "com.taitran.neris" and certificate root = H"…"` |
| 5 | Finder shows the mark, not a placeholder | Open `target/bundle` in Finder | `Neris.app` shows the white N on the ocean gradient; no white corner behind it |
| 6 | It is still menu-bar only | `defaults read "$PWD/target/bundle/Neris.app/Contents/Info.plist" LSUIElement` | `1` |
| 7 | State is carried over on first launch | Launch `target/bundle/Neris.app`; watch Console for the app's stderr, or run `log stream --predicate 'process == "translator"'` alongside | `Neris: carried settings, history and keys over from …/com.taitran.translator`; the app opens with the old language pair, shortcuts, history and no re-entry of keys |
| 8 | The data landed under the new identifier | `ls "$HOME/Library/Application Support/com.taitran.neris/"` | `settings.json`, `history.sqlite3`, `-wal` and `-shm` sit together; no `.migrating` directory remains |
| 9 | The old data was copied, not moved | `ls "$HOME/Library/Application Support/com.taitran.translator/"` | The original files are all still there |
| 10 | The Keychain items were copied | `security find-generic-password -s com.taitran.neris -a groq` (and `-a azure`, `-a mymemory`) | Exits 0 for each provider you had saved; the old service's items still resolve too |
| 11 | The menu bar item is legible in both appearances | Switch Appearance between Light and Dark; look at the menu bar | The item follows the menu bar (black on light, white on dark); no coloured tile |
| 12 | The Quit item and tooltip say Neris | Hover the menu bar item, open its menu | Tooltip `Neris`; the item reads `Quit Neris` |
| 13 | The welcome panel shows the mark | With onboarding not yet done, launch the app | The N mark sits above `Welcome to Neris` |
| 14 | The Settings header shows the mark | Open Settings from the menu bar | The mark sits left of `Settings`, in the accent colour |
| 15 | Accessibility is re-granted once | Fire Quick Translate (⌃⌥E) on a selection in another app | macOS asks for Accessibility for the new bundle; after granting in System Settings → Privacy & Security → Accessibility, the shortcut works and does not ask again |
| 16 | Start at login is re-enabled once | Settings → Startup | The toggle is off; switch it on, log out and back in, and the app starts in the menu bar |
| 17 | The popup keeps its own name | Fire Quick Translate | The popup is still titled `Quick Translate` — a feature, not the product |
| 18 | A second launch does not migrate again | Quit and relaunch | No `carried … over` line; state is unchanged |
| 19 | A half-finished migration retries | Quit, `rm -rf "$HOME/Library/Application Support/com.taitran.neris"`, relaunch | The `carried … over` line appears again and step 8 holds |

Steps 1–4 are also asserted by `scripts/build.sh` and `scripts/check-identity.sh`;
the rest are the runtime half. Going back to 0.1.0 is safe: it still reads its own
directory and its own Keychain service, both left untouched by the copy.
