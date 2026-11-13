//! The six collaborators the engine depends on. Every one is a trait so tests can
//! drive the engine with fakes and a second platform can supply its own bindings.

use crate::error::{
    CaptureError, CredentialError, HistoryError, ProviderError, SettingsError, SpeechError,
};
use crate::language::Language;
use crate::model::{
    HistoryEntry, Improvement, ProviderId, ProviderTranslation, Selection, Settings, Timestamp,
};

/// Reads the Selection from the focused app and overwrites it on Replace.
pub trait SelectionCapture: Send + Sync {
    fn read_selection(&self) -> Result<Selection, CaptureError>;
    fn replace_selection(&self, text: &str) -> Result<(), CaptureError>;

    /// Whether the platform has granted this app the permission capture needs.
    /// Platforms without such a permission answer `true`.
    fn accessibility_granted(&self) -> bool;

    /// Ask the platform to prompt for the permission. Answers whether it is
    /// granted now; the user may still be deciding.
    fn request_accessibility(&self) -> bool;

    /// Open the platform's settings pane where the permission is granted.
    fn open_accessibility_settings(&self) -> Result<(), CaptureError>;
}

/// One request to a translation provider. `source` is `None` when the direction is
/// left to the provider's detection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslateRequest {
    pub text: String,
    pub source: Option<Language>,
    pub target: Language,
}

/// One request to the Improve model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImproveRequest {
    pub text: String,
}

/// The engine's only route to the network. The provider ladder (ticket 06) sits
/// behind this trait.
pub trait TranslationProvider: Send + Sync {
    fn translate(&self, request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError>;
    fn improve(&self, request: &ImproveRequest) -> Result<Improvement, ProviderError>;
}

/// Speaks the Selection with the system voice. It never speaks a translation.
pub trait SpeechOutput: Send + Sync {
    fn speak(&self, text: &str) -> Result<(), SpeechError>;
}

/// One row per translation, with the 30-day purge.
pub trait HistoryStore: Send + Sync {
    fn record(&self, entry: &HistoryEntry) -> Result<(), HistoryError>;
    fn search(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError>;
    fn purge_older_than(&self, cutoff: Timestamp) -> Result<usize, HistoryError>;
}

/// The Language Pair, shortcut bindings and start-at-login preference.
pub trait SettingsStore: Send + Sync {
    fn load(&self) -> Result<Settings, SettingsError>;
    fn save(&self, settings: &Settings) -> Result<(), SettingsError>;
}

/// API keys, one item per provider. The Keychain backs this on macOS.
pub trait CredentialStore: Send + Sync {
    fn get(&self, provider: ProviderId) -> Result<Option<String>, CredentialError>;
    fn set(&self, provider: ProviderId, secret: &str) -> Result<(), CredentialError>;
    fn delete(&self, provider: ProviderId) -> Result<(), CredentialError>;
}
