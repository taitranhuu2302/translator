//! Provider implementations. The ladder is a rule about text, so all of this
//! lives in `core`; only the HTTP client underneath is injected.

pub mod azure;
pub mod cache;
pub mod ladder;
pub mod mymemory;

pub use azure::AzureProvider;
pub use cache::CachedProvider;
pub use ladder::ProviderLadder;
pub use mymemory::MyMemoryProvider;

use crate::error::ProviderError;
use crate::model::{Improvement, ProviderTranslation};
use crate::ports::{ImproveRequest, TranslateRequest, TranslationProvider};

/// Percent-encode one query component with the RFC 3986 unreserved set. Shared
/// by the providers so a language code or a piece of text cannot break the URL.
pub(crate) fn encode_query_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(*byte as char)
            }
            other => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                encoded.push('%');
                encoded.push(HEX[(other >> 4) as usize] as char);
                encoded.push(HEX[(other & 0x0F) as usize] as char);
            }
        }
    }
    encoded
}

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
