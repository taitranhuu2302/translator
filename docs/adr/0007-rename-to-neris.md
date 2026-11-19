# Rename the app to Neris

The product shipped as "Translator", with the Neris name reserved for the brand and the design system. That split was doing no work: the window title, the menu bar tooltip, the Quit item and the onboarding copy all either said "Translator" or had to explain which of the two names applied. The app is now simply Neris — brand, product and design system are one name.

The bundle identifier moves with it, from `com.taitran.translator` to `com.taitran.neris`, and the version to 0.2.0.

The identifier is not just a label (see [ADR-0004](0004-stable-signing-from-first-commit.md)). Three things key on it, and all three break on a rename:

- **The Accessibility grant.** TCC records the designated requirement, which pins the identifier, so the grant does not survive. It is re-granted once, from System Settings.
- **`~/Library/Application Support/<identifier>/`**, where `settings.json` and `history.sqlite3` live.
- **The Keychain service**, one generic-password item per provider.

The last two are carried over by a one-time migration that runs at the start of `main()`, before any store is built (`crates/platform-macos/src/migration.rs`).

The migration copies rather than moves, and is safe to retry. `Application Support/<identifier>` is the completion marker: the state is copied into a `.migrating` sibling and renamed into place only after every file and every Keychain item is across. A failure midway leaves the marker absent and the next launch tries again with no half-migrated state to reason about. Only the SQLite database, its `-wal` and `-shm` files and `settings.json` travel — the write-ahead log holds commits the main file does not. The old directory and the old Keychain items are left exactly as they were, so rolling back to a previous build still finds its data.

Only the packaged app migrates. A `cargo run` binary or a test binary reports no bundle identifier, and migrating from one would write into a real installation's directory from a development shell.

The legacy identifier is authored in `packaging/legacy-identifier.txt` and compiled in by `build.rs`, for the same reason the live one is authored in `packaging/Info.plist`: `scripts/check-identity.sh` fails if either appears in `crates/` or `scripts/` a second time.

**Not renamed:** the repository, the git remote, the crate names, the binary (`translator`), `CFBundleExecutable`, and the "Quick Translate" popup — a feature name, not a product name.

**Consequences:** one re-grant of Accessibility and one re-enable of "Start at login", both once per installation. Old state stays on disk under `com.taitran.translator` until it is deleted by hand; the migration never deletes it for the user.
