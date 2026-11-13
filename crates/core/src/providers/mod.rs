//! Provider implementations. The ladder is a rule about text, so all of this
//! lives in `core`; only the HTTP client underneath is injected.

pub mod azure;
pub mod cache;
pub mod groq;
pub mod improve_stream;
pub mod ladder;
pub mod mymemory;

pub use azure::AzureProvider;
pub use cache::CachedProvider;
pub use groq::GroqProvider;
pub use improve_stream::ImprovementStream;
pub use ladder::ProviderLadder;
pub use mymemory::MyMemoryProvider;

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
