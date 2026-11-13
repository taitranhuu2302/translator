//! Fakes for the six ports, kept in one module so the port shapes live in one
//! place. Every fake is cheap to clone: the test keeps a handle for assertions
//! while the engine owns its own.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use std::collections::VecDeque;

use crate::error::{
    CaptureError, CredentialError, HistoryError, HttpError, ProviderError, SettingsError,
    SpeechError,
};
use crate::language::Language;
use crate::model::{
    HistoryEntry, ImproveDelta, Improvement, ProviderId, ProviderTranslation, Selection, Settings,
    Timestamp,
};
use crate::ports::{
    CredentialStore, HistoryStore, HttpClient, HttpRequest, HttpResponse, ImproveRequest,
    SelectionCapture, SettingsStore, SpeechOutput, TranslateRequest, TranslationProvider,
};

/// A capture port that can be told to succeed, fail, or accept or refuse writes.
/// Without a configured selection it fails the way the real port does, so a test
/// cannot pass on text no capture would have returned.
#[derive(Clone, Default)]
pub struct FakeCapture {
    state: Arc<Mutex<CaptureState>>,
}

struct CaptureState {
    selection: Option<Selection>,
    read_error: Option<CaptureError>,
    write_error: Option<CaptureError>,
    reads: usize,
    writes: Vec<String>,
    accessibility_granted: bool,
    prompt_grants_accessibility: bool,
    accessibility_requests: usize,
    settings_panes_opened: usize,
}

impl Default for CaptureState {
    fn default() -> Self {
        Self {
            selection: None,
            read_error: None,
            write_error: None,
            reads: 0,
            writes: Vec::new(),
            accessibility_granted: true,
            prompt_grants_accessibility: true,
            accessibility_requests: 0,
            settings_panes_opened: 0,
        }
    }
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

    /// Accessibility denied, and the system prompt will keep it denied.
    pub fn not_granted(self) -> Self {
        let mut state = self.state.lock().unwrap();
        state.accessibility_granted = false;
        state.prompt_grants_accessibility = false;
        drop(state);
        self
    }

    /// Accessibility denied until the user answers the system prompt.
    pub fn granted_after_prompt(self) -> Self {
        let mut state = self.state.lock().unwrap();
        state.accessibility_granted = false;
        state.prompt_grants_accessibility = true;
        drop(state);
        self
    }

    pub fn reads(&self) -> usize {
        self.state.lock().unwrap().reads
    }

    pub fn writes(&self) -> Vec<String> {
        self.state.lock().unwrap().writes.clone()
    }

    pub fn accessibility_requests(&self) -> usize {
        self.state.lock().unwrap().accessibility_requests
    }

    pub fn settings_panes_opened(&self) -> usize {
        self.state.lock().unwrap().settings_panes_opened
    }
}

impl SelectionCapture for FakeCapture {
    fn read_selection(&self) -> Result<Selection, CaptureError> {
        let mut state = self.state.lock().unwrap();
        state.reads += 1;
        if let Some(error) = &state.read_error {
            return Err(error.clone());
        }
        state.selection.clone().ok_or(CaptureError::NothingSelected)
    }

    fn replace_selection(&self, text: &str) -> Result<(), CaptureError> {
        let mut state = self.state.lock().unwrap();
        // The fake retains the Selection its read returned, the way the real
        // port retains the Accessibility target. With nothing retained there is
        // nowhere to write, and it says so instead of pretending it replaced.
        if state.selection.is_none() {
            return Err(CaptureError::NothingToReplace);
        }
        if let Some(error) = &state.write_error {
            return Err(error.clone());
        }
        state.writes.push(text.to_owned());
        Ok(())
    }

    fn accessibility_granted(&self) -> bool {
        self.state.lock().unwrap().accessibility_granted
    }

    fn request_accessibility(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        state.accessibility_requests += 1;
        if state.prompt_grants_accessibility {
            state.accessibility_granted = true;
        }
        state.accessibility_granted
    }

