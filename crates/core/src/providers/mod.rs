//! Provider implementations. The ladder is a rule about text, so all of this
//! lives in `core`; only the HTTP client underneath is injected.

pub mod cache;
pub mod ladder;
pub mod mymemory;

pub use cache::CachedProvider;
pub use ladder::ProviderLadder;
pub use mymemory::MyMemoryProvider;

use crate::error::ProviderError;
use crate::model::{Improvement, ProviderTranslation};
use crate::ports::{ImproveRequest, TranslateRequest, TranslationProvider};

/// Answers every request with a typed "not configured" failure. Kept for the
/// shell's first-run wiring and for tests that need a provider that never
/// pretends to have translated anything.
pub struct UnconfiguredProvider;

impl TranslationProvider for UnconfiguredProvider {
    fn translate(&self, _request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError> {
        Err(ProviderError::NotConfigured)
    }

    fn improve(&self, _request: &ImproveRequest) -> Result<Improvement, ProviderError> {
        Err(ProviderError::NotConfigured)
    }
}
