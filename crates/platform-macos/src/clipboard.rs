//! The system pasteboard, for the popup's Copy button.

use std::fmt;

/// The write was refused, or this is not macOS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipboardError;

impl fmt::Display for ClipboardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the clipboard refused the text")
    }
}

impl std::error::Error for ClipboardError {}

/// Replace the system clipboard with `text` as plain text. A string containing
/// a NUL byte cannot cross the C bridge and is reported as a failure rather than
/// silently truncated.
pub fn write_text(text: &str) -> Result<(), ClipboardError> {
    let value = std::ffi::CString::new(text).map_err(|_| ClipboardError)?;

    #[cfg(target_os = "macos")]
    {
        unsafe extern "C" {
            fn tl_write_clipboard(text: *const std::ffi::c_char) -> bool;
        }
        // SAFETY: `value` is NUL-terminated and outlives the call.
        if unsafe { tl_write_clipboard(value.as_ptr()) } {
            Ok(())
        } else {
            Err(ClipboardError)
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = value;
        Err(ClipboardError)
    }
}
