//! The provider ladder. Providers are tried in order and the first useful answer
//! wins; Azure sits ahead of MyMemory, and the engine does not need to change.
//! A provider that is merely not configured (no key yet) never hides the
//! providers behind it.
//!
//! A rung that fails authentication or quota is skipped for the rest of the
//! session (ADR-0003). The skip is deliberately narrow: `Offline`, `Network`,
//! `MalformedResponse` and `Unavailable` (a 429 is a retry, not a ban) leave the
//! rung in the ladder. Saving or clearing the provider's key clears its skip.

use std::sync::{Arc, Mutex};

use crate::error::ProviderError;
use crate::model::{ImproveDelta, Improvement, ProviderId, ProviderTranslation};
use crate::ports::{ImproveRequest, TranslateRequest, TranslationProvider};

struct Rung {
    id: ProviderId,
    provider: Arc<dyn TranslationProvider>,
}

pub struct ProviderLadder {
    rungs: Vec<Rung>,
    /// Providers skipped until the app restarts. Kept in rung order so the
    /// readable status is stable.
    skipped: Mutex<Vec<ProviderId>>,
}

impl ProviderLadder {
    pub fn new(rungs: Vec<(ProviderId, Arc<dyn TranslationProvider>)>) -> Self {
        Self {
            rungs: rungs
                .into_iter()
                .map(|(id, provider)| Rung { id, provider })
                .collect(),
            skipped: Mutex::new(Vec::new()),
        }
    }

    /// One provider is the common case in tests; this keeps the shell's wiring
    /// readable.
    pub fn single(id: ProviderId, provider: Arc<dyn TranslationProvider>) -> Self {
        Self::new(vec![(id, provider)])
    }

    fn is_skipped(&self, provider: ProviderId) -> bool {
        self.skipped
            .lock()
            .expect("session skip lock")
            .contains(&provider)
    }

    fn skip(&self, provider: ProviderId) {
        let mut skipped = self.skipped.lock().expect("session skip lock");
        if !skipped.contains(&provider) {
            skipped.push(provider);
        }
    }

    /// Try each rung in order. A rung skipped this session is not attempted; an
    /// auth or quota failure marks its rung; every other failure merely moves on.
    fn run<T>(
        &self,
        mut attempt: impl FnMut(&dyn TranslationProvider) -> Result<T, ProviderError>,
    ) -> Result<T, ProviderError> {
        let mut last_error = None;
        for rung in &self.rungs {
            if self.is_skipped(rung.id) {
                continue;
            }
            match attempt(rung.provider.as_ref()) {
                Ok(answer) => return Ok(answer),
                Err(error) => {
                    if ends_the_session(&error) {
                        self.skip(rung.id);
                    }
                    last_error = Some(error);
                }
            }
        }
        Err(last_error.unwrap_or(ProviderError::NotConfigured))
    }
}

/// Only an auth or quota failure is a ban. Everything else may succeed on the
/// next shortcut press, so it must keep its place in the ladder.
fn ends_the_session(error: &ProviderError) -> bool {
    matches!(
        error,
        ProviderError::AuthenticationFailed { .. } | ProviderError::QuotaExceeded { .. }
    )
}

