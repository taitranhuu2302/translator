//! The one way an AppKit operation is posted to the main thread.
//!
//! Menus, tray icons, panels and Carbon shortcut registrations belong to the
//! main thread, but commands run on worker threads. Both `main.rs` and the
//! shortcut module wait for the answer with a bounded timeout, so a shell that
//! cannot reach the main thread reports a failure instead of hanging.

use std::sync::mpsc;
use std::time::Duration;

use tauri::AppHandle;

/// Run `operation` on the main thread and wait up to `timeout` for its answer.
/// `None` means the operation was not scheduled or the main thread did not
/// answer in time; it never means the operation ran and failed.
pub fn on_main_thread<R: Send + 'static>(
    app: &AppHandle,
    timeout: Duration,
    operation: impl FnOnce(&AppHandle) -> R + Send + 'static,
) -> Option<R> {
    let (finished, wait) = mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = finished.send(operation(&handle));
    })
    .ok()?;
    wait.recv_timeout(timeout).ok()
}
