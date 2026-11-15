//! The one table that turns a transport or HTTP outcome into a `ProviderError`.
//!
//! Every rung supplies a `FailurePolicy`: which statuses mean authentication,
//! which provider codes sharpen that into a cause or a quota, how it reports a
//! server error, and its sentences. The shared statuses are enumerated here
//! once, so adding a rung or changing what a rate limit means is a data change
//! rather than a fourth copied match.
//!
//! This module is internal to the providers side: the crate's public interface
//! is still the ports and the engine.

use std::fmt;

use crate::error::{AuthFailure, CredentialError, HttpError, ProviderError};
use crate::model::ProviderId;

const RATE_LIMIT: u16 = 429;

/// What an authentication status means for this provider. Azure can say that
/// the key is bad, the resource refuses the request, or the quota is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Meaning {
    Authentication(AuthFailure),
    Quota,
}

/// One status that means authentication, and the body codes that sharpen it.
pub(crate) struct AuthStatus {
    pub status: u16,
    /// Codes carried in the body, first match wins. Empty means the status does
    /// not read a code at all (Groq's 401 and 403, Azure's 401).
    pub codes: &'static [(i64, Meaning)],
    /// What the status means when the body carries no recognised code.
    pub fallback: Meaning,
}

/// How a provider reports a server error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ServerFamily {
    /// A server error is a transport-level failure: `Network`.
    Network,
    /// A server error is a temporary outage: `Unavailable`.
    Unavailable,
}

/// The status range a rung calls a server error. Azure and Groq use 500–599;
/// MyMemory treats every status from 500 up as unavailable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ServerErrors {
    pub family: ServerFamily,
    pub from: u16,
    pub to: u16,
}

/// How to describe the body of an unrecognised status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BodyDetail {
    /// The body as it arrived.
    Raw,
    /// The provider's `error.message`, or a short excerpt.
    ErrorMessage,
}

/// What an unrecognised status means for this rung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Unrecognised {
    /// Report a malformed response; the detail comes from the body.
    Malformed(BodyDetail),
    /// The status is not a verdict: the rung reads the body for the real one
    /// (MyMemory reports its semantic errors in a 200 body).
    ReadBody,
}

/// One provider's failure policy, supplied as data by its rung.
pub(crate) struct FailurePolicy {
    pub provider: ProviderId,
    pub auth_statuses: &'static [AuthStatus],
    /// Statuses that are a temporary inability to serve this request, with the
    /// provider's sentence. `{status}` is replaced by the status number.
    pub unavailable_statuses: &'static [(u16, &'static str)],
    /// The provider's sentence for a rate limit.
    pub rate_limit_reason: &'static str,
    pub server_errors: ServerErrors,
    pub unrecognised: Unrecognised,
    /// Case-insensitive markers that mean the free quota is spent. MyMemory is
    /// the only provider that reports this in a successful body.
    pub quota_markers: &'static [&'static str],
}

impl FailurePolicy {
    pub(crate) fn authentication(&self, cause: AuthFailure) -> ProviderError {
        ProviderError::AuthenticationFailed {
            provider: self.provider,
            cause,
        }
    }

    pub(crate) fn quota(&self) -> ProviderError {
        ProviderError::QuotaExceeded {
            provider: self.provider,
        }
    }

    pub(crate) fn network(&self, message: impl Into<String>) -> ProviderError {
        ProviderError::Network {
            provider: self.provider,
            message: message.into(),
        }
    }

    pub(crate) fn unavailable(&self, reason: impl Into<String>) -> ProviderError {
        ProviderError::Unavailable {
            reason: reason.into(),
        }
    }

    pub(crate) fn malformed(&self, message: impl Into<String>) -> ProviderError {
        ProviderError::MalformedResponse {
            provider: self.provider,
            message: message.into(),
        }
    }
}

/// Map a failure that happened before any status could be read.
pub(crate) fn transport(policy: &FailurePolicy, error: HttpError) -> ProviderError {
    match error {
        HttpError::Unreachable { .. } => ProviderError::Offline,
        HttpError::Request { message } => policy.network(message),
    }
}

