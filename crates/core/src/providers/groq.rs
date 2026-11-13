//! Groq's OpenAI-compatible chat completions, the Improve rung. It is the only
//! provider that improves text: Azure and MyMemory translate, and returning
//! `NotConfigured` from them lets the ladder fall through to this one.
//!
//! Structured Outputs (`json_schema`, `strict`) cannot stream, so the request
//! asks for `json_object` and the answer is accumulated. The shape is then
//! enforced at the end by a strict `serde_json` parse: anything that is not
//! exactly the two fields is a typed `MalformedResponse`, never prose the view
//! has to split. Streaming is only presentation — the returned `Improvement` is
//! the authority.
//!
//! The key is read from the credential store when a request is about to be made,
//! never at construction, so saving a key takes effect without a restart. No key
//! is `MissingKey { Groq }`, which names the provider and the fix and still lets
//! the ladder fall through.

use std::sync::Arc;

use serde::Deserialize;

use crate::error::{AuthFailure, HttpError, ProviderError};
use crate::model::{ImproveDelta, Improvement, ProviderId, ProviderTranslation};
use crate::ports::{
    CredentialStore, HttpClient, HttpMethod, HttpRequest, ImproveRequest, TranslateRequest,
    TranslationProvider,
};
use crate::providers::improve_stream::ImprovementStream;

pub const ENDPOINT: &str = "https://api.groq.com/openai/v1/chat/completions";
const MODEL: &str = "openai/gpt-oss-120b";
const CONTENT_TYPE: &str = "application/json";
const ACCEPT: &str = "text/event-stream";

/// The instructions go in the user message, not a system prompt: Groq's own
/// guidance for gpt-oss is to avoid system prompts, and JSON mode requires the
/// word "JSON" to appear in the conversation.
const PROMPT: &str = "You are an English writing assistant. Improve the English text below.\n\
Return JSON only, with exactly two string fields and no other text:\n\
{\"corrected\": \"...\", \"better\": \"...\"}\n\
\"corrected\" has the same meaning and register as the original; change only grammar, spelling and punctuation errors.\n\
\"better\" has the same meaning, rewritten to be more natural and concise in English.\n\
Both fields must be non-empty. Do not merge them and do not add commentary.";

pub struct GroqProvider {
    http: Arc<dyn HttpClient>,
    credentials: Arc<dyn CredentialStore>,
}

impl GroqProvider {
    pub fn new(http: Arc<dyn HttpClient>, credentials: Arc<dyn CredentialStore>) -> Self {
        Self { http, credentials }
    }

    fn key(&self) -> Result<String, ProviderError> {
        match self.credentials.get(ProviderId::Groq) {
            Ok(Some(key)) if !key.trim().is_empty() => Ok(key),
            // Not configured is not "nothing to say": it names the provider and
            // where to fix it, because Improve has no keyless fallback.
            Ok(_) => Err(ProviderError::MissingKey {
                provider: ProviderId::Groq,
            }),
            Err(error) => Err(ProviderError::Unavailable {
                reason: format!("the Groq key could not be read: {error}"),
            }),
        }
    }

    fn body(&self, text: &str) -> Result<String, ProviderError> {
        let content = format!("{PROMPT}\n\nText:\n{text}");
        serde_json::to_string(&serde_json::json!({
            "model": MODEL,
            "messages": [{ "role": "user", "content": content }],
            "temperature": 0.6,
            "top_p": 0.95,
            "max_completion_tokens": 2048,
            "reasoning_effort": "low",
            // `parsed` keeps the thinking out of `content`; `raw` is rejected
            // with JSON mode, and would put reasoning into the answer.
            "reasoning_format": "parsed",
            "response_format": { "type": "json_object" },
            "stream": true,
        }))
        .map_err(|error| ProviderError::MalformedResponse {
            provider: ProviderId::Groq,
            message: format!("the request body could not be built: {error}"),
        })
    }

