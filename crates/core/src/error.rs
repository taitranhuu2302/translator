use serde::{Deserialize, Serialize};

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
    #[error("the focused app does not accept a replacement")]
    NotWritable,
    #[error("selection capture is unavailable: {reason}")]
    Unavailable { reason: String },
}

/// Why a provider request failed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum ProviderError {
    #[error("no translation provider is configured")]
    NotConfigured,
    #[error("{provider:?} quota exceeded")]
    QuotaExceeded { provider: ProviderId },
    #[error("{provider:?} rejected the credentials")]
    AuthenticationFailed { provider: ProviderId },
    #[error("the machine is offline")]
    Offline,
    #[error("{provider:?} could not be reached: {message}")]
    Network {
        provider: ProviderId,
        message: String,
    },
    #[error("{provider:?} returned a malformed response: {message}")]
    MalformedResponse {
        provider: ProviderId,
        message: String,
    },
    #[error("the provider is unavailable: {reason}")]
    Unavailable { reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum SpeechError {
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
