//! Fakes for the six ports, kept in one module so the port shapes live in one
//! place. Every fake is cheap to clone: the test keeps a handle for assertions
//! while the engine owns its own.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::{
    CaptureError, CredentialError, HistoryError, ProviderError, SettingsError, SpeechError,
};
use crate::model::{
    HistoryEntry, Improvement, ProviderId, ProviderTranslation, Selection, Settings, Timestamp,
};
use crate::ports::{
    CredentialStore, HistoryStore, ImproveRequest, SelectionCapture, SettingsStore, SpeechOutput,
    TranslateRequest, TranslationProvider,
};

/// A capture port that can be told to succeed, fail, or accept or refuse writes.
/// Without a configured selection it fails the way the real port does, so a test
/// cannot pass on text no capture would have returned.
#[derive(Clone, Default)]
pub struct FakeCapture {
    state: Arc<Mutex<CaptureState>>,
}

#[derive(Default)]
struct CaptureState {
    selection: Option<Selection>,
    read_error: Option<CaptureError>,
    write_error: Option<CaptureError>,
    reads: usize,
    writes: Vec<String>,
}

impl FakeCapture {
    pub fn with_selection(text: impl Into<String>) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().selection = Some(Selection::new(text));
        fake
    }

    pub fn failing_reads_with(error: CaptureError) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().read_error = Some(error);
        fake
    }

    pub fn failing_writes_with(self, error: CaptureError) -> Self {
        self.state.lock().unwrap().write_error = Some(error);
        self
    }

    pub fn reads(&self) -> usize {
        self.state.lock().unwrap().reads
    }

    pub fn writes(&self) -> Vec<String> {
        self.state.lock().unwrap().writes.clone()
    }
}

impl SelectionCapture for FakeCapture {
    fn read_selection(&self) -> Result<Selection, CaptureError> {
        let mut state = self.state.lock().unwrap();
        state.reads += 1;
        if let Some(error) = &state.read_error {
            return Err(error.clone());
        }
        state
            .selection
            .clone()
            .ok_or(CaptureError::NothingSelected)
    }

    fn replace_selection(&self, text: &str) -> Result<(), CaptureError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = &state.write_error {
            return Err(error.clone());
        }
        state.writes.push(text.to_owned());
        Ok(())
    }
}

/// A provider that returns a scripted response, or throws a scripted error.
#[derive(Clone, Default)]
pub struct FakeProvider {
    state: Arc<Mutex<ProviderState>>,
}

#[derive(Default)]
struct ProviderState {
    translation: Option<ProviderTranslation>,
    translate_error: Option<ProviderError>,
    improvement: Option<Improvement>,
    improve_error: Option<ProviderError>,
    requests: Vec<TranslateRequest>,
    improve_requests: Vec<ImproveRequest>,
}

impl FakeProvider {
    pub fn responding_with(translation: ProviderTranslation) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().translation = Some(translation);
        fake
    }

    pub fn failing_with(self, error: ProviderError) -> Self {
        self.state.lock().unwrap().translate_error = Some(error);
        self
    }

    pub fn improving_with(improvement: Improvement) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().improvement = Some(improvement);
        fake
    }

    pub fn failing_improve_with(self, error: ProviderError) -> Self {
        self.state.lock().unwrap().improve_error = Some(error);
        self
    }

    pub fn requests(&self) -> Vec<TranslateRequest> {
        self.state.lock().unwrap().requests.clone()
    }

    pub fn improve_requests(&self) -> Vec<ImproveRequest> {
        self.state.lock().unwrap().improve_requests.clone()
    }
}

