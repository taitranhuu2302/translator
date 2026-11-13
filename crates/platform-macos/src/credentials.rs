//! API keys in the macOS login Keychain, one generic-password item per provider.
//!
//! The service is fixed so later versions can still find keys saved today; the
//! account is the provider's name. Reads happen when a request is about to be
//! made, so a key saved in Settings takes effect without a restart. Nothing here
//! ever logs the secret.

#[cfg(target_os = "macos")]
use security_framework::passwords::{
    delete_generic_password, generic_password, set_generic_password, PasswordOptions,
};
#[cfg(target_os = "macos")]
use security_framework_sys::base::errSecItemNotFound;

use translator_core::{CredentialError, ProviderId};

/// The Keychain service these items live under. A stable identifier, not the
/// bundle display name, because it is the lookup key stored by macOS.
pub const SERVICE: &str = "com.taitran.translator";

/// The Keychain account for a provider. MyMemory needs no key; it is included so
/// the mapping stays total and a stray save cannot collide with another item.
fn account(provider: ProviderId) -> &'static str {
    match provider {
        ProviderId::Azure => "azure",
        ProviderId::Groq => "groq",
        ProviderId::MyMemory => "mymemory",
    }
}

#[cfg(target_os = "macos")]
pub fn get(provider: ProviderId) -> Result<Option<String>, CredentialError> {
    match generic_password(PasswordOptions::new_generic_password(SERVICE, account(provider))) {
        Ok(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|error| CredentialError::Storage {
                message: format!("the stored key was not valid UTF-8: {error}"),
            }),
        // No item is the normal first-run state, not a failure.
        Err(error) if error.code() == errSecItemNotFound => Ok(None),
        Err(error) => Err(storage_error(error)),
    }
}

#[cfg(target_os = "macos")]
pub fn set(provider: ProviderId, secret: &str) -> Result<(), CredentialError> {
    set_generic_password(SERVICE, account(provider), secret.as_bytes()).map_err(storage_error)
}

#[cfg(target_os = "macos")]
pub fn delete(provider: ProviderId) -> Result<(), CredentialError> {
    match delete_generic_password(SERVICE, account(provider)) {
        Ok(()) => Ok(()),
        // Deleting what is not there is the state the caller asked for.
        Err(error) if error.code() == errSecItemNotFound => Ok(()),
        Err(error) => Err(storage_error(error)),
    }
}

#[cfg(target_os = "macos")]
fn storage_error(error: security_framework::base::Error) -> CredentialError {
    CredentialError::Storage {
        message: error.to_string(),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn get(_provider: ProviderId) -> Result<Option<String>, CredentialError> {
    Err(CredentialError::Unavailable {
        reason: "the Keychain is only available on macOS".to_owned(),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn set(_provider: ProviderId, _secret: &str) -> Result<(), CredentialError> {
    Err(CredentialError::Unavailable {
        reason: "the Keychain is only available on macOS".to_owned(),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn delete(_provider: ProviderId) -> Result<(), CredentialError> {
    Err(CredentialError::Unavailable {
        reason: "the Keychain is only available on macOS".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_provider_has_its_own_account_under_the_fixed_service() {
        assert_eq!(SERVICE, "com.taitran.translator");
        assert_eq!(account(ProviderId::Azure), "azure");
        assert_eq!(account(ProviderId::Groq), "groq");
        assert_eq!(account(ProviderId::MyMemory), "mymemory");
    }
}
