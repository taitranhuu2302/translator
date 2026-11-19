//! One-time migration of local state from the identifier this app shipped
//! under before it was renamed to Neris.
//!
//! `Application Support/<identifier>` is the only completion marker: state is
//! copied into a staging directory beside it and renamed into place only once
//! every file and every Keychain item has crossed, so a failure halfway leaves
//! the guard untouched and the next launch tries again. Nothing is moved or
//! deleted — the old directory stays readable if the rename is rolled back
//! (docs/adr/0007-rename-to-neris.md).

use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

use translator_core::CredentialError;

/// The identifier the app used before the rename, compiled in from
/// `packaging/legacy-identifier.txt`. It is authored there for the same reason
/// the live identifier lives in `packaging/Info.plist`: a bundle identifier
/// written into Rust would be a second copy that could drift.
const LEGACY_IDENTIFIER: &str = env!("TRANSLATOR_LEGACY_BUNDLE_ID");

/// Everything the app keeps under Application Support. The SQLite journal and
/// shared-memory files are part of the database: copying `history.sqlite3`
/// without them drops whatever the write-ahead log still held.
const STATE_FILES: [&str; 4] = [
    "settings.json",
    "history.sqlite3",
    "history.sqlite3-wal",
    "history.sqlite3-shm",
];

/// What one launch did, so the shell can say so in one line.
#[derive(Debug, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// Either the new directory is already there or the old one never was.
    NotNeeded,
    /// State was copied; the old directory was left exactly as it was.
    Copied { from: PathBuf },
}

#[derive(Debug)]
pub enum MigrationError {
    Io(io::Error),
    Keychain(CredentialError),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Keychain(error) => write!(formatter, "{error}"),
        }
    }
}

impl From<io::Error> for MigrationError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<CredentialError> for MigrationError {
    fn from(error: CredentialError) -> Self {
        Self::Keychain(error)
    }
}

/// Carry settings, history and stored keys over from the previous identity.
///
/// Only the shipped app migrates: a `cargo run` binary shares the plist's
/// identifier but is not the installation whose state is being carried over,
/// and a test binary has no state to carry.
pub fn migrate_legacy_state() -> Result<MigrationOutcome, MigrationError> {
    if !crate::bundle::running_from_bundle() {
        return Ok(MigrationOutcome::NotNeeded);
    }

    let current = crate::bundle::application_support_dir();
    if current.exists() {
        return Ok(MigrationOutcome::NotNeeded);
    }

    let legacy = crate::bundle::application_support_dir_for(LEGACY_IDENTIFIER);
    if !legacy.is_dir() {
        return Ok(MigrationOutcome::NotNeeded);
    }

    let staging = PathBuf::from(format!("{}.migrating", current.display()));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    for name in STATE_FILES {
        let source = legacy.join(name);
        if source.is_file() {
            fs::copy(&source, staging.join(name))?;
        }
    }

    // Keys are copied, not moved. A Keychain failure aborts before the rename,
    // so the settings and history already staged are copied again next launch
    // rather than being treated as a finished migration.
    crate::credentials::copy_service(LEGACY_IDENTIFIER, &crate::bundle::identifier())?;

    fs::rename(&staging, &current)?;
    Ok(MigrationOutcome::Copied { from: legacy })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legacy_identifier_is_not_the_one_the_app_runs_under() {
        assert_ne!(LEGACY_IDENTIFIER, crate::bundle::identifier());
    }

    #[test]
    fn the_write_ahead_log_and_shared_memory_files_travel_with_the_database() {
        assert!(STATE_FILES.iter().any(|name| name.ends_with("-wal")));
        assert!(STATE_FILES.iter().any(|name| name.ends_with("-shm")));
    }

    #[test]
    fn a_binary_without_a_bundle_never_migrates() {
        assert_eq!(
            migrate_legacy_state().expect("no bundle, nothing to do"),
            MigrationOutcome::NotNeeded
        );
    }
}
