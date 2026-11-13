//! The seam test: the engine is driven through its public API with fake ports, and
//! every assertion is on an observable outcome — the translation returned, the
//! typed error, the bytes sent to a provider, what was written to history.

use std::sync::Arc;

use translator_core::testing::{
    FakeCapture, FakeCredentials, FakeProvider, InMemoryHistory, InMemorySettings, RecordingSpeech,
    SpokenUtterance,
};
use translator_core::{
    CachedProvider, CaptureError, Direction, EngineError, HistoryEntry, HistoryStore, ImproveDelta,
    ImprovePart, ImproveRequest, Improvement, Language, LanguagePair, Mode, PopupPrefill,
    ProviderError, ProviderId, ProviderTranslation, Selection, Settings, SettingsStore,
    SpeechError, TextOutcome, Timestamp, TranslateRequest, Translation, TranslatorEngine,
};

struct Harness {
    capture: FakeCapture,
    provider: FakeProvider,
    speech: RecordingSpeech,
    history: InMemoryHistory,
    settings: InMemorySettings,
    credentials: FakeCredentials,
}

impl Harness {
    fn new(capture: FakeCapture, provider: FakeProvider) -> Self {
        Self {
            capture,
            provider,
            speech: RecordingSpeech::default(),
            history: InMemoryHistory::default(),
            settings: InMemorySettings::with(Settings {
                language_pair: LanguagePair::new(Language::english(), Language::vietnamese()),
                ..Settings::default()
            }),
            credentials: FakeCredentials::default(),
        }
    }

    fn with_speech(mut self, speech: RecordingSpeech) -> Self {
        self.speech = speech;
        self
    }

    fn engine(&self) -> TranslatorEngine {
        TranslatorEngine::new(
            Arc::new(self.capture.clone()),
            Arc::new(self.provider.clone()),
            Arc::new(self.speech.clone()),
            Arc::new(self.history.clone()),
            Arc::new(self.settings.clone()),
            Arc::new(self.credentials.clone()),
        )
    }
}

fn provider_saying(text: &str) -> FakeProvider {
    FakeProvider::responding_with(ProviderTranslation {
        text: text.to_owned(),
        provider: ProviderId::MyMemory,
        detected_source: Some(Language::vietnamese()),
    })
}

#[test]
fn quick_translate_sends_the_selection_to_the_provider_and_records_it_in_history() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );

    let outcome = harness.engine().quick_translate().expect("quick translate");

    assert_eq!(outcome.source_text, "Xin chào");
    assert_eq!(outcome.translation.text, "Hello");
    assert_eq!(outcome.translation.provider, ProviderId::MyMemory);
    assert_eq!(outcome.translation.target, Language::english());
    assert_eq!(
        harness.provider.requests(),
        vec![TranslateRequest {
            text: "Xin chào".to_owned(),
            source: Some(Language::vietnamese()),
            target: Language::english(),
        }]
    );

    let recorded = harness.history.entries();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].source, "Xin chào");
    assert_eq!(recorded[0].result, "Hello");
    assert_eq!(recorded[0].provider, ProviderId::MyMemory);
    assert_eq!(recorded[0].mode, Mode::Translate);
}

#[test]
fn a_vietnamese_selection_is_translated_into_the_english_target() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );

    let outcome = harness.engine().quick_translate().expect("quick translate");

    assert_eq!(outcome.translation.target, Language::english());
    assert_eq!(
        harness.provider.requests(),
        vec![TranslateRequest {
            text: "Xin chào".to_owned(),
            source: Some(Language::vietnamese()),
            target: Language::english(),
        }]
    );
}

#[test]
fn an_english_selection_is_translated_into_the_partner_when_english_is_the_target() {
    let harness = Harness::new(
        FakeCapture::with_selection("Good morning"),
        provider_saying("Chào buổi sáng"),
    );

    let outcome = harness.engine().quick_translate().expect("quick translate");

    assert_eq!(outcome.translation.target, Language::vietnamese());
    assert_eq!(
        harness.provider.requests(),
        vec![TranslateRequest {
            text: "Good morning".to_owned(),
            source: Some(Language::english()),
            target: Language::vietnamese(),
        }]
    );
}

#[test]
fn with_vietnamese_as_the_target_an_english_selection_goes_to_vietnamese() {
    let mut harness = Harness::new(
        FakeCapture::with_selection("Good morning"),
        provider_saying("Chào buổi sáng"),
    );
    harness.settings = InMemorySettings::with(Settings {
        language_pair: LanguagePair::new(Language::vietnamese(), Language::english()),
        ..Settings::default()
    });

    let outcome = harness.engine().quick_translate().expect("quick translate");

    assert_eq!(outcome.translation.target, Language::vietnamese());
    assert_eq!(
        harness.provider.requests(),
        vec![TranslateRequest {
            text: "Good morning".to_owned(),
            source: Some(Language::english()),
            target: Language::vietnamese(),
        }]
    );
}