/// Map a non-success status. `None` means this rung's policy leaves the verdict
/// to the body, so the caller must read it.
pub(crate) fn status(policy: &FailurePolicy, status: u16, body: &str) -> Option<ProviderError> {
    if let Some(auth) = policy
        .auth_statuses
        .iter()
        .find(|auth| auth.status == status)
    {
        return Some(authentication(policy, auth, body));
    }
    if status == RATE_LIMIT {
        return Some(policy.unavailable(policy.rate_limit_reason));
    }
    if let Some((_, reason)) = policy
        .unavailable_statuses
        .iter()
        .find(|(unavailable, _)| *unavailable == status)
    {
        return Some(policy.unavailable(reason.replace("{status}", &status.to_string())));
    }
    if (policy.server_errors.from..=policy.server_errors.to).contains(&status) {
        let message = format!("{} answered HTTP {status}", policy.provider);
        return Some(match policy.server_errors.family {
            ServerFamily::Network => policy.network(message),
            ServerFamily::Unavailable => policy.unavailable(message),
        });
    }
    match policy.unrecognised {
        Unrecognised::ReadBody => None,
        Unrecognised::Malformed(detail) => Some(policy.malformed(format!(
            "{} answered HTTP {status}: {}",
            policy.provider,
            detail_of(detail, body)
        ))),
    }
}

fn authentication(policy: &FailurePolicy, auth: &AuthStatus, body: &str) -> ProviderError {
    let code = body_code(body);
    let meaning = auth
        .codes
        .iter()
        .find(|(candidate, _)| code == Some(*candidate))
        .map(|(_, meaning)| *meaning)
        .unwrap_or(auth.fallback);
    match meaning {
        Meaning::Authentication(cause) => policy.authentication(cause),
        Meaning::Quota => policy.quota(),
    }
}

/// A code from a provider's JSON error envelope: `{"error": {"code": …}}`. The
/// code may be a number or a numeric string. A body that is not that shape has
/// no code, which leaves the status's fallback in charge.
fn body_code(body: &str) -> Option<i64> {
    let envelope: serde_json::Value = serde_json::from_str(body).ok()?;
    let code = &envelope.get("error")?.get("code")?;
    code.as_i64()
        .or_else(|| code.as_str().and_then(|code| code.parse().ok()))
}

fn detail_of(detail: BodyDetail, body: &str) -> String {
    match detail {
        BodyDetail::Raw => body.to_owned(),
        BodyDetail::ErrorMessage => body_message(body),
    }
}

/// The `error.message` of an error envelope, or a short excerpt of the body.
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

/// Detect a quota that a provider reports inside an otherwise successful body.
/// The rung decodes the fields; the markers and the decision live here as data.
pub(crate) fn body_quota(
    policy: &FailurePolicy,
    quota_finished: bool,
    details: &str,
    translated: &str,
) -> Option<ProviderError> {
    if quota_finished {
        return Some(policy.quota());
    }
    let details = details.to_uppercase();
    let translated = translated.to_uppercase();
    policy
        .quota_markers
        .iter()
        .any(|marker| details.contains(marker) || translated.contains(marker))
        .then(|| policy.quota())
}

/// A credential store that could not be read is not "not configured": it is an
/// unavailable provider, and the sentence is the table's, not the rung's.
pub(crate) fn unreadable_key(policy: &FailurePolicy, error: &CredentialError) -> ProviderError {
    policy.unavailable(format!(
        "the {} key could not be read: {error}",
        policy.provider
    ))
}

/// The user-facing sentence for a provider failure. It lives next to the table
/// so a provider's words are written once, not in every rung.
impl fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderError::NotConfigured => {
                formatter.write_str("no translation provider is configured")
            }
            ProviderError::MissingKey { provider } => {
                write!(formatter, "{provider} needs an API key. Add one in Settings.")
            }
            ProviderError::QuotaExceeded { provider } => write!(
                formatter,
                "{provider} has used up its free translation quota; it works again when the quota resets"
            ),
            ProviderError::AuthenticationFailed {
                provider,
                cause: AuthFailure::InvalidKey,
            } => write!(
                formatter,
                "{provider} rejected its key. Enter a new key in Settings."
            ),
            ProviderError::AuthenticationFailed {
                provider,
                cause: AuthFailure::ResourceNotAuthorized,
            } => write!(
                formatter,
                "{provider} refused the request for this resource. Check the Azure resource: it may be disabled, suspended, or in another region."
            ),
            ProviderError::Offline => formatter.write_str("the machine is offline"),
            ProviderError::Network { provider, message } => {
                write!(formatter, "{provider} could not be reached: {message}")
            }
            ProviderError::MalformedResponse { provider, message } => {
                write!(formatter, "{provider} returned a malformed response: {message}")
            }
            ProviderError::Unavailable { reason } => {
                write!(formatter, "the provider is unavailable: {reason}")
            }
        }
    }
}