    fn headers(&self, key: &str) -> Vec<(String, String)> {
        vec![
            ("Content-Type".to_owned(), CONTENT_TYPE.to_owned()),
            ("Accept".to_owned(), ACCEPT.to_owned()),
            ("Authorization".to_owned(), format!("Bearer {key}")),
        ]
    }
}

impl TranslationProvider for GroqProvider {
    /// Model translation is out of scope for v1: Groq exists for Improve only,
    /// and saying so lets the ladder move on to a translation provider.
    fn translate(&self, _request: &TranslateRequest) -> Result<ProviderTranslation, ProviderError> {
        Err(ProviderError::NotConfigured)
    }

    fn improve(&self, request: &ImproveRequest) -> Result<Improvement, ProviderError> {
        // One code path: the non-streaming answer is the streaming answer with
        // nobody listening.
        self.improve_streaming(request, &mut |_| {})
    }

    fn improve_streaming(
        &self,
        request: &ImproveRequest,
        sink: &mut dyn FnMut(&ImproveDelta),
    ) -> Result<Improvement, ProviderError> {
        if request.text.trim().is_empty() {
            return Ok(Improvement {
                corrected: String::new(),
                better: String::new(),
            });
        }

        // Before the network, so a keyless Improve costs nothing.
        let key = self.key()?;
        let body = self.body(&request.text)?;
        let http_request = HttpRequest {
            method: HttpMethod::Post,
            url: ENDPOINT.to_owned(),
            headers: self.headers(&key),
            body: Some(body),
        };

        let mut lines = SseLines::default();
        let mut document = String::new();
        let mut extractor = ImprovementStream::new();
        let mut raw = String::new();
        let mut finished: Option<String> = None;
        let mut saw_done = false;
        let mut failure: Option<String> = None;

        let status = self
            .http
            .stream(&http_request, &mut |chunk| {
                raw.push_str(&String::from_utf8_lossy(chunk));
                for line in lines.push(chunk) {
                    match read_line(&line, &mut document, &mut extractor, &mut *sink) {
                        ReadLine::Continue => {}
                        ReadLine::Done => {
                            saw_done = true;
                            return false;
                        }
                        ReadLine::Finished(reason) => {
                            finished = Some(reason);
                            return false;
                        }
                        ReadLine::Failed(message) => {
                            failure = Some(message);
                            return false;
                        }
                    }
                }
                true
            })
            .map_err(map_transport)?;

        // A final line may arrive without its trailing newline.
        if !saw_done && finished.is_none() && failure.is_none() {
            if let Some(line) = lines.finish() {
                match read_line(&line, &mut document, &mut extractor, &mut *sink) {
                    ReadLine::Continue => {}
                    ReadLine::Done => saw_done = true,
                    ReadLine::Finished(reason) => finished = Some(reason),
                    ReadLine::Failed(message) => failure = Some(message),
                }
            }
        }

        if !(200..=299).contains(&status) {
            return Err(failure_status(status, &raw));
        }
        if let Some(message) = failure {
            return Err(malformed(message));
        }
        if !saw_done && finished.is_none() {
            return Err(malformed(
                "the stream ended before the model finished the answer",
            ));
        }
        match finished.as_deref() {
            Some("stop") | None => {}
            Some("length") => {
                return Err(malformed(
                    "the answer was cut off at the token limit before both fields were finished",
                ))
            }
            Some(other) => {
                return Err(malformed(format!(
                    "the model stopped without an answer ({other})"
                )))
            }
        }

        // The accumulated document is the authority. The deltas were only there
        // to show progress; a stream that never produced valid JSON with two
        // non-empty fields is a failure, not a partial answer.
        let improvement: Improvement = serde_json::from_str(&document)
            .map_err(|error| malformed(format!("the answer was not the expected JSON: {error}")))?;
        if improvement.corrected.trim().is_empty() {
            return Err(malformed("the answer carried an empty corrected field"));
        }
        if improvement.better.trim().is_empty() {
            return Err(malformed("the answer carried an empty better field"));
        }
        Ok(improvement)
    }
}