#[test]
fn direction_never_asks_a_provider_to_translate_a_language_into_itself() {
    // A pair that names the same language twice has no partner to send the text
    // to, so the guard falls back to the other language of the v1 pair instead of
    // sending `en|en`, which the provider rejects.
    let mut harness = Harness::new(
        FakeCapture::with_selection("Good morning"),
        provider_saying("Chào buổi sáng"),
    );
    harness.settings = InMemorySettings::with(Settings {
        language_pair: LanguagePair::new(Language::english(), Language::english()),
        ..Settings::default()
    });

    harness.engine().quick_translate().expect("quick translate");

    let request = &harness.provider.requests()[0];
    assert_ne!(request.source, Some(request.target.clone()));
}

#[test]
fn a_rejected_history_write_never_fails_the_translation() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );
    let engine = TranslatorEngine::new(
        Arc::new(harness.capture.clone()),
        Arc::new(harness.provider.clone()),
        Arc::new(harness.speech.clone()),
        Arc::new(InMemoryHistory::failing_writes_with(
            translator_core::HistoryError::Unavailable {
                reason: "history storage lands in ticket 09".to_owned(),
            },
        )),
        Arc::new(harness.settings.clone()),
        Arc::new(harness.credentials.clone()),
    );

    let outcome = engine.quick_translate().expect("history is best-effort");

    assert_eq!(outcome.translation.text, "Hello");
    assert_eq!(harness.provider.requests().len(), 1);
}

#[test]
fn replace_selection_writes_the_translation_over_the_selection() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );

    let outcome = harness
        .engine()
        .replace_selection()
        .expect("replace selection");

    assert_eq!(outcome.translation.text, "Hello");
    assert_eq!(harness.capture.writes(), vec!["Hello".to_owned()]);
    assert_eq!(harness.history.entries().len(), 1);
}

#[test]
fn the_popup_replace_writes_the_translation_the_user_saw_without_reading_again() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );
    let translation = Translation {
        text: "Hello".to_owned(),
        provider: ProviderId::MyMemory,
        detected_source: Some(Language::vietnamese()),
        target: Language::english(),
    };

    let outcome = harness
        .engine()
        .replace_with_translation("Xin chào", translation.clone())
        .expect("replace the translation the popup showed");

    assert_eq!(outcome.source_text, "Xin chào");
    assert_eq!(outcome.translation, translation);
    assert_eq!(harness.capture.writes(), vec!["Hello".to_owned()]);
    assert_eq!(
        harness.capture.reads(),
        0,
        "the popup already holds the translation; Replace must not re-read the Selection"
    );
    assert!(
        harness.provider.requests().is_empty(),
        "the user must get exactly the text they saw, not a second translation"
    );

    let recorded = harness.history.entries();
    assert!(
        recorded.is_empty(),
        "Quick Translate recorded this translation when it produced it; Enter writes no second row"
    );
}

#[test]
fn a_popup_replace_with_no_captured_selection_writes_nothing() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));
    let translation = Translation {
        text: "Hello".to_owned(),
        provider: ProviderId::MyMemory,
        detected_source: None,
        target: Language::english(),
    };

    let error = harness
        .engine()
        .replace_with_translation("Xin chào", translation)
        .unwrap_err();

    assert_eq!(error, EngineError::Capture(CaptureError::NothingToReplace));
    assert!(harness.capture.writes().is_empty());
    assert!(harness.history.entries().is_empty());
}

#[test]
fn a_popup_replace_that_the_app_refuses_is_typed_and_records_no_history() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào").failing_writes_with(CaptureError::NotWritable),
        provider_saying("Hello"),
    );
    let translation = Translation {
        text: "Hello".to_owned(),
        provider: ProviderId::MyMemory,
        detected_source: None,
        target: Language::english(),
    };

    let error = harness
        .engine()
        .replace_with_translation("Xin chào", translation)
        .unwrap_err();

    assert_eq!(error, EngineError::Capture(CaptureError::NotWritable));
    assert!(harness.capture.writes().is_empty());
    assert!(harness.history.entries().is_empty());
}

#[test]
fn secure_input_refuses_a_popup_replace_before_anything_is_written() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào").failing_writes_with(CaptureError::SecureInput),
        provider_saying("Hello"),
    );
    let translation = Translation {
        text: "Hello".to_owned(),
        provider: ProviderId::MyMemory,
        detected_source: None,
        target: Language::english(),
    };

    let error = harness
        .engine()
        .replace_with_translation("Xin chào", translation)
        .unwrap_err();

    assert_eq!(error, EngineError::Capture(CaptureError::SecureInput));
    assert!(harness.capture.writes().is_empty());
    assert!(harness.history.entries().is_empty());
}

