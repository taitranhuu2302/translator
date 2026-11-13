//! The Tauri v2 shell: it builds the ports, owns the engine and exposes IPC.
//! Views, panels and menu items call these commands; they never reach a port
//! implementation directly.

use std::sync::{Arc, Mutex};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use translator_core::{
    EngineError, HistoryEntry, Mode, ProviderStatus, QuickTranslation, ReadAloud, Replacement,
    Selection, Settings, TextOutcome, TranslatorEngine, UnconfiguredProvider,
};
use translator_platform_macos::{
    MacCredentialStore, MacHistoryStore, MacSelectionCapture, MacSettingsStore, MacSpeechOutput,
};

/// Used when the settings file cannot be read, so the window can still be opened.
const DEFAULT_TRANSLATE_SHORTCUT: &str = "Ctrl+Alt+E";

struct AppState {
    engine: TranslatorEngine,
    /// The Selection captured when the window was opened, waiting for the UI to
    /// take it. Capture happens before the window is shown, while the app the
    /// user was typing in still has focus.
    prefill: Mutex<Option<Prefill>>,
}

impl AppState {
    fn put_prefill(&self, prefill: Prefill) {
        *self.prefill.lock().expect("prefill lock") = Some(prefill);
    }

    fn take_prefill(&self) -> Option<Prefill> {
        self.prefill.lock().expect("prefill lock").take()
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
    let engine = TranslatorEngine::new(
        Arc::new(MacSelectionCapture),
        Arc::new(UnconfiguredProvider),
        Arc::new(MacSpeechOutput),
        Arc::new(MacHistoryStore),
        Arc::new(MacSettingsStore::default()),
        Arc::new(MacCredentialStore),
    );

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        open_translate_window(app);
                    }
                })
                .build(),
        )
        .manage(AppState {
            engine,
            prefill: Mutex::new(None),
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
            accessibility_granted,
            request_accessibility,
            open_accessibility_settings,
            complete_onboarding
        ])
        .setup(|app| {
            // Menu-bar only: the app never takes a Dock icon or a place in ⌘Tab.
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let settings = app.state::<AppState>().engine.settings();
            let binding = settings
                .as_ref()
                .map(|settings| settings.shortcuts.translate_window.clone())
                .unwrap_or_else(|_| DEFAULT_TRANSLATE_SHORTCUT.to_owned());

            build_tray(app.handle(), &binding)?;

            match binding.parse::<tauri_plugin_global_shortcut::Shortcut>() {
                Ok(shortcut) => {
                    if let Err(error) = app.global_shortcut().register(shortcut) {
                        eprintln!("Translator: could not register {binding}: {error}");
                    }
                }
                Err(error) => eprintln!("Translator: invalid shortcut binding {binding}: {error}"),
            }

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
