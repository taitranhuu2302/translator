//! The Tauri v2 shell: it builds the ports, owns the engine and exposes IPC.
//! Views, panels and menu items call these commands; they never reach a port
//! implementation directly.

mod frame;
mod hud;
mod main_thread;
mod panel;
mod shortcuts;
mod tray;
mod workflows;

use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use frame::{Frame, FrameStore, ScreenRect};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_global_shortcut::{Shortcut, ShortcutState};
use translator_core::{
    AzureProvider, CachedProvider, CaptureError, Direction, EngineError, GroqProvider,
    HistoryEntry, ImproveDelta, ImprovePart, Mode, MyMemoryProvider, PopupPrefill, ProviderId,
    ProviderLadder, ProviderStatus, QuickTranslation, ReadAloud, ReplacePhase, Replacement,
    Selection, Settings,
    SettingsStore, ShortcutBindings, SpeechError, TextOutcome, Translation, TranslatorEngine,
    UreqHttpClient,
};
use translator_platform_macos::{
    clipboard, migrate_legacy_state, MacCredentialStore, MacHistoryStore, MacSelectionCapture,
    MacSettingsStore, MacSpeechOutput, MigrationOutcome,
};

/// How many translations the in-memory cache remembers.
const TRANSLATION_CACHE_SIZE: usize = 100;

struct AppState {
    /// Shared so a command can hold it across an awaits-free worker hop while
    /// the borrow of `State` ends.
    engine: Arc<TranslatorEngine>,
    /// The same store the engine holds, for the shell workflows that change a
    /// preference beside a platform state.
    settings: Arc<dyn SettingsStore>,
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
    /// The Replace running now, if any. One at a time: a second shortcut while
    /// the first is still waiting on a provider would race two writes into the
    /// same Selection. It also carries the HUD that says what the Replace is
    /// doing.
    replace: Mutex<Option<hud::ReplaceHud>>,
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

    /// Claim the one Replace slot and arm its HUD. `false` means another
    /// Replace still runs: the caller drops this one rather than racing it.
    fn begin_replace(&self, app: &AppHandle, phase: ReplacePhase) -> bool {
        let mut slot = self.replace.lock().expect("replace lock");
        if slot.is_some() {
            return false;
        }
        *slot = Some(hud::ReplaceHud::start(app, phase));
        true
    }

    /// The engine moved to its next step. Ignored when no Replace owns the slot
    /// (its session is already being taken away).
    fn advance_replace(&self, app: &AppHandle, phase: ReplacePhase) {
        if let Some(session) = self.replace.lock().expect("replace lock").as_ref() {
            session.phase(app, phase);
        }
    }

    /// End the Replace, whatever its outcome. The slot is freed before the HUD
    /// waits for the main thread, so a Replace that starts while this one is
    /// hiding is never refused behind a lock.
    fn end_replace(&self, app: &AppHandle) {
        if let Some(session) = self.replace.lock().expect("replace lock").take() {
            session.finish(app);
        }
    }

