//! The only crate that may touch macOS APIs: AppKit, ApplicationServices,
//! AVFoundation and the Keychain belong here, never in `core` or the UI.
//!
//! Ticket 01 lands the port shapes and the single place the shell builds them.
//! Each implementation below is an explicit typed failure until the ticket that
//! owns it replaces it — nothing here silently succeeds.

use translator_core::{
    CaptureError, CredentialError, CredentialStore, HistoryEntry, HistoryError, HistoryStore,
    ProviderId, Selection, SelectionCapture, Settings, SettingsError, SettingsStore, SpeechError,
    SpeechOutput, Timestamp,
};

/// Accessibility first, clipboard fallback. REPLACED IN: ticket 02.
pub struct MacSelectionCapture;

impl SelectionCapture for MacSelectionCapture {
    fn read_selection(&self) -> Result<Selection, CaptureError> {
        Err(CaptureError::Unavailable {
            reason: "Accessibility capture lands in ticket 02".to_owned(),
        })
    }

    fn replace_selection(&self, _text: &str) -> Result<(), CaptureError> {
        Err(CaptureError::Unavailable {
            reason: "Replace lands in ticket 05".to_owned(),
        })
    }
}

/// The system voice. REPLACED IN: ticket 03.
pub struct MacSpeechOutput;

impl SpeechOutput for MacSpeechOutput {
    fn speak(&self, _text: &str) -> Result<(), SpeechError> {
        Err(SpeechError::Unavailable {
            reason: "speech synthesis lands in ticket 03".to_owned(),
        })
    }
}

/// SQLite under Application Support. REPLACED IN: ticket 09.
pub struct MacHistoryStore;

impl HistoryStore for MacHistoryStore {
    fn record(&self, _entry: &HistoryEntry) -> Result<(), HistoryError> {
        Err(HistoryError::Unavailable {
            reason: "history storage lands in ticket 09".to_owned(),
        })
    }

    fn search(&self, _query: &str, _limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        Err(HistoryError::Unavailable {
            reason: "history storage lands in ticket 09".to_owned(),
        })
    }

    fn purge_older_than(&self, _cutoff: Timestamp) -> Result<usize, HistoryError> {
        Err(HistoryError::Unavailable {
            reason: "history storage lands in ticket 09".to_owned(),
        })
    }
}

/// The local settings file. REPLACED IN: ticket 10.
pub struct MacSettingsStore;

impl SettingsStore for MacSettingsStore {
    fn load(&self) -> Result<Settings, SettingsError> {
        // Defaults come from `core`; the file lands with ticket 10.
        Ok(Settings::default())
    }

    fn save(&self, _settings: &Settings) -> Result<(), SettingsError> {
        Err(SettingsError::Unavailable {
            reason: "the settings file lands in ticket 10".to_owned(),
        })
    }
}

/// The Keychain, one item per provider. REPLACED IN: ticket 10.
pub struct MacCredentialStore;

impl CredentialStore for MacCredentialStore {
    fn get(&self, _provider: ProviderId) -> Result<Option<String>, CredentialError> {
        Ok(None)
    }

    fn set(&self, _provider: ProviderId, _secret: &str) -> Result<(), CredentialError> {
        Err(CredentialError::Unavailable {
            reason: "Keychain access lands in ticket 10".to_owned(),
        })
    }

    fn delete(&self, _provider: ProviderId) -> Result<(), CredentialError> {
        Err(CredentialError::Unavailable {
            reason: "Keychain access lands in ticket 10".to_owned(),
        })
    }
}
