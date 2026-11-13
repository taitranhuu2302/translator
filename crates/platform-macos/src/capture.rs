//! Safe wrapper over the Objective-C capture bridge in `macos/capture.m`.

use std::ffi::{c_char, CStr};

use translator_core::{CaptureError, Selection};

const NO_ACCESSIBILITY: i32 = 1;
const SECURE_INPUT: i32 = 2;
const NOTHING_SELECTED: i32 = 3;
/// What the bridge leaves behind when it fails for a reason of its own. It is
/// also the value `status` starts at, so a bridge that returned NULL without
/// writing cannot be mistaken for one of the typed failures above.
const UNAVAILABLE: i32 = 4;

extern "C" {
    fn tl_read_selection(status: *mut i32) -> *mut c_char;
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
