use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use tauri::{AppHandle, Manager, WebviewUrl};
use tauri_nspanel::{
    objc2_app_kit::NSResponder, tauri_panel, CollectionBehavior, ManagerExt, PanelBuilder,
    PanelLevel, StyleMask,
};

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

#[tauri::command]
fn log_key(key: String) {
    eprintln!("SPIKE_KEY {}", key);
}

#[tauri::command]
fn hide_panel(app: AppHandle) {
    if let Ok(panel) = app.get_webview_panel("popup") {
        panel.hide();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_nspanel::init())
        .invoke_handler(tauri::generate_handler![hide_panel, log_key])
        .setup(|app| {
            // No Dock icon: the app never activates itself, the panel does the work.
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let panel = PanelBuilder::<_, PopupPanel>::new(app.handle(), "popup")
                .url(WebviewUrl::App("index.html".into()))
                .with_window(|window| window.decorations(false))
                .add_style_mask(StyleMask::empty().nonactivating_panel())
                .no_activate(true)
                .level(PanelLevel::Floating)
                .collection_behavior(
                    CollectionBehavior::new()
                        .can_join_all_spaces()
                        .full_screen_auxiliary()
                        .ignores_cycle(),
                )
                .build()?;

            let manager = GlobalHotKeyManager::new()?;
            let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyD);
            manager.register(hotkey)?;
            // The manager must outlive the app; leaking it for the spike is fine.
            std::mem::forget(manager);

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let receiver = GlobalHotKeyEvent::receiver();
                while let Ok(event) = receiver.recv() {
                    if event.state == HotKeyState::Pressed {
                        let panel = panel.clone();
                        let app_handle = app_handle.clone();
                        let _ = app_handle.clone().run_on_main_thread(move || {
                            panel.show_and_make_key();
                            focus_webview(&app_handle, &panel);
                        });
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Finds the real web view inside Tauri's view hierarchy.
///
/// `ns_view` returns Wry's `WryWebViewParent`, a plain `NSView` that wraps the `WryWebView`
/// (a `WKWebView` subclass). A plain `NSView` forwards `keyDown:` up the responder chain rather
/// than down to its subviews, so making the parent first responder means the web content never
/// sees a keystroke — which is exactly what happened before this fix.
fn find_webview(obj: *mut AnyObject) -> Option<*mut AnyObject> {
    use tauri_nspanel::objc2::msg_send;
    unsafe {
        let view = &*(obj as *const AnyObject);
        let subviews: *mut AnyObject = msg_send![view, subviews];
        if subviews.is_null() {
            return None;
        }
        let count: usize = msg_send![subviews, count];
        for i in 0..count {
            let sub: *mut AnyObject = msg_send![subviews, objectAtIndex: i];
            let name = (&*sub).class().name().to_string_lossy().to_string();
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

/// `show_and_make_key` makes the panel's content view first responder, which is not enough:
/// the panel is key and the app stays inactive, but the keystroke dies in the container view.
/// Point the panel at the web view itself.
fn focus_webview(app: &AppHandle, panel: &std::sync::Arc<dyn tauri_nspanel::Panel>) {
    let Some(window) = app.get_webview_window("popup") else {
        eprintln!("SPIKE_FOCUS: no window");
        return;
    };
    let Ok(view) = window.ns_view() else {
        eprintln!("SPIKE_FOCUS: no ns_view");
        return;
    };
    let parent = view as *mut AnyObject;
    let target = find_webview(parent).unwrap_or(parent);
    // SAFETY: `target` is a live view owned by Tauri; it outlives this call.
    let responder = unsafe { &*(target as *const NSResponder) };
    let ok = panel.make_first_responder(Some(responder));
    let name = unsafe { (&*target).class().name().to_string_lossy().to_string() };
    eprintln!("SPIKE_FOCUS {ok} target={name}");
}
