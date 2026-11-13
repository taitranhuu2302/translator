//! The keyless MyMemory provider. One `GET` per chunk, with the direction the
//! engine resolved. The HTTP status is never the verdict: MyMemory reports its
//! semantic errors in the body (`responseStatus`), so this module reads the body
//! and maps it to the typed provider errors.

use std::sync::Arc;

use serde::Deserialize;

use crate::chunking::{chunk_text, join_chunks, MAX_CHUNK_CHARS};
use crate::error::{HttpError, ProviderError};
use crate::model::{Improvement, ProviderId, ProviderTranslation};
use crate::ports::{
    HttpClient, HttpMethod, HttpRequest, ImproveRequest, TranslateRequest, TranslationProvider,
};
use crate::providers::encode_query_component;

pub const ENDPOINT: &str = "https://api.mymemory.translated.net/get";

/// MyMemory, with an injected HTTP client so tests never open a socket.
pub struct MyMemoryProvider {
    http: Arc<dyn HttpClient>,
}

impl MyMemoryProvider {
    pub fn new(http: Arc<dyn HttpClient>) -> Self {
        Self { http }
    }

    fn translate_chunk(
        &self,
        text: &str,
        source: &crate::language::Language,
        target: &crate::language::Language,
    ) -> Result<String, ProviderError> {
        let url = format!(
            "{ENDPOINT}?q={}&langpair={}%7C{}",
            encode_query_component(text),
            encode_query_component(source.code()),
            encode_query_component(target.code()),
        );
        let response = self
            .http
            .execute(&HttpRequest {
                method: HttpMethod::Get,
                url,
                headers: Vec::new(),
                body: None,
            })
            .map_err(map_transport)?;
        read_answer(response.status, &response.body)
    }
}

impl TranslationProvider for MyMemoryProvider {
    fn translate(&self, request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError> {
        if request.text.is_empty() {
            return Ok(ProviderTranslation {
                text: String::new(),
                provider: ProviderId::MyMemory,
                detected_source: None,
            });
        }
        let source = request
            .source
            .clone()
            .ok_or_else(|| ProviderError::MalformedResponse {
                provider: ProviderId::MyMemory,
                message: "the direction was not resolved before the request".to_owned(),
            })?;
        let chunks = chunk_text(&request.text, MAX_CHUNK_CHARS);
        let mut translated = Vec::with_capacity(chunks.len());
        for chunk in &chunks {
            translated.push(self.translate_chunk(chunk, &source, &request.target)?);
        }
        Ok(ProviderTranslation {
            text: join_chunks(&translated),
            provider: ProviderId::MyMemory,
            detected_source: None,
        })
    }

    /// Improve belongs to the model provider (ticket 08); MyMemory only translates.
    fn improve(&self, _request: &ImproveRequest) -> Result<Improvement, ProviderError> {
        Err(ProviderError::NotConfigured)
    }
}

fn map_transport(error: HttpError) -> ProviderError {
    match error {
        HttpError::Unreachable { .. } => ProviderError::Offline,
        HttpError::Request { message } => ProviderError::Network {
            provider: ProviderId::MyMemory,
            message,
        },
    }
}

#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "responseData", default)]
    response_data: Option<ResponseData>,
    #[serde(rename = "quotaFinished", default)]
    quota_finished: Option<bool>,
    #[serde(rename = "responseDetails", default)]
    response_details: String,
    #[serde(rename = "responseStatus", default)]
    response_status: serde_json::Value,
}

#[derive(Deserialize)]
struct ResponseData {
    #[serde(rename = "translatedText", default)]
    translated_text: Option<String>,
}

