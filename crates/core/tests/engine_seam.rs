//! The seam test: the engine is driven through its public API with fake ports, and
//! every assertion is on an observable outcome — the translation returned, the
//! typed error, the bytes sent to a provider, what was written to history.

use std::sync::Arc;

use translator_core::testing::{
    FakeCapture, FakeCredentials, FakeProvider, InMemoryHistory, InMemorySettings, RecordingSpeech,
};
use translator_core::{
    CaptureError, EngineError, Improvement, Language, LanguagePair, Mode, ProviderId,
    ProviderTranslation, Selection, Settings, TextOutcome, TranslateRequest, TranslatorEngine,
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
    let harness = Harness::new(FakeCapture::with_selection("Xin chào"), provider_saying("Hello"));

    let outcome = harness.engine().quick_translate().expect("quick translate");

    assert_eq!(outcome.source_text, "Xin chào");
    assert_eq!(outcome.translation.text, "Hello");
    assert_eq!(outcome.translation.provider, ProviderId::MyMemory);
    assert_eq!(outcome.translation.target, Language::english());
    assert_eq!(
        harness.provider.requests(),
        vec![TranslateRequest {
            text: "Xin chào".to_owned(),
            source: None,
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
fn replace_selection_writes_the_translation_over_the_selection() {
    let harness = Harness::new(FakeCapture::with_selection("Xin chào"), provider_saying("Hello"));

    let outcome = harness
        .engine()
        .replace_selection()
        .expect("replace selection");

    assert_eq!(outcome.translation.text, "Hello");
    assert_eq!(harness.capture.writes(), vec!["Hello".to_owned()]);
    assert_eq!(harness.history.entries().len(), 1);
}

#[test]
fn read_aloud_speaks_the_selection_and_never_the_translation() {
    let harness = Harness::new(FakeCapture::with_selection("Xin chào"), provider_saying("Hello"));

    let outcome = harness.engine().read_aloud().expect("read aloud");

    assert_eq!(outcome.spoken_text, "Xin chào");
    assert_eq!(harness.speech.spoken(), vec!["Xin chào".to_owned()]);
    assert!(harness.provider.requests().is_empty());
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
    let harness = Harness::new(FakeCapture::with_selection("ignored"), provider_saying("Hello"));

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
fn provider_status_reports_which_keys_are_stored() {
    let mut harness = Harness::new(FakeCapture::default(), provider_saying("Hello"));
    harness.credentials = FakeCredentials::with_secret(ProviderId::Azure, "azure-key");

    let status = harness.engine().provider_status().expect("provider status");

    assert!(status.azure_configured);
    assert!(!status.groq_configured);
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
    let denied = Harness::new(FakeCapture::default().not_granted(), provider_saying("Hello"));
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
    let harness = Harness::new(FakeCapture::with_selection("Xin chào"), provider_saying("Hello"));

    assert!(!harness.engine().settings().unwrap().onboarding_seen);

    harness.engine().complete_onboarding().expect("complete onboarding");

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
