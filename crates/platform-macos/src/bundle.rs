//! The running bundle's identifier, and the places on disk derived from it.
//!
//! The identifier is authored once, in `packaging/Info.plist` — the artifact
//! macOS reads. The running app asks its own bundle through NSBundle, and the
//! plist's value is compiled in by `build.rs` as the fallback for executables
//! that have no bundle (`cargo test` runs a bare binary). Nothing here names
//! the identifier a second time.

use std::path::PathBuf;

/// The identifier read from `packaging/Info.plist` at compile time.
const PLIST_IDENTIFIER: &str = env!("TRANSLATOR_BUNDLE_ID");

/// The identity of this process: the bundle it runs from, or the plist's value
/// when it is not inside one.
pub(crate) fn identifier() -> String {
    runtime_identifier().unwrap_or_else(|| PLIST_IDENTIFIER.to_owned())
}

/// `~/Library/Application Support/<bundle identifier>`, where history and
/// settings live. Because it follows the identifier, renaming the bundle moves
/// both files to a new directory (docs/adr/0004-stable-signing-from-first-commit.md).
pub(crate) fn application_support_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join("Library/Application Support").join(identifier())
}

#[cfg(target_os = "macos")]
fn runtime_identifier() -> Option<String> {
    use std::ffi::{c_char, CStr};

    extern "C" {
        fn tl_bundle_identifier() -> *mut c_char;
        fn tl_string_free(text: *mut c_char);
    }

    // SAFETY: the bridge returns either NULL or a NUL-terminated string it
    // allocated, which `tl_string_free` releases exactly once.
    let text = unsafe { tl_bundle_identifier() };
    if text.is_null() {
        return None;
    }
    let identifier = unsafe { CStr::from_ptr(text) }
        .to_str()
        .ok()
        .map(str::to_owned);
    unsafe { tl_string_free(text) };
    identifier
}

#[cfg(not(target_os = "macos"))]
fn runtime_identifier() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_binary_without_a_bundle_falls_back_to_the_plist_identifier() {
        assert_eq!(runtime_identifier(), None);
        assert_eq!(identifier(), PLIST_IDENTIFIER);
        assert!(!PLIST_IDENTIFIER.is_empty());
    }

    #[test]
    fn application_support_lives_under_the_bundle_identifier() {
        assert!(application_support_dir().ends_with(identifier()));
    }
}
