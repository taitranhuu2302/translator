//! The Tauri v2 shell: it builds the ports, owns the engine and exposes IPC.
//! Views, panels and menu items call these commands; they never reach a port
//! implementation directly.

mod frame;
mod main_thread;
mod panel;
mod shortcuts;
mod tray;

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use frame::{Frame, FrameStore, ScreenRect};
use serde::Serialize;
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};
use translator_core::{
    AzureProvider, CachedProvider, CaptureError, Direction, EngineError, GroqProvider,
    HistoryEntry, ImproveDelta, ImprovePart, Mode, MyMemoryProvider, PopupPrefill, ProviderId,
    ProviderLadder, ProviderStatus, QuickTranslation, ReadAloud, Replacement, Selection, Settings,
    SettingsStore, ShortcutBindings, SpeechError, TextOutcome, Translation, TranslatorEngine,
    UreqHttpClient,
};
use translator_platform_macos::{
    clipboard, MacCredentialStore, MacHistoryStore, MacSelectionCapture, MacSettingsStore,
    MacSpeechOutput,
};

/// How many translations the in-memory cache remembers.
const TRANSLATION_CACHE_SIZE: usize = 100;

/// How long a panel operation waits for the main thread before giving up.
const PANEL_TIMEOUT: Duration = Duration::from_secs(1);

