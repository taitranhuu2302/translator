use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{CaptureError, EngineError};
use crate::language::{guess_language, Direction, Language, LanguagePair};
use crate::model::{
    HistoryEntry, ImproveDelta, Improvement, Mode, PopupPrefill, ProviderId, ProviderStatus,
    QuickTranslation, ReadAloud, Replacement, Selection, Settings, TextOutcome, Timestamp,
    Translation,
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
        record_translation(self.history.as_ref(), &selection.text, &translation, None);
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
        record_translation(self.history.as_ref(), &selection.text, &translation, None);
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
    ///
    /// It records nothing: Quick Translate already recorded this translation
    /// when it produced it, and one translation is one history row.
    pub fn replace_with_translation(
        &self,
        source_text: &str,
        translation: Translation,
    ) -> Result<Replacement, EngineError> {
        self.capture.replace_selection(&translation.text)?;
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
        self.translate_text_in(text, mode, None)
    }

    /// The same translation, optionally in a direction the caller chose for this
    /// request. The Translate Window's Swap passes one so the reversed direction
    /// applies to that window only: an override neither reads nor writes the
    /// saved Language Pair. Everything else — the provider ladder, caching and
    /// history — is identical to `translate_text`, so a swap is not a second
    /// translation path.
    pub fn translate_text_in(
        &self,
        text: &str,
        mode: Mode,
        direction: Option<Direction>,
    ) -> Result<TextOutcome, EngineError> {
        match mode {
            Mode::Translate => {
                let (translation, direction) = match direction {
                    Some(direction) => (
                        translate_in_direction(self.provider.as_ref(), text, direction.clone())?,
                        Some(direction),
                    ),
                    None => {
                        let settings = self.settings.load()?;
                        (
                            resolve_translation(
                                self.provider.as_ref(),
                                text,
                                &settings.language_pair,
                            )?,
                            None,
                        )
                    }
                };
                record_translation(self.history.as_ref(), text, &translation, direction);
                Ok(TextOutcome::Translation(translation))
            }
            Mode::Improve => {
                let improvement = improve_with(self.provider.as_ref(), text)?;
                record_improvement(self.history.as_ref(), text, &improvement);
                Ok(TextOutcome::Improvement(improvement))
            }
        }
    }

    /// Improve text the user typed, reporting each field as the model writes it.
    /// The returned `Improvement` is still the authority: a stream that never
    /// produced both fields is an error, and the view replaces what it showed
    /// with these two values on done.
    ///
    /// Whitespace-only text has nothing to improve: it answers with two empty
    /// parts, never touches the network (or the Keychain) and records nothing.
    /// Every successful improvement is recorded exactly once, whichever of the
    /// two entry points produced it.
    pub fn improve_streaming(
        &self,
        text: &str,
        sink: &mut dyn FnMut(&ImproveDelta),
    ) -> Result<Improvement, EngineError> {
        if text.trim().is_empty() {
            return Ok(Improvement {
                corrected: String::new(),
                better: String::new(),
            });
        }
        let improvement = self.provider.improve_streaming(
            &ImproveRequest {
                text: text.to_owned(),
            },
            sink,
        )?;
        record_improvement(self.history.as_ref(), text, &improvement);
        Ok(improvement)
    }

    /// Which providers have a key in the credential store, and which the session
    /// has skipped after an auth or quota failure.
    pub fn provider_status(&self) -> Result<ProviderStatus, EngineError> {
        let skipped = self.provider.skipped();
        Ok(ProviderStatus {
            azure_configured: self.credentials.get(ProviderId::Azure)?.is_some(),
            groq_configured: self.credentials.get(ProviderId::Groq)?.is_some(),
            azure_skipped: skipped.contains(&ProviderId::Azure),
            groq_skipped: skipped.contains(&ProviderId::Groq),
        })
    }

    /// Save one provider's key in the Keychain. An empty or whitespace-only key
    /// clears it, so Settings needs only one field: typing nothing means "no key".
    /// Either way the session skip is forgotten, so a fixed key takes effect
    /// without restarting the app.
    pub fn save_provider_key(
        &self,
        provider: ProviderId,
        key: &str,
    ) -> Result<ProviderStatus, EngineError> {
        let key = key.trim();
        if key.is_empty() {
            self.credentials.delete(provider)?;
        } else {
            self.credentials.set(provider, key)?;
        }
        self.provider.clear_skip(provider);
        self.provider_status()
    }

    /// Forget one provider's key and its session skip.
    pub fn clear_provider_key(&self, provider: ProviderId) -> Result<ProviderStatus, EngineError> {
        self.credentials.delete(provider)?;
        self.provider.clear_skip(provider);
        self.provider_status()
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

    /// Delete history older than the 30-day retention window, answering how many
    /// rows went. The shell calls this once at launch and again daily; a failure
    /// is logged there and never blocks startup.
    pub fn purge_history(&self) -> Result<usize, EngineError> {
        Ok(self.history.purge_older_than(history_cutoff(now()))?)
    }

    /// Shape a stored entry for the Translate Window again. It reads no
    /// provider and writes no history row: reopening something already on this
    /// Mac must cost no quota and must not duplicate the entry.
    ///
    /// A translation keeps the direction it was produced in. An older entry with
    /// no known direction falls back to the saved pair's Target, which is what
    /// any new translation would have used. An improvement returns both stored
    /// parts, with an empty corrected part when an older row never had one.
    pub fn reopen(&self, entry: &HistoryEntry) -> Result<TextOutcome, EngineError> {
        match entry.mode {
            Mode::Translate => {
                let target = match &entry.direction {
                    Some(direction) => direction.target.clone(),
                    None => self.settings.load()?.language_pair.target,
                };
                Ok(TextOutcome::Translation(Translation {
                    text: entry.result.clone(),
                    provider: entry.provider,
                    detected_source: entry
                        .direction
                        .as_ref()
                        .map(|direction| direction.source.clone()),
                    target,
                }))
            }
            Mode::Improve => Ok(TextOutcome::Improvement(Improvement {
                corrected: entry.corrected.clone().unwrap_or_default(),
                better: entry.result.clone(),
            })),
        }
    }
}

/// How long history is kept: the spec's 30 days.
pub const HISTORY_RETENTION_SECONDS: i64 = 30 * 24 * 60 * 60;

/// The instant rows must be older than to be purged. A row exactly this old
/// survives; a row one second older does not (`purge_older_than` deletes
/// `created_at < cutoff`).
pub fn history_cutoff(now: Timestamp) -> Timestamp {
    Timestamp(now.0 - HISTORY_RETENTION_SECONDS)
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

/// Translate in a direction chosen by the caller (the Translate Window's Swap).
fn translate_in_direction(
    provider: &dyn TranslationProvider,
    text: &str,
    direction: Direction,
) -> Result<Translation, crate::error::ProviderError> {
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

/// Recording history is best-effort: a rejected write is logged, never turned
/// into a failed translation. The direction is recorded only when it is known —
/// the provider's detection, or a direction the window chose for itself.
fn record_translation(
    history: &dyn HistoryStore,
    source: &str,
    translation: &Translation,
    known_direction: Option<Direction>,
) {
    let direction = known_direction.or_else(|| {
        translation.detected_source.clone().map(|source| Direction {
            source,
            target: translation.target.clone(),
        })
    });
    let entry = HistoryEntry {
        source: source.to_owned(),
        result: translation.text.clone(),
        provider: translation.provider,
        mode: Mode::Translate,
        created_at: now(),
        direction,
        corrected: None,
    };
    if let Err(error) = history.record(&entry) {
        eprintln!("Translator: the translation was not recorded in history: {error}");
    }
}

/// Improve is recorded with both labelled parts: `result` holds the better
/// sentence and `corrected` the corrected one. v1 has exactly one Improve
/// provider, so the provider is Groq; a second one would have to put the
/// provider that answered into the improvement itself.
fn record_improvement(history: &dyn HistoryStore, source: &str, improvement: &Improvement) {
    let entry = HistoryEntry {
        source: source.to_owned(),
        result: improvement.better.clone(),
        provider: ProviderId::Groq,
        mode: Mode::Improve,
        created_at: now(),
        direction: None,
        corrected: Some(improvement.corrected.clone()),
    };
    if let Err(error) = history.record(&entry) {
        eprintln!("Translator: the improvement was not recorded in history: {error}");
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

#[cfg(test)]
mod tests {
    use super::{history_cutoff, HISTORY_RETENTION_SECONDS};
    use crate::model::{HistoryEntry, Mode, ProviderId, Timestamp};
    use crate::ports::HistoryStore;
    use crate::testing::InMemoryHistory;

    #[test]
    fn the_cutoff_is_exactly_thirty_days_before_now() {
        let now = Timestamp(2_000_000_000);
        let cutoff = history_cutoff(now);

        assert_eq!(HISTORY_RETENTION_SECONDS, 30 * 24 * 60 * 60);
        assert_eq!(cutoff, Timestamp(now.0 - HISTORY_RETENTION_SECONDS));
    }

    #[test]
    fn an_entry_exactly_thirty_days_old_survives_and_one_second_older_is_purged() {
        let now = Timestamp(2_000_000_000);
        let cutoff = history_cutoff(now);
        let history = InMemoryHistory::default();
        let mut entry = HistoryEntry {
            source: "Xin chào".to_owned(),
            result: "Hello".to_owned(),
            provider: ProviderId::MyMemory,
            mode: Mode::Translate,
            created_at: cutoff,
            direction: None,
            corrected: None,
        };

        history.record(&entry).expect("record the boundary entry");
        entry.created_at = Timestamp(cutoff.0 - 1);
        history.record(&entry).expect("record the older entry");

        let removed = history.purge_older_than(cutoff).expect("purge older rows");

        assert_eq!(removed, 1);
        assert_eq!(history.entries().len(), 1);
        assert_eq!(history.entries()[0].created_at, cutoff);
    }
}