fn malformed(message: impl Into<String>) -> ProviderError {
    ProviderError::MalformedResponse {
        provider: ProviderId::Groq,
        message: message.into(),
    }
}

fn map_transport(error: HttpError) -> ProviderError {
    match error {
        HttpError::Unreachable { .. } => ProviderError::Offline,
        HttpError::Request { message } => ProviderError::Network {
            provider: ProviderId::Groq,
            message,
        },
    }
}

/// Map a non-2xx answer by its status, as the research note's table does. The
/// body only sharpens the message; it never changes the family of failure.
fn failure_status(status: u16, body: &str) -> ProviderError {
    match status {
        401 | 403 => ProviderError::AuthenticationFailed {
            provider: ProviderId::Groq,
            cause: AuthFailure::InvalidKey,
        },
        // A rate limit is a retry, not a ban: `Unavailable` does not skip the
        // rung for the session.
        429 => ProviderError::Unavailable {
            reason: "Groq is rate-limiting requests; try again shortly".to_owned(),
        },
        422 | 498 => ProviderError::Unavailable {
            reason: format!("Groq could not serve this request right now (HTTP {status})"),
        },
        500..=599 => ProviderError::Network {
            provider: ProviderId::Groq,
            message: format!("Groq answered HTTP {status}"),
        },
        _ => malformed(format!(
            "Groq answered HTTP {status}: {}",
            body_message(body)
        )),
    }
}

/// The `error.message` of Groq's error envelope, or a short excerpt of the body.
fn body_message(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(message) = value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(|message| message.as_str())
        {
            return message.to_owned();
        }
    }
    let excerpt: String = body.trim().chars().take(200).collect();
    if excerpt.is_empty() {
        "no detail".to_owned()
    } else {
        excerpt
    }
}

/// One decoded `data:` line from the stream.
enum ReadLine {
    Continue,
    /// `[DONE]`: the stream said it is over.
    Done,
    /// The final chunk carried a `finish_reason`.
    Finished(String),
    /// The line was not what the protocol promised.
    Failed(String),
}

fn read_line(
    line: &str,
    document: &mut String,
    extractor: &mut ImprovementStream,
    sink: &mut dyn FnMut(&ImproveDelta),
) -> ReadLine {
    let line = line.trim_end_matches('\r');
    if line.is_empty() || line.starts_with(':') {
        // Blank separators and comments such as `: ping` carry no answer.
        return ReadLine::Continue;
    }
    let payload = match line.strip_prefix("data:") {
        Some(payload) => payload.strip_prefix(' ').unwrap_or(payload),
        // `event:`, `id:` and other fields are not part of the answer.
        None => return ReadLine::Continue,
    };
    if payload.trim() == "[DONE]" {
        return ReadLine::Done;
    }
    let event: StreamEvent = match serde_json::from_str(payload) {
        Ok(event) => event,
        Err(error) => {
            return ReadLine::Failed(format!("an event in the stream was not JSON: {error}"))
        }
    };
    // `choices` is empty on the optional usage chunk.
    let Some(choice) = event.choices.into_iter().next() else {
        return ReadLine::Continue;
    };
    if let Some(content) = choice.delta.content {
        if !content.is_empty() {
            document.push_str(&content);
            extractor.feed(content.as_bytes(), sink);
        }
    }
    match choice.finish_reason {
        Some(reason) => ReadLine::Finished(reason),
        None => ReadLine::Continue,
    }
}

#[derive(Deserialize)]
struct StreamEvent {
    #[serde(default)]
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: StreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Default, Deserialize)]
struct StreamDelta {
    /// The token slice. `reasoning` is deliberately not read: with
    /// `reasoning_format: "parsed"` it is separate, and it is not the answer.
    #[serde(default)]
    content: Option<String>,
}

