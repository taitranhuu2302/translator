//! Speech on platforms this milestone does not ship: a typed failure rather than
//! a silent success, exactly like the capture fallback.

use translator_core::{Language, SpeechError};

pub fn speak(_text: &str, _language: &Language) -> Result<(), SpeechError> {
    Err(SpeechError::Unavailable {
        reason: "speech synthesis is implemented for macOS only in v1".to_owned(),
    })
}