impl TranslationProvider for FakeProvider {
    fn translate(&self, request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError> {
        let mut state = self.state.lock().unwrap();
        state.requests.push(request.clone());
        if let Some(error) = &state.translate_error {
            return Err(error.clone());
        }
        state
            .translation
            .clone()
            .ok_or(ProviderError::NotConfigured)
    }

    fn improve(&self, request: &ImproveRequest) -> Result<Improvement, ProviderError> {
        let mut state = self.state.lock().unwrap();
        state.improve_requests.push(request.clone());
        if let Some(error) = &state.improve_error {
            return Err(error.clone());
        }
        state.improvement.clone().ok_or(ProviderError::NotConfigured)
    }
}

/// A speech port that records everything it was asked to say.
#[derive(Clone, Default)]
pub struct RecordingSpeech {
    spoken: Arc<Mutex<Vec<String>>>,
}

impl RecordingSpeech {
    pub fn spoken(&self) -> Vec<String> {
        self.spoken.lock().unwrap().clone()
    }
}

impl SpeechOutput for RecordingSpeech {
    fn speak(&self, text: &str) -> Result<(), SpeechError> {
        self.spoken.lock().unwrap().push(text.to_owned());
        Ok(())
    }
}

/// An in-memory history store. Search matches either side, newest first.
#[derive(Clone, Default)]
pub struct InMemoryHistory {
    entries: Arc<Mutex<Vec<HistoryEntry>>>,
}

impl InMemoryHistory {
    pub fn entries(&self) -> Vec<HistoryEntry> {
        self.entries.lock().unwrap().clone()
    }
}

impl HistoryStore for InMemoryHistory {
    fn record(&self, entry: &HistoryEntry) -> Result<(), HistoryError> {
        self.entries.lock().unwrap().push(entry.clone());
        Ok(())
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let needle = query.to_lowercase();
        let entries = self.entries.lock().unwrap();
        Ok(entries
            .iter()
            .rev()
            .filter(|entry| {
                entry.source.to_lowercase().contains(&needle)
                    || entry.result.to_lowercase().contains(&needle)
            })
            .take(limit)
            .cloned()
            .collect())
    }

    fn purge_older_than(&self, cutoff: Timestamp) -> Result<usize, HistoryError> {
        let mut entries = self.entries.lock().unwrap();
        let before = entries.len();
        entries.retain(|entry| entry.created_at >= cutoff);
        Ok(before - entries.len())
    }
}

/// An in-memory settings store.
#[derive(Clone)]
pub struct InMemorySettings {
    settings: Arc<Mutex<Settings>>,
}

impl Default for InMemorySettings {
    fn default() -> Self {
        Self::with(Settings::default())
    }
}

impl InMemorySettings {
    pub fn with(settings: Settings) -> Self {
        Self {
            settings: Arc::new(Mutex::new(settings)),
        }
    }

    pub fn current(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }
}

impl SettingsStore for InMemorySettings {
    fn load(&self) -> Result<Settings, SettingsError> {
        Ok(self.current())
    }

    fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        *self.settings.lock().unwrap() = settings.clone();
        Ok(())
    }
}

/// An in-memory credential store.
#[derive(Clone, Default)]
pub struct FakeCredentials {
    secrets: Arc<Mutex<HashMap<ProviderId, String>>>,
}

impl FakeCredentials {
    pub fn with_secret(provider: ProviderId, secret: impl Into<String>) -> Self {
        let fake = Self::default();
        fake.secrets.lock().unwrap().insert(provider, secret.into());
        fake
    }

    pub fn secrets(&self) -> HashMap<ProviderId, String> {
        self.secrets.lock().unwrap().clone()
    }
}

impl CredentialStore for FakeCredentials {
    fn get(&self, provider: ProviderId) -> Result<Option<String>, CredentialError> {
        Ok(self.secrets.lock().unwrap().get(&provider).cloned())
    }

    fn set(&self, provider: ProviderId, secret: &str) -> Result<(), CredentialError> {
        self.secrets
            .lock()
            .unwrap()
            .insert(provider, secret.to_owned());
        Ok(())
    }

    fn delete(&self, provider: ProviderId) -> Result<(), CredentialError> {
        self.secrets.lock().unwrap().remove(&provider);
        Ok(())
    }
}