impl TranslationProvider for ProviderLadder {
    fn translate(&self, request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError> {
        self.run(|provider| provider.translate(request))
    }

    fn improve(&self, request: &ImproveRequest) -> Result<Improvement, ProviderError> {
        self.run(|provider| provider.improve(request))
    }

    /// The same ladder rules for a streamed Improve. The rung that answers owns
    /// the sink; a rung that fails may have already reported text, which is why
    /// the view replaces both fields from the final `Improvement` on done.
    fn improve_streaming(
        &self,
        request: &ImproveRequest,
        sink: &mut dyn FnMut(&ImproveDelta),
    ) -> Result<Improvement, ProviderError> {
        self.run(|provider| provider.improve_streaming(request, &mut *sink))
    }

    fn skipped(&self) -> Vec<ProviderId> {
        self.skipped.lock().expect("session skip lock").clone()
    }

    fn clear_skip(&self, provider: ProviderId) {
        self.skipped
            .lock()
            .expect("session skip lock")
            .retain(|skipped| *skipped != provider);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AuthFailure;
    use crate::language::Language;
    use crate::model::{ImprovePart, Improvement};
    use crate::testing::FakeProvider;

    fn request() -> TranslateRequest {
        TranslateRequest {
            text: "Xin chào".to_owned(),
            source: Some(Language::vietnamese()),
            target: Language::english(),
        }
    }

    fn improve_request() -> ImproveRequest {
        ImproveRequest {
            text: "He go home.".to_owned(),
        }
    }

    fn improvement() -> Improvement {
        Improvement {
            corrected: "He goes home.".to_owned(),
            better: "He's heading home.".to_owned(),
        }
    }

    #[test]
    fn an_improve_stream_comes_from_the_rung_that_answers() {
        let azure = FakeProvider::default();
        let groq = FakeProvider::streaming_improvement(
            improvement(),
            vec![
                ImproveDelta {
                    part: ImprovePart::Corrected,
                    text: "He goes home.".to_owned(),
                },
                ImproveDelta {
                    part: ImprovePart::Better,
                    text: "He's heading home.".to_owned(),
                },
            ],
        );
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure)),
            (ProviderId::Groq, Arc::new(groq)),
        ]);

        let mut seen = Vec::new();
        let answer = ladder
            .improve_streaming(&improve_request(), &mut |delta| seen.push(delta.clone()))
            .expect("the model rung answers");

        assert_eq!(answer, improvement());
        assert_eq!(
            seen,
            vec![
                ImproveDelta {
                    part: ImprovePart::Corrected,
                    text: "He goes home.".to_owned(),
                },
                ImproveDelta {
                    part: ImprovePart::Better,
                    text: "He's heading home.".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn a_missing_model_key_falls_through_and_is_never_a_session_ban() {
        let missing = FakeProvider::default().failing_improve_with(ProviderError::MissingKey {
            provider: ProviderId::Groq,
        });
        let fallback = FakeProvider::improving_with(improvement());
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Groq, Arc::new(missing.clone())),
            (ProviderId::MyMemory, Arc::new(fallback)),
        ]);

        ladder
            .improve(&improve_request())
            .expect("the next rung answers");
        ladder
            .improve(&improve_request())
            .expect("the next rung answers");

        assert_eq!(
            missing.improve_requests().len(),
            2,
            "a missing key is not a failure to skip"
        );
        assert!(ladder.skipped().is_empty());
    }

    fn translation(text: &str, provider: ProviderId) -> ProviderTranslation {
        ProviderTranslation {
            text: text.to_owned(),
            provider,
            detected_source: None,
        }
    }

    fn azure_and_my_memory() -> (FakeProvider, FakeProvider, ProviderLadder) {
        let azure = FakeProvider::responding_with(translation("from Azure", ProviderId::Azure));
        let my_memory =
            FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure.clone())),
            (ProviderId::MyMemory, Arc::new(my_memory.clone())),
        ]);
        (azure, my_memory, ladder)
    }

    #[test]
    fn the_first_provider_that_answers_wins_and_the_rest_are_not_asked() {
        let (azure, my_memory, ladder) = azure_and_my_memory();

        let answer = ladder
            .translate(&request())
            .expect("the first rung answers");
        assert_eq!(answer.text, "from Azure");
        assert_eq!(azure.requests().len(), 1);
        assert!(
            my_memory.requests().is_empty(),
            "the second rung is not used"
        );
    }

    #[test]
    fn a_failing_provider_falls_through_to_the_next_rung() {
        let azure = FakeProvider::default().failing_with(ProviderError::QuotaExceeded {
            provider: ProviderId::Azure,
        });
        let my_memory =
            FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure)),
            (ProviderId::MyMemory, Arc::new(my_memory)),
        ]);

        assert_eq!(
            ladder
                .translate(&request())
                .expect("the next rung answers")
                .text,
            "from MyMemory"
        );
    }

    #[test]
    fn an_authentication_failure_is_never_retried_that_session() {
        let azure = FakeProvider::default().failing_with(ProviderError::AuthenticationFailed {
            provider: ProviderId::Azure,
            cause: AuthFailure::InvalidKey,
        });
        let my_memory =
            FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure.clone())),
            (ProviderId::MyMemory, Arc::new(my_memory.clone())),
        ]);

        assert_eq!(
            ladder
                .translate(&request())
                .expect("the fallback answers")
                .text,
            "from MyMemory"
        );
        assert_eq!(
            ladder
                .translate(&request())
                .expect("the fallback answers")
                .text,
            "from MyMemory"
        );

        assert_eq!(azure.requests().len(), 1, "Azure is not tried again");
        assert_eq!(my_memory.requests().len(), 2, "MyMemory answers both");
        assert_eq!(ladder.skipped(), vec![ProviderId::Azure]);
    }

    #[test]
    fn a_quota_failure_is_never_retried_that_session() {
        let azure = FakeProvider::default().failing_with(ProviderError::QuotaExceeded {
            provider: ProviderId::Azure,
        });
        let my_memory =
            FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure.clone())),
            (ProviderId::MyMemory, Arc::new(my_memory)),
        ]);

        ladder.translate(&request()).expect("the fallback answers");
        ladder.translate(&request()).expect("the fallback answers");

        assert_eq!(azure.requests().len(), 1);
        assert_eq!(ladder.skipped(), vec![ProviderId::Azure]);
    }

    #[test]
    fn an_offline_failure_leaves_the_provider_in_the_ladder() {
        let azure = FakeProvider::default().failing_with(ProviderError::Offline);
        let my_memory =
            FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure.clone())),
            (ProviderId::MyMemory, Arc::new(my_memory)),
        ]);

        ladder.translate(&request()).expect("the fallback answers");
        ladder.translate(&request()).expect("the fallback answers");

        assert_eq!(azure.requests().len(), 2, "the network may come back");
        assert!(ladder.skipped().is_empty());
    }

    #[test]
    fn a_rate_limit_leaves_the_provider_in_the_ladder() {
        let azure = FakeProvider::default().failing_with(ProviderError::Unavailable {
            reason: "Azure is rate-limiting requests".to_owned(),
        });
        let my_memory =
            FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure.clone())),
            (ProviderId::MyMemory, Arc::new(my_memory)),
        ]);

        ladder.translate(&request()).expect("the fallback answers");
        ladder.translate(&request()).expect("the fallback answers");

        assert_eq!(azure.requests().len(), 2, "a 429 is a retry, not a ban");
        assert!(ladder.skipped().is_empty());
    }

    #[test]
    fn a_malformed_response_leaves_the_provider_in_the_ladder() {
        let azure = FakeProvider::default().failing_with(ProviderError::MalformedResponse {
            provider: ProviderId::Azure,
            message: "not JSON".to_owned(),
        });
        let my_memory =
            FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure.clone())),
            (ProviderId::MyMemory, Arc::new(my_memory)),
        ]);

        ladder.translate(&request()).expect("the fallback answers");
        ladder.translate(&request()).expect("the fallback answers");

        assert_eq!(azure.requests().len(), 2);
        assert!(ladder.skipped().is_empty());
    }

    #[test]
    fn a_missing_key_skips_nothing_and_falls_through_without_an_error() {
        let azure = FakeProvider::default().failing_with(ProviderError::NotConfigured);
        let my_memory =
            FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure.clone())),
            (ProviderId::MyMemory, Arc::new(my_memory)),
        ]);

        assert_eq!(
            ladder
                .translate(&request())
                .expect("the keyless rung answers")
                .text,
            "from MyMemory"
        );
        ladder
            .translate(&request())
            .expect("the keyless rung answers");

        assert_eq!(
            azure.requests().len(),
            2,
            "a missing key is not a failure to skip"
        );
        assert!(ladder.skipped().is_empty());
    }

    #[test]
    fn saving_a_key_clears_the_session_skip_for_that_provider() {
        let azure = FakeProvider::default().failing_with(ProviderError::AuthenticationFailed {
            provider: ProviderId::Azure,
            cause: AuthFailure::InvalidKey,
        });
        let my_memory =
            FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(azure.clone())),
            (ProviderId::MyMemory, Arc::new(my_memory)),
        ]);

        ladder.translate(&request()).expect("the fallback answers");
        assert_eq!(ladder.skipped(), vec![ProviderId::Azure]);

        ladder.clear_skip(ProviderId::Azure);

        assert!(ladder.skipped().is_empty());
        ladder.translate(&request()).expect("the fallback answers");
        assert_eq!(
            azure.requests().len(),
            2,
            "Azure is tried again after the fix"
        );
    }

    #[test]
    fn a_ladder_that_all_fail_reports_the_last_typed_error() {
        let first = FakeProvider::default().failing_with(ProviderError::Offline);
        let second = FakeProvider::default().failing_with(ProviderError::Unavailable {
            reason: "down for maintenance".to_owned(),
        });
        let ladder = ProviderLadder::new(vec![
            (ProviderId::Azure, Arc::new(first)),
            (ProviderId::MyMemory, Arc::new(second)),
        ]);

        assert_eq!(
            ladder.translate(&request()).expect_err("nothing answered"),
            ProviderError::Unavailable {
                reason: "down for maintenance".to_owned()
            }
        );
    }

    #[test]
    fn an_empty_ladder_is_not_configured() {
        assert_eq!(
            ProviderLadder::new(Vec::new())
                .translate(&request())
                .expect_err("an empty ladder has no answer"),
            ProviderError::NotConfigured
        );
    }
}
