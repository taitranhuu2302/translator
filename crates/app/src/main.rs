//! The Tauri v2 shell: it builds the ports, owns the engine and exposes IPC.
//! Views, panels and menu items call these commands; they never reach a port
//! implementation directly.

mod panel;

use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use translator_core::{
    CachedProvider, EngineError, HistoryEntry, Mode, MyMemoryProvider, PopupPrefill,
    ProviderLadder, ProviderStatus, QuickTranslation, ReadAloud, Replacement, Selection, Settings,
    SpeechError, TextOutcome, TranslatorEngine, UreqHttpClient,
};
use translator_platform_macos::{
    clipboard, MacCredentialStore, MacHistoryStore, MacSelectionCapture, MacSettingsStore,
    MacSpeechOutput,
};

/// Used when the settings file cannot be read, so the shortcuts still work.
const DEFAULT_QUICK_TRANSLATE_SHORTCUT: &str = "Ctrl+Alt+D";
const DEFAULT_TRANSLATE_SHORTCUT: &str = "Ctrl+Alt+E";
const DEFAULT_READ_ALOUD_SHORTCUT: &str = "Ctrl+Alt+S";

/// How many translations the in-memory cache remembers.
const TRANSLATION_CACHE_SIZE: usize = 100;

/// What a registered shortcut does. The one global handler routes on the
/// shortcut's identity because one plugin owns every binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Route {
    QuickTranslate,
    TranslateWindow,
    ReadAloud,
}

/// The shortcuts that registered successfully. A binding that could not be
/// registered is absent, so pressing it does nothing instead of triggering the
/// wrong action.
#[derive(Clone, Copy, Default)]
struct ShortcutRoutes {
    quick_translate: Option<Shortcut>,
    translate_window: Option<Shortcut>,
    read_aloud: Option<Shortcut>,
}

impl ShortcutRoutes {
    fn route(&self, shortcut: &Shortcut) -> Option<Route> {
        if is_the_same_binding(self.read_aloud, shortcut) {
            Some(Route::ReadAloud)
        } else if is_the_same_binding(self.translate_window, shortcut) {
            Some(Route::TranslateWindow)
        } else if is_the_same_binding(self.quick_translate, shortcut) {
            Some(Route::QuickTranslate)
        } else {
            None
        }
    }
}

/// Compare the key combination, not the plugin's registration id: the id is a
/// per-process handle, so two bindings parsed from the same string differ in it.
fn is_the_same_binding(registered: Option<Shortcut>, shortcut: &Shortcut) -> bool {
    registered.is_some_and(|registered| {
        registered.mods == shortcut.mods && registered.key == shortcut.key
    })
}

struct AppState {
    engine: TranslatorEngine,
    /// The Selection captured when the window was opened, waiting for the UI to
    /// take it. Capture happens before the window is shown, while the app the
    /// user was typing in still has focus.
    prefill: Mutex<Option<Prefill>>,
    /// The same idea for Quick Translate: captured before the popup is shown.
    popup_prefill: Mutex<Option<PopupPrefill>>,
    routes: Mutex<ShortcutRoutes>,
}

impl AppState {
    fn put_prefill(&self, prefill: Prefill) {
        *self.prefill.lock().expect("prefill lock") = Some(prefill);
    }

    fn take_prefill(&self) -> Option<Prefill> {
        self.prefill.lock().expect("prefill lock").take()
    }

    fn put_popup_prefill(&self, prefill: PopupPrefill) {
        *self.popup_prefill.lock().expect("popup prefill lock") = Some(prefill);
    }

    fn take_popup_prefill(&self) -> Option<PopupPrefill> {
        self.popup_prefill.lock().expect("popup prefill lock").take()
    }

    fn route(&self, shortcut: &Shortcut) -> Option<Route> {
        self.routes
            .lock()
            .expect("shortcut routes lock")
            .route(shortcut)
    }
}