#[test]
fn a_rejected_history_write_never_fails_a_replace() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );
    let engine = TranslatorEngine::new(
        Arc::new(harness.capture.clone()),
        Arc::new(harness.provider.clone()),
        Arc::new(harness.speech.clone()),
        Arc::new(InMemoryHistory::failing_writes_with(
            translator_core::HistoryError::Unavailable {
                reason: "history storage lands in ticket 09".to_owned(),
            },
        )),
        Arc::new(harness.settings.clone()),
        Arc::new(harness.credentials.clone()),
    );

    let outcome = engine.replace_selection().expect("history is best-effort");

    assert_eq!(outcome.translation.text, "Hello");
    assert_eq!(harness.capture.writes(), vec!["Hello".to_owned()]);
}

#[test]
fn read_aloud_speaks_the_selection_and_never_the_translation() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );

    let outcome = harness.engine().read_aloud().expect("read aloud");

    assert_eq!(outcome.spoken_text, "Xin chào");
    assert_eq!(
        harness.speech.spoken(),
        vec![SpokenUtterance {
            text: "Xin chào".to_owned(),
            language: Language::vietnamese(),
        }]
    );
    assert!(harness.provider.requests().is_empty());
}

#[test]
fn read_aloud_picks_the_voice_from_the_language_the_selection_is_written_in() {
    let vietnamese = Harness::new(
        FakeCapture::with_selection("Tiếng Việt"),
        provider_saying("English"),
    );
    let english = Harness::new(
        FakeCapture::with_selection("Good morning"),
        provider_saying("Chào buổi sáng"),
    );

    vietnamese.engine().read_aloud().expect("read vietnamese");
    english.engine().read_aloud().expect("read english");

    assert_eq!(
        vietnamese.speech.spoken()[0].language,
        Language::vietnamese()
    );
    assert_eq!(english.speech.spoken()[0].language, Language::english());
}

#[test]
fn read_aloud_falls_back_to_english_when_there_is_no_letter_to_judge_by() {
    let harness = Harness::new(
        FakeCapture::with_selection("42 — 3.14"),
        provider_saying("Hello"),
    );

    harness.engine().read_aloud().expect("read aloud");

    assert_eq!(harness.speech.spoken()[0].language, Language::english());
}

#[test]
fn read_aloud_with_nothing_selected_succeeds_without_speaking() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));

    let outcome = harness.engine().read_aloud().expect("nothing selected");

    assert_eq!(outcome.spoken_text, "");
    assert!(harness.speech.spoken().is_empty());
    assert!(harness.provider.requests().is_empty());
}

#[test]
fn read_aloud_reports_a_missing_accessibility_grant_instead_of_reading() {
    let harness = Harness::new(
        FakeCapture::failing_reads_with(CaptureError::NoAccessibility),
        provider_saying("Hello"),
    );

    let error = harness.engine().read_aloud().unwrap_err();

    assert_eq!(error, EngineError::Capture(CaptureError::NoAccessibility));
    assert!(harness.speech.spoken().is_empty());
    assert!(harness.provider.requests().is_empty());
}

#[test]
fn secure_input_stops_read_aloud_before_anything_is_spoken() {
    let harness = Harness::new(
        FakeCapture::failing_reads_with(CaptureError::SecureInput),
        provider_saying("Hello"),
    );

    let error = harness.engine().read_aloud().unwrap_err();

    assert_eq!(error, EngineError::Capture(CaptureError::SecureInput));
    assert!(harness.speech.spoken().is_empty());
}

#[test]
fn a_missing_voice_is_a_typed_speech_failure_that_names_the_language() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    )
    .with_speech(RecordingSpeech::failing_with(SpeechError::NoVoice {
        language: Language::vietnamese(),
    }));

    let error = harness.engine().read_aloud().unwrap_err();

    assert_eq!(
        error,
        EngineError::Speech(SpeechError::NoVoice {
            language: Language::vietnamese(),
        })
    );
}

#[test]
fn a_capture_failure_is_a_typed_error_and_never_reaches_the_provider() {
    let harness = Harness::new(
        FakeCapture::failing_reads_with(CaptureError::SecureInput),
        provider_saying("Hello"),
    );

    let error = harness.engine().quick_translate().unwrap_err();

    assert_eq!(error, EngineError::Capture(CaptureError::SecureInput));
    assert!(harness.provider.requests().is_empty());
}

