//! The six collaborators the engine depends on. Every one is a trait so tests can
//! drive the engine with fakes and a second platform can supply its own bindings.

use crate::error::{
    CaptureError, CredentialError, HistoryError, HttpError, ProviderError, SettingsError,
    SpeechError,
};
use crate::language::Language;
use crate::model::{
    HistoryEntry, ImproveDelta, Improvement, ProviderId, ProviderTranslation, Selection, Settings,
    Timestamp,
};

/// The HTTP methods the providers use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

/// One request to the network. Providers build this; the client runs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

/// A response that arrived, whatever its status. Provider errors are semantic and
/// live in the body, so a 4xx/5xx is still an answer the provider must read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// The engine's only route to the network. Providers depend on this trait, never
/// on a concrete client, so tests can script every answer and assert the exact
/// URL and query they were given.
pub trait HttpClient: Send + Sync {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpError>;

    /// Run a request and hand its body over in chunks as it arrives. `on_chunk`
    /// returns whether to keep reading; a frozen stream stops early when the
    /// provider has everything it needs. A non-2xx status still streams its body,
    /// so the provider — not the transport — maps the failure.
    ///
    /// The default calls `execute` and hands the whole body over once, so a
    /// client that cannot stream keeps working.
    fn stream(
        &self,
        request: &HttpRequest,
        on_chunk: &mut dyn FnMut(&[u8]) -> bool,
    ) -> Result<u16, HttpError> {
        let response = self.execute(request)?;
        on_chunk(response.body.as_bytes());
        Ok(response.status)
    }
}

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

    /// Improve, reporting the decoded text of each field as it becomes
    /// available. The returned `Improvement` is the authority: a stream that
    /// never produces both fields is a failure, so a view can never show prose
    /// it had to parse itself.
    ///
    /// The default streams nothing and calls `improve`, which keeps providers
    /// that do not stream (and every existing fake) compiling.
    fn improve_streaming(
        &self,
        request: &ImproveRequest,
        _sink: &mut dyn FnMut(&ImproveDelta),
    ) -> Result<Improvement, ProviderError> {
        self.improve(request)
    }

    /// Providers the session has skipped after an auth or quota failure. The
    /// engine reports this in `provider_status`; providers that do not skip
    /// anything keep the empty default.
    fn skipped(&self) -> Vec<ProviderId> {
        Vec::new()
    }

    /// Forget the session skip for one provider, called when its key is saved or
    /// cleared: an explicit fix must take effect without restarting the app.
    fn clear_skip(&self, _provider: ProviderId) {}
}

/// Speaks the Selection with the system voice. It never speaks a translation.
pub trait SpeechOutput: Send + Sync {
    /// Speaks `text` in `language`: the voice depends on the language, so the
    /// engine resolves it and the port does not guess. A platform with no voice
    /// for that language answers `SpeechError::NoVoice`.
    fn speak(&self, text: &str, language: &Language) -> Result<(), SpeechError>;
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