/// What the Translate Window should start with: the Selection, or the typed
/// reason there is none. A missing Selection arrives as an empty `Selection`,
/// never as a failure.
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Prefill {
    Selection(Selection),
    Failure(EngineError),
}

// Every command is `async` so Tauri runs it on its worker threads. A synchronous
// command runs on the main thread, where an Accessibility capture or a provider
// request would freeze the webview and the run loop for its whole duration.

#[tauri::command]
async fn quick_translate(state: State<'_, AppState>) -> Result<QuickTranslation, EngineError> {
    state.engine.quick_translate()
}

#[tauri::command]
async fn replace_selection(state: State<'_, AppState>) -> Result<Replacement, EngineError> {
    state.engine.replace_selection()
}

#[tauri::command]
async fn read_aloud(state: State<'_, AppState>) -> Result<ReadAloud, EngineError> {
    state.engine.read_aloud()
}

#[tauri::command]
async fn translate_text(
    state: State<'_, AppState>,
    text: String,
    mode: Mode,
) -> Result<TextOutcome, EngineError> {
    state.engine.translate_text(&text, mode)
}

#[tauri::command]
async fn provider_status(state: State<'_, AppState>) -> Result<ProviderStatus, EngineError> {
    state.engine.provider_status()
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<Settings, EngineError> {
    state.engine.settings()
}

#[tauri::command]
async fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), EngineError> {
    state.engine.save_settings(settings)
}

#[tauri::command]
async fn search_history(
    state: State<'_, AppState>,
    query: String,
    limit: u32,
) -> Result<Vec<HistoryEntry>, EngineError> {
    state.engine.history(&query, limit as usize)
}

#[tauri::command]
async fn take_selection_prefill(state: State<'_, AppState>) -> Result<Option<Prefill>, ()> {
    Ok(state.take_prefill())
}

/// The Selection captured before the popup was shown, or the typed reason there
/// is none. Claiming it answers `None` until the next Quick Translate.
#[tauri::command]
async fn take_popup_prefill(state: State<'_, AppState>) -> Result<Option<PopupPrefill>, ()> {
    Ok(state.take_popup_prefill())
}

/// Dismiss the popup. Panel operations must run on the main thread.
#[tauri::command]
async fn hide_popup(app: AppHandle) -> Result<(), ()> {
    let handle = app.clone();
    app.run_on_main_thread(move || panel::hide(&handle))
        .map_err(|_| ())
}

/// The popup's Copy button. Writing through AppKit does not depend on the web
/// view's clipboard permission.
#[tauri::command]
async fn copy_text(text: String) -> Result<(), ()> {
    clipboard::write_text(&text).map_err(|_| ())
}

#[tauri::command]
async fn accessibility_granted(state: State<'_, AppState>) -> Result<bool, ()> {
    Ok(state.engine.accessibility_granted())
}

#[tauri::command]
async fn request_accessibility(state: State<'_, AppState>) -> Result<bool, ()> {
    Ok(state.engine.request_accessibility())
}

#[tauri::command]
async fn open_accessibility_settings(state: State<'_, AppState>) -> Result<(), EngineError> {
    state.engine.open_accessibility_settings()
}

#[tauri::command]
async fn complete_onboarding(state: State<'_, AppState>) -> Result<Settings, EngineError> {
    state.engine.complete_onboarding()
}

/// Quick Translate: capture the Selection on a worker thread **before** the
/// panel is shown, while the source app still owns focus, then position and show
/// the panel on the main thread. Showing first would read this app instead.
fn open_quick_translate(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let prefill = app.state::<AppState>().engine.popup_prefill();
        app.state::<AppState>().put_popup_prefill(prefill);

        let handle = app.clone();
        if let Err(error) = app.run_on_main_thread(move || {
            if let Err(error) = panel::show(&handle) {
                eprintln!("Translator: the Quick Translate popup could not open: {error}");
                return;
            }
            let _ = handle.emit_to(panel::POPUP_LABEL, panel::POPUP_PREFILL_EVENT, ());
        }) {
            eprintln!("Translator: the Quick Translate popup could not be scheduled: {error}");
        }
    });
}

