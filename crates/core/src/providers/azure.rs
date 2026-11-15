//! Azure AI Translator v3, the primary rung of the ladder. One `POST` per piece
//! of text: Azure's 50,000-character request limit is far beyond anything the
//! app translates, so there is no chunking here.
//!
//! The key is read from the credential store **when a request is about to be
//! made**, never at launch, so a key saved in Settings takes effect without a
//! restart. `crate::http::UreqHttpClient` uses `or_any_status`, which means every
//! HTTP status arrives as an `Ok(HttpResponse)`; this module reads the status and
//! the body itself and maps them to the typed failures.

use std::sync::Arc;

use serde::Deserialize;

use crate::error::{AuthFailure, ProviderError};
use crate::language::Language;
use crate::model::{Improvement, ProviderId, ProviderTranslation};
use crate::ports::{
    CredentialStore, HttpClient, HttpMethod, HttpRequest, ImproveRequest, TranslateRequest,
    TranslationProvider,
};
use crate::providers::encode_query_component;
use crate::providers::failure::{
    self, AuthStatus, BodyDetail, FailurePolicy, Meaning, ServerErrors, ServerFamily, Unrecognised,
};

pub const ENDPOINT: &str = "https://api.cognitive.microsofttranslator.com/translate";
const API_VERSION: &str = "3.0";
/// Azure expects UTF-8 JSON, sent either zipped or plain; this provider sends it
/// plain.
const CONTENT_TYPE: &str = "application/json; charset=UTF-8";

/// Azure's failure policy: which statuses and 6-digit codes mean what. A 401 is
/// always a bad key; a 403 is the quota (`403001`), a refusing resource
/// (`403000`), or — with no recognised code — still a bad key, never a quota.
const FAILURES: FailurePolicy = FailurePolicy {
    provider: ProviderId::Azure,
    auth_statuses: &[
        AuthStatus {
            status: 401,
            codes: &[],
            fallback: Meaning::Authentication(AuthFailure::InvalidKey),
        },
        AuthStatus {
            status: 403,
            codes: &[
                (403001, Meaning::Quota),
                (
                    403000,
                    Meaning::Authentication(AuthFailure::ResourceNotAuthorized),
                ),
            ],
            fallback: Meaning::Authentication(AuthFailure::InvalidKey),
        },
    ],
    unavailable_statuses: &[],
    rate_limit_reason: "Azure is rate-limiting requests; try again shortly",
    server_errors: ServerErrors {
        family: ServerFamily::Network,
        from: 500,
        to: 599,
    },
    unrecognised: Unrecognised::Malformed(BodyDetail::Raw),
    quota_markers: &[],
};

/// Where the region header comes from. Settings can change while the app runs,
/// so it is read per request rather than captured at construction.
pub type RegionSource = dyn Fn() -> String + Send + Sync;

/// Azure Translator, with an injected HTTP client and credential store so tests
/// never open a socket or touch the Keychain.
pub struct AzureProvider {
    http: Arc<dyn HttpClient>,
    credentials: Arc<dyn CredentialStore>,
    region: Arc<RegionSource>,
}

impl AzureProvider {
    pub fn new(
        http: Arc<dyn HttpClient>,
        credentials: Arc<dyn CredentialStore>,
        region: Arc<RegionSource>,
    ) -> Self {
        Self {
            http,
            credentials,
            region,
        }
    }

    fn key(&self) -> Result<String, ProviderError> {
        match self.credentials.get(ProviderId::Azure) {
            Ok(Some(key)) if !key.trim().is_empty() => Ok(key),
            // No key is the normal first-run state: the ladder falls through to
            // the keyless provider and the user sees no error at all.
            Ok(_) => Err(ProviderError::NotConfigured),
            // A Keychain read that failed is not "not configured": report it as
            // unavailable so the ladder still falls through, but the reason is
            // not hidden from logs.
            Err(error) => Err(failure::unreadable_key(&FAILURES, &error)),
        }
    }

    fn endpoint(&self, request: &TranslateRequest) -> String {
        let mut url = format!(
            "{ENDPOINT}?api-version={API_VERSION}&to={}",
            encode_query_component(request.target.code()),
        );
        if let Some(source) = &request.source {
            url.push_str("&from=");
            url.push_str(&encode_query_component(source.code()));
        }
        url
    }

    fn headers(&self, key: &str) -> Vec<(String, String)> {
        let mut headers = vec![
            ("Content-Type".to_owned(), CONTENT_TYPE.to_owned()),
            ("Ocp-Apim-Subscription-Key".to_owned(), key.to_owned()),
        ];
        // A global resource rejects a region header; a regional one needs it.
        let region = (self.region)();
        let region = region.trim();
        if !region.is_empty() {
            headers.push(("Ocp-Apim-Subscription-Region".to_owned(), region.to_owned()));
        }
        headers
    }
}