    /// The step the Replace running now is on, or `None` when none runs.
    fn replace_phase(&self) -> Option<ReplacePhase> {
        self.replace
            .lock()
            .expect("replace lock")
            .as_ref()
            .map(hud::ReplaceHud::phase_now)
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
async fn replace_selection(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Replacement, EngineError> {
    let engine = Arc::clone(&state.engine);
    // No popup is part of this path — `⌃⌥R` translates and replaces without one
    // — so the workflow has nothing to hide or bring back. The HUD is armed for
    // every Replace, whichever surface asked for it.
    if !state.begin_replace(&app, ReplacePhase::Capturing) {
        return Err(replace_already_running());
    }
    let outcome = workflows::replace(None, || {
        engine.replace_selection(|phase| state.advance_replace(&app, phase))
    });
    state.end_replace(&app);
    replace_result(outcome)
}

/// The answer when a Replace is refused because another one still runs. The
/// shortcut path drops its own silently; this exists for the surfaces that can
/// say something.
fn replace_already_running() -> EngineError {
    EngineError::Capture(CaptureError::Unavailable {
        reason: "a Replace is already running, so nothing was replaced".to_owned(),
    })
}

/// Turn the workflow's outcome into the command's answer. A popup that could
/// not be brought back is logged here, apart from the Replace failure it
/// followed, rather than being folded into (or replacing) that error.
fn replace_result(outcome: workflows::ReplaceOutcome) -> Result<Replacement, EngineError> {
    match outcome {
        workflows::ReplaceOutcome::Replaced(replacement) => Ok(replacement),
        workflows::ReplaceOutcome::Failed { error, restore } => {
            if let workflows::PopupRestore::Failed(problem) = restore {
                eprintln!(
                    "Neris: the popup could not be shown after a failed Replace: {problem}"
                );
            }
            Err(error)
        }
    }
}

/// The popup's Enter: Replace the Selection with the translation already on
/// screen, without capturing or translating again.
///
/// The hide → Replace → restore order lives in `workflows::replace`. The panel
/// holds the keyboard while it is shown, so the clipboard fallback's synthesized
/// ⌘V would land in our own page; the workflow hides it first and shows it again
/// if the write fails so the popup can explain the failure inline.
#[tauri::command]
async fn replace_with_translation(
    app: AppHandle,
    state: State<'_, AppState>,
    source_text: String,
    translation: Translation,
) -> Result<Replacement, EngineError> {
    // The popup already showed the answer; only the write is left to report,
    // and it starts in that step.
    if !state.begin_replace(&app, ReplacePhase::Replacing) {
        return Err(replace_already_running());
    }
    let engine = Arc::clone(&state.engine);
    let outcome = main_thread::on_main_thread(&app, move |handle| {
        workflows::replace(
            Some(&workflows::ShellPanelPresenter::new(handle.clone())),
            || engine.replace_with_translation(&source_text, translation),
        )
    });
    state.end_replace(&app);
    match outcome {
        Some(outcome) => replace_result(outcome),
        None => {
            eprintln!("Neris: Replace did not run: the main thread did not answer");
            Err(EngineError::Capture(CaptureError::Unavailable {
                reason: "the main thread did not answer, so nothing was replaced".to_owned(),
            }))
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
    main_thread::on_main_thread(&app, panel::hide).map_or(Err(()), |()| Ok(()))
}

/// The step the Replace running now is on, or `null` when none runs. The HUD
/// page claims this on mount: an event emitted before the page finished loading
/// would otherwise be lost, leaving an empty capsule on screen.
#[tauri::command]
async fn replace_hud_phase(state: State<'_, AppState>) -> Result<Option<ReplacePhase>, ()> {
    Ok(state.replace_phase())
}

/// The popup's page measured its own content. The height it asks for is
/// clamped and applied on the main thread, and the panel is placed again so the
/// card stays where it opened. A failure is not worth a notice of its own: the
/// card simply keeps the height it had.
#[tauri::command]
async fn set_popup_height(app: AppHandle, height: f64) -> Result<(), ()> {
    let scheduled = main_thread::on_main_thread(&app, move |handle| {
        panel::set_height(handle, height)
    })
    .ok_or(())?;
    scheduled.map_err(|_| ())
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

        let shown = main_thread::on_main_thread(&app, move |handle| {
            if let Err(error) = panel::show(handle) {
                eprintln!("Neris: the Quick Translate popup could not open: {error}");
                return;
            }
            let _ = handle.emit_to(panel::POPUP_LABEL, panel::POPUP_PREFILL_EVENT, ());
        });
        if shown.is_none() {
            eprintln!(
                "Neris: the Quick Translate popup could not be shown: the main thread did not answer"
            );
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
                eprintln!("Neris: the window frame was not saved: {error}");
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
                    eprintln!("Neris: the window frame was not saved: {error}");
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
                eprintln!("Neris: purged {removed} history entries older than 30 days");
            }
            Ok(_) => {}
            Err(error) => eprintln!("Neris: history purge failed: {error}"),
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
/// thread so a slow provider never freezes the app. It runs through the same
/// workflow as the popup's Replace, with no popup to hide or restore. There is
/// no popup to carry a failure either, so every typed failure gets a dialog — a
/// Replace that did nothing must never look like it worked.
fn replace_selection_directly(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        // A press while the previous Replace still waits on a provider is
        // dropped: two writes into one Selection would race each other.
        if !state.begin_replace(&app, ReplacePhase::Capturing) {
            return;
        }
        let engine = Arc::clone(&state.engine);
        let outcome = workflows::replace(None, || {
            engine.replace_selection(|phase| state.advance_replace(&app, phase))
        });
        state.end_replace(&app);
        if let workflows::ReplaceOutcome::Failed { error, .. } = outcome {
            show_replace_failure(&app, &error);
        }
    });
}

/// This native dialog is a shell surface, not one of the two UI surfaces the
/// capture-notice model is scoped to, so it owns its own copy deliberately: it
/// cannot draw a TypeScript model, and its sentences name what this operation
/// did ("nothing was replaced") rather than restating the popup's notice. The
/// catch-all arm keeps a newly added `CaptureError` variant handled here until
/// this dialog gives it words of its own.
fn show_replace_failure(app: &AppHandle, error: &EngineError) {
    eprintln!("Neris: Replace failed: {error}");
    let message = match error {
        EngineError::Capture(CaptureError::NoAccessibility) => {
            "Neris needs Accessibility to replace your selection. Open System Settings → \
             Privacy & Security → Accessibility, enable Neris, then try again."
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
    eprintln!("Neris: Read Aloud failed: {error}");
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
/// macOS refusing, the settings file refusing — live in `shortcuts.rs` and the
/// apply order in `workflows.rs`; the command only hops to the main thread and
/// refreshes the tray once the hop has returned.
#[tauri::command]
async fn rebind_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    action: shortcuts::ShortcutAction,
    binding: String,
) -> Result<shortcuts::RebindResult, ()> {
    let settings = Arc::clone(&state.settings);
    let outcome = main_thread::on_main_thread(&app, move |handle| {
        workflows::rebind_shortcut(
            &workflows::ShellKeyRegistry::new(handle.clone()),
            &workflows::ShellShortcutRoute::new(handle.clone()),
            settings.as_ref(),
            action,
            &binding,
        )
    });
    let outcome = match outcome {
        Some(Ok(outcome)) => outcome,
        Some(Err(())) => return Err(()),
        None => {
            eprintln!("Neris: the shortcut change did not run: the main thread did not answer");
            return Err(());
        }
    };
    if outcome.applied {
        app.state::<AppState>().forget_conflict(action);
        tray::refresh_tray(&app, &outcome.settings.shortcuts);
    }
    Ok(shortcuts::RebindResult {
        binding: outcome.binding,
        problem: outcome.problem,
    })
}

/// Put the shipped shortcuts and the default pair back. History and provider
/// keys are not touched. The reset reuses the apply sequence; the command only
/// clears the launch conflicts and refreshes the tray once the hop has returned.
#[tauri::command]
async fn reset_shortcuts_and_language_pair(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<shortcuts::ResetOutcome, ()> {
    let settings = Arc::clone(&state.settings);
    let outcome = main_thread::on_main_thread(&app, move |handle| {
        workflows::reset_shortcuts(
            &workflows::ShellKeyRegistry::new(handle.clone()),
            &workflows::ShellShortcutRoute::new(handle.clone()),
            settings.as_ref(),
        )
    });
    let outcome = match outcome {
        Some(Ok(outcome)) => outcome,
        Some(Err(())) => return Err(()),
        None => {
            eprintln!("Neris: the shortcuts were not reset: the main thread did not answer");
            return Err(());
        }
    };
    app.state::<AppState>().clear_conflicts();
    tray::refresh_tray(&app, &outcome.settings.shortcuts);
    Ok(outcome)
}

/// The login item's actual state, asked of the system rather than read from the
/// settings file.
#[tauri::command]
async fn start_at_login_enabled(app: AppHandle) -> Result<bool, ()> {
    workflows::LoginItem::is_enabled(&workflows::ShellLoginItem::new(app)).map_err(|error| {
        eprintln!("Neris: the login item state could not be read: {error}");
    })
}

/// The read → change → save → roll-back order lives in the workflow; the
/// command only hops to the main thread and maps the result.
#[tauri::command]
async fn set_start_at_login(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<workflows::StartAtLoginOutcome, ()> {
    let settings = Arc::clone(&state.settings);
    let outcome = main_thread::on_main_thread(&app, move |handle| {
        workflows::set_start_at_login(
            &workflows::ShellLoginItem::new(handle.clone()),
            settings.as_ref(),
            enabled,
        )
    });
    outcome.ok_or_else(|| {
        eprintln!("Neris: the login item change did not run: the main thread did not answer");
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
                eprintln!("Neris: Accessibility settings could not be opened: {error}");
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

    // A template image: macOS draws it in the menu bar's own foreground colour,
    // so it stays legible in both appearances. It is drawn at 18 pt, and the
    // file is 72 px — four pixels per point for a Retina menu bar to sample.
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
    TrayIconBuilder::with_id(tray::TRAY_ID)
        .tooltip("Neris")
        .icon(icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| handle_tray_event(app, event.id().as_ref()))
        .build(app)?;
    Ok(())
}

fn main() {
    // Before any store is built: the previous identity's settings, history and
    // keys are copied into this one's directory, so the first read below finds
    // them. Failing here is not fatal — the app starts with whatever it has and
    // the copy is retried next launch.
    match migrate_legacy_state() {
        Ok(MigrationOutcome::Copied { from }) => {
            eprintln!(
                "Neris: carried settings, history and keys over from {}",
                from.display()
            );
        }
        Ok(MigrationOutcome::NotNeeded) => {}
        Err(error) => eprintln!("Neris: the previous version's state was not carried over: {error}"),
    }

    // Azure F0 first when a key is stored, then the keyless provider. Both sit
    // behind the cache, so a repeated Selection costs nothing. The region is read
    // from settings per request: changing it takes effect without a restart.
    let http = Arc::new(UreqHttpClient::default());
    let credentials = Arc::new(MacCredentialStore);
    let settings_store: Arc<dyn SettingsStore> = Arc::new(MacSettingsStore::default());
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
        Arc::clone(&settings_store),
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
            settings: settings_store,
            prefill: Mutex::new(None),
            popup_prefill: Mutex::new(None),
            routes: Mutex::new(shortcuts::ShortcutRoutes::default()),
            launch_conflicts: Mutex::new(Vec::new()),
            accessibility: Mutex::new(false),
            replace: Mutex::new(None),
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
            replace_hud_phase,
            hide_popup,
            set_popup_height,
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
            // The HUD is a panel too, but a quieter one: never key, never
            // clicked, shown only while a Replace runs.
            hud::prepare(app.handle())?;
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
        .expect("error while running Neris");
}