#[test]
fn a_replace_that_the_focused_app_refuses_is_a_typed_error() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào").failing_writes_with(CaptureError::NotWritable),
        provider_saying("Hello"),
    );

    let error = harness.engine().replace_selection().unwrap_err();

    assert_eq!(error, EngineError::Capture(CaptureError::NotWritable));
    assert!(harness.history.entries().is_empty());
}

#[test]
fn translate_text_returns_a_translation_without_touching_the_selection() {
    let harness = Harness::new(
        FakeCapture::with_selection("ignored"),
        provider_saying("Hello"),
    );

    let outcome = harness
        .engine()
        .translate_text("Xin chào", Mode::Translate)
        .expect("translate text");

    match outcome {
        TextOutcome::Translation(translation) => assert_eq!(translation.text, "Hello"),
        other => panic!("expected a translation, got {other:?}"),
    }
    assert_eq!(harness.capture.reads(), 0);
}

#[test]
fn translate_text_records_what_it_translated_in_history() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));

    harness
        .engine()
        .translate_text("Xin chào", Mode::Translate)
        .expect("translate text");

    let entries = harness.history.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, "Xin chào");
    assert_eq!(entries[0].result, "Hello");
    assert_eq!(entries[0].mode, Mode::Translate);
}

#[test]
fn translating_in_a_given_direction_uses_it_and_never_saves_the_settings() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));

    harness
        .engine()
        .translate_text_in(
            "Xin chào",
            Mode::Translate,
            Some(Direction {
                source: Language::vietnamese(),
                target: Language::english(),
            }),
        )
        .expect("translate text");

    let request = harness
        .provider
        .requests()
        .pop()
        .expect("one provider request");
    assert_eq!(request.source, Some(Language::vietnamese()));
    assert_eq!(request.target, Language::english());
    assert_eq!(
        harness.settings.current().language_pair,
        LanguagePair::new(Language::english(), Language::vietnamese()),
        "Swap is for this window only"
    );
    assert_eq!(harness.settings.saves(), 0);
}

#[test]
fn provider_status_reports_which_keys_are_stored() {
    let mut harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));
    harness.credentials = FakeCredentials::with_secret(ProviderId::Azure, "azure-key");

    let status = harness.engine().provider_status().expect("provider status");

    assert!(status.azure_configured);
    assert!(!status.groq_configured);
}

#[test]
fn provider_status_reports_which_providers_the_session_has_skipped() {
    let mut harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));
    harness.provider = FakeProvider::default().skipping(ProviderId::Azure);

    let status = harness.engine().provider_status().expect("provider status");

    assert!(status.azure_skipped);
    assert!(!status.groq_skipped);
}

#[test]
fn saving_a_key_stores_it_trimmed_and_forgets_the_session_skip() {
    let mut harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));
    harness.provider = FakeProvider::default().skipping(ProviderId::Azure);

    let status = harness
        .engine()
        .save_provider_key(ProviderId::Azure, "  new-azure-key\n")
        .expect("the key is saved");

    assert!(status.azure_configured);
    assert!(!status.azure_skipped, "a fixed key is worth trying again");
    assert_eq!(
        harness.credentials.secrets().get(&ProviderId::Azure),
        Some(&"new-azure-key".to_owned())
    );
}

#[test]
fn saving_a_blank_key_clears_the_stored_key_without_touching_the_settings_file() {
    let mut harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));
    harness.credentials = FakeCredentials::with_secret(ProviderId::Azure, "old-azure-key");
    harness.provider = FakeProvider::default().skipping(ProviderId::Azure);

    let status = harness
        .engine()
        .save_provider_key(ProviderId::Azure, "   ")
        .expect("a blank key means clear");

    assert!(!status.azure_configured);
    assert!(!status.azure_skipped);
    assert!(harness.credentials.secrets().is_empty());
    assert_eq!(
        harness.settings.load().expect("settings load"),
        Settings {
            language_pair: LanguagePair::new(Language::english(), Language::vietnamese()),
            ..Settings::default()
        },
        "the key never lands in the settings file"
    );
}

#[test]
fn clearing_a_key_removes_it_and_forgets_the_session_skip() {
    let mut harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));
    harness.credentials = FakeCredentials::with_secret(ProviderId::Azure, "old-azure-key");
    harness.provider = FakeProvider::default().skipping(ProviderId::Azure);

    let status = harness
        .engine()
        .clear_provider_key(ProviderId::Azure)
        .expect("the key is cleared");

    assert!(!status.azure_configured);
    assert!(!status.azure_skipped);
    assert!(harness.credentials.secrets().is_empty());
}

#[test]
fn the_translate_window_opens_with_the_selection_in_its_input() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào, thế giới"),
        provider_saying("Hello"),
    );

    let selection = harness
        .engine()
        .selection_for_window()
        .expect("selection for the window");

    assert_eq!(selection, Selection::new("Xin chào, thế giới"));
    assert_eq!(harness.capture.reads(), 1);
    assert!(harness.provider.requests().is_empty());
    assert!(harness.history.entries().is_empty());
}

