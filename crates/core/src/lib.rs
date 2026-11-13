//! The Translator engine and the ports the platform must satisfy.
//!
//! Nothing here may depend on AppKit, Tauri or any other macOS crate: this crate
//! is the cross-platform promise the v1 spec makes, and it is tested with `cargo test`
//! without launching the app.

pub mod chunking;
pub mod engine;
pub mod error;
pub mod http;
pub mod language;
pub mod model;
pub mod ports;
pub mod providers;
pub mod testing;

pub use chunking::{chunk_text, join_chunks, MAX_CHUNK_CHARS};
pub use engine::TranslatorEngine;
pub use error::{
    AuthFailure, CaptureError, CredentialError, EngineError, HistoryError, HttpError,
    ProviderError, SettingsError, SpeechError,
};
pub use http::UreqHttpClient;
pub use language::{guess_language, Direction, Language, LanguagePair};
pub use model::{
    utf16_length, HistoryEntry, ImproveDelta, ImprovePart, Improvement, Mode, PopupPrefill,
    ProviderId, ProviderStatus, ProviderTranslation, QuickTranslation, ReadAloud, Replacement,
    Selection, Settings, ShortcutBindings, TextOutcome, Timestamp, Translation,
};
pub use ports::{
    CredentialStore, HistoryStore, HttpClient, HttpMethod, HttpRequest, HttpResponse,
    ImproveRequest, SelectionCapture, SettingsStore, SpeechOutput, TranslateRequest,
    TranslationProvider,
};
pub use providers::{
    AzureProvider, CachedProvider, GroqProvider, ImprovementStream, MyMemoryProvider,
    ProviderLadder, UnconfiguredProvider,
};
