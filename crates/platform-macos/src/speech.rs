//! Safe wrapper over the Objective-C speech bridge in `macos/speech.m`.

use std::ffi::{c_char, CString};

use translator_core::{Language, SpeechError};

const SPEECH_OK: i32 = 0;
const NO_VOICE: i32 = 1;

extern "C" {
    fn tl_speak(text: *const c_char, language: *const c_char) -> i32;
}

pub fn speak(text: &str, language: &Language) -> Result<(), SpeechError> {
    let text = CString::new(text).map_err(|_| SpeechError::Unavailable {
        reason: "the text to speak contained a NUL byte".to_owned(),
    })?;
    let code = CString::new(language.code()).map_err(|_| SpeechError::Unavailable {
        reason: "the language code contained a NUL byte".to_owned(),
    })?;

    // SAFETY: both pointers are NUL-terminated and stay alive for the call; the
    // bridge copies what it needs before returning and then speaks on its own.
    let status = unsafe { tl_speak(text.as_ptr(), code.as_ptr()) };

    match status {
        SPEECH_OK => Ok(()),
        NO_VOICE => Err(SpeechError::NoVoice {
            language: language.clone(),
        }),
        _ => Err(SpeechError::Unavailable {
            reason: "the speech synthesizer could not be started".to_owned(),
        }),
    }
}
