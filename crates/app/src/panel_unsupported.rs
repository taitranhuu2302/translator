//! The Quick Translate popup where AppKit is not available.
//!
//! On macOS the declared window is converted into a non-activating `NSPanel`
//! (`panel.rs`, ADR-0005). No other platform has an `NSPanel`, so this module
//! keeps the same surface — a window that shows, resizes and hides — without
//! the panel behaviour: here the popup is an ordinary window that takes focus
//! when it opens. Making it non-activating, and placing it under the cursor, is
//! part of the port (`.scratch/windows-port/spec.md`), not of the build.

use tauri::{AppHandle, Manager};

pub const POPUP_LABEL: &str = "popup";
/// Fired after the popup is shown, so an already-loaded page can claim the new
/// prefill. A page that is still loading claims it on mount instead.
pub const POPUP_PREFILL_EVENT: &str = "popup-prefill";

const POPUP_WIDTH: f64 = 420.0;
/// The page reports its own content height; these bound what it may ask for so
/// a bad measurement cannot make the window unusable.
const MIN_POPUP_HEIGHT: f64 = 120.0;
const MAX_POPUP_HEIGHT: f64 = 600.0;

/// Nothing to convert: the window is already an ordinary one.
pub fn prepare(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

pub fn show(app: &AppHandle) -> tauri::Result<()> {
    let window = popup(app)?;
    window.show()?;
    window.set_focus()
}

pub fn set_height(app: &AppHandle, content_height: f64) -> tauri::Result<()> {
    popup(app)?.set_size(tauri::LogicalSize::new(
        POPUP_WIDTH,
        content_height.clamp(MIN_POPUP_HEIGHT, MAX_POPUP_HEIGHT),
    ))
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(POPUP_LABEL) {
        let _ = window.hide();
    }
}

fn popup(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    app.get_webview_window(POPUP_LABEL).ok_or_else(|| {
        tauri::Error::Io(std::io::Error::other(
            "the Quick Translate window is missing from the configuration",
        ))
    })
}