#[test]
fn the_translate_window_opens_empty_when_nothing_is_selected() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));

    let selection = harness
        .engine()
        .selection_for_window()
        .expect("nothing selected is not an error");

    assert_eq!(selection, Selection::new(""));
    assert!(harness.provider.requests().is_empty());
}

#[test]
fn the_translate_window_reports_a_missing_accessibility_grant_instead_of_reading() {
    let harness = Harness::new(
        FakeCapture::failing_reads_with(CaptureError::NoAccessibility),
        provider_saying("Hello"),
    );

    let error = harness.engine().selection_for_window().unwrap_err();

    assert_eq!(error, EngineError::Capture(CaptureError::NoAccessibility));
    assert!(harness.provider.requests().is_empty());
}

#[test]
fn secure_input_stops_the_translate_window_before_any_text_reaches_a_provider() {
    let harness = Harness::new(
        FakeCapture::failing_reads_with(CaptureError::SecureInput),
        provider_saying("Hello"),
    );

    let error = harness.engine().selection_for_window().unwrap_err();

    assert_eq!(error, EngineError::Capture(CaptureError::SecureInput));
    assert!(harness.provider.requests().is_empty());
    assert!(harness.history.entries().is_empty());
}

#[test]
fn the_accessibility_state_comes_from_the_capture_port() {
    let denied = Harness::new(
        FakeCapture::default().not_granted(),
        provider_saying("Hello"),
    );
    let granted = Harness::new(FakeCapture::default(), provider_saying("Hello"));

    assert!(!denied.engine().accessibility_granted());
    assert!(granted.engine().accessibility_granted());
}

#[test]
fn requesting_accessibility_asks_the_platform_and_reports_the_result() {
    let harness = Harness::new(
        FakeCapture::default().granted_after_prompt(),
        provider_saying("Hello"),
    );

    assert!(!harness.engine().accessibility_granted());
    assert!(harness.engine().request_accessibility());
    assert_eq!(harness.capture.accessibility_requests(), 1);
    assert!(harness.engine().accessibility_granted());
}

#[test]
fn opening_the_accessibility_settings_pane_goes_through_the_platform() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));

    harness
        .engine()
        .open_accessibility_settings()
        .expect("open settings pane");

    assert_eq!(harness.capture.settings_panes_opened(), 1);
}

#[test]
fn onboarding_stays_unseen_until_it_is_completed_and_the_flag_persists() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );

    assert!(!harness.engine().settings().unwrap().onboarding_seen);

    harness
        .engine()
        .complete_onboarding()
        .expect("complete onboarding");

    assert!(harness.settings.current().onboarding_seen);
    assert!(harness.engine().settings().unwrap().onboarding_seen);
    assert!(harness.engine().complete_onboarding().is_ok());
}

#[test]
fn improve_text_returns_the_two_labelled_parts() {
    let harness = Harness::new(
        FakeCapture::default(),
        FakeProvider::improving_with(Improvement {
            corrected: "He is going home.".to_owned(),
            better: "He's heading home.".to_owned(),
        }),
    );

    let outcome = harness
        .engine()
        .translate_text("He go home.", Mode::Improve)
        .expect("improve text");

    match outcome {
        TextOutcome::Improvement(improvement) => {
            assert_eq!(improvement.corrected, "He is going home.");
            assert_eq!(improvement.better, "He's heading home.");
        }
        other => panic!("expected an improvement, got {other:?}"),
    }
}

#[test]
fn the_popup_starts_with_the_selection_captured_before_it_was_shown() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );

    assert_eq!(
        harness.engine().popup_prefill(),
        PopupPrefill::Selection(Selection::new("Xin chào"))
    );
    assert!(
        harness.provider.requests().is_empty(),
        "capturing the prefill must not translate anything yet"
    );
}

#[test]
fn the_popup_says_nothing_was_selected_for_an_empty_selection() {
    let harness = Harness::new(
        FakeCapture::with_selection("   \n"),
        provider_saying("Hello"),
    );

    assert_eq!(
        harness.engine().popup_prefill(),
        PopupPrefill::NothingSelected
    );
}

#[test]
fn the_popup_says_nothing_was_selected_when_the_focused_app_has_no_selection() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));

    assert_eq!(
        harness.engine().popup_prefill(),
        PopupPrefill::NothingSelected
    );
}

#[test]
fn the_popup_reports_a_missing_accessibility_grant_as_a_typed_failure() {
    let harness = Harness::new(
        FakeCapture::failing_reads_with(CaptureError::NoAccessibility),
        provider_saying("Hello"),
    );

    assert_eq!(
        harness.engine().popup_prefill(),
        PopupPrefill::Failure(CaptureError::NoAccessibility)
    );
}

