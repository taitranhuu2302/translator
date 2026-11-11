//! Provider implementations live in `core` because the ladder is a rule about
//! text, not a platform. Ticket 06 lands Azure, MyMemory and Groq; until then the
//! shell wires this placeholder so every failure stays typed.

use crate::error::ProviderError;
use crate::model::{Improvement, ProviderTranslation};
use crate::ports::{ImproveRequest, TranslateRequest, TranslationProvider};

/// Answers every request with a typed "not configured" failure. It never pretends
/// to have translated anything.
pub struct UnconfiguredProvider;

impl TranslationProvider for UnconfiguredProvider {
    fn translate(&self, _request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError> {
        Err(ProviderError::NotConfigured)
    }

    fn improve(&self, _request: &ImproveRequest) -> Result<Improvement, ProviderError> {
        Err(ProviderError::NotConfigured)
    }
}