/// Capture the Selection now, while the app the user is working in is still
/// focused, then open the Translate Window with it. Capturing after the window is
/// shown would read this app instead of the source app.
fn open_translate_window(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let prefill = {
            let state = app.state::<AppState>();
            match state.engine.selection_for_window() {
                Ok(selection) => Prefill::Selection(selection),
                Err(error) => Prefill::Failure(error),
            }
        };
        app.state::<AppState>().put_prefill(prefill);

        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
        let _ = app.emit("selection-prefill", ());
    });
}

/// Read Aloud: capture and speak off the main thread, so a slow capture or a long
/// utterance never delays the next shortcut. Nothing is shown when nothing was
/// selected — the shortcut simply stays quiet — but a typed failure gets a dialog,
/// because speech that failed silently would look like the shortcut never fired.
fn read_aloud_selection(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || match app.state::<AppState>().engine.read_aloud() {
        Ok(outcome) if outcome.spoken_text.is_empty() => {}
        Ok(_) => {}
        Err(error) => show_read_aloud_failure(&app, &error),
    });
}

fn show_read_aloud_failure(app: &AppHandle, error: &EngineError) {
    eprintln!("Translator: Read Aloud failed: {error}");
    let message = match error {
        EngineError::Speech(SpeechError::NoVoice { language }) => format!(
            "No voice is installed for {language}. Add one in System Settings → \
             Accessibility → Spoken Content → System Voice → Manage Voices."
        ),
        other => other.to_string(),
    };
    app.dialog()
        .message(message)
        .title("Read Aloud")
        .kind(MessageDialogKind::Error)
        .show(|_| {});
}

/// Register one binding and answer the shortcut that is now live, or `None` when
/// the binding is invalid or another app owns it. A conflict is reported rather
/// than silently dropped (ticket 10 surfaces it in the settings UI).
fn register_shortcut(app: &AppHandle, binding: &str) -> Option<Shortcut> {
    match binding.parse::<Shortcut>() {
        Ok(shortcut) => match app.global_shortcut().register(shortcut) {
            Ok(()) => Some(shortcut),
            Err(error) => {
                eprintln!("Translator: could not register {binding}: {error}");
                None
            }
        },
        Err(error) => {
            eprintln!("Translator: invalid shortcut binding {binding}: {error}");
            None
        }
    }
}

