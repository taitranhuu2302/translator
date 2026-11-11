//! The Tauri v2 shell: it builds the ports, owns the engine and exposes IPC.
//! Views, panels and menu items call these commands; they never reach a port
//! implementation directly.

use std::sync::Arc;

use tauri::State;
use translator_core::{
    EngineError, HistoryEntry, Mode, ProviderStatus, QuickTranslation, ReadAloud, Replacement,
    Settings, TextOutcome, TranslatorEngine, UnconfiguredProvider,
};
use translator_platform_macos::{
    MacCredentialStore, MacHistoryStore, MacSelectionCapture, MacSettingsStore, MacSpeechOutput,
};

struct AppState {
    engine: TranslatorEngine,
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
async fn save_settings(
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), EngineError> {
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

fn main() {
    let engine = TranslatorEngine::new(
        Arc::new(MacSelectionCapture),
        Arc::new(UnconfiguredProvider),
        Arc::new(MacSpeechOutput),
        Arc::new(MacHistoryStore),
        Arc::new(MacSettingsStore),
        Arc::new(MacCredentialStore),
    );

    tauri::Builder::default()
        .manage(AppState { engine })
        .invoke_handler(tauri::generate_handler![
            quick_translate,
            replace_selection,
            read_aloud,
            translate_text,
            provider_status,
            get_settings,
            save_settings,
            search_history
        ])
        .setup(|app| {
            // Menu-bar only: the app never takes a Dock icon or a place in ⌘Tab.
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Translator");
}
