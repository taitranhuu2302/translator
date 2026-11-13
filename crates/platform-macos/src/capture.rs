//! Safe wrapper over the Objective-C capture bridge in `macos/capture.m`.

use std::ffi::{c_char, CStr, CString};

use translator_core::{utf16_length, CaptureError, Selection};

const NO_ACCESSIBILITY: i32 = 1;
const SECURE_INPUT: i32 = 2;
const NOTHING_SELECTED: i32 = 3;
/// What the bridge leaves behind when it fails for a reason of its own. It is
/// also the value `status` starts at, so a bridge that returned NULL without
/// writing cannot be mistaken for one of the typed failures above.
const UNAVAILABLE: i32 = 4;
const NOT_WRITABLE: i32 = 5;
const NOTHING_TO_REPLACE: i32 = 6;

extern "C" {
    fn tl_read_selection(status: *mut i32) -> *mut c_char;
    fn tl_replace_selection(text: *const c_char, utf16_length: usize, status: *mut i32) -> bool;
    fn tl_accessibility_granted() -> bool;
    fn tl_request_accessibility() -> bool;
    fn tl_open_accessibility_settings() -> bool;
    fn tl_string_free(text: *mut c_char);
}

pub fn read_selection() -> Result<Selection, CaptureError> {
    let mut status = UNAVAILABLE;
    // SAFETY: the bridge always writes `status` and returns either NULL or a
    // NUL-terminated string it allocated, which `tl_string_free` releases.
    let text = unsafe { tl_read_selection(&mut status) };
    if text.is_null() {
        return Err(match status {
            NO_ACCESSIBILITY => CaptureError::NoAccessibility,
            SECURE_INPUT => CaptureError::SecureInput,
            NOTHING_SELECTED => CaptureError::NothingSelected,
            _ => CaptureError::Unavailable {
                reason: "the Accessibility API returned no selection".to_owned(),
            },
        });
    }
    let selection = unsafe { CStr::from_ptr(text) }
        .to_str()
        .map(Selection::new)
        .map_err(|_| CaptureError::Unavailable {
            reason: "the selected text was not valid UTF-8".to_owned(),
        });
    // SAFETY: the pointer came from `tl_read_selection` and is freed exactly once.
    unsafe { tl_string_free(text) };
    selection
}

/// Overwrite the retained Selection with `text`. The bridge attempts the
/// Accessibility write and proves it by the element's text changing — the
/// settable flag and the return code prove nothing — and otherwise pastes with a
/// synthesized ⌘V. The length it passes is UTF-16 code units, the unit a text
/// range uses — not UTF-8 bytes, which would leave the Selection extending past
/// a Vietnamese translation.
pub fn replace_selection(text: &str) -> Result<(), CaptureError> {
    let value = CString::new(text).map_err(|_| CaptureError::Unavailable {
        reason: "the translation contained a NUL byte".to_owned(),
    })?;
    let length = utf16_length(text);
    let mut status = UNAVAILABLE;
    // SAFETY: `value` is NUL-terminated and outlives the call; the bridge writes
    // `status` on every path.
    let replaced = unsafe { tl_replace_selection(value.as_ptr(), length, &mut status) };
    if replaced {
        Ok(())
    } else {
        Err(match status {
            NO_ACCESSIBILITY => CaptureError::NoAccessibility,
            SECURE_INPUT => CaptureError::SecureInput,
            NOTHING_SELECTED => CaptureError::NothingSelected,
            NOT_WRITABLE => CaptureError::NotWritable,
            NOTHING_TO_REPLACE => CaptureError::NothingToReplace,
            _ => CaptureError::Unavailable {
                reason: "the Accessibility API refused the replacement".to_owned(),
            },
        })
    }
}

pub fn accessibility_granted() -> bool {
    unsafe { tl_accessibility_granted() }
}

pub fn request_accessibility() -> bool {
    unsafe { tl_request_accessibility() }
}

pub fn open_accessibility_settings() -> Result<(), CaptureError> {
    if unsafe { tl_open_accessibility_settings() } {
        Ok(())
    } else {
        Err(CaptureError::Unavailable {
            reason: "System Settings could not be opened".to_owned(),
        })
    }
}