/// Read one response body. `status` is the HTTP status, which only matters for
/// the transport-level failures (429 and 5xx); every semantic verdict is in the
/// body.
fn read_answer(status: u16, body: &str) -> Result<String, ProviderError> {
    if status == 429 {
        // MyMemory's documented rate limit is unverified in practice, and there
        // is deliberately no `RateLimited` variant: report it as unavailable.
        return Err(ProviderError::Unavailable {
            reason: "MyMemory is rate-limiting requests; try again shortly".to_owned(),
        });
    }
    if status >= 500 {
        return Err(ProviderError::Unavailable {
            reason: format!("MyMemory answered HTTP {status}"),
        });
    }

    let envelope: Envelope =
        serde_json::from_str(body).map_err(|error| ProviderError::MalformedResponse {
            provider: ProviderId::MyMemory,
            message: format!("the body was not the expected JSON: {error}"),
        })?;
    let details = envelope.response_details;
    let translated = envelope
        .response_data
        .and_then(|data| data.translated_text)
        .unwrap_or_default();
    let upper_details = details.to_uppercase();
    let upper_translated = translated.to_uppercase();

    let quota_spent = envelope.quota_finished == Some(true)
        || upper_details.contains("MYMEMORY WARNING")
        || upper_details.contains("USED ALL AVAILABLE FREE TRANSLATIONS")
        || upper_translated.contains("MYMEMORY WARNING")
        || upper_translated.contains("USED ALL AVAILABLE FREE TRANSLATIONS");
    if quota_spent {
        return Err(ProviderError::QuotaExceeded {
            provider: ProviderId::MyMemory,
        });
    }

    if !is_success_status(&envelope.response_status) {
        let message = if !details.trim().is_empty() {
            details
        } else if !translated.trim().is_empty() {
            translated
        } else {
            "MyMemory rejected the request without a reason".to_owned()
        };
        return Err(ProviderError::MalformedResponse {
            provider: ProviderId::MyMemory,
            message,
        });
    }

    if translated.trim().is_empty() {
        return Err(ProviderError::MalformedResponse {
            provider: ProviderId::MyMemory,
            message: "the response carried no translated text".to_owned(),
        });
    }
    Ok(html_unescape(&translated))
}

/// Success is the number 200 or the string `"200"`; failures are usually the
/// string `"403"`, so a plain `u16` deserialization would lose the distinction.
fn is_success_status(status: &serde_json::Value) -> bool {
    status.as_u64() == Some(200) || status.as_str() == Some("200")
}