struct AppState {
    /// Shared so a command can hold it across an awaits-free worker hop while
    /// the borrow of `State` ends.
    engine: Arc<TranslatorEngine>,
    /// The Selection captured when the window was opened, waiting for the UI to
    /// take it. Capture happens before the window is shown, while the app the
    /// user was typing in still has focus.
    prefill: Mutex<Option<Prefill>>,
    /// The same idea for Quick Translate: captured before the popup is shown.
    popup_prefill: Mutex<Option<PopupPrefill>>,
    routes: Mutex<shortcuts::ShortcutRoutes>,
    /// The shortcuts macOS refused at launch, as sentences Settings shows until
    /// that action is rebound or the shortcuts are reset.
    launch_conflicts: Mutex<Vec<shortcuts::ShortcutStatus>>,
    /// The Accessibility permission as last observed, so the tray is only
    /// rebuilt when it actually changes rather than on every poll.
    accessibility: Mutex<bool>,
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
        self.popup_prefill
            .lock()
            .expect("popup prefill lock")
            .take()
    }

    fn route(&self, shortcut: &Shortcut) -> Option<shortcuts::ShortcutAction> {
        self.routes
            .lock()
            .expect("shortcut routes lock")
            .route(shortcut)
    }

    fn route_for(&self, action: shortcuts::ShortcutAction) -> Option<Shortcut> {
        self.routes
            .lock()
            .expect("shortcut routes lock")
            .get(action)
    }

    fn set_route(&self, action: shortcuts::ShortcutAction, shortcut: Option<Shortcut>) {
        self.routes
            .lock()
            .expect("shortcut routes lock")
            .set(action, shortcut);
    }

    fn conflicts(&self) -> Vec<shortcuts::ShortcutStatus> {
        self.launch_conflicts
            .lock()
            .expect("shortcut conflicts lock")
            .clone()
    }

    fn forget_conflict(&self, action: shortcuts::ShortcutAction) {
        shortcuts::forget_conflict(
            &mut self
                .launch_conflicts
                .lock()
                .expect("shortcut conflicts lock"),
            action,
        );
    }

    fn clear_conflicts(&self) {
        self.launch_conflicts
            .lock()
            .expect("shortcut conflicts lock")
            .clear();
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

/// One streamed piece of a result. Translation and Improve share one event
/// shape so the view listens once; only Improve sets `part`, and its absence
/// means the whole chunk belongs to the single translation pane.
#[derive(Clone, serde::Serialize)]
struct TranslationChunkPayload {
    request_id: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    part: Option<ImprovePart>,
}

#[derive(Clone, serde::Serialize)]
struct TranslationDonePayload {
    request_id: String,
    outcome: TextOutcome,
}

#[derive(Clone, serde::Serialize)]
struct TranslationErrorPayload {
    request_id: String,
    error: EngineError,
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

/// The popup's Enter: Replace the Selection with the translation already on
/// screen, without capturing or translating again.
///
/// The panel holds the keyboard while it is shown, so the clipboard fallback's
/// synthesized ⌘V would land in our own page. It is hidden first, and shown
/// again if the write fails so the popup can explain the failure inline.
#[tauri::command]
async fn replace_with_translation(
    app: AppHandle,
    state: State<'_, AppState>,
    source_text: String,
    translation: Translation,
) -> Result<Replacement, EngineError> {
    let engine = Arc::clone(&state.engine);

    main_thread::on_main_thread(&app, PANEL_TIMEOUT, panel::hide);
    match engine.replace_with_translation(&source_text, translation) {
        Ok(replacement) => Ok(replacement),
        Err(error) => {
            main_thread::on_main_thread(&app, PANEL_TIMEOUT, move |handle| {
                if let Err(show_error) = panel::show(handle) {
                    eprintln!(
                        "Translator: the popup could not be shown after a failed Replace: {show_error}"
                    );
                }
            });
            Err(error)
        }
    }
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
    direction: Option<Direction>,
) -> Result<TextOutcome, EngineError> {
    state.engine.translate_text_in(&text, mode, direction)
}

/// The Translate Window's Improve mode: corrected and better, streamed into the
/// pane as the model writes them. The request runs on its own thread (a model
/// answer is slow and blocking), and every event carries the request id the view
/// chose, so a stream from a superseded request can be ignored instead of
/// landing in the wrong pane.
///
/// Chunks and done both carry the model's fields: the view appends chunks for
/// progress, then replaces both fields from `done`, which is the only value the
/// provider strictly validated.
#[tauri::command]
async fn improve_text(
    app: AppHandle,
    state: State<'_, AppState>,
    request_id: String,
    text: String,
) -> Result<(), ()> {
    let engine = Arc::clone(&state.engine);
    std::thread::spawn(move || {
        let chunk_app = app.clone();
        let chunk_request = request_id.clone();
        let mut sink = move |delta: &ImproveDelta| {
            let _ = chunk_app.emit(
                "translation-chunk",
                TranslationChunkPayload {
                    request_id: chunk_request.clone(),
                    text: delta.text.clone(),
                    part: Some(delta.part),
                },
            );
        };
        match engine.improve_streaming(&text, &mut sink) {
            Ok(improvement) => {
                let _ = app.emit(
                    "translation-done",
                    TranslationDonePayload {
                        request_id,
                        outcome: TextOutcome::Improvement(improvement),
                    },
                );
            }
            Err(error) => {
                let _ = app.emit(
                    "translation-error",
                    TranslationErrorPayload { request_id, error },
                );
            }
        }
    });
    Ok(())
}

#[tauri::command]
async fn provider_status(state: State<'_, AppState>) -> Result<ProviderStatus, EngineError> {
    state.engine.provider_status()
}

/// Save one provider's API key in the Keychain. The key is passed straight to the
/// engine and never echoed, logged or written to the settings file. An empty key
/// clears it.
#[tauri::command]
async fn save_provider_key(
    state: State<'_, AppState>,
    provider: ProviderId,
    key: String,
) -> Result<ProviderStatus, EngineError> {
    state.engine.save_provider_key(provider, &key)
}

#[tauri::command]
async fn clear_provider_key(
    state: State<'_, AppState>,
    provider: ProviderId,
) -> Result<ProviderStatus, EngineError> {
    state.engine.clear_provider_key(provider)
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

/// Shape a stored history entry for the Translate Window again. It reads no
/// provider and writes no row: reopening yesterday's translation spends no
/// quota and does not duplicate it.
#[tauri::command]
async fn reopen_history_entry(
    state: State<'_, AppState>,
    entry: HistoryEntry,
) -> Result<TextOutcome, EngineError> {
    state.engine.reopen(&entry)
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

/// The Accessibility permission, as observed now. A change rebuilds the tray
/// menu, whose accessibility row would otherwise go stale while the app runs.
#[tauri::command]
async fn accessibility_granted(app: AppHandle, state: State<'_, AppState>) -> Result<bool, ()> {
    let granted = state.engine.accessibility_granted();
    let changed = {
        let mut observed = state.accessibility.lock().expect("accessibility lock");
        let changed = *observed != granted;
        *observed = granted;
        changed
    };
    if changed {
        let bindings = state
            .engine
            .settings()
            .map(|settings| settings.shortcuts)
            .unwrap_or_default();
        tray::refresh_tray(&app, &bindings);
    }
    Ok(granted)
}

/// The shortcuts macOS refused at launch. The Settings surface shows each one
/// until a rebind of that action or a reset clears it.
#[tauri::command]
async fn shortcut_conflicts(
    state: State<'_, AppState>,
) -> Result<Vec<shortcuts::ShortcutStatus>, ()> {
    Ok(state.conflicts())
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

/// Where the remembered frame lives: its own file beside the settings document,
/// never inside it. `None` when the app data directory is unavailable, in which
/// case the window simply keeps the default frame.
fn frame_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|directory| directory.join("window-frame.json"))
}

fn frame_of(window: &tauri::WebviewWindow) -> Option<Frame> {
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;
    Some(Frame {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

/// Put the window back where it was, before it is shown for the first time.
/// A frame that no longer touches a connected display is dropped, so unplugging
/// a monitor can never strand the window off screen.
fn restore_frame(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Some(path) = frame_path(app) else {
        return;
    };
    let Some(frame) = FrameStore::at(path).load() else {
        return;
    };

    let screens: Vec<ScreenRect> = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            ScreenRect {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            }
        })
        .collect();
    if !frame.is_on_any_screen(&screens) {
        return;
    }

    // Size first, then position, so the window is never briefly larger than its
    // remembered frame while it is being placed.
    let _ = window.set_size(tauri::PhysicalSize::new(frame.width, frame.height));
    let _ = window.set_position(tauri::PhysicalPosition::new(frame.x, frame.y));
}

/// Remember the frame on move and resize, coalescing a drag's burst of events
/// into one write. The window is hidden rather than destroyed when it is closed,
/// so the same window (and its typed text) survives until the next shortcut.
fn track_frame(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let Some(path) = frame_path(app) else {
        return;
    };
    let store = Arc::new(FrameStore::at(path));
    let (sender, receiver) = mpsc::channel::<Frame>();

    let saver = Arc::clone(&store);
    std::thread::spawn(move || {
        while let Ok(mut latest) = receiver.recv() {
            // Keep taking newer frames until the user stops moving or resizing.
            loop {
                match receiver.recv_timeout(Duration::from_millis(400)) {
                    Ok(newer) => latest = newer,
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            if let Err(error) = saver.save(latest) {
                eprintln!("Translator: the window frame was not saved: {error}");
            }
        }
    });

    // The event handler needs the window itself to read the half that did not
    // change (the size on a move, the position on a resize).
    let events_window = window.clone();
    window.on_window_event(move |event| match event {
        tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_) => {
            if let Some(frame) = frame_of(&events_window) {
                let _ = sender.send(frame);
            }
        }
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(frame) = frame_of(&events_window) {
                if let Err(error) = store.save(frame) {
                    eprintln!("Translator: the window frame was not saved: {error}");
                }
            }
            let _ = events_window.hide();
        }
        _ => {}
    });
}

/// Purge history at launch and once a day afterwards, on a background thread:
/// a local database that is slow to open, or a purge that fails, must never
/// delay startup or block a shortcut.
fn maintain_history(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        match app.state::<AppState>().engine.purge_history() {
            Ok(removed) if removed > 0 => {
                eprintln!("Translator: purged {removed} history entries older than 30 days");
            }
            Ok(_) => {}
            Err(error) => eprintln!("Translator: history purge failed: {error}"),
        }
        std::thread::sleep(Duration::from_secs(24 * 60 * 60));
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

/// Replace (`⌃⌥R`): capture, translate and write in one step, off the main
/// thread so a slow provider never freezes the app. There is no popup to carry
/// a failure, so every typed failure gets a dialog — a Replace that did nothing
/// must never look like it worked.
fn replace_selection_directly(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(
        move || match app.state::<AppState>().engine.replace_selection() {
            Ok(_) => {}
            Err(error) => show_replace_failure(&app, &error),
        },
    );
}

fn show_replace_failure(app: &AppHandle, error: &EngineError) {
    eprintln!("Translator: Replace failed: {error}");
    let message = match error {
        EngineError::Capture(CaptureError::NoAccessibility) => {
            "Translator needs Accessibility to replace your selection. Open System Settings → \
             Privacy & Security → Accessibility, enable Translator, then try again."
                .to_owned()
        }
        EngineError::Capture(CaptureError::SecureInput) => {
            "This is a secure field, so nothing was replaced. Passwords stay on your Mac."
                .to_owned()
        }
        EngineError::Capture(CaptureError::NothingSelected) => {
            "Select some text first, then press the Replace shortcut again.".to_owned()
        }
        EngineError::Capture(CaptureError::NothingToReplace) => {
            "There is no captured selection to replace. Select text and try again.".to_owned()
        }
        EngineError::Capture(CaptureError::NotWritable) => {
            "This app does not accept a replacement. Your text was left unchanged.".to_owned()
        }
        other => other.to_string(),
    };
    app.dialog()
        .message(message)
        .title("Replace")
        .kind(MessageDialogKind::Error)
        .show(|_| {});
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

/// Apply one shortcut change. The rules — invalid, already in use, no change,
/// macOS refusing, the settings file refusing — live in `shortcuts.rs`; the
/// command only carries the window's request and answers what happened.
#[tauri::command]
async fn rebind_shortcut(
    app: AppHandle,
    action: shortcuts::ShortcutAction,
    binding: String,
) -> Result<shortcuts::RebindResult, ()> {
    shortcuts::rebind_shortcut(&app, action, &binding)
}

/// Put the shipped shortcuts and the default pair back. History and provider
/// keys are not touched.
#[tauri::command]
async fn reset_shortcuts_and_language_pair(app: AppHandle) -> Result<shortcuts::ResetOutcome, ()> {
    shortcuts::reset_shortcuts_and_language_pair(&app)
}

/// What changing the login item left behind. `enabled` is the plugin's actual
/// state, `settings` what the engine now holds, and `problem` a sentence when
/// the change could not be made or persisted. The command never fails with a
/// bare `()`, so the surface can always show what is true.
#[derive(Serialize)]
struct StartAtLoginOutcome {
    enabled: bool,
    settings: Settings,
    problem: Option<String>,
}

/// The login item's actual state, asked of the plugin rather than read from the
/// settings file.
#[tauri::command]
fn start_at_login_enabled(app: AppHandle) -> Result<bool, ()> {
    app.autolaunch().is_enabled().map_err(|error| {
        eprintln!("Translator: the login item state could not be read: {error}");
    })
}

/// Enable or disable the login item and save the preference. Loading the
/// settings first means a file that cannot be read refuses the change before
/// anything is touched. A failed write rolls the login item back, so what the
/// file says and what macOS does never disagree after a relaunch.
#[tauri::command]
fn set_start_at_login(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<StartAtLoginOutcome, ()> {
    let autolaunch = app.autolaunch();
    let mut settings = match state.engine.settings() {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("Translator: the login item was not changed: {error}");
            return Ok(StartAtLoginOutcome {
                enabled: autolaunch.is_enabled().unwrap_or(false),
                settings: Settings::default(),
                problem: Some(
                    "The login item was not changed: the settings file could not be read."
                        .to_owned(),
                ),
            });
        }
    };

    let previous = settings.start_at_login;
    let change = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    if let Err(error) = change {
        eprintln!("Translator: the login item was not changed: {error}");
        return Ok(StartAtLoginOutcome {
            enabled: autolaunch.is_enabled().unwrap_or(settings.start_at_login),
            settings,
            problem: Some(
                "Translator could not change the login item; it was left as it was.".to_owned(),
            ),
        });
    }

    settings.start_at_login = enabled;
    if let Err(error) = state.engine.save_settings(settings.clone()) {
        eprintln!("Translator: the login item change could not be saved: {error}");
        // Put back whatever the file still says, which is the value loaded
        // above, not simply the opposite of the requested one.
        let reverted = if previous {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        let problem = match reverted {
            Ok(()) => {
                "The login item was changed back because the settings file could not be written."
                    .to_owned()
            }
            Err(revert_error) => format!(
                "The settings file could not be written and the login item could not be changed back: {revert_error}"
            ),
        };
        settings.start_at_login = previous;
        return Ok(StartAtLoginOutcome {
            enabled: autolaunch.is_enabled().unwrap_or(previous),
            settings,
            problem: Some(problem),
        });
    }

    Ok(StartAtLoginOutcome {
        enabled: autolaunch.is_enabled().unwrap_or(enabled),
        settings,
        problem: None,
    })
}

/// Do what one shortcut action does, whether it arrived from a global shortcut
/// press or from the tray menu.
fn run_action(app: &AppHandle, action: shortcuts::ShortcutAction) {
    match action {
        shortcuts::ShortcutAction::QuickTranslate => open_quick_translate(app),
        shortcuts::ShortcutAction::Replace => replace_selection_directly(app),
        shortcuts::ShortcutAction::ReadAloud => read_aloud_selection(app),
        shortcuts::ShortcutAction::TranslateWindow => open_translate_window(app),
    }
}

/// A tray row was clicked. The four action rows fire the same handlers as the
/// global shortcuts; the accessibility row opens the pane where the permission
/// is granted; Quit exits.
fn handle_tray_event(app: &AppHandle, id: &str) {
    if let Some(action) = tray::action_for_id(id) {
        run_action(app, action);
        return;
    }
    match id {
        tray::ACCESSIBILITY_NOT_GRANTED_ID => {
            if let Err(error) = app.state::<AppState>().engine.open_accessibility_settings() {
                eprintln!("Translator: Accessibility settings could not be opened: {error}");
            }
        }
        tray::QUIT_ID => app.exit(0),
        _ => {}
    }
}

/// Build the menu bar item from the live bindings and the permission macOS
/// reports now. Later changes go through `tray::refresh_tray`.
fn build_tray(app: &AppHandle, bindings: &ShortcutBindings, granted: bool) -> tauri::Result<()> {
    let menu = tray::menu(app, bindings, granted)?;

    // The 1024 px app icon is scaled to the menu bar by the tray implementation.
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;
    TrayIconBuilder::with_id(tray::TRAY_ID)
        .tooltip("Translator")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_tray_event(app, event.id().as_ref()))
        .build(app)?;
    Ok(())
}

fn main() {
    // Azure F0 first when a key is stored, then the keyless provider. Both sit
    // behind the cache, so a repeated Selection costs nothing. The region is read
    // from settings per request: changing it takes effect without a restart.
    let http = Arc::new(UreqHttpClient::default());
    let credentials = Arc::new(MacCredentialStore);
    let settings_store = Arc::new(MacSettingsStore::default());
    let region_settings = Arc::clone(&settings_store);
    let azure = Arc::new(AzureProvider::new(
        http.clone(),
        credentials.clone(),
        Arc::new(move || {
            region_settings
                .load()
                .map(|settings| settings.azure_region)
                .unwrap_or_default()
        }),
    ));
    let my_memory = Arc::new(MyMemoryProvider::new(http.clone()));
    // Improve's model rung. It translates nothing, so it sits last: translation
    // never waits on it, and Improve falls through Azure and MyMemory's
    // `NotConfigured` to reach it.
    let groq = Arc::new(GroqProvider::new(http, credentials.clone()));
    let ladder = Arc::new(ProviderLadder::new(vec![
        (ProviderId::Azure, azure),
        (ProviderId::MyMemory, my_memory),
        (ProviderId::Groq, groq),
    ]));
    let provider = Arc::new(CachedProvider::new(ladder, TRANSLATION_CACHE_SIZE));

    let engine = TranslatorEngine::new(
        Arc::new(MacSelectionCapture),
        provider,
        Arc::new(MacSpeechOutput),
        Arc::new(MacHistoryStore::default()),
        settings_store,
        credentials,
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent)
                .build(),
        )
        .plugin(tauri_nspanel::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if let Some(action) = app.state::<AppState>().route(shortcut) {
                        run_action(app, action);
                    }
                })
                .build(),
        )
        .manage(AppState {
            engine: Arc::new(engine),
            prefill: Mutex::new(None),
            popup_prefill: Mutex::new(None),
            routes: Mutex::new(shortcuts::ShortcutRoutes::default()),
            launch_conflicts: Mutex::new(Vec::new()),
            accessibility: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            quick_translate,
            replace_selection,
            replace_with_translation,
            read_aloud,
            translate_text,
            improve_text,
            provider_status,
            save_provider_key,
            clear_provider_key,
            get_settings,
            save_settings,
            search_history,
            reopen_history_entry,
            take_selection_prefill,
            take_popup_prefill,
            hide_popup,
            copy_text,
            accessibility_granted,
            request_accessibility,
            open_accessibility_settings,
            complete_onboarding,
            rebind_shortcut,
            reset_shortcuts_and_language_pair,
            shortcut_conflicts,
            start_at_login_enabled,
            set_start_at_login
        ])
        .setup(|app| {
            // Menu-bar only: the app never takes a Dock icon or a place in ⌘Tab.
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let settings = app.state::<AppState>().engine.settings();
            // A settings file that cannot be read still boots with the shipped
            // shortcuts, so a broken file never leaves the app unreachable.
            let bindings = settings
                .as_ref()
                .map(|settings| settings.shortcuts.clone())
                .unwrap_or_default();

            let granted = app.state::<AppState>().engine.accessibility_granted();
            build_tray(app.handle(), &bindings, granted)?;
            *app.state::<AppState>()
                .accessibility
                .lock()
                .expect("accessibility lock") = granted;

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

            // The Translate Window's remembered frame is applied while the window
            // is still hidden, so the first paint is already in the right place.
            restore_frame(app.handle());
            track_frame(app.handle());
            // History ages out at launch and once a day after that; it runs on
            // its own thread so the database can never slow the first shortcut.
            maintain_history(app.handle());

            let routes = shortcuts::ShortcutRoutes::default();
            *app.state::<AppState>()
                .routes
                .lock()
                .expect("shortcut routes lock") = routes;
            for action in shortcuts::ShortcutAction::ALL {
                let binding = action.binding(&bindings).to_owned();
                let live = shortcuts::register_binding(app.handle(), &binding);
                if live.is_none() {
                    app.state::<AppState>()
                        .launch_conflicts
                        .lock()
                        .expect("shortcut conflicts lock")
                        .push(shortcuts::ShortcutStatus {
                            action,
                            problem: Some(shortcuts::launch_problem(&binding)),
                            binding,
                        });
                }
                app.state::<AppState>().set_route(action, live);
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
