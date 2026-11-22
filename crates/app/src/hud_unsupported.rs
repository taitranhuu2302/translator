//! The Replace HUD where AppKit is not available.
//!
//! On macOS the HUD is the shell's second `NSPanel` (`hud.rs`). Replace itself
//! is macOS-only until the port, so there is never a step worth showing here:
//! this keeps the same surface with the capsule left out, and still remembers
//! the phase so the HUD page's claim on mount answers something sensible.

use std::sync::Mutex;

use tauri::AppHandle;
use translator_core::ReplacePhase;

/// Nothing to convert: the window stays hidden and is never a panel.
pub fn prepare(_app: &AppHandle) -> tauri::Result<()> {
    Ok(())
}

/// The same type the shell holds for one Replace, without a thread or a window.
pub struct ReplaceHud {
    phase: Mutex<ReplacePhase>,
}

impl ReplaceHud {
    pub fn start(_app: &AppHandle, phase: ReplacePhase) -> Self {
        Self {
            phase: Mutex::new(phase),
        }
    }

    pub fn phase(&self, _app: &AppHandle, phase: ReplacePhase) {
        *self.phase.lock().expect("hud phase lock") = phase;
    }

    pub fn phase_now(&self) -> ReplacePhase {
        *self.phase.lock().expect("hud phase lock")
    }

    pub fn finish(self, _app: &AppHandle) {}
}