    fn open_accessibility_settings(&self) -> Result<(), CaptureError> {
        self.state.lock().unwrap().settings_panes_opened += 1;
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
    improve_deltas: Vec<ImproveDelta>,
    requests: Vec<TranslateRequest>,
    improve_requests: Vec<ImproveRequest>,
    skipped: Vec<ProviderId>,
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

    /// Answer an improve request with this improvement and report these deltas,
    /// in order, before returning it. The delta text is the test's to keep
    /// consistent with the improvement; the engine must not merge or infer.
    pub fn streaming_improvement(improvement: Improvement, deltas: Vec<ImproveDelta>) -> Self {
        let fake = Self::default();
        let mut state = fake.state.lock().unwrap();
        state.improvement = Some(improvement);
        state.improve_deltas = deltas;
        drop(state);
        fake
    }

    /// Report a provider as skipped for this session, as the real ladder does
    /// after an auth or quota failure.
    pub fn skipping(self, provider: ProviderId) -> Self {
        self.state.lock().unwrap().skipped.push(provider);
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
        state
            .improvement
            .clone()
            .ok_or(ProviderError::NotConfigured)
    }

    fn improve_streaming(
        &self,
        request: &ImproveRequest,
        sink: &mut dyn FnMut(&ImproveDelta),
    ) -> Result<Improvement, ProviderError> {
        let (answer, deltas) = {
            let mut state = self.state.lock().unwrap();
            state.improve_requests.push(request.clone());
            if let Some(error) = &state.improve_error {
                return Err(error.clone());
            }
            let answer = state
                .improvement
                .clone()
                .ok_or(ProviderError::NotConfigured)?;
            (answer, state.improve_deltas.clone())
        };
        for delta in &deltas {
            sink(delta);
        }
        Ok(answer)
    }

    fn skipped(&self) -> Vec<ProviderId> {
        self.state.lock().unwrap().skipped.clone()
    }

    fn clear_skip(&self, provider: ProviderId) {
        self.state
            .lock()
            .unwrap()
            .skipped
            .retain(|skipped| *skipped != provider);
    }
}

/// An HTTP client that records every request and answers from a script. It is the
/// only way provider tests touch the wire: no test opens a socket.
#[derive(Clone, Default)]
pub struct FakeHttpClient {
    state: Arc<Mutex<HttpState>>,
}

#[derive(Default)]
struct HttpState {
    /// Answers consumed in order. An empty script uses `fallback`.
    scripted: VecDeque<Result<HttpResponse, HttpError>>,
    fallback: Option<Result<HttpResponse, HttpError>>,
    /// Streamed answers consumed in order. An empty script uses `stream_fallback`.
    streamed: VecDeque<Result<StreamedAnswer, HttpError>>,
    stream_fallback: Option<Result<StreamedAnswer, HttpError>>,
    requests: Vec<HttpRequest>,
}

/// One streamed response: the status plus the exact byte chunks the test chose.
/// The chunk boundaries are the point — a UTF-8 character or an SSE line may be
/// split across them, exactly as a socket may deliver it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamedAnswer {
    pub status: u16,
    pub chunks: Vec<Vec<u8>>,
}

impl StreamedAnswer {
    /// One chunk holding the whole body.
    pub fn whole(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            chunks: vec![body.into().into_bytes()],
        }
    }
}

impl FakeHttpClient {
    /// Always answer with this response (status and body are the provider's to
    /// interpret, exactly as a real client would hand them over).
    pub fn responding_with(status: u16, body: impl Into<String>) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().fallback = Some(Ok(HttpResponse {
            status,
            body: body.into(),
        }));
        fake
    }

    /// Always fail this way.
    pub fn failing_with(error: HttpError) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().fallback = Some(Err(error));
        fake
    }

    /// Answer each request in turn. Once the script is spent, requests fail as
    /// "the test scripted no answer", which is the shape chunked translation
    /// needs: a missing answer is a test bug, not a silent success.
    pub fn responding_in_turn(answers: Vec<Result<HttpResponse, HttpError>>) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().scripted = answers.into();
        fake
    }

    /// Stream this body in these exact chunks to every `stream` call. A status
    /// outside 2xx streams its body too, so the provider can read the error.
    pub fn streaming_in_chunks(chunks: Vec<Vec<u8>>) -> Self {
        Self::streaming_with_status(200, chunks)
    }

    /// Stream this body in these exact chunks with a chosen status.
    pub fn streaming_with_status(status: u16, chunks: Vec<Vec<u8>>) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().stream_fallback = Some(Ok(StreamedAnswer { status, chunks }));
        fake
    }

    /// Stream one body as a single chunk.
    pub fn streaming(status: u16, body: impl Into<String>) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().stream_fallback = Some(Ok(StreamedAnswer::whole(status, body)));
        fake
    }

    /// Fail every `stream` call before any body arrives.
    pub fn failing_stream_with(error: HttpError) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().stream_fallback = Some(Err(error));
        fake
    }

    pub fn requests(&self) -> Vec<HttpRequest> {
        self.state.lock().unwrap().requests.clone()
    }
}

impl HttpClient for FakeHttpClient {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpError> {
        let mut state = self.state.lock().unwrap();
        state.requests.push(request.clone());
        if let Some(answer) = state.scripted.pop_front() {
            return answer;
        }
        match &state.fallback {
            Some(answer) => answer.clone(),
            None => Err(HttpError::Request {
                message: "the test scripted no answer".to_owned(),
            }),
        }
    }