#[test]
fn the_popup_reports_secure_input_as_a_typed_failure() {
    let harness = Harness::new(
        FakeCapture::failing_reads_with(CaptureError::SecureInput),
        provider_saying("Hello"),
    );

    assert_eq!(
        harness.engine().popup_prefill(),
        PopupPrefill::Failure(CaptureError::SecureInput)
    );
}

#[test]
fn translating_the_same_text_twice_through_the_engine_asks_the_provider_once() {
    let provider = provider_saying("Hello");
    let engine = TranslatorEngine::new(
        Arc::new(FakeCapture::with_selection("unused")),
        Arc::new(CachedProvider::new(Arc::new(provider.clone()), 8)),
        Arc::new(RecordingSpeech::default()),
        Arc::new(InMemoryHistory::default()),
        Arc::new(InMemorySettings::with(Settings {
            language_pair: LanguagePair::new(Language::english(), Language::vietnamese()),
            ..Settings::default()
        })),
        Arc::new(FakeCredentials::default()),
    );

    let first = engine
        .translate_text("Xin chào", Mode::Translate)
        .expect("first translation");
    let second = engine
        .translate_text("Xin chào", Mode::Translate)
        .expect("second translation");

    assert_eq!(first, second);
    assert_eq!(provider.requests().len(), 1);
}

#[test]
fn improve_streaming_reports_both_parts_in_order_and_returns_the_authority() {
    let harness = Harness::new(
        FakeCapture::default(),
        FakeProvider::streaming_improvement(
            Improvement {
                corrected: "He is going home.".to_owned(),
                better: "He's heading home.".to_owned(),
            },
            vec![
                ImproveDelta {
                    part: ImprovePart::Corrected,
                    text: "He is ".to_owned(),
                },
                ImproveDelta {
                    part: ImprovePart::Corrected,
                    text: "going home.".to_owned(),
                },
                ImproveDelta {
                    part: ImprovePart::Better,
                    text: "He's heading home.".to_owned(),
                },
            ],
        ),
    );

    let mut corrected = String::new();
    let mut better = String::new();
    let improvement = harness
        .engine()
        .improve_streaming("He go home.", &mut |delta| match delta.part {
            ImprovePart::Corrected => corrected.push_str(&delta.text),
            ImprovePart::Better => better.push_str(&delta.text),
        })
        .expect("improve streaming");

    assert_eq!(corrected, "He is going home.");
    assert_eq!(better, "He's heading home.");
    assert_eq!(improvement.corrected, "He is going home.");
    assert_eq!(improvement.better, "He's heading home.");
    assert_eq!(
        harness.provider.improve_requests(),
        vec![ImproveRequest {
            text: "He go home.".to_owned(),
        }]
    );
}

#[test]
fn malformed_model_output_is_a_typed_provider_error_not_a_guessed_answer() {
    let harness = Harness::new(
        FakeCapture::default(),
        FakeProvider::default().failing_improve_with(ProviderError::MalformedResponse {
            provider: ProviderId::Groq,
            message: "the answer carried an empty better field".to_owned(),
        }),
    );

    let error = harness
        .engine()
        .improve_streaming("He go home.", &mut |_| {})
        .expect_err("a malformed answer fails");

    match error {
        EngineError::Provider(ProviderError::MalformedResponse { provider, message }) => {
            assert_eq!(provider, ProviderId::Groq);
            assert!(message.contains("empty better field"), "{message}");
        }
        other => panic!("expected a typed malformed response, got {other:?}"),
    }
}

#[test]
fn a_missing_groq_key_names_groq_in_the_engine_error() {
    let harness = Harness::new(
        FakeCapture::default(),
        FakeProvider::default().failing_improve_with(ProviderError::MissingKey {
            provider: ProviderId::Groq,
        }),
    );

    let error = harness
        .engine()
        .improve_streaming("He go home.", &mut |_| {})
        .expect_err("there is no key");

    assert_eq!(
        error.to_string(),
        "translation failed: Groq needs an API key. Add one in Settings."
    );
}

#[test]
fn improving_blank_text_asks_no_provider_and_writes_no_history() {
    let harness = Harness::new(
        FakeCapture::default(),
        FakeProvider::streaming_improvement(
            Improvement {
                corrected: "unused".to_owned(),
                better: "unused".to_owned(),
            },
            vec![ImproveDelta {
                part: ImprovePart::Corrected,
                text: "unused".to_owned(),
            }],
        ),
    );

    let mut deltas = 0;
    let improvement = harness
        .engine()
        .improve_streaming("   \n\t", &mut |_| deltas += 1)
        .expect("blank text has nothing to improve");

    assert_eq!(
        improvement,
        Improvement {
            corrected: String::new(),
            better: String::new(),
        }
    );
    assert_eq!(deltas, 0);
    assert!(harness.provider.improve_requests().is_empty());
    assert!(harness.history.entries().is_empty());
}

