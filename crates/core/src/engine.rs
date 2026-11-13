use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{CaptureError, EngineError};
use crate::language::{guess_language, Language, LanguagePair};
use crate::model::{
    HistoryEntry, Mode, PopupPrefill, ProviderId, ProviderStatus, QuickTranslation, ReadAloud,
    Replacement, Selection, Settings, TextOutcome, Timestamp, Translation,
};
use crate::ports::{
    CredentialStore, HistoryStore, ImproveRequest, SelectionCapture, SettingsStore, SpeechOutput,
    TranslateRequest, TranslationProvider,
};

/// The one seam every non-UI behaviour goes through. Views, panels, menu items and
/// the shell's IPC commands talk to this type and never to a port implementation.
pub struct TranslatorEngine {
    capture: Arc<dyn SelectionCapture>,
    provider: Arc<dyn TranslationProvider>,
    speech: Arc<dyn SpeechOutput>,
    history: Arc<dyn HistoryStore>,
    settings: Arc<dyn SettingsStore>,
    credentials: Arc<dyn CredentialStore>,
}

impl TranslatorEngine {
    pub fn new(
        capture: Arc<dyn SelectionCapture>,
        provider: Arc<dyn TranslationProvider>,
        speech: Arc<dyn SpeechOutput>,
        history: Arc<dyn HistoryStore>,
        settings: Arc<dyn SettingsStore>,
        credentials: Arc<dyn CredentialStore>,
    ) -> Self {
        Self {
            capture,
            provider,
            speech,
            history,
            settings,
            credentials,
        }
    }

    /// Quick Translate: capture the Selection, translate it and remember it.
    pub fn quick_translate(&self) -> Result<QuickTranslation, EngineError> {
        let settings = self.settings.load()?;
        let selection = self.capture.read_selection()?;
        let translation = resolve_translation(
            self.provider.as_ref(),
            &selection.text,
            &settings.language_pair,
        )?;
        record_translation(self.history.as_ref(), &selection.text, &translation);
        Ok(QuickTranslation {
            source_text: selection.text,
            translation,
        })
    }

    /// Replace: translate the Selection and overwrite it in one step (`⌃⌥R`).
    pub fn replace_selection(&self) -> Result<Replacement, EngineError> {
        let settings = self.settings.load()?;
        let selection = self.capture.read_selection()?;
        let translation = resolve_translation(
            self.provider.as_ref(),
            &selection.text,
            &settings.language_pair,
        )?;
        self.capture.replace_selection(&translation.text)?;
        record_translation(self.history.as_ref(), &selection.text, &translation);
        Ok(Replacement {
            source_text: selection.text,
            translation,
        })
    }

    /// The Quick Translate popup's Enter: overwrite the captured Selection with
    /// the translation already on screen. It never reads or translates again —
    /// the user must get exactly what they saw — and it writes through the
    /// target retained when the popup was captured. With nothing retained the
    /// capture port answers `NothingToReplace` rather than pasting somewhere
    /// else.
    pub fn replace_with_translation(
        &self,
        source_text: &str,
        translation: Translation,
    ) -> Result<Replacement, EngineError> {
        self.capture.replace_selection(&translation.text)?;
        record_translation(self.history.as_ref(), source_text, &translation);
        Ok(Replacement {
            source_text: source_text.to_owned(),
            translation,
        })
    }

    /// The Translate Window opens with the Selection already in its input. An
    /// empty Selection is an empty input, not a failure: pressing the shortcut
    /// with nothing selected must not look like the app broke.
    pub fn selection_for_window(&self) -> Result<Selection, EngineError> {
        match self.capture.read_selection() {
            Ok(selection) => Ok(selection),
            Err(CaptureError::NothingSelected) => Ok(Selection::new("")),
            Err(error) => Err(error.into()),
        }
    }

    /// Quick Translate's opening state. It is captured on a worker thread before
    /// the panel is shown, while the app the user is working in still has focus,
    /// and carried into the panel as one typed value. An empty Selection is
    /// "nothing selected", not an empty translating state, and a refused read is
    /// a typed failure the panel can explain.
    pub fn popup_prefill(&self) -> PopupPrefill {
        match self.capture.read_selection() {
            Ok(selection) if selection.text.trim().is_empty() => PopupPrefill::NothingSelected,
            Ok(selection) => PopupPrefill::Selection(selection),
            Err(CaptureError::NothingSelected) => PopupPrefill::NothingSelected,
            Err(error) => PopupPrefill::Failure(error),
        }
    }

    /// Whether the platform has granted the permission Selection capture needs.
    pub fn accessibility_granted(&self) -> bool {
        self.capture.accessibility_granted()
    }