/// Decode the HTML entities MyMemory puts in `translatedText` (`&#39;`, `&quot;`,
/// `&amp;`, …). Unknown entities are left untouched.
fn html_unescape(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(ampersand) = rest.find('&') {
        decoded.push_str(&rest[..ampersand]);
        rest = &rest[ampersand..];
        let entity_end = rest
            .char_indices()
            .take(12)
            .find(|(_, character)| *character == ';')
            .map(|(index, _)| index);
        match entity_end.and_then(|end| decode_entity(&rest[1..end]).map(|character| (end, character))) {
            Some((end, character)) => {
                decoded.push(character);
                rest = &rest[end + 1..];
            }
            None => {
                decoded.push('&');
                rest = &rest[1..];
            }
        }
    }
    decoded.push_str(rest);
    decoded
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        numeric => {
            let number = numeric.strip_prefix('#')?;
            let value = match number.strip_prefix('x').or_else(|| number.strip_prefix('X')) {
                Some(hexadecimal) => u32::from_str_radix(hexadecimal, 16).ok()?,
                None => number.parse::<u32>().ok()?,
            };
            char::from_u32(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::Language;
    use crate::testing::FakeHttpClient;

    fn vietnamese_to_english(text: &str) -> TranslateRequest {
        TranslateRequest {
            text: text.to_owned(),
            source: Some(Language::vietnamese()),
            target: Language::english(),
        }
    }

    fn success_body(translated: &str) -> String {
        format!(
            r#"{{"responseData":{{"translatedText":"{translated}","match":0.99}},
                "quotaFinished":false,"responseDetails":"","responseStatus":200,
                "matches":[{{"segment":"x","translation":"{translated}"}}]}}"#
        )
    }

    fn provider(fake: &FakeHttpClient) -> MyMemoryProvider {
        MyMemoryProvider::new(Arc::new(fake.clone()))
    }

    #[test]
    fn a_successful_answer_is_the_translated_text_with_the_provider_named() {
        let http = FakeHttpClient::responding_with(200, success_body("Hello world"));
        let answer = provider(&http)
            .translate(&vietnamese_to_english("Xin chào thế giới"))
            .expect("a 200 answer translates");

        assert_eq!(answer.text, "Hello world");
        assert_eq!(answer.provider, ProviderId::MyMemory);

        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, HttpMethod::Get);
        assert_eq!(
            requests[0].url,
            "https://api.mymemory.translated.net/get?q=Xin%20ch%C3%A0o%20th%E1%BA%BF%20gi%E1%BB%9Bi\
             &langpair=vi%7Cen"
        );
    }

    #[test]
    fn the_query_is_percent_encoded_so_reserved_characters_survive() {
        let http = FakeHttpClient::responding_with(200, success_body("ok"));
        provider(&http)
            .translate(&vietnamese_to_english("a&b c"))
            .expect("the fake answers");

        assert_eq!(
            http.requests()[0].url,
            "https://api.mymemory.translated.net/get?q=a%26b%20c&langpair=vi%7Cen"
        );
    }

    #[test]
    fn html_entities_in_the_translation_are_decoded_before_it_is_shown() {
        let http = FakeHttpClient::responding_with(
            200,
            success_body("It&#39;s &quot;fine&quot; &amp; done &lt;3"),
        );
        let answer = provider(&http)
            .translate(&vietnamese_to_english("Ổn"))
            .expect("the fake answers");
        assert_eq!(answer.text, "It's \"fine\" & done <3");
    }

    #[test]
    fn a_string_status_of_200_is_a_success_too() {
        let body = r#"{"responseData":{"translatedText":"Hello"},"responseDetails":"",
                       "responseStatus":"200","quotaFinished":null,"matches":""}"#;
        let http = FakeHttpClient::responding_with(200, body);
        assert_eq!(
            provider(&http)
                .translate(&vietnamese_to_english("Chào"))
                .expect("the string 200 is a success")
                .text,
            "Hello"
        );
    }

    #[test]
    fn a_finished_quota_is_a_quota_failure() {
        let body = r#"{"responseData":{"translatedText":"MYMEMORY WARNING: YOU USED ALL AVAILABLE FREE TRANSLATIONS FOR TODAY"},
                       "responseStatus":200,"quotaFinished":true,"responseDetails":"","matches":""}"#;
        let http = FakeHttpClient::responding_with(200, body);
        let error = provider(&http)
            .translate(&vietnamese_to_english("Chào"))
            .expect_err("the daily quota is gone");
        assert_eq!(
            error,
            ProviderError::QuotaExceeded {
                provider: ProviderId::MyMemory
            }
        );
    }

    #[test]
    fn a_my_memory_warning_is_a_quota_failure_even_without_the_boolean() {
        let body = r#"{"responseData":{"translatedText":"Chào"},"responseStatus":200,
                       "quotaFinished":null,
                       "responseDetails":"MYMEMORY WARNING: YOU USED ALL AVAILABLE FREE TRANSLATIONS FOR TODAY.","matches":""}"#;
        let http = FakeHttpClient::responding_with(200, body);
        let error = provider(&http)
            .translate(&vietnamese_to_english("Hi"))
            .expect_err("the warning is the quota signal");
        assert!(matches!(
            error,
            ProviderError::QuotaExceeded {
                provider: ProviderId::MyMemory
            }
        ));
    }

    #[test]
    fn a_semantic_403_is_a_malformed_response_never_a_success() {
        let body = r#"{"responseData":{"translatedText":"PLEASE SELECT TWO DISTINCT LANGUAGES"},
                       "responseStatus":"403","quotaFinished":null,
                       "responseDetails":"PLEASE SELECT TWO DISTINCT LANGUAGES","matches":""}"#;
        let http = FakeHttpClient::responding_with(200, body);
        let error = provider(&http)
            .translate(&vietnamese_to_english("Hi"))
            .expect_err("HTTP 200 does not make a semantic 403 a success");
        match error {
            ProviderError::MalformedResponse { provider, message } => {
                assert_eq!(provider, ProviderId::MyMemory);
                assert!(message.contains("DISTINCT LANGUAGES"), "{message}");
            }
            other => panic!("expected a malformed response, got {other:?}"),
        }
    }

    #[test]
    fn a_body_that_is_not_json_is_a_malformed_response() {
        let http = FakeHttpClient::responding_with(200, "upstream is down");
        let error = provider(&http)
            .translate(&vietnamese_to_english("Hi"))
            .expect_err("plain text is not the envelope");
        assert!(matches!(
            error,
            ProviderError::MalformedResponse {
                provider: ProviderId::MyMemory,
                ..
            }
        ));
    }

    #[test]
    fn a_missing_translation_is_a_malformed_response() {
        let body = r#"{"responseStatus":200,"responseDetails":"","quotaFinished":false}"#;
        let http = FakeHttpClient::responding_with(200, body);
        let error = provider(&http)
            .translate(&vietnamese_to_english("Hi"))
            .expect_err("nothing was translated");
        assert!(matches!(
            error,
            ProviderError::MalformedResponse {
                provider: ProviderId::MyMemory,
                ..
            }
        ));
    }

    #[test]
    fn an_unreachable_network_is_reported_as_offline() {
        let http = FakeHttpClient::failing_with(HttpError::Unreachable {
            message: "no route to host".to_owned(),
        });
        let error = provider(&http)
            .translate(&vietnamese_to_english("Hi"))
            .expect_err("the machine is offline");
        assert_eq!(error, ProviderError::Offline);
    }

    #[test]
    fn a_request_that_failed_for_another_reason_is_a_network_error() {
        let http = FakeHttpClient::failing_with(HttpError::Request {
            message: "the request was malformed".to_owned(),
        });
        let error = provider(&http)
            .translate(&vietnamese_to_english("Hi"))
            .expect_err("the request failed");
        assert!(matches!(
            error,
            ProviderError::Network {
                provider: ProviderId::MyMemory,
                ..
            }
        ));
    }

    #[test]
    fn a_rate_limit_or_server_failure_is_unavailable() {
        for status in [429, 500, 503] {
            let http = FakeHttpClient::responding_with(status, "");
            let error = provider(&http)
                .translate(&vietnamese_to_english("Hi"))
                .expect_err("the server did not answer usefully");
            assert!(
                matches!(error, ProviderError::Unavailable { .. }),
                "HTTP {status} should be unavailable, got {error:?}"
            );
        }
    }

    #[test]
    fn long_text_is_chunked_by_characters_and_rejoined_in_order() {
        let text = format!("{} {}", "a".repeat(500), "b".repeat(100));
        let http = FakeHttpClient::responding_in_turn(vec![
            Ok(crate::ports::HttpResponse {
                status: 200,
                body: success_body("A"),
            }),
            Ok(crate::ports::HttpResponse {
                status: 200,
                body: success_body("B"),
            }),
        ]);
        let answer = provider(&http)
            .translate(&vietnamese_to_english(&text))
            .expect("both chunks translate");

        assert_eq!(answer.text, "AB");
        let requests = http.requests();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].url.contains(&format!("q={}", "a".repeat(500))));
        assert!(requests[1].url.contains(&format!("q=%20{}", "b".repeat(100))));
        assert!(requests.iter().all(|request| request.url.ends_with("langpair=vi%7Cen")));
    }

    #[test]
    fn empty_text_is_answered_without_touching_the_network() {
        let http = FakeHttpClient::responding_with(200, success_body("unused"));
        let answer = provider(&http)
            .translate(&vietnamese_to_english(""))
            .expect("empty text is trivially translated");
        assert_eq!(answer.text, "");
        assert!(http.requests().is_empty());
    }

    #[test]
    fn improve_belongs_to_the_model_provider_and_is_not_configured_here() {
        let http = FakeHttpClient::responding_with(200, success_body("unused"));
        assert_eq!(
            provider(&http)
                .improve(&ImproveRequest {
                    text: "helo".to_owned()
                })
                .expect_err("MyMemory does not improve"),
            ProviderError::NotConfigured
        );
        assert!(http.requests().is_empty());
    }

    #[test]
    fn entities_the_provider_does_not_produce_are_left_alone() {
        assert_eq!(html_unescape("Tom &amp; Jerry"), "Tom & Jerry");
        assert_eq!(html_unescape("no entity here"), "no entity here");
        assert_eq!(html_unescape("&bogus; &#x1F600; &#169;"), "&bogus; 😀 ©");
        assert_eq!(html_unescape("a lone & ampersand"), "a lone & ampersand");
    }
}