    fn stream(
        &self,
        request: &HttpRequest,
        on_chunk: &mut dyn FnMut(&[u8]) -> bool,
    ) -> Result<u16, HttpError> {
        let answer = {
            let mut state = self.state.lock().unwrap();
            state.requests.push(request.clone());
            if let Some(answer) = state.streamed.pop_front() {
                answer
            } else if let Some(answer) = &state.stream_fallback {
                answer.clone()
            } else {
                // A scripted `execute` answer still streams, one chunk, so a test
                // need not repeat the body.
                match &state.fallback {
                    Some(Ok(response)) => Ok(StreamedAnswer::whole(
                        response.status,
                        response.body.clone(),
                    )),
                    Some(Err(error)) => Err(error.clone()),
                    None => Err(HttpError::Request {
                        message: "the test scripted no answer".to_owned(),
                    }),
                }
            }
        };
        match answer {
            Ok(answer) => {
                for chunk in &answer.chunks {
                    if !on_chunk(chunk) {
                        break;
                    }
                }
                Ok(answer.status)
            }
            Err(error) => Err(error),
        }
    }
}

/// One thing the fake speech port was asked to say, and the language the engine
/// asked for it to be said in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpokenUtterance {
    pub text: String,
    pub language: Language,
}

/// A speech port that records everything it was asked to say, or fails every
/// request with a scripted error.
#[derive(Clone, Default)]
pub struct RecordingSpeech {
    state: Arc<Mutex<SpeechState>>,
}

#[derive(Default)]
struct SpeechState {
    spoken: Vec<SpokenUtterance>,
    error: Option<SpeechError>,
}

impl RecordingSpeech {
    pub fn failing_with(error: SpeechError) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().error = Some(error);
        fake
    }

    pub fn spoken(&self) -> Vec<SpokenUtterance> {
        self.state.lock().unwrap().spoken.clone()
    }
}

impl SpeechOutput for RecordingSpeech {
    fn speak(&self, text: &str, language: &Language) -> Result<(), SpeechError> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = &state.error {
            return Err(error.clone());
        }
        state.spoken.push(SpokenUtterance {
            text: text.to_owned(),
            language: language.clone(),
        });
        Ok(())
    }
}

/// An in-memory history store. Search matches either side, newest first. Writes
/// can be scripted to fail, which is how the best-effort rule is tested.
#[derive(Clone, Default)]
pub struct InMemoryHistory {
    state: Arc<Mutex<HistoryState>>,
}

#[derive(Default)]
struct HistoryState {
    entries: Vec<HistoryEntry>,
    write_error: Option<HistoryError>,
    records: usize,
}

impl InMemoryHistory {
    pub fn failing_writes_with(error: HistoryError) -> Self {
        let fake = Self::default();
        fake.state.lock().unwrap().write_error = Some(error);
        fake
    }

    pub fn entries(&self) -> Vec<HistoryEntry> {
        self.state.lock().unwrap().entries.clone()
    }

    /// How many times `record` was called, including rejected writes. It is what
    /// proves one translation writes one row, not two.
    pub fn records(&self) -> usize {
        self.state.lock().unwrap().records
    }
}

impl HistoryStore for InMemoryHistory {
    fn record(&self, entry: &HistoryEntry) -> Result<(), HistoryError> {
        let mut state = self.state.lock().unwrap();
        state.records += 1;
        if let Some(error) = &state.write_error {
            return Err(error.clone());
        }
        state.entries.push(entry.clone());
        Ok(())
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, HistoryError> {
        let needle = query.to_lowercase();
        let state = self.state.lock().unwrap();
        Ok(state
            .entries
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
        let mut state = self.state.lock().unwrap();
        let before = state.entries.len();
        state.entries.retain(|entry| entry.created_at >= cutoff);
        Ok(before - state.entries.len())
    }
}

/// An in-memory settings store.
#[derive(Clone)]
pub struct InMemorySettings {
    settings: Arc<Mutex<Settings>>,
    saves: Arc<Mutex<usize>>,
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
            saves: Arc::new(Mutex::new(0)),
        }
    }

    pub fn current(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// How many times the app wrote the settings document. A window that changes
    /// its own direction must leave this at zero.
    pub fn saves(&self) -> usize {
        *self.saves.lock().unwrap()
    }
}

impl SettingsStore for InMemorySettings {
    fn load(&self) -> Result<Settings, SettingsError> {
        Ok(self.current())
    }

    fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        *self.settings.lock().unwrap() = settings.clone();
        *self.saves.lock().unwrap() += 1;
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
