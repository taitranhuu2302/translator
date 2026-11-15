//! The one way an AppKit operation is posted to the main thread.
//!
//! Menus, tray icons, panels and Carbon shortcut registrations belong to the
//! main thread, but commands run on worker threads. Every hop waits for the
//! answer with the same bounded timeout, so a shell that cannot reach the main
//! thread reports a failure instead of hanging.

use std::sync::mpsc;
use std::time::Duration;

use tauri::AppHandle;

/// How long any main-thread operation may take before the caller reports
/// failure. One value for every hop in the shell — panels, shortcut
/// registrations and menu swaps — so there is one answer to "how long may the
/// main thread take".
pub const MAIN_THREAD_TIMEOUT: Duration = Duration::from_secs(2);

/// Run `operation` on the main thread and wait up to [`MAIN_THREAD_TIMEOUT`] for
/// its answer. `None` means the operation was not scheduled or the main thread
/// did not answer in time; it never means the operation ran and failed.
pub fn on_main_thread<R: Send + 'static>(
    app: &AppHandle,
    operation: impl FnOnce(&AppHandle) -> R + Send + 'static,
) -> Option<R> {
    let (finished, wait) = mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = finished.send(operation(&handle));
    })
    .ok()?;
    wait.recv_timeout(MAIN_THREAD_TIMEOUT).ok()
}
