# 11 — Bundle identity: manual run

The identity is now authored once, in `packaging/Info.plist`. The unit and shell
checks prove the copies are gone and the plist, config and workspace agree; the
steps below prove the *packaged* bundle reports that identity and that the files
it writes live under it. They need the app to run, which the agent does not do.

Build first: `./scripts/build.sh`.

| # | Check | How | Expected |
| --- | --- | --- | --- |
| 1 | The bundle reports the authored identifier | `defaults read "$PWD/target/bundle/Translator.app/Contents/Info.plist" CFBundleIdentifier` | `com.taitran.translator` |
| 2 | The signature pins it | `codesign -d -r- target/bundle/Translator.app` | `designated => identifier "com.taitran.translator" and certificate root = H"0210dfaace23b661886724d8588f09a5ee30882b"` |
| 3 | It is menu-bar only | `defaults read "$PWD/target/bundle/Translator.app/Contents/Info.plist" LSUIElement` | `1` |
| 4 | Settings derive from the bundle identifier | Delete `$HOME/Library/Application Support/com.taitran.translator/settings.json`, launch the packaged app, finish onboarding, quit. | `$HOME/Library/Application Support/com.taitran.translator/settings.json` is recreated. No new `Library/Application Support/<other id>` directory appears. |
| 5 | History derives from the bundle identifier | Translate once, then `ls "$HOME/Library/Application Support/com.taitran.translator/"` | `history.sqlite3` sits beside `settings.json`. |
| 6 | The Keychain service derives from the bundle identifier | Save a provider key in Settings, then `security find-generic-password -s com.taitran.translator -a groq` | exits 0 and names the saved item. |

Steps 1–3 are also asserted by `scripts/build.sh`; steps 4–6 are the runtime
half. Because the identifier did not change, an existing installation keeps its
current files — the rename consequence in ADR-0004 only applies to a future
identifier change.