impl TranslationProvider for AzureProvider {
    fn translate(&self, request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError> {
        if request.text.is_empty() {
            return Ok(ProviderTranslation {
                text: String::new(),
                provider: ProviderId::Azure,
                detected_source: None,
            });
        }

        // Before the network, so a keyless install costs nothing.
        let key = self.key()?;

        let body = serde_json::to_string(&serde_json::json!([{ "text": request.text }]))
            .map_err(|error| {
                FAILURES.malformed(format!("the request body could not be built: {error}"))
            })?;

        let response = self
            .http
            .execute(&HttpRequest {
                method: HttpMethod::Post,
                url: self.endpoint(request),
                headers: self.headers(&key),
                body: Some(body),
            })
            .map_err(|error| failure::transport(&FAILURES, error))?;

        read_answer(response.status, &response.body)
    }

    /// Improve belongs to the model provider (ticket 08).
    fn improve(&self, _request: &ImproveRequest) -> Result<Improvement, ProviderError> {
        Err(ProviderError::NotConfigured)
    }
}

#[derive(Deserialize)]
struct TranslationResult {
    #[serde(rename = "detectedLanguage", default)]
    detected_language: Option<DetectedLanguage>,
    #[serde(default)]
    translations: Vec<TranslatedText>,
}

#[derive(Deserialize)]
struct DetectedLanguage {
    language: String,
}

#[derive(Deserialize)]
struct TranslatedText {
    #[serde(default)]
    text: String,
}

/// Read one response. The status decides the family of failure; the body can
/// only sharpen it — Azure puts a 6-digit `error.code` in the JSON. The shared
/// table owns that decision; this module only decodes a successful body.
fn read_answer(status: u16, body: &str) -> Result<ProviderTranslation, ProviderError> {
    if (200..=299).contains(&status) {
        return read_translation(body);
    }
    Err(
        failure::status(&FAILURES, status, body)
            .expect("Azure's failure policy settles every non-success status"),
    )
}

fn read_translation(body: &str) -> Result<ProviderTranslation, ProviderError> {
    let mut results: Vec<TranslationResult> = serde_json::from_str(body).map_err(|error| {
        FAILURES.malformed(format!("the body was not the expected JSON: {error}"))
    })?;
    let result = results
        .drain(..)
        .next()
        .ok_or_else(|| FAILURES.malformed("the response carried no translation result"))?;
    let text = result
        .translations
        .into_iter()
        .next()
        .map(|translation| translation.text)
        .unwrap_or_default();
    if text.trim().is_empty() {
        return Err(FAILURES.malformed("the response carried no translated text"));
    }
    Ok(ProviderTranslation {
        text,
        provider: ProviderId::Azure,
        detected_source: result
            .detected_language
            .map(|detected| Language::new(detected.language)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::HttpError;
    use crate::testing::{FakeCredentials, FakeHttpClient};

    fn request(text: &str, source: Option<Language>) -> TranslateRequest {
        TranslateRequest {
            text: text.to_owned(),
            source,
            target: Language::english(),
        }
    }

    fn vietnamese_to_english(text: &str) -> TranslateRequest {
        request(text, Some(Language::vietnamese()))
    }

    fn provider_with(
        http: &FakeHttpClient,
        credentials: &FakeCredentials,
        region: &'static str,
    ) -> AzureProvider {
        AzureProvider::new(
            Arc::new(http.clone()),
            Arc::new(credentials.clone()),
            Arc::new(move || region.to_owned()),
        )
    }

    fn keyed(http: &FakeHttpClient) -> AzureProvider {
        provider_with(
            http,
            &FakeCredentials::with_secret(ProviderId::Azure, "azure-key"),
            "",
        )
    }

    fn success_body() -> &'static str {
        r#"[{"detectedLanguage":{"language":"vi","score":1.0},
             "translations":[{"text":"Hello world","to":"en"}]}]"#
    }

    #[test]
    fn no_key_is_not_configured_without_touching_the_network() {
        let http = FakeHttpClient::responding_with(200, success_body());
        let error = provider_with(&http, &FakeCredentials::default(), "")
            .translate(&vietnamese_to_english("Xin chào"))
            .expect_err("there is no key");

        assert_eq!(error, ProviderError::NotConfigured);
        assert!(http.requests().is_empty());
    }

    #[test]
    fn a_successful_answer_is_the_translated_text_with_the_provider_and_detection() {
        let http = FakeHttpClient::responding_with(200, success_body());
        let answer = keyed(&http)
            .translate(&vietnamese_to_english("Xin chào"))
            .expect("a 200 answer translates");

        assert_eq!(answer.text, "Hello world");
        assert_eq!(answer.provider, ProviderId::Azure);
        assert_eq!(answer.detected_source, Some(Language::vietnamese()));

        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, HttpMethod::Post);
        assert_eq!(
            requests[0].url,
            "https://api.cognitive.microsofttranslator.com/translate\
             ?api-version=3.0&to=en&from=vi"
        );
        assert!(requests[0]
            .headers
            .contains(&("Ocp-Apim-Subscription-Key".to_owned(), "azure-key".to_owned())));
        assert!(requests[0]
            .headers
            .contains(&("Content-Type".to_owned(), CONTENT_TYPE.to_owned())));
        assert_eq!(requests[0].body.as_deref(), Some(r#"[{"text":"Xin chào"}]"#));
    }

    #[test]
    fn the_request_body_escapes_quotes_and_newlines() {
        let http = FakeHttpClient::responding_with(200, success_body());
        let text = "He said \"hi\"\nthen left";
        keyed(&http)
            .translate(&vietnamese_to_english(text))
            .expect("the fake answers");

        let body = http.requests()[0].body.clone().expect("a JSON body");
        let parsed: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert_eq!(parsed[0]["text"], text);
    }

    #[test]
    fn a_regional_resource_sends_the_region_header_and_a_global_one_does_not() {
        let credentials = FakeCredentials::with_secret(ProviderId::Azure, "azure-key");
        let regional_http = FakeHttpClient::responding_with(200, success_body());
        provider_with(&regional_http, &credentials, "westeurope")
            .translate(&vietnamese_to_english("Xin chào"))
            .expect("the fake answers");

        let global_http = FakeHttpClient::responding_with(200, success_body());
        provider_with(&global_http, &credentials, "")
            .translate(&vietnamese_to_english("Xin chào"))
            .expect("the fake answers");

        assert!(regional_http.requests()[0]
            .headers
            .contains(&("Ocp-Apim-Subscription-Region".to_owned(), "westeurope".to_owned())));
        assert!(!global_http.requests()[0]
            .headers
            .iter()
            .any(|(name, _)| name == "Ocp-Apim-Subscription-Region"));
    }

    #[test]
    fn with_no_resolved_source_from_is_omitted_and_azure_detects_the_language() {
        let http = FakeHttpClient::responding_with(200, success_body());
        let answer = keyed(&http)
            .translate(&request("Xin chào", None))
            .expect("the fake answers");

        assert_eq!(
            http.requests()[0].url,
            "https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&to=en"
        );
        assert_eq!(answer.detected_source, Some(Language::vietnamese()));
    }

    #[test]
    fn an_empty_translation_reads_azure_detection_into_detected_source() {
        let http = FakeHttpClient::responding_with(
            200,
            r#"[{"detectedLanguage":{"language":"en","score":0.9},
                 "translations":[{"text":"Xin chào","to":"vi"}]}]"#,
        );
        let answer = keyed(&http)
            .translate(&request("Hello", None))
            .expect("the fake answers");

        assert_eq!(answer.detected_source, Some(Language::english()));
    }

