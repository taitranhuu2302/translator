//! The Translator engine and the ports the platform must satisfy.
//!
//! Nothing here may depend on AppKit, Tauri or any other macOS crate: this crate
//! is the cross-platform promise the v1 spec makes, and it is tested with `cargo test`
//! without launching the app.

pub mod engine;
pub mod error;
pub mod language;
pub mod model;
pub mod ports;
pub mod providers;
pub mod testing;

pub use engine::TranslatorEngine;
pub use error::{
    CaptureError, CredentialError, EngineError, HistoryError, ProviderError, SettingsError,
    SpeechError,
};
pub use language::{guess_language, Language, LanguagePair};
pub use model::{
    HistoryEntry, Improvement, Mode, ProviderId, ProviderStatus, ProviderTranslation,
    QuickTranslation, ReadAloud, Replacement, Selection, Settings, ShortcutBindings, TextOutcome,
    Timestamp, Translation,
};
pub use ports::{
    CredentialStore, HistoryStore, ImproveRequest, SelectionCapture, SettingsStore, SpeechOutput,
    TranslateRequest, TranslationProvider,
};
pub use providers::UnconfiguredProvider;
