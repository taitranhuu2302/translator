//! The provider ladder. Providers are tried in order and the first useful answer
//! wins; ticket 06 inserts Azure ahead of MyMemory, and the engine does not need
//! to change. A provider that is merely not configured (no key yet) never hides
//! the providers behind it.

use std::sync::Arc;

use crate::error::ProviderError;
use crate::model::{Improvement, ProviderTranslation};
use crate::ports::{ImproveRequest, TranslateRequest, TranslationProvider};

pub struct ProviderLadder {
    providers: Vec<Arc<dyn TranslationProvider>>,
}

impl ProviderLadder {
    pub fn new(providers: Vec<Arc<dyn TranslationProvider>>) -> Self {
        Self { providers }
    }

    /// One provider is the common case today; this keeps the shell's wiring
    /// readable and gives ticket 06 somewhere to add the next rung.
    pub fn single(provider: Arc<dyn TranslationProvider>) -> Self {
        Self::new(vec![provider])
    }
}

impl TranslationProvider for ProviderLadder {
    fn translate(&self, request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError> {
        run_ladder(self.providers.iter(), |provider| {
            provider.translate(request)
        })
    }

    fn improve(&self, request: &ImproveRequest) -> Result<Improvement, ProviderError> {
        run_ladder(self.providers.iter(), |provider| provider.improve(request))
    }
}

/// Try each provider until one answers; otherwise report the last typed failure
/// (or `NotConfigured` when the ladder is empty). Request-level errors are not
/// special-cased: the next rung may be a different host.
fn run_ladder<'a, T, I, F>(providers: I, mut attempt: F) -> Result<T, ProviderError>
where
    I: Iterator<Item = &'a Arc<dyn TranslationProvider>>,
    F: FnMut(&dyn TranslationProvider) -> Result<T, ProviderError>,
{
    let mut last_error = None;
    for provider in providers {
        match attempt(provider.as_ref()) {
            Ok(answer) => return Ok(answer),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or(ProviderError::NotConfigured))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use crate::model::ProviderId;
    use crate::testing::FakeProvider;

    fn request() -> TranslateRequest {
        TranslateRequest {
            text: "Xin chào".to_owned(),
            source: Some(Language::vietnamese()),
            target: Language::english(),
        }
    }

    fn translation(text: &str, provider: ProviderId) -> ProviderTranslation {
        ProviderTranslation {
            text: text.to_owned(),
            provider,
            detected_source: None,
        }
    }

    #[test]
    fn the_first_provider_that_answers_wins_and_the_rest_are_not_asked() {
        let first = FakeProvider::responding_with(translation("from Azure", ProviderId::Azure));
        let second = FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![Arc::new(first.clone()), Arc::new(second.clone())]);

        let answer = ladder.translate(&request()).expect("the first rung answers");
        assert_eq!(answer.text, "from Azure");
        assert_eq!(first.requests().len(), 1);
        assert!(second.requests().is_empty(), "the second rung is not used");
    }

    #[test]
    fn a_failing_provider_falls_through_to_the_next_rung() {
        let first = FakeProvider::default().failing_with(ProviderError::QuotaExceeded {
            provider: ProviderId::Azure,
        });
        let second = FakeProvider::responding_with(translation("from MyMemory", ProviderId::MyMemory));
        let ladder = ProviderLadder::new(vec![Arc::new(first), Arc::new(second)]);

        assert_eq!(
            ladder.translate(&request()).expect("the next rung answers").text,
            "from MyMemory"
        );
    }

    #[test]
    fn a_ladder_that_all_fail_reports_the_last_typed_error() {
        let first = FakeProvider::default().failing_with(ProviderError::Offline);
        let second = FakeProvider::default().failing_with(ProviderError::Unavailable {
            reason: "down for maintenance".to_owned(),
        });
        let ladder = ProviderLadder::new(vec![Arc::new(first), Arc::new(second)]);

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
