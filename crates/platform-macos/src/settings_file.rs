//! The local settings file: one JSON document under Application Support, never
//! synced. Ticket 10 grows the UI and the remaining settings on top of it; it
//! lands here now because first-launch onboarding has to remember it has run.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use translator_core::{Settings, SettingsError, SettingsStore};

pub struct MacSettingsStore {
    path: PathBuf,
}

impl MacSettingsStore {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn default_path() -> PathBuf {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        home.join("Library/Application Support/com.taitran.translator/settings.json")
    }
}

impl Default for MacSettingsStore {
    fn default() -> Self {
        Self::at(Self::default_path())
    }
}

impl SettingsStore for MacSettingsStore {
    fn load(&self) -> Result<Settings, SettingsError> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                SettingsError::Storage {
                    message: format!("{}: {error}", self.path.display()),
                }
            }),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Settings::default()),
            Err(error) => Err(SettingsError::Storage {
                message: format!("{}: {error}", self.path.display()),
            }),
        }
    }

    fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        write_atomically(&self.path, settings).map_err(|error| SettingsError::Storage {
            message: format!("{}: {error}", self.path.display()),
        })
    }
}

/// A half-written settings file is worse than a missing one, so the bytes land in
/// a sibling file and are renamed into place.
fn write_atomically(path: &Path, settings: &Settings) -> std::io::Result<()> {
    let directory = path.parent().ok_or_else(|| {
        std::io::Error::new(ErrorKind::InvalidInput, "the settings path has no directory")
    })?;
    fs::create_dir_all(directory)?;

    let json = serde_json::to_vec_pretty(settings)
        .map_err(|error| std::io::Error::new(ErrorKind::InvalidData, error))?;
    let staging = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&staging)?;
        file.write_all(&json)?;
        file.sync_all()?;
    }
    fs::rename(staging, path)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use translator_core::LanguagePair;

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("translator-settings-{nanos}"))
            .join(name)
    }

    #[test]
    fn a_missing_file_loads_the_defaults() {
        let store = MacSettingsStore::at(temp_path("settings.json"));

        let settings = store.load().expect("missing file loads defaults");

        assert_eq!(settings, Settings::default());
        assert!(!settings.onboarding_seen);
    }

    #[test]
    fn settings_survive_a_round_trip() {
        let path = temp_path("settings.json");
        let store = MacSettingsStore::at(&path);
        let settings = Settings {
            language_pair: LanguagePair::new(
                translator_core::Language::vietnamese(),
                translator_core::Language::english(),
            ),
            onboarding_seen: true,
            ..Settings::default()
        };

        store.save(&settings).expect("save settings");

        assert_eq!(store.load().expect("load settings"), settings);
        assert!(path.exists());
    }

    #[test]
    fn a_corrupt_file_is_a_typed_storage_error() {
        let path = temp_path("settings.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not json").unwrap();
        let store = MacSettingsStore::at(&path);

        let error = store.load().unwrap_err();

        match error {
            SettingsError::Storage { message } => assert!(message.contains("settings.json")),
            other => panic!("expected a storage error, got {other:?}"),
        }
    }
}