fn build_tray(app: &AppHandle, accelerator: &str) -> tauri::Result<()> {
    let open = MenuItem::with_id(
        app,
        "open",
        "Open Translate Window",
        true,
        Some(accelerator),
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit Translator", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &open,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    // The 1024 px app icon is scaled to the menu bar by the tray implementation.
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;
    TrayIconBuilder::with_id("translator")
        .tooltip("Translator")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => open_translate_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn main() {
    // The keyless provider answers before any key is configured; the cache in
    // front of the ladder makes a repeated Selection free.
    let http = Arc::new(UreqHttpClient::default());
    let my_memory = Arc::new(MyMemoryProvider::new(http));
    let ladder = Arc::new(ProviderLadder::single(my_memory));
    let provider = Arc::new(CachedProvider::new(ladder, TRANSLATION_CACHE_SIZE));

    let engine = TranslatorEngine::new(
        Arc::new(MacSelectionCapture),
        provider,
        Arc::new(MacSpeechOutput),
        Arc::new(MacHistoryStore),
        Arc::new(MacSettingsStore::default()),
        Arc::new(MacCredentialStore),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_nspanel::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    match app.state::<AppState>().route(shortcut) {
                        Some(Route::QuickTranslate) => open_quick_translate(app),
                        Some(Route::TranslateWindow) => open_translate_window(app),
                        Some(Route::ReadAloud) => read_aloud_selection(app),
                        None => {}
                    }
                })
                .build(),
        )
        .manage(AppState {
            engine,
            prefill: Mutex::new(None),
            popup_prefill: Mutex::new(None),
            routes: Mutex::new(ShortcutRoutes::default()),
        })
        .invoke_handler(tauri::generate_handler![
            quick_translate,
            replace_selection,
            read_aloud,
            translate_text,
            provider_status,
            get_settings,
            save_settings,
            search_history,
            take_selection_prefill,
            take_popup_prefill,
            hide_popup,
            copy_text,
            accessibility_granted,
            request_accessibility,
            open_accessibility_settings,
            complete_onboarding
        ])
        .setup(|app| {
            // Menu-bar only: the app never takes a Dock icon or a place in ⌘Tab.
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let settings = app.state::<AppState>().engine.settings();
            let quick_translate_binding = settings
                .as_ref()
                .map(|settings| settings.shortcuts.quick_translate.clone())
                .unwrap_or_else(|_| DEFAULT_QUICK_TRANSLATE_SHORTCUT.to_owned());
            let translate_binding = settings
                .as_ref()
                .map(|settings| settings.shortcuts.translate_window.clone())
                .unwrap_or_else(|_| DEFAULT_TRANSLATE_SHORTCUT.to_owned());
            let read_aloud_binding = settings
                .as_ref()
                .map(|settings| settings.shortcuts.read_aloud.clone())
                .unwrap_or_else(|_| DEFAULT_READ_ALOUD_SHORTCUT.to_owned());

            build_tray(app.handle(), &translate_binding)?;

            // The popup window exists from startup (hidden) and is converted
            // once. Losing key is the dismissal gesture: clicking anywhere else
            // closes it, and typing returns to the source app.
            panel::prepare(app.handle())?;
            if let Some(window) = app.get_webview_window(panel::POPUP_LABEL) {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if matches!(event, tauri::WindowEvent::Focused(false)) {
                        panel::hide(&handle);
                    }
                });
            }

            let routes = ShortcutRoutes {
                quick_translate: register_shortcut(app.handle(), &quick_translate_binding),
                translate_window: register_shortcut(app.handle(), &translate_binding),
                read_aloud: register_shortcut(app.handle(), &read_aloud_binding),
            };
            *app.state::<AppState>()
                .routes
                .lock()
                .expect("shortcut routes lock") = routes;

            // The onboarding is the first thing the app has to say, so the first
            // launch opens the Translate Window to say it.
            let first_launch = settings
                .map(|settings| !settings.onboarding_seen)
                .unwrap_or(true);
            if first_launch {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Translator");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shortcut(binding: &str) -> Shortcut {
        binding.parse().expect("a valid shortcut binding")
    }

    #[test]
    fn each_registered_shortcut_routes_to_its_own_action() {
        let routes = ShortcutRoutes {
            quick_translate: Some(shortcut("Ctrl+Alt+D")),
            translate_window: Some(shortcut("Ctrl+Alt+E")),
            read_aloud: Some(shortcut("Ctrl+Alt+S")),
        };

        assert_eq!(
            routes.route(&shortcut("Ctrl+Alt+D")),
            Some(Route::QuickTranslate)
        );
        assert_eq!(
            routes.route(&shortcut("Ctrl+Alt+E")),
            Some(Route::TranslateWindow)
        );
        assert_eq!(
            routes.route(&shortcut("Ctrl+Alt+S")),
            Some(Route::ReadAloud)
        );
    }

    #[test]
    fn a_shortcut_that_did_not_register_routes_nowhere() {
        let routes = ShortcutRoutes {
            quick_translate: None,
            translate_window: Some(shortcut("Ctrl+Alt+E")),
            read_aloud: None,
        };

        assert_eq!(routes.route(&shortcut("Ctrl+Alt+S")), None);
        assert_eq!(routes.route(&shortcut("Ctrl+Alt+D")), None);
    }
}