/// Splits a byte stream into complete lines. A line is only decoded once its
/// `\n` has arrived, so a UTF-8 character split across two reads is never
/// decoded half-way; a `\n` byte cannot occur inside a multi-byte character.
#[derive(Default)]
struct SseLines {
    pending: Vec<u8>,
}

impl SseLines {
    fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.pending.extend_from_slice(bytes);
        let mut lines = Vec::new();
        while let Some(index) = self.pending.iter().position(|&byte| byte == b'\n') {
            let line: Vec<u8> = self.pending.drain(..=index).collect();
            lines.push(decode_line(&line[..line.len() - 1]));
        }
        lines
    }

    /// A final line with no trailing newline, if one is left.
    fn finish(&mut self) -> Option<String> {
        if self.pending.is_empty() {
            return None;
        }
        let line: Vec<u8> = self.pending.drain(..).collect();
        Some(decode_line(&line))
    }
}

fn decode_line(bytes: &[u8]) -> String {
    let bytes = match bytes.last() {
        Some(b'\r') => &bytes[..bytes.len() - 1],
        _ => bytes,
    };
    String::from_utf8_lossy(bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ImprovePart;
    use crate::testing::{FakeCredentials, FakeHttpClient};

    fn provider_with(http: &FakeHttpClient, credentials: &FakeCredentials) -> GroqProvider {
        GroqProvider::new(Arc::new(http.clone()), Arc::new(credentials.clone()))
    }

    fn keyed(http: &FakeHttpClient) -> GroqProvider {
        provider_with(
            http,
            &FakeCredentials::with_secret(ProviderId::Groq, "groq-key"),
        )
    }

    fn request(text: &str) -> ImproveRequest {
        ImproveRequest {
            text: text.to_owned(),
        }
    }

    fn content_event(content: &str) -> String {
        format!(
            "data: {}\n\n",
            serde_json::json!({
                "choices": [{
                    "index": 0,
                    "delta": { "content": content },
                    "finish_reason": null
                }]
            })
        )
    }

    fn finish_event(reason: &str) -> String {
        format!(
            "data: {}\n\n",
            serde_json::json!({
                "choices": [{ "index": 0, "delta": {}, "finish_reason": reason }]
            })
        )
    }

    const DONE: &str = "data: [DONE]\n\n";

    /// A complete answer split across three token-sized content events, then a
    /// stop chunk and the terminator.
    fn successful_stream() -> String {
        format!(
            "{}{}{}{}{}",
            content_event("{\"corrected\":\"He goes "),
            content_event("home.\",\"better\":\"He's "),
            content_event("heading home.\"}"),
            finish_event("stop"),
            DONE,
        )
    }

    fn improvement() -> Improvement {
        Improvement {
            corrected: "He goes home.".to_owned(),
            better: "He's heading home.".to_owned(),
        }
    }

    #[test]
    fn no_key_is_a_named_missing_key_and_never_touches_the_network() {
        let http = FakeHttpClient::streaming(200, successful_stream());
        let error = provider_with(&http, &FakeCredentials::default())
            .improve_streaming(&request("He go home."), &mut |_| {})
            .expect_err("there is no key");

        assert_eq!(
            error,
            ProviderError::MissingKey {
                provider: ProviderId::Groq
            }
        );
        assert!(
            http.requests().is_empty(),
            "a keyless Improve must cost no request"
        );
    }

    #[test]
    fn a_blank_key_counts_as_missing() {
        let http = FakeHttpClient::streaming(200, successful_stream());
        let credentials = FakeCredentials::with_secret(ProviderId::Groq, "   ");
        let error = provider_with(&http, &credentials)
            .improve(&request("He go home."))
            .expect_err("a blank key is no key");

        assert_eq!(
            error,
            ProviderError::MissingKey {
                provider: ProviderId::Groq
            }
        );
        assert!(http.requests().is_empty());
    }

    #[test]
    fn a_streaming_answer_returns_both_parts_and_reports_them_as_they_arrive() {
        let http = FakeHttpClient::streaming(200, successful_stream());
        let mut corrected = String::new();
        let mut better = String::new();
        let answer = keyed(&http)
            .improve_streaming(&request("He go home."), &mut |delta| match delta.part {
                ImprovePart::Corrected => corrected.push_str(&delta.text),
                ImprovePart::Better => better.push_str(&delta.text),
            })
            .expect("a clean stream improves");

        assert_eq!(answer, improvement());
        assert_eq!(corrected, "He goes home.");
        assert_eq!(better, "He's heading home.");
    }

    #[test]
    fn every_byte_boundary_of_the_stream_gives_the_same_answer() {
        let stream = successful_stream();
        let bytes = stream.as_bytes();
        for split in 0..=bytes.len() {
            let http = FakeHttpClient::streaming_in_chunks(vec![
                bytes[..split].to_vec(),
                bytes[split..].to_vec(),
            ]);
            let mut corrected = String::new();
            let mut better = String::new();
            let answer = keyed(&http)
                .improve_streaming(&request("He go home."), &mut |delta| match delta.part {
                    ImprovePart::Corrected => corrected.push_str(&delta.text),
                    ImprovePart::Better => better.push_str(&delta.text),
                })
                .unwrap_or_else(|error| panic!("split at byte {split} failed: {error}"));

            assert_eq!(answer, improvement(), "split at byte {split}");
            assert_eq!(corrected, "He goes home.", "split at byte {split}");
            assert_eq!(better, "He's heading home.", "split at byte {split}");
        }
    }

    #[test]
    fn one_byte_at_a_time_decodes_the_same_answer() {
        let bytes = successful_stream().into_bytes();
        let http =
            FakeHttpClient::streaming_in_chunks(bytes.chunks(1).map(<[u8]>::to_vec).collect());
        let answer = keyed(&http)
            .improve(&request("He go home."))
            .expect("the stream decodes one byte at a time");

        assert_eq!(answer, improvement());
    }

    #[test]
    fn the_stream_stops_at_the_terminator_and_ignores_whatever_follows() {
        // Bytes after `[DONE]` are not part of the answer; if the provider kept
        // reading, this garbage would fail the event parse.
        let body = format!("{}not an SSE event at all", successful_stream());
        let http = FakeHttpClient::streaming(200, body);
        let answer = keyed(&http)
            .improve(&request("He go home."))
            .expect("nothing after [DONE] is read");

        assert_eq!(answer, improvement());
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let body = format!(
            ": ping\n\n{}: keep-alive\n\n{}",
            content_event("{\"corrected\":\"Fine.\",\"better\":\"Good.\"}"),
            finish_event("stop")
        );
        let http = FakeHttpClient::streaming(200, body);
        let answer = keyed(&http)
            .improve(&request("Fine."))
            .expect("comments carry no answer");

        assert_eq!(
            answer,
            Improvement {
                corrected: "Fine.".to_owned(),
                better: "Good.".to_owned(),
            }
        );
    }

    #[test]
    fn a_done_terminator_without_a_finish_reason_still_ends_the_answer() {
        let body = format!(
            "{}{DONE}",
            content_event("{\"corrected\":\"Fine.\",\"better\":\"Good.\"}")
        );
        let http = FakeHttpClient::streaming(200, body);
        let answer = keyed(&http)
            .improve(&request("Fine."))
            .expect("done ends it");

        assert_eq!(answer.corrected, "Fine.");
        assert_eq!(answer.better, "Good.");
    }

    #[test]
    fn a_done_terminator_without_a_trailing_newline_still_ends_the_answer() {
        let body = format!(
            "{}{}",
            content_event("{\"corrected\":\"Fine.\",\"better\":\"Good.\"}"),
            "data: [DONE]"
        );
        let http = FakeHttpClient::streaming(200, body);
        let answer = keyed(&http)
            .improve(&request("Fine."))
            .expect("the final line is read");

        assert_eq!(answer.corrected, "Fine.");
    }

    #[test]
    fn the_request_is_a_streaming_json_object_request_with_the_bearer_key() {
        let http = FakeHttpClient::streaming(200, successful_stream());
        keyed(&http)
            .improve(&request("He go home."))
            .expect("the fake answers");

        let requests = http.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, HttpMethod::Post);
        assert_eq!(requests[0].url, ENDPOINT);
        assert!(requests[0]
            .headers
            .contains(&("Authorization".to_owned(), "Bearer groq-key".to_owned())));
        assert!(requests[0]
            .headers
            .contains(&("Content-Type".to_owned(), CONTENT_TYPE.to_owned())));
        assert!(requests[0]
            .headers
            .contains(&("Accept".to_owned(), ACCEPT.to_owned())));

        let body: serde_json::Value =
            serde_json::from_str(requests[0].body.as_deref().expect("a body"))
                .expect("the body is JSON");
        assert_eq!(body["model"], MODEL);
        assert_eq!(body["stream"], true);
        assert_eq!(body["response_format"]["type"], "json_object");
        assert_eq!(body["reasoning_format"], "parsed");
        assert_eq!(body["reasoning_effort"], "low");
        assert_eq!(body["max_completion_tokens"], 2048);

        let prompt = body["messages"][0]["content"]
            .as_str()
            .expect("a user message");
        assert!(prompt.contains("JSON"), "{prompt}");
        assert!(prompt.contains("corrected"), "{prompt}");
        assert!(prompt.contains("better"), "{prompt}");
        assert!(prompt.contains("He go home."), "{prompt}");
    }

    #[test]
    fn a_401_is_a_bad_key() {
        let http = FakeHttpClient::streaming_with_status(
            401,
            vec![br#"{"error":{"message":"Invalid API Key"}}"#.to_vec()],
        );
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("a rejected key fails");

        assert_eq!(
            error,
            ProviderError::AuthenticationFailed {
                provider: ProviderId::Groq,
                cause: AuthFailure::InvalidKey,
            }
        );
    }

    #[test]
    fn a_429_is_unavailable_and_not_a_session_ban() {
        let http = FakeHttpClient::streaming_with_status(
            429,
            vec![br#"{"error":{"message":"Rate limit reached"}}"#.to_vec()],
        );
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("a rate limit fails this request");

        assert!(
            matches!(error, ProviderError::Unavailable { .. }),
            "a 429 must not end the rung's session: {error:?}"
        );
    }

    #[test]
    fn a_500_is_a_network_failure_naming_groq() {
        let http = FakeHttpClient::streaming_with_status(500, vec![b"oops".to_vec()]);
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("a server error fails");

        assert_eq!(
            error,
            ProviderError::Network {
                provider: ProviderId::Groq,
                message: "Groq answered HTTP 500".to_owned(),
            }
        );
    }

    #[test]
    fn a_bad_request_is_a_typed_malformed_response_with_the_message() {
        let http = FakeHttpClient::streaming_with_status(
            400,
            vec![br#"{"error":{"message":"json_object mode needs the word JSON"}}"#.to_vec()],
        );
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("a 400 fails");

        match error {
            ProviderError::MalformedResponse { provider, message } => {
                assert_eq!(provider, ProviderId::Groq);
                assert!(message.contains("json_object mode"), "{message}");
            }
            other => panic!("expected a typed malformed response, got {other:?}"),
        }
    }

    #[test]
    fn an_unreachable_network_is_offline() {
        let http = FakeHttpClient::failing_stream_with(HttpError::Unreachable {
            message: "no route".to_owned(),
        });
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("offline fails");

        assert_eq!(error, ProviderError::Offline);
    }

    #[test]
    fn an_answer_that_is_not_json_is_a_typed_error() {
        let body = format!(
            "{}{}",
            content_event("Here is your improved sentence!"),
            finish_event("stop")
        );
        let http = FakeHttpClient::streaming(200, body);
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("prose is not an improvement");

        assert!(
            matches!(
                error,
                ProviderError::MalformedResponse {
                    provider: ProviderId::Groq,
                    ..
                }
            ),
            "malformed output must be typed: {error:?}"
        );
    }

    #[test]
    fn an_empty_field_is_a_typed_error_not_a_half_answer() {
        let body = format!(
            "{}{}",
            content_event("{\"corrected\":\"He goes home.\",\"better\":\"   \"}"),
            finish_event("stop")
        );
        let http = FakeHttpClient::streaming(200, body);
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("a blank half must not be shown");

        assert!(matches!(error, ProviderError::MalformedResponse { .. }));
    }

    #[test]
    fn a_missing_field_is_a_typed_error() {
        let body = format!(
            "{}{}",
            content_event("{\"corrected\":\"He goes home.\"}"),
            finish_event("stop")
        );
        let http = FakeHttpClient::streaming(200, body);
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("one field is not both");

        assert!(matches!(error, ProviderError::MalformedResponse { .. }));
    }

    #[test]
    fn a_length_finish_is_a_typed_error() {
        // The JSON would be truncated; the token cap must be visible as a
        // failure, never as a quieter, shorter answer.
        let body = format!(
            "{}{}",
            content_event("{\"corrected\":\"He goes home.\",\"better\":\"He's head"),
            finish_event("length")
        );
        let http = FakeHttpClient::streaming(200, body);
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("a cut-off answer is a failure");

        match error {
            ProviderError::MalformedResponse { message, .. } => {
                assert!(message.contains("token limit"), "{message}");
            }
            other => panic!("expected a typed malformed response, got {other:?}"),
        }
    }

    #[test]
    fn a_stream_that_ends_without_a_finish_is_a_typed_error() {
        let body = content_event("{\"corrected\":\"He goes home.\",\"better\":\"Fine.\"}");
        let http = FakeHttpClient::streaming(200, body);
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("an abrupt end is not a finished answer");

        assert!(matches!(error, ProviderError::MalformedResponse { .. }));
    }

    #[test]
    fn an_event_that_is_not_json_is_a_typed_error() {
        let body = format!("data: not json\n\n{}", finish_event("stop"));
        let http = FakeHttpClient::streaming(200, body);
        let error = keyed(&http)
            .improve(&request("He go home."))
            .expect_err("a broken event is malformed");

        assert!(matches!(error, ProviderError::MalformedResponse { .. }));
    }

    #[test]
    fn a_usage_chunk_with_no_choices_is_ignored() {
        let body = format!(
            "{}{}{DONE}",
            content_event("{\"corrected\":\"Fine.\",\"better\":\"Good.\"}"),
            "data: {\"choices\":[],\"usage\":{\"total_tokens\":42}}\n\n"
        );
        let http = FakeHttpClient::streaming(200, body);
        let answer = keyed(&http)
            .improve(&request("Fine."))
            .expect("usage is not an answer");

        assert_eq!(answer.corrected, "Fine.");
    }

    #[test]
    fn an_empty_request_is_an_empty_improvement_without_a_request() {
        let http = FakeHttpClient::streaming(200, successful_stream());
        let answer = keyed(&http)
            .improve(&request("   "))
            .expect("empty text has nothing to improve");

        assert_eq!(
            answer,
            Improvement {
                corrected: String::new(),
                better: String::new(),
            }
        );
        assert!(http.requests().is_empty());
    }

    #[test]
    fn a_whole_body_in_one_chunk_decodes() {
        // A client that cannot stream hands the whole body over once; the
        // provider must decode that identically.
        let http = FakeHttpClient::streaming(200, successful_stream());
        let answer = keyed(&http)
            .improve(&request("He go home."))
            .expect("one chunk decodes");

        assert_eq!(answer, improvement());
    }
}
