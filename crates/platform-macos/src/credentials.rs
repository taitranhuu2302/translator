//! API keys in the macOS login Keychain, one generic-password item per provider.
//!
//! The service is the running bundle's identifier, so it is authored once in
//! `packaging/Info.plist` rather than copied here; the account is the
//! provider's name. Reads happen when a request is about to be made, so a key
//! saved in Settings takes effect without a restart. Nothing here ever logs the
//! secret.

#[cfg(target_os = "macos")]
use security_framework::passwords::{
    delete_generic_password, generic_password, set_generic_password, PasswordOptions,
};
#[cfg(target_os = "macos")]
use security_framework_sys::base::errSecItemNotFound;

use translator_core::{CredentialError, ProviderId};

/// The Keychain service these items live under, derived from the bundle rather
/// than named again. A rename moves it with the rest of the identity
/// (docs/adr/0004-stable-signing-from-first-commit.md).
#[cfg(target_os = "macos")]
fn service() -> String {
    crate::bundle::identifier()
}

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
    match generic_password(PasswordOptions::new_generic_password(
        &service(),
        account(provider),
    )) {
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
    set_generic_password(&service(), account(provider), secret.as_bytes()).map_err(storage_error)
}

#[cfg(target_os = "macos")]
pub fn delete(provider: ProviderId) -> Result<(), CredentialError> {
    match delete_generic_password(&service(), account(provider)) {
        Ok(()) => Ok(()),
        // Deleting what is not there is the state the caller asked for.
        Err(error) if error.code() == errSecItemNotFound => Ok(()),
        Err(error) => Err(storage_error(error)),
    }
}

/// Copy every stored key from one Keychain service to another, leaving the
/// originals in place. Used once, when the bundle identifier changes: the
/// previous build's items stay readable if the rename is rolled back.
#[cfg(target_os = "macos")]
pub fn copy_service(from: &str, to: &str) -> Result<(), CredentialError> {
    if from == to {
        return Ok(());
    }
    for provider in [ProviderId::Azure, ProviderId::Groq, ProviderId::MyMemory] {
        let account = account(provider);
        match generic_password(PasswordOptions::new_generic_password(from, account)) {
            Ok(secret) => {
                set_generic_password(to, account, &secret).map_err(storage_error)?;
            }
            // A provider with no stored key is the ordinary case, not a failure.
            Err(error) if error.code() == errSecItemNotFound => {}
            Err(error) => return Err(storage_error(error)),
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn copy_service(_from: &str, _to: &str) -> Result<(), CredentialError> {
    Err(CredentialError::Unavailable {
        reason: "the Keychain is only available on macOS".to_owned(),
    })
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
    fn each_provider_has_its_own_account() {
        assert_eq!(account(ProviderId::Azure), "azure");
        assert_eq!(account(ProviderId::Groq), "groq");
        assert_eq!(account(ProviderId::MyMemory), "mymemory");
    }
}
