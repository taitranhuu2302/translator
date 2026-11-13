//! The only crate that may touch macOS APIs: AppKit, ApplicationServices,
//! AVFoundation and the Keychain belong here, never in `core` or the UI.
//!
//! Ticket 02 lands Selection capture and the settings file it needs for
//! onboarding. The ports still to come are explicit typed failures rather than
//! code paths that silently succeed.

#[cfg(target_os = "macos")]
mod capture;
#[cfg(not(target_os = "macos"))]
#[path = "capture_unsupported.rs"]
mod capture;
mod settings_file;

pub use settings_file::MacSettingsStore;

use translator_core::{
    CaptureError, CredentialError, CredentialStore, HistoryEntry, HistoryError, HistoryStore,
    ProviderId, Selection, SelectionCapture, SpeechError, SpeechOutput, Timestamp,
};

/// Accessibility first, clipboard fallback, secure input refused outright
/// (docs/adr/0002-accessibility-first-clipboard-fallback.md).
pub struct MacSelectionCapture;

impl SelectionCapture for MacSelectionCapture {
    fn read_selection(&self) -> Result<Selection, CaptureError> {
        capture::read_selection()
    }

    fn replace_selection(&self, _text: &str) -> Result<(), CaptureError> {
        Err(CaptureError::Unavailable {
            reason: "Replace lands in ticket 05".to_owned(),
        })
    }

    fn accessibility_granted(&self) -> bool {
        capture::accessibility_granted()
    }

    fn request_accessibility(&self) -> bool {
        capture::request_accessibility()
    }

    fn open_accessibility_settings(&self) -> Result<(), CaptureError> {
        capture::open_accessibility_settings()
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