    #[test]
    fn a_401_is_a_bad_key() {
        let http = FakeHttpClient::responding_with(
            401,
            r#"{"error":{"code":401000,"message":"credentials missing or invalid"}}"#,
        );
        assert_eq!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("the key is wrong"),
            ProviderError::AuthenticationFailed {
                provider: ProviderId::Azure,
                cause: AuthFailure::InvalidKey,
            }
        );
    }

    #[test]
    fn a_403_with_code_403001_is_an_exhausted_quota() {
        let http = FakeHttpClient::responding_with(
            403,
            r#"{"error":{"code":403001,"message":"the free quota is exhausted"}}"#,
        );
        assert_eq!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("the quota is gone"),
            ProviderError::QuotaExceeded {
                provider: ProviderId::Azure
            }
        );
    }

    #[test]
    fn a_403_with_code_403000_points_at_the_resource_not_the_quota() {
        let http = FakeHttpClient::responding_with(
            403,
            r#"{"error":{"code":403000,"message":"the resource is disabled"}}"#,
        );
        assert_eq!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("the resource refuses"),
            ProviderError::AuthenticationFailed {
                provider: ProviderId::Azure,
                cause: AuthFailure::ResourceNotAuthorized,
            }
        );
    }

    #[test]
    fn a_403_without_a_recognised_code_is_still_an_authentication_failure() {
        let http = FakeHttpClient::responding_with(403, "Forbidden");
        assert_eq!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("the request is refused"),
            ProviderError::AuthenticationFailed {
                provider: ProviderId::Azure,
                cause: AuthFailure::InvalidKey,
            }
        );
    }

    #[test]
    fn a_rate_limit_is_unavailable_and_never_a_session_ban() {
        let http = FakeHttpClient::responding_with(
            429,
            r#"{"error":{"code":429001,"message":"request limits exceeded"}}"#,
        );
        assert!(matches!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("rate limited"),
            ProviderError::Unavailable { .. }
        ));
    }

    #[test]
    fn a_server_fault_is_a_network_failure() {
        for status in [500, 503] {
            let http = FakeHttpClient::responding_with(status, "");
            assert!(
                matches!(
                    keyed(&http)
                        .translate(&vietnamese_to_english("Xin chào"))
                        .expect_err("the server did not answer"),
                    ProviderError::Network {
                        provider: ProviderId::Azure,
                        ..
                    }
                ),
                "HTTP {status} should be a network failure"
            );
        }
    }

    #[test]
    fn an_unreachable_network_is_offline() {
        let http = FakeHttpClient::failing_with(HttpError::Unreachable {
            message: "no route to host".to_owned(),
        });
        assert_eq!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("there is no network"),
            ProviderError::Offline
        );
    }

    #[test]
    fn a_request_that_failed_before_an_answer_is_a_network_error() {
        let http = FakeHttpClient::failing_with(HttpError::Request {
            message: "the request was malformed".to_owned(),
        });
        assert!(matches!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("the request failed"),
            ProviderError::Network {
                provider: ProviderId::Azure,
                ..
            }
        ));
    }

    #[test]
    fn a_body_that_is_not_json_is_a_malformed_response() {
        let http = FakeHttpClient::responding_with(200, "upstream is down");
        assert!(matches!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("plain text is not the envelope"),
            ProviderError::MalformedResponse {
                provider: ProviderId::Azure,
                ..
            }
        ));
    }

    #[test]
    fn an_empty_result_array_is_a_malformed_response() {
        let http = FakeHttpClient::responding_with(200, "[]");
        assert!(matches!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("nothing was translated"),
            ProviderError::MalformedResponse {
                provider: ProviderId::Azure,
                ..
            }
        ));
    }

    #[test]
    fn an_empty_translation_is_a_malformed_response() {
        let http = FakeHttpClient::responding_with(
            200,
            r#"[{"translations":[{"text":"   ","to":"en"}]}]"#,
        );
        assert!(matches!(
            keyed(&http)
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("nothing was translated"),
            ProviderError::MalformedResponse {
                provider: ProviderId::Azure,
                ..
            }
        ));
    }

    #[test]
    fn empty_text_is_answered_without_touching_the_network() {
        let http = FakeHttpClient::responding_with(200, success_body());
        let credentials = FakeCredentials::default();
        let answer = provider_with(&http, &credentials, "")
            .translate(&vietnamese_to_english(""))
            .expect("empty text is trivially translated");

        assert_eq!(answer.text, "");
        assert!(http.requests().is_empty());
    }

    #[test]
    fn the_key_is_read_when_a_request_is_made_so_a_new_key_takes_effect_at_once() {
        let http = FakeHttpClient::responding_with(200, success_body());
        let credentials = FakeCredentials::default();
        let provider = provider_with(&http, &credentials, "");

        assert_eq!(
            provider
                .translate(&vietnamese_to_english("Xin chào"))
                .expect_err("no key yet"),
            ProviderError::NotConfigured
        );

        credentials
            .set(ProviderId::Azure, "saved-in-settings")
            .expect("the fake stores it");

        provider
            .translate(&vietnamese_to_english("Xin chào"))
            .expect("the new key is used without a restart");
        assert_eq!(http.requests().len(), 1);
        assert!(http.requests()[0]
            .headers
            .contains(&("Ocp-Apim-Subscription-Key".to_owned(), "saved-in-settings".to_owned())));
    }

    #[test]
    fn improve_belongs_to_the_model_provider_and_is_not_configured_here() {
        let http = FakeHttpClient::responding_with(200, success_body());
        assert_eq!(
            keyed(&http)
                .improve(&ImproveRequest {
                    text: "helo".to_owned()
                })
                .expect_err("Azure does not improve"),
            ProviderError::NotConfigured
        );
        assert!(http.requests().is_empty());
    }
}
