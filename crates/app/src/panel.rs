//! The Quick Translate popup: a Tauri window that becomes a non-activating
//! `NSPanel`, so it takes the keyboard without the app becoming active
//! (docs/adr/0005-non-activating-panel-popup.md).
//!
//! The window itself is declared in `tauri.conf.json` and converted once, at
//! startup. Every panel operation here must run on the main thread.

use tauri::{AppHandle, Manager, WebviewWindow};
// `tauri_panel!` brings `AnyObject` into scope for the generated panel type.
use tauri_nspanel::{
    objc2_app_kit::NSResponder, tauri_panel, CollectionBehavior, ManagerExt, Panel, PanelLevel,
    StyleMask, WebviewWindowExt,
};
use translator_platform_macos::pointer;

pub const POPUP_LABEL: &str = "popup";
/// Fired after the popup is shown, so an already-loaded page can claim the new
/// prefill. A page that is still loading claims it on mount instead.
pub const POPUP_PREFILL_EVENT: &str = "popup-prefill";

const POPUP_WIDTH: f64 = 420.0;
const POPUP_HEIGHT: f64 = 220.0;
/// Space between the cursor and the popup, in points.
const CURSOR_GAP: f64 = 10.0;

tauri_panel! {
    panel!(PopupPanel {
        config: {
            can_become_key_window: true,
            can_become_main_window: false,
            becomes_key_only_if_needed: false,
            is_floating_panel: true
        }
    })
}

/// Convert the config-declared window into the popup panel and leave it hidden.
/// Called once during setup, before any shortcut can fire.
pub fn prepare(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(POPUP_LABEL) else {
        return Ok(());
    };

    let panel = window.to_panel::<PopupPanel>()?;
    panel
        .add_style_mask(StyleMask::empty().nonactivating_panel().into())
        .map_err(|error| {
            tauri::Error::Io(std::io::Error::other(format!(
                "the popup's non-activating style mask was rejected: {error:?}"
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
    // Dismissal is ours to decide: losing key hides it, but deactivating the
    // whole app must not take it away mid-answer.
    panel.set_hides_on_deactivate(false);
    panel.hide();
    Ok(())
}

/// Show the panel at the cursor, sized and placed from the real screen it lands
/// on, then point the web view at the keyboard. Main thread only.
pub fn show(app: &AppHandle) -> tauri::Result<()> {
    let panel = app
        .get_webview_panel(POPUP_LABEL)
        .map_err(|_| not_prepared())?;
    let Some(window) = app.get_webview_window(POPUP_LABEL) else {
        return Err(not_prepared());
    };

    size_and_place(&window)?;
    panel.show_and_make_key();
    focus_webview(&window, panel.as_ref());
    Ok(())
}

/// Hide the panel. Main thread only.
pub fn hide(app: &AppHandle) {
    if let Ok(panel) = app.get_webview_panel(POPUP_LABEL) {
        panel.hide();
    }
}

fn not_prepared() -> tauri::Error {
    tauri::Error::Io(std::io::Error::other(
        "the Quick Translate panel was not prepared",
    ))
}

/// Set the size **first** and read it back, so the clamp and the bottom-edge
/// flip use the window's real dimensions rather than a guess.
///
/// `panel.to_window()` is not used: in `tauri-nspanel` 2.1.1 it converts the
/// panel *back* into a regular window and deregisters it. The Tauri window
/// handle wraps the same `NSWindow`, so sizing and moving through it leaves the
/// panel intact.
fn size_and_place(window: &WebviewWindow) -> tauri::Result<()> {
    window.set_size(tauri::LogicalSize::new(POPUP_WIDTH, POPUP_HEIGHT))?;

    // The window's current scale is the factor Tauri's macOS backend divides a
    // physical position by, so it is also the factor `to_physical` multiplies
    // with. Read it before moving the window to another screen.
    let scale = window.scale_factor()?;
    let measured = window.outer_size()?;
    let size = pointer::Size {
        width: if measured.width > 0 {
            measured.width as f64 / scale
        } else {
            POPUP_WIDTH
        },
        height: if measured.height > 0 {
            measured.height as f64 / scale
        } else {
            POPUP_HEIGHT
        },
    };

    let Some(screen) = pointer::screen_under_cursor() else {
        // No screens: leave the configured position rather than guessing.
        return Ok(());
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

/// `show_and_make_key` makes the panel's *container* first responder, which
/// swallows keystrokes. Point the panel at the real web view (ADR-0005).
fn focus_webview(window: &WebviewWindow, panel: &dyn Panel<tauri::Wry>) {
    let Ok(view) = window.ns_view() else {
        return;
    };
    let parent = view as *mut AnyObject;
    let target = find_webview(parent).unwrap_or(parent);
    // SAFETY: `target` is a live view owned by Tauri's webview hierarchy and
    // outlives this call.
    let responder = unsafe { &*(target as *const NSResponder) };
    panel.make_first_responder(Some(responder));
}

/// Wry wraps the `WKWebView` in a plain `WryWebViewParent`. A plain `NSView`
/// forwards `keyDown:` up the responder chain instead of down to its subviews,
/// so the web view itself has to be found.
fn find_webview(obj: *mut AnyObject) -> Option<*mut AnyObject> {
    use tauri_nspanel::objc2::msg_send;
    // SAFETY: every pointer here comes from AppKit and is only read.
    unsafe {
        let view = &*(obj as *const AnyObject);
        let subviews: *mut AnyObject = msg_send![view, subviews];
        if subviews.is_null() {
            return None;
        }
        let count: usize = msg_send![subviews, count];
        for index in 0..count {
            let sub: *mut AnyObject = msg_send![subviews, objectAtIndex: index];
            let name = (&*sub).class().name().to_string_lossy();
            if name.contains("WryWebView") || name.contains("WKWebView") {
                return Some(sub);
            }
            if let Some(found) = find_webview(sub) {
                return Some(found);
            }
        }
        None
    }
}
