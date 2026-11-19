# Rename to Neris

Status: done

## Problem Statement

The app shipped as "Translator", with Neris reserved for the brand and the design system. The two names never separated cleanly in use: the window title, the menu bar tooltip, the Quit item, the Accessibility prompt and the onboarding copy each had to choose one, and `DESIGN.md` opened with a Brand/Product table explaining the difference. A person reading the app cannot be told which name it has.

The logo artwork also exists only as two downloads: a lockup PNG and an SVG whose background is a raster JPEG with two real vector paths — the N mark. Nothing in the repository uses either, so the icon, the menu bar item and the in-app header have no brand at all.

## Solution

Make Neris the one name, at every level the person using the app can see, and put the mark into the three places it belongs: the bundle icon, the menu bar, and the app's own header surfaces.

This changes three things that outlive a build:

1. **The bundle identifier** — `com.taitran.translator` → `com.taitran.neris`.
2. **The Accessibility grant** — TCC pins the identifier, so it is granted once more.
3. **Where state lives** — Application Support and the Keychain service both follow the identifier, so a one-time migration carries settings, history and keys across.

The repository, the git remote, the crate names, the binary (`translator`) and "Quick Translate" keep their names: they are not the product.

## User Stories

1. As someone using the app, I want one name everywhere — the title, the menu bar, the Quit item, the onboarding — so that I never have to work out whether "Translator" and "Neris" are the same thing.
2. As someone who already has settings, history and API keys, I want them to survive the rename without re-entering anything, so that a redesign of the name is not a redesign of my setup.
3. As someone whose migration failed halfway, I want the next launch to try again rather than leave me half-migrated.
4. As someone who wants to go back to the previous build, I want the old data and old keys to still be there, so that a rename is not a one-way door.
5. As someone scanning the Dock or Finder, I want an icon that reads as the mark rather than a placeholder.
6. As someone living in the menu bar, I want the item to look native in light and dark menu bars.
7. As someone opening the app for the first time, I want the welcome panel to show who is talking.
8. As a developer reading the repository, I want the naming rule written down, so that a later change does not reintroduce "Translator" into half the strings.
9. As a developer changing the identity in future, I want the identifier authored in one file and checked by the build, so that a rename cannot half-happen.

## Implementation Decisions

- **One name.** Brand, product and design system are all Neris. `DESIGN.md` loses its Brand/Product split.
- **Renamed:** `productName`, `CFBundleDisplayName`, `CFBundleName`, `CFBundleIconFile`, the window `<title>`, the tray tooltip and Quit item, the Accessibility copy, diagnostic `Neris:` log prefixes, the HTTP `User-Agent` (`Neris/<version>`), onboarding and Settings copy, and the README/GLOSSARY/DESIGN titles.
- **Not renamed:** the repository folder, the git remote, the crate names, the binary `translator`, `CFBundleExecutable`, the `TRAY_ID`, `TranslatorEngine`, the Azure "Translator" product name, and "Quick Translate" (a feature, not a product).
- **Identifier authored once.** Still `packaging/Info.plist`; the legacy identifier is authored in `packaging/legacy-identifier.txt` and compiled in by `build.rs`. `scripts/check-identity.sh` now guards both, and fails if either appears in `crates/` or `scripts/` a second time.
- **Migration copies, never moves.** It runs at the start of `main()`, before any store is constructed. `settings.json`, `history.sqlite3` and its `-wal`/`-shm` files are staged into `<new dir>.migrating`, the Keychain items are copied to the new service, and the staged directory is renamed into place. The directory's absence is the completion marker, so a failure is retried next launch.
- **Only the packaged app migrates.** `bundle::running_from_bundle()` gates it, so `cargo run` and tests never write into a real installation's directory.
- **Logo treatment.** The mark is vector; the SVG's background is not. The app icon is rebuilt from the mark alone: white on a diagonal `#0d1129 → #3d5993` gradient, in the 824 px rounded-square tile inside a 1024 px canvas, with transparent margins. The menu bar uses a redrawn black template image at 72 px with `icon_as_template(true)`.
- **The brand blue is not a UI token.** `#0042CC` stays in the lockup and doc assets; the interface stays on the Neris Ocean palette.
- **The mark is not a Lucide icon.** `ui/src/components/NerisMark.tsx` draws it in `currentColor`; it replaces the `Waves` entry in `DESIGN.md`.
- **Version** goes to 0.2.0.

## Consequences

- Accessibility is re-granted once, and "Start at login" re-enabled once, per installation.
- The previous data directory and Keychain items remain on disk. The app never deletes them.

## Out of Scope

- Notarisation, distribution, or auto-update.
- Renaming crates, the binary or the repository.
