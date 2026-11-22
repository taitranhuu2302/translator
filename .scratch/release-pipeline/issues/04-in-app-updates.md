# 04: In-app updates, in the menu bar and in Settings

**What to build:** An `updates` module in the shell that owns one status value
— disabled, idle, checking, up to date, downloading, ready, failed — and the
two ways it changes: a background check on launch and every six hours, and the
user asking. Tauri's updater plugin is compiled in only behind a new `updater`
cargo feature, which the release build enables; a local build reports
`disabled` and cannot replace itself. A downloaded version adds one menu-bar
row, "Restart to Update to X.Y.Z…", which installs and restarts; Settings gains
an Updates section with the current version, that row's action when it applies,
a "Check now" button and an "Check for updates automatically" switch that is
saved in the settings file and defaults to on. The update key's public half is
authored in `tauri.conf.json`; a placeholder value fails the release script
rather than shipping an app that can never verify an update.

**Blocked by:** None (can start immediately)

**Status:** in-progress

- [ ] A local build (`./scripts/build.sh`, no `updater` feature) reports
      `disabled`; Settings shows the section with the check control inert and
      says why.
- [ ] The release build checks on launch and every six hours, downloads
      silently, and reaches `ready` without restarting.
- [ ] When ready, the menu bar carries the restart row and Settings offers the
      same action; installing restarts into the new version.
- [ ] The automatic-check switch persists in the settings file, defaults to on
      for files written before the field existed, and turning it off stops the
      background loop for the session.
- [x] `set-version`/release refuses a placeholder update public key.
- [x] The update status is reachable only through `ui/src/ipc.ts`; no component
      imports the updater plugin.

## Implementation

`crates/app/src/updates.rs` owns `UpdateStatus` and the two ways it changes: the
launch-and-six-hourly loop and the user's `check_for_updates`. The plugin is
compiled in only behind the `updater` feature; without it every command answers
`disabled`. `ui/src/UpdateSettings.tsx` renders the section (Version row,
**Check now**, the ready card's **Restart to Update**, the automatic-check
switch), reachable through `SettingsPane.tsx`. `Settings` gained
`auto_check_updates: bool` with a serde default of `true`, written through the
existing `settingsWriter.change` path as an `auto_check_updates` mutation
(`ui/src/settingsWrite.ts`), so a save is what wakes or stops the loop. The
endpoint and the public key are both authored in `crates/app/tauri.conf.json`:
`https://github.com/taitranhuu2302/translator/releases/latest/download/latest.json`.

Verified on 2026-11-22: `cargo check -p translator-app --features
custom-protocol,updater` compiles; `./scripts/release.sh` refuses a placeholder
public key and a missing `TAURI_SIGNING_PRIVATE_KEY`; the IPC boundary grep
passes; 176 UI tests including three new `auto_check_updates` write tests;
`cargo test --workspace` 299 tests. Not verified: anything that needs the app
open — the `disabled` section in a local build, the six-hour loop, the menu-bar
restart row, and the switch surviving a relaunch.