    /// Ask the platform to prompt for the permission, answering whether it is
    /// granted now. The window re-checks this when it regains focus, so a grant
    /// made in System Settings takes effect without relaunching the app.
    pub fn request_accessibility(&self) -> bool {
        self.capture.request_accessibility()
    }

    /// Open the settings pane where the permission is granted.
    pub fn open_accessibility_settings(&self) -> Result<(), EngineError> {
        Ok(self.capture.open_accessibility_settings()?)
    }

    /// Read Aloud: speak the Selection, never the translation, and never call a
    /// provider. With nothing selected there is nothing to hear, so it succeeds
    /// with empty spoken text rather than looking like the app broke; a missing
    /// permission or secure input stays a typed error.
    pub fn read_aloud(&self) -> Result<ReadAloud, EngineError> {
        let selection = match self.capture.read_selection() {
            Ok(selection) => selection,
            Err(CaptureError::NothingSelected) => {
                return Ok(ReadAloud {
                    spoken_text: String::new(),
                })
            }
            Err(error) => return Err(error.into()),
        };
        if selection.text.trim().is_empty() {
            return Ok(ReadAloud {
                spoken_text: String::new(),
            });
        }
        let language = guess_language(&selection.text).unwrap_or_else(Language::english);
        self.speech.speak(&selection.text, &language)?;
        Ok(ReadAloud {
            spoken_text: selection.text,
        })
    }

    /// Translate Window: translate or improve text the user typed, without
    /// touching the Selection.
    pub fn translate_text(&self, text: &str, mode: Mode) -> Result<TextOutcome, EngineError> {
        match mode {
            Mode::Translate => {
                let settings = self.settings.load()?;
                let translation = resolve_translation(
                    self.provider.as_ref(),
                    text,
                    &settings.language_pair,
                )?;
                record_translation(self.history.as_ref(), text, &translation);
                Ok(TextOutcome::Translation(translation))
            }
            Mode::Improve => Ok(TextOutcome::Improvement(improve_with(
                self.provider.as_ref(),
                text,
            )?)),
        }
    }

    /// Which providers have a key in the credential store.
    pub fn provider_status(&self) -> Result<ProviderStatus, EngineError> {
        Ok(ProviderStatus {
            azure_configured: self.credentials.get(ProviderId::Azure)?.is_some(),
            groq_configured: self.credentials.get(ProviderId::Groq)?.is_some(),
        })
    }

    pub fn settings(&self) -> Result<Settings, EngineError> {
        Ok(self.settings.load()?)
    }

    pub fn save_settings(&self, settings: Settings) -> Result<(), EngineError> {
        Ok(self.settings.save(&settings)?)
    }

    /// Dismiss the first-launch onboarding for good, and answer the settings the
    /// app now holds.
    pub fn complete_onboarding(&self) -> Result<Settings, EngineError> {
        let mut settings = self.settings.load()?;
        if settings.onboarding_seen {
            return Ok(settings);
        }
        settings.onboarding_seen = true;
        self.settings.save(&settings)?;
        Ok(settings)
    }

    pub fn history(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, EngineError> {
        Ok(self.history.search(query, limit)?)
    }
}

/// The engine resolves direction from the Language Pair and the local guess.
fn resolve_translation(
    provider: &dyn TranslationProvider,
    text: &str,
    pair: &LanguagePair,
) -> Result<Translation, crate::error::ProviderError> {
    let direction = pair.direction_for(text);
    let response = provider.translate(&TranslateRequest {
        text: text.to_owned(),
        source: Some(direction.source),
        target: direction.target.clone(),
    })?;
    Ok(Translation {
        text: response.text,
        provider: response.provider,
        detected_source: response.detected_source,
        target: direction.target,
    })
}

/// Recording history is best-effort until ticket 09 makes the real store work: a
/// rejected write is logged, never turned into a failed translation.
fn record_translation(history: &dyn HistoryStore, source: &str, translation: &Translation) {
    let entry = HistoryEntry {
        source: source.to_owned(),
        result: translation.text.clone(),
        provider: translation.provider,
        mode: Mode::Translate,
        created_at: now(),
    };
    if let Err(error) = history.record(&entry) {
        eprintln!("Translator: the translation was not recorded in history: {error}");
    }
}

fn now() -> Timestamp {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default();
    Timestamp(seconds)
}

fn improve_with(
    provider: &dyn TranslationProvider,
    text: &str,
) -> Result<crate::model::Improvement, crate::error::ProviderError> {
    provider.improve(&ImproveRequest {
        text: text.to_owned(),
    })
}
