use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::EngineError;
use crate::language::LanguagePair;
use crate::model::{
    HistoryEntry, Mode, ProviderId, ProviderStatus, QuickTranslation, ReadAloud, Replacement,
    Settings, TextOutcome, Timestamp, Translation,
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
        record_translation(self.history.as_ref(), &selection.text, &translation)?;
        Ok(QuickTranslation {
            source_text: selection.text,
            translation,
        })
    }

    /// Replace: translate the Selection and overwrite it in one step.
    pub fn replace_selection(&self) -> Result<Replacement, EngineError> {
        let settings = self.settings.load()?;
        let selection = self.capture.read_selection()?;
        let translation = resolve_translation(
            self.provider.as_ref(),
            &selection.text,
            &settings.language_pair,
        )?;
        self.capture.replace_selection(&translation.text)?;
        record_translation(self.history.as_ref(), &selection.text, &translation)?;
        Ok(Replacement {
            source_text: selection.text,
            translation,
        })
    }

    /// Read Aloud: speak the Selection, never the translation.
    pub fn read_aloud(&self) -> Result<ReadAloud, EngineError> {
        let selection = self.capture.read_selection()?;
        self.speech.speak(&selection.text)?;
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
                record_translation(self.history.as_ref(), text, &translation)?;
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

    pub fn history(&self, query: &str, limit: usize) -> Result<Vec<HistoryEntry>, EngineError> {
        Ok(self.history.search(query, limit)?)
    }
}

/// The engine resolves direction from the Language Pair. Detection-driven
/// direction arrives with the provider ladder (ticket 06).
fn resolve_translation(
    provider: &dyn TranslationProvider,
    text: &str,
    pair: &LanguagePair,
) -> Result<Translation, crate::error::ProviderError> {
    let target = pair.destination_for(None);
    let response = provider.translate(&TranslateRequest {
        text: text.to_owned(),
        source: None,
        target: target.clone(),
    })?;
    Ok(Translation {
        text: response.text,
        provider: response.provider,
        detected_source: response.detected_source,
        target,
    })
}

fn record_translation(
    history: &dyn HistoryStore,
    source: &str,
    translation: &Translation,
) -> Result<(), crate::error::HistoryError> {
    history.record(&HistoryEntry {
        source: source.to_owned(),
        result: translation.text.clone(),
        provider: translation.provider,
        mode: Mode::Translate,
        created_at: now(),
    })
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
