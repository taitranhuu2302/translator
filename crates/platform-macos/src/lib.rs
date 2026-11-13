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
pub mod clipboard;
mod credentials;
mod history;
pub mod pointer;
mod settings_file;
#[cfg(target_os = "macos")]
mod speech;
#[cfg(not(target_os = "macos"))]
#[path = "speech_unsupported.rs"]
mod speech;

pub use history::MacHistoryStore;
pub use settings_file::MacSettingsStore;

use translator_core::{
    CaptureError, CredentialError, CredentialStore, Language, ProviderId, Selection,
    SelectionCapture, SpeechError, SpeechOutput,
};

/// Accessibility first, clipboard fallback, secure input refused outright
/// (docs/adr/0002-accessibility-first-clipboard-fallback.md).
pub struct MacSelectionCapture;

impl SelectionCapture for MacSelectionCapture {
    fn read_selection(&self) -> Result<Selection, CaptureError> {
        capture::read_selection()
    }

    fn replace_selection(&self, text: &str) -> Result<(), CaptureError> {
        capture::replace_selection(text)
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

/// The system voice: AVFoundation's speech synthesizer, asynchronously, in the
/// voice for the language the engine resolved. It stops the previous utterance
/// when a new one starts.
pub struct MacSpeechOutput;

impl SpeechOutput for MacSpeechOutput {
    fn speak(&self, text: &str, language: &Language) -> Result<(), SpeechError> {
        speech::speak(text, language)
    }
}

/// The Keychain, one item per provider. The key never leaves the Keychain:
/// no settings file, user default or log line ever contains it.
pub struct MacCredentialStore;

impl CredentialStore for MacCredentialStore {
    fn get(&self, provider: ProviderId) -> Result<Option<String>, CredentialError> {
        credentials::get(provider)
    }

    fn set(&self, provider: ProviderId, secret: &str) -> Result<(), CredentialError> {
        credentials::set(provider, secret)
    }

    fn delete(&self, provider: ProviderId) -> Result<(), CredentialError> {
        credentials::delete(provider)
    }
}