#[test]
fn a_translation_records_the_direction_the_provider_detected() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );

    harness.engine().quick_translate().expect("quick translate");

    let entry = harness.history.entries().pop().expect("one history entry");
    assert_eq!(
        entry.direction,
        Some(Direction {
            source: Language::vietnamese(),
            target: Language::english(),
        }),
        "when the provider named the source language, the direction is known"
    );
}

#[test]
fn a_translation_with_no_detected_source_records_no_direction() {
    let harness = Harness::new(
        FakeCapture::with_selection("42"),
        FakeProvider::responding_with(ProviderTranslation {
            text: "42".to_owned(),
            provider: ProviderId::MyMemory,
            detected_source: None,
        }),
    );

    harness.engine().quick_translate().expect("quick translate");

    let entry = harness.history.entries().pop().expect("one history entry");
    assert_eq!(entry.direction, None);
}

#[test]
fn a_window_translation_in_a_chosen_direction_records_that_direction() {
    let harness = Harness::new(
        FakeCapture::default(),
        FakeProvider::responding_with(ProviderTranslation {
            text: "Xin chào".to_owned(),
            provider: ProviderId::MyMemory,
            detected_source: None,
        }),
    );
    let direction = Direction {
        source: Language::english(),
        target: Language::vietnamese(),
    };

    harness
        .engine()
        .translate_text_in("Hello", Mode::Translate, Some(direction.clone()))
        .expect("translate in the chosen direction");

    let entry = harness.history.entries().pop().expect("one history entry");
    assert_eq!(
        entry.direction,
        Some(direction),
        "Swap knows its direction even when the provider detects nothing"
    );
    assert_eq!(entry.source, "Hello");
    assert_eq!(entry.result, "Xin chào");
}

#[test]
fn a_popup_enter_after_quick_translate_writes_one_history_row_not_two() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );
    let engine = harness.engine();

    let quick = engine.quick_translate().expect("quick translate");
    engine
        .replace_with_translation(&quick.source_text, quick.translation)
        .expect("popup Enter replaces");

    assert_eq!(
        harness.history.records(),
        1,
        "Quick Translate recorded the translation; Enter must not record it again"
    );
    assert_eq!(harness.history.entries().len(), 1);
}

#[test]
fn replace_selection_records_one_row_through_the_seam() {
    let harness = Harness::new(
        FakeCapture::with_selection("Xin chào"),
        provider_saying("Hello"),
    );

    harness
        .engine()
        .replace_selection()
        .expect("replace selection");

    assert_eq!(harness.history.records(), 1);
    assert_eq!(harness.history.entries().len(), 1);
}

#[test]
fn improve_streaming_records_both_parts_and_the_mode_exactly_once() {
    let harness = Harness::new(
        FakeCapture::default(),
        FakeProvider::streaming_improvement(
            Improvement {
                corrected: "He is going home.".to_owned(),
                better: "He's heading home.".to_owned(),
            },
            Vec::new(),
        ),
    );

    harness
        .engine()
        .improve_streaming("He go home.", &mut |_| {})
        .expect("improve streaming");

    assert_eq!(harness.history.records(), 1);
    let entry = harness.history.entries().pop().expect("one history entry");
    assert_eq!(entry.source, "He go home.");
    assert_eq!(entry.result, "He's heading home.");
    assert_eq!(entry.corrected.as_deref(), Some("He is going home."));
    assert_eq!(entry.mode, Mode::Improve);
    assert_eq!(entry.provider, ProviderId::Groq);
    assert_eq!(entry.direction, None);
}

#[test]
fn improve_through_the_window_records_once_too() {
    let harness = Harness::new(
        FakeCapture::default(),
        FakeProvider::improving_with(Improvement {
            corrected: "He is going home.".to_owned(),
            better: "He's heading home.".to_owned(),
        }),
    );

    harness
        .engine()
        .translate_text("He go home.", Mode::Improve)
        .expect("improve text");

    assert_eq!(harness.history.records(), 1);
    let entry = harness.history.entries().pop().expect("one history entry");
    assert_eq!(entry.mode, Mode::Improve);
    assert_eq!(entry.result, "He's heading home.");
    assert_eq!(entry.corrected.as_deref(), Some("He is going home."));
}

