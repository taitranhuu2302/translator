//! The Replace HUD: the small capsule that says what `⌥⇧R` is doing while it
//! runs.
//!
//! It is the shell's second `NSPanel`. Unlike the popup it is never a surface
//! the user talks to: it never takes the keyboard, it ignores the mouse, and it
//! never activates the app. The popup is shown because the user pressed a
//! shortcut to see an answer; this is shown because a Replace is taking long
//! enough to be worth a word.
//!
//! It is armed at the start of a Replace but appears only if the Replace is
//! still running after [`SHOW_DELAY`]. Most Replacements — a cached pair, a
//! short sentence — finish inside that window, and a capsule that flashes for
//! one frame is worse than none.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tauri_nspanel::{
    tauri_panel, CollectionBehavior, ManagerExt, PanelLevel, StyleMask, WebviewWindowExt,
};
use translator_core::ReplacePhase;
use translator_platform_macos::pointer;

use crate::main_thread;

pub const HUD_LABEL: &str = "hud";
/// Fired for every step and again when the capsule appears, so an already
/// loaded page can follow the Replace. A page that is still loading claims the
/// current step through `replace_hud_phase` on mount instead.
pub const HUD_PHASE_EVENT: &str = "replace-hud-phase";

/// A Replace that finishes inside this never shows the HUD: a capsule that
/// blinks on a fast answer reads as a glitch, not as progress.
const SHOW_DELAY: Duration = Duration::from_millis(120);
const HUD_WIDTH: f64 = 260.0;
const HUD_HEIGHT: f64 = 52.0;
/// Space between the cursor and the capsule, in points. The same gesture as the
/// popup's: the thing that appeared belongs to the cursor that asked for it.
const CURSOR_GAP: f64 = 10.0;

tauri_panel! {
    panel!(ReplaceHudPanel {
        config: {
            can_become_key_window: false,
            can_become_main_window: false,
            becomes_key_only_if_needed: false,
            is_floating_panel: true
        }
    })
}

/// Convert the config-declared window into the HUD panel and leave it hidden.
/// Called once during setup, beside the popup's own preparation.
pub fn prepare(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(HUD_LABEL) else {
        return Ok(());
    };

    let panel = window.to_panel::<ReplaceHudPanel>()?;
    panel
        .add_style_mask(StyleMask::empty().nonactivating_panel().into())
        .map_err(|error| {
            tauri::Error::Io(std::io::Error::other(format!(
                "the Replace HUD's non-activating style mask was rejected: {error:?}"
            )))
        })?;
    panel.set_level(PanelLevel::Floating.value());
    panel.set_collection_behavior(
        CollectionBehavior::new()
            .can_join_all_spaces()
            .full_screen_auxiliary()
            .ignores_cycle()
            .value(),
    );
    panel.set_floating_panel(true);
    panel.set_becomes_key_only_if_needed(false);
    panel.set_hides_on_deactivate(false);
    // The capsule is decoration. A click where it sits belongs to the window
    // underneath, not to the HUD.
    panel.set_ignores_mouse_events(true);
    panel.hide();
    Ok(())
}

/// One Replace's HUD. Arming it is free; the delayed thread decides whether the
/// capsule is ever shown.
pub struct ReplaceHud {
    phase: Arc<Mutex<ReplacePhase>>,
    cancelled: Arc<AtomicBool>,
}

impl ReplaceHud {
    /// Start watching a Replace that has just begun on `phase`. The capsule
    /// appears after [`SHOW_DELAY`] unless [`finish`](Self::finish) came first.
    pub fn start(app: &AppHandle, phase: ReplacePhase) -> Self {
        let phase = Arc::new(Mutex::new(phase));
        let cancelled = Arc::new(AtomicBool::new(false));

        let show_app = app.clone();
        let show_phase = Arc::clone(&phase);
        let show_cancelled = Arc::clone(&cancelled);
        std::thread::spawn(move || {
            std::thread::sleep(SHOW_DELAY);
            if show_cancelled.load(Ordering::SeqCst) {
                return;
            }
            let _ = main_thread::on_main_thread(&show_app, move |handle| {
                // A Replace that ended while this was queued must not flash the
                // capsule on its way out.
                if show_cancelled.load(Ordering::SeqCst) {
                    return;
                }
                let phase = *show_phase.lock().expect("hud phase lock");
                show(handle, phase);
            });
        });

        Self { phase, cancelled }
    }

    /// Record the step the Replace has reached and tell a page that is already
    /// showing the capsule. Safe from the worker thread running the Replace.
    pub fn phase(&self, app: &AppHandle, phase: ReplacePhase) {
        *self.phase.lock().expect("hud phase lock") = phase;
        let _ = app.emit(HUD_PHASE_EVENT, phase);
    }

    /// The step the Replace is on now, for a page that claims it on mount.
    pub fn phase_now(&self) -> ReplacePhase {
        *self.phase.lock().expect("hud phase lock")
    }

    /// The Replace is over, whatever its outcome: stop a show that has not
    /// happened and take the capsule away. A failure is explained by the native
    /// dialog the caller shows next; the HUD never carries the bad news.
    ///
    /// It waits for the main thread, so it must not be called from it.
    pub fn finish(self, app: &AppHandle) {
        self.cancelled.store(true, Ordering::SeqCst);
        let _ = main_thread::on_main_thread(app, |handle| hide(handle));
    }
}

/// Show the capsule at the cursor. Main thread only.
fn show(app: &AppHandle, phase: ReplacePhase) {
    let Ok(panel) = app.get_webview_panel(HUD_LABEL) else {
        eprintln!("Neris: the Replace HUD panel was not prepared, so nothing was shown");
        return;
    };
    let Some(window) = app.get_webview_window(HUD_LABEL) else {
        return;
    };

    if let Some(screen) = pointer::screen_under_cursor() {
        if let Err(error) = place(&window, screen) {
            eprintln!("Neris: the Replace HUD could not be placed: {error}");
        }
    }
    // A loaded page updates from this event; the claim on mount covers the
    // first opening, when the page may still be starting.
    let _ = app.emit(HUD_PHASE_EVENT, phase);
    panel.show();
}

/// Hide the capsule. Main thread only.
fn hide(app: &AppHandle) {
    if let Ok(panel) = app.get_webview_panel(HUD_LABEL) {
        panel.hide();
    }
}

/// Put the window where `place_popup` says, using the size it actually has. The
/// same arithmetic as the popup's placement, so the two surfaces sit in the
/// same relation to the cursor.
fn place(window: &WebviewWindow, screen: pointer::ScreenUnderCursor) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    let measured = window.outer_size()?;
    let size = pointer::Size {
        width: if measured.width > 0 {
            measured.width as f64 / scale
        } else {
            HUD_WIDTH
        },
        height: if measured.height > 0 {
            measured.height as f64 / scale
        } else {
            HUD_HEIGHT
        },
    };

    let top_left = pointer::place_popup(
        screen.cursor,
        size,
        screen.visible,
        screen.desktop_height,
        CURSOR_GAP,
    );
    let physical = pointer::to_physical(
        top_left,
        screen.desktop_height,
        screen.desktop_pixels,
        scale,
    );
    window.set_position(tauri::PhysicalPosition::new(
        physical.x.round() as i32,
        physical.y.round() as i32,
    ))
}
