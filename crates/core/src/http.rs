//! The one real HTTP client, behind the `HttpClient` port. Blocking `ureq` with
//! rustls: providers translate one request at a time on a worker thread, so an
//! async runtime would buy nothing, and rustls keeps the build free of OpenSSL.

use std::io::Read;

use ureq::OrAnyStatus;

use crate::error::HttpError;
use crate::ports::{HttpClient, HttpMethod, HttpRequest, HttpResponse};

/// A blocking HTTP client. It is a thin adapter: it turns ureq's failures into
/// the two cases the providers care about, "the network could not be reached"
/// and "the request was rejected before an answer could be read".
pub struct UreqHttpClient {
    agent: ureq::Agent,
}

impl Default for UreqHttpClient {
    fn default() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout_connect(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(30))
                .user_agent(concat!("Translator/", env!("CARGO_PKG_VERSION")))
                .build(),
        }
    }
}

impl HttpClient for UreqHttpClient {
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, HttpError> {
        let response = self.send(request)?;
        let status = response.status();
        let body = response.into_string().map_err(|error| HttpError::Request {
            message: error.to_string(),
        })?;
        Ok(HttpResponse { status, body })
    }

    /// Read the body as the server writes it. The status is only known once the
    /// headers arrive, so a non-2xx body is streamed too and the provider maps
    /// it after the fact. Reading stops as soon as `on_chunk` answers false, which
    /// is what a finished SSE stream does.
    fn stream(
        &self,
        request: &HttpRequest,
        on_chunk: &mut dyn FnMut(&[u8]) -> bool,
    ) -> Result<u16, HttpError> {
        let response = self.send(request)?;
        let status = response.status();
        let mut reader = response.into_reader();
        let mut buffer = [0u8; 4096];
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|error| HttpError::Request {
                    message: error.to_string(),
                })?;
            if read == 0 {
                break;
            }
            if !on_chunk(&buffer[..read]) {
                break;
            }
        }
        Ok(status)
    }
}

impl UreqHttpClient {
    /// Build and send one request with `or_any_status`: every HTTP status is an
    /// answer the provider must read, not a transport failure. The transport
    /// error is mapped here so neither caller carries ureq's large error type.
    fn send(&self, request: &HttpRequest) -> Result<ureq::Response, HttpError> {
        let mut builder = match request.method {
            HttpMethod::Get => self.agent.get(&request.url),
            HttpMethod::Post => self.agent.post(&request.url),
        };
        for (name, value) in &request.headers {
            builder = builder.set(name, value);
        }
        match &request.body {
            Some(body) => builder.send_string(body),
            None => builder.call(),
        }
        .or_any_status()
        .map_err(failed_before_an_answer)
    }
}

/// Classify a failure that happened before any response body could be read.
fn failed_before_an_answer(transport: ureq::Transport) -> HttpError {
    let message = transport.to_string();
    match transport.kind() {
        // A broken request, a broken proxy setup or a redirect loop: the network
        // is there, this request cannot be served. Never report it as offline.
        ureq::ErrorKind::InvalidUrl
        | ureq::ErrorKind::UnknownScheme
        | ureq::ErrorKind::InsecureRequestHttpsOnly
        | ureq::ErrorKind::TooManyRedirects
        | ureq::ErrorKind::BadStatus
        | ureq::ErrorKind::BadHeader
        | ureq::ErrorKind::InvalidProxyUrl
        | ureq::ErrorKind::ProxyUnauthorized => HttpError::Request { message },
        // DNS, connect, TLS or a read that died: the machine could not reach the
        // network. This is the case the popup reports as "you are offline".
        ureq::ErrorKind::Dns
        | ureq::ErrorKind::ConnectionFailed
        | ureq::ErrorKind::Io
        | ureq::ErrorKind::ProxyConnect
        | ureq::ErrorKind::HTTP => HttpError::Unreachable { message },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::HttpMethod;

    fn request(url: &str) -> HttpRequest {
        HttpRequest {
            method: HttpMethod::Get,
            url: url.to_owned(),
            headers: Vec::new(),
            body: None,
        }
    }

    #[test]
    fn a_malformed_url_is_a_request_error_not_an_offline_one() {
        let error = UreqHttpClient::default()
            .execute(&request("not a url"))
            .expect_err("a malformed URL cannot be fetched");
        assert!(
            matches!(error, HttpError::Request { .. }),
            "a bad URL means the request was wrong, not that the machine is offline: {error:?}"
        );
    }

    #[test]
    fn an_unknown_scheme_is_a_request_error_not_an_offline_one() {
        let error = UreqHttpClient::default()
            .execute(&request("ftp://example.com/file"))
            .expect_err("only http(s) is spoken here");
        assert!(
            matches!(error, HttpError::Request { .. }),
            "an unsupported scheme is a broken request: {error:?}"
        );
    }
}
