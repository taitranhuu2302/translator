//! The capture port on platforms this milestone does not ship. Every path is a
//! typed failure rather than a silent success, exactly like the macOS stubs were.

use translator_core::{CaptureError, Selection};

fn unavailable() -> CaptureError {
    CaptureError::Unavailable {
        reason: "selection capture is implemented for macOS only in v1".to_owned(),
    }
}

pub fn read_selection() -> Result<Selection, CaptureError> {
    Err(unavailable())
}

pub fn accessibility_granted() -> bool {
    true
}

pub fn request_accessibility() -> bool {
    true
}

pub fn open_accessibility_settings() -> Result<(), CaptureError> {
    Err(unavailable())
}
