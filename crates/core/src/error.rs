use serde::{Deserialize, Serialize};

use crate::language::Language;
use crate::model::ProviderId;

/// Why reading or writing the Selection failed. Every capture path reports one of
/// these rather than silently doing nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum CaptureError {
    #[error("Accessibility permission has not been granted")]
    NoAccessibility,
    #[error("secure input is active; the Selection is never read")]
    SecureInput,
    #[error("nothing is selected")]
    NothingSelected,
    /// A write was asked for when no readable target had been captured, so
    /// there is nowhere to put the text. It is the guard against pasting into
    /// whatever happens to be focused now instead of the captured Selection.
    #[error("there is no captured Selection to replace")]
    NothingToReplace,
    #[error("the focused app does not accept a replacement")]
    NotWritable,
    #[error("selection capture is unavailable: {reason}")]
    Unavailable { reason: String },
}

/// Why an authentication failure happened. Azure separates a bad key (401) from
/// a resource that is disabled, suspended or in another region (403/403000): both
/// are authentication failures, but the user's fix differs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthFailure {
    /// The key is missing, wrong or revoked: entering a new key fixes it.
    #[default]
    InvalidKey,
    /// The key is valid but the resource behind it refuses the request.
    ResourceNotAuthorized,
}

/// Why a provider request failed.
///
/// `Display` is written for the user: every message names the provider and the
/// fix, because the same strings end up in a dialog when a Replace fails.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderError {
    NotConfigured,
    /// The provider needs a key it does not have. It names the provider, because
    /// "something is not configured" leaves the user hunting; the ladder still
    /// falls through to the next rung and never skips this one for the session.
    MissingKey {
        provider: ProviderId,
    },
    QuotaExceeded {
        provider: ProviderId,
    },
    AuthenticationFailed {
        provider: ProviderId,
        /// Older messages did not carry the cause; a missing one is a bad key.
        #[serde(default)]
        cause: AuthFailure,
    },
    Offline,
    Network {
        provider: ProviderId,
        message: String,
    },
    MalformedResponse {
        provider: ProviderId,
        message: String,
    },
    Unavailable {
        reason: String,
    },
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

impl std::error::Error for ProviderError {}

/// Why an HTTP request failed before the caller could read a status.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum HttpError {
    /// The request never reached a server: DNS, connect, timeout or TLS. This is
    /// the "there is no network" case, which the popup reports specifically.
    #[error("the network could not be reached: {message}")]
    Unreachable { message: String },
    /// The request was made but failed before a response could be read.
    #[error("the request failed: {message}")]
    Request { message: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum SpeechError {
    /// The system has no voice installed for the language the text is in, so
    /// Read Aloud fails visibly instead of speaking in the wrong voice.
    #[error("no voice is installed for {language}")]
    NoVoice { language: Language },
    #[error("speech synthesis is unavailable: {reason}")]
    Unavailable { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum HistoryError {
    #[error("history storage is unavailable: {reason}")]
    Unavailable { reason: String },
    #[error("history storage failed: {message}")]
    Storage { message: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum SettingsError {
    #[error("settings storage is unavailable: {reason}")]
    Unavailable { reason: String },
    #[error("settings storage failed: {message}")]
    Storage { message: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum CredentialError {
    #[error("the credential store is unavailable: {reason}")]
    Unavailable { reason: String },
    #[error("the credential store failed: {message}")]
    Storage { message: String },
}

/// Everything an engine operation can fail with. There is no untyped failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum EngineError {
    #[error("selection capture failed: {0}")]
    Capture(#[from] CaptureError),
    #[error("translation failed: {0}")]
    Provider(#[from] ProviderError),
    #[error("speech failed: {0}")]
    Speech(#[from] SpeechError),
    #[error("history failed: {0}")]
    History(#[from] HistoryError),
    #[error("settings failed: {0}")]
    Settings(#[from] SettingsError),
    #[error("credentials failed: {0}")]
    Credentials(#[from] CredentialError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bad_key_names_the_provider_and_the_fix() {
        let message = ProviderError::AuthenticationFailed {
            provider: ProviderId::Azure,
            cause: AuthFailure::InvalidKey,
        }
        .to_string();

        assert!(message.contains("Azure"), "{message}");
        assert!(message.contains("rejected its key"), "{message}");
        assert!(message.contains("Enter a new key in Settings"), "{message}");
    }

    #[test]
    fn a_disabled_resource_points_at_the_resource_not_the_quota() {
        let message = ProviderError::AuthenticationFailed {
            provider: ProviderId::Azure,
            cause: AuthFailure::ResourceNotAuthorized,
        }
        .to_string();

        assert!(message.contains("Azure"), "{message}");
        assert!(message.contains("resource"), "{message}");
        assert!(!message.to_lowercase().contains("quota"), "{message}");
    }

    #[test]
    fn a_missing_key_names_the_provider_and_the_fix() {
        let message = ProviderError::MissingKey {
            provider: ProviderId::Groq,
        }
        .to_string();

        assert_eq!(message, "Groq needs an API key. Add one in Settings.");
    }

    #[test]
    fn an_authentication_failure_written_before_the_cause_existed_reads_as_a_bad_key() {
        let error: ProviderError =
            serde_json::from_str(r#"{"authentication_failed":{"provider":"azure"}}"#)
                .expect("the older shape still deserializes");

        assert_eq!(
            error,
            ProviderError::AuthenticationFailed {
                provider: ProviderId::Azure,
                cause: AuthFailure::InvalidKey,
            }
        );
    }
}