#[test]
fn a_rejected_improvement_history_write_never_fails_the_improvement() {
    let harness = Harness::new(
        FakeCapture::default(),
        FakeProvider::streaming_improvement(
            Improvement {
                corrected: "He is going home.".to_owned(),
                better: "He's heading home.".to_owned(),
            },
            Vec::new(),
        ),
    );
    let engine = TranslatorEngine::new(
        Arc::new(harness.capture.clone()),
        Arc::new(harness.provider.clone()),
        Arc::new(harness.speech.clone()),
        Arc::new(InMemoryHistory::failing_writes_with(
            translator_core::HistoryError::Unavailable {
                reason: "the disk is full".to_owned(),
            },
        )),
        Arc::new(harness.settings.clone()),
        Arc::new(harness.credentials.clone()),
    );

    let improvement = engine
        .improve_streaming("He go home.", &mut |_| {})
        .expect("history is best-effort");

    assert_eq!(improvement.better, "He's heading home.");
}

fn translation_entry(direction: Option<Direction>) -> HistoryEntry {
    HistoryEntry {
        source: "Xin chào".to_owned(),
        result: "Hello".to_owned(),
        provider: ProviderId::MyMemory,
        mode: Mode::Translate,
        created_at: Timestamp(1_700_000_000),
        direction,
        corrected: None,
    }
}

#[test]
fn reopening_a_translation_uses_the_stored_result_and_direction_without_a_provider() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("ignored"));

    let outcome = harness
        .engine()
        .reopen(&translation_entry(Some(Direction {
            source: Language::vietnamese(),
            target: Language::english(),
        })))
        .expect("reopen the entry");

    match outcome {
        TextOutcome::Translation(translation) => {
            assert_eq!(translation.text, "Hello");
            assert_eq!(translation.provider, ProviderId::MyMemory);
            assert_eq!(translation.detected_source, Some(Language::vietnamese()));
            assert_eq!(translation.target, Language::english());
        }
        other => panic!("expected a translation, got {other:?}"),
    }
    assert!(
        harness.provider.requests().is_empty(),
        "reopening must spend no quota"
    );
    assert_eq!(
        harness.history.records(),
        0,
        "reopening must not add a history row"
    );
}

#[test]
fn reopening_a_translation_without_a_known_direction_uses_the_saved_target() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("ignored"));

    let outcome = harness
        .engine()
        .reopen(&translation_entry(None))
        .expect("reopen the entry");

    match outcome {
        TextOutcome::Translation(translation) => {
            assert_eq!(translation.target, Language::english());
            assert_eq!(translation.detected_source, None);
        }
        other => panic!("expected a translation, got {other:?}"),
    }
}

#[test]
fn reopening_an_improvement_restores_both_stored_parts_without_a_provider() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("ignored"));
    let entry = HistoryEntry {
        source: "He go home.".to_owned(),
        result: "He's heading home.".to_owned(),
        provider: ProviderId::Groq,
        mode: Mode::Improve,
        created_at: Timestamp(1_700_000_000),
        direction: None,
        corrected: Some("He is going home.".to_owned()),
    };

    let outcome = harness.engine().reopen(&entry).expect("reopen the entry");

    match outcome {
        TextOutcome::Improvement(improvement) => {
            assert_eq!(improvement.corrected, "He is going home.");
            assert_eq!(improvement.better, "He's heading home.");
        }
        other => panic!("expected an improvement, got {other:?}"),
    }
    assert!(harness.provider.improve_requests().is_empty());
    assert_eq!(harness.history.records(), 0);
}

#[test]
fn reopening_an_improvement_written_before_the_corrected_part_existed_answers_empty() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("ignored"));
    let entry = HistoryEntry {
        source: "He go home.".to_owned(),
        result: "He's heading home.".to_owned(),
        provider: ProviderId::Groq,
        mode: Mode::Improve,
        created_at: Timestamp(1_700_000_000),
        direction: None,
        corrected: None,
    };

    let outcome = harness.engine().reopen(&entry).expect("reopen the entry");

    match outcome {
        TextOutcome::Improvement(improvement) => {
            assert_eq!(improvement.corrected, "");
            assert_eq!(improvement.better, "He's heading home.");
        }
        other => panic!("expected an improvement, got {other:?}"),
    }
}

#[test]
fn purging_history_removes_rows_older_than_thirty_days() {
    let harness = Harness::new(FakeCapture::default(), provider_saying("ignored"));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after the epoch")
        .as_secs() as i64;
    let mut entry = translation_entry(None);
    entry.created_at = Timestamp(now - 31 * 24 * 60 * 60);
    harness.history.record(&entry).expect("record an old row");
    entry.created_at = Timestamp(now - 24 * 60 * 60);
    harness.history.record(&entry).expect("record a recent row");

    let removed = harness.engine().purge_history().expect("purge history");

    assert_eq!(removed, 1);
    assert_eq!(harness.history.entries().len(), 1);
}
