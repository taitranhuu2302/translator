# 01 — Move the identity, and carry the state across

Status: done

## What

Change the bundle identifier to `com.taitran.neris` and the version to 0.2.0, and run a one-time migration at startup that copies settings, history and Keychain items from the previous identifier.

## Where

- `packaging/Info.plist` — display name, icon file, identifier, versions
- `packaging/legacy-identifier.txt` — the old identifier, new file
- `crates/app/tauri.conf.json`, `Cargo.toml` — product name, identifier, version
- `crates/platform-macos/build.rs` — compile in `TRANSLATOR_LEGACY_BUNDLE_ID`
- `crates/platform-macos/src/bundle.rs` — `application_support_dir_for`, `running_from_bundle`
- `crates/platform-macos/src/credentials.rs` — `copy_service`
- `crates/platform-macos/src/migration.rs` — the migration, new file
- `crates/app/src/main.rs` — call it before any store is built
- `scripts/check-identity.sh` — guard the legacy identifier too

## Done when

- `scripts/check-identity.sh` passes with both identifiers guarded.
- `cargo test -p translator-platform-macos` passes, including the guard that the legacy identifier differs from the live one.
- A packaged build on a machine that ran 0.1.0 starts with the old settings, history and keys intact.
