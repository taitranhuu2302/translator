//! In-app updates: the check, the silent download and the restart that applies
//! it (docs/adr/0009-self-signed-distribution-and-in-app-updates.md).
//!
//! Only a Release build can update itself. The `updater` cargo feature is what
//! registers the plugin, and `scripts/package.sh` enables it while a local
//! `scripts/build.sh` run does not. A build without it reports `disabled` and
//! every entry point here is a no-op, so a checkout can never replace itself
//! with a Release.
//!
//! No update restarts the app by itself. A ready update waits as one menu bar
//! row and one Settings button, and the user chooses the moment.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::AppState;

/// The event every surface listens to. The payload is [`UpdateStatus`].
pub const EVENT: &str = "update-status";

/// How often a Release build looks for a newer one while auto-check is on.
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Where the update is in its life. `disabled` is a build that cannot update
/// itself; `idle` is a Release whose first check has not finished yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateState {
    #[default]
    Disabled,
    Idle,
    Checking,
    Downloading,
    UpToDate,
    Ready,
    Failed,
}

/// What the menu bar, Settings and the IPC report. It is replaced as a whole on
/// every change, so no surface has to merge two partial states.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct UpdateStatus {
    pub state: UpdateState,
    /// The version this build is. Shown even when updates are disabled.
    pub current_version: String,
    /// The version waiting for a restart when `state` is `ready`, and the one
    /// being downloaded while `state` is `downloading`.
    pub available_version: Option<String>,
    /// The sentence shown when `state` is `failed`.
    pub problem: Option<String>,
}

/// The updater's state, held by [`AppState`] for the life of the process.
pub struct Updates {
    status: Mutex<UpdateStatus>,
    /// One check at a time: a manual Check now must not race the six-hourly one
    /// into two downloads.
    busy: AtomicBool,
    /// Wakes the six-hourly sleep when the auto-check preference changes, so
    /// turning it off stops the next check and turning it on checks now.
    wake: Mutex<Option<mpsc::Sender<()>>>,
}

impl Updates {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(UpdateStatus::default()),
            busy: AtomicBool::new(false),
            wake: Mutex::new(None),
        }
    }

    pub fn status(&self) -> UpdateStatus {
        self.status.lock().expect("update status lock").clone()
    }

    /// Store the new status, and say whether the menu bar's ready row changed.
    fn store(&self, status: UpdateStatus) -> bool {
        let mut slot = self.status.lock().expect("update status lock");
        let before = ready_of(&slot).cloned();
        let after = ready_of(&status).cloned();
        *slot = status;
        before != after
    }

    fn claim(&self) -> bool {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    fn release(&self) {
        self.busy.store(false, Ordering::SeqCst);
    }

    fn arm(&self, sender: mpsc::Sender<()>) {
        *self.wake.lock().expect("update wake lock") = Some(sender);
    }

    fn wake_now(&self) {
        if let Some(sender) = self.wake.lock().expect("update wake lock").as_ref() {
            let _ = sender.send(());
        }
    }
}

impl Default for Updates {
    fn default() -> Self {
        Self::new()
    }
}

/// The version waiting for a restart, when there is one. The menu bar's update
/// row exists only while this answers `Some`.
pub fn ready_version(app: &AppHandle) -> Option<String> {
    let status = status(app);
    if status.state == UpdateState::Ready {
        status.available_version
    } else {
        None
    }
}

/// The ready version of a status in hand, for comparing two of them.
fn ready_of(status: &UpdateStatus) -> Option<&String> {
    if status.state == UpdateState::Ready {
        status.available_version.as_ref()
    } else {
        None
    }
}

/// What every command and surface reads.
pub fn status(app: &AppHandle) -> UpdateStatus {
    app.state::<AppState>().updates.status()
}

/// Store a status, tell the views, and rebuild the menu bar when its row
/// appeared or went away. Everything that changes the status goes through here,
/// so the three surfaces can never disagree.
fn publish(app: &AppHandle, status: UpdateStatus) {
    let tray_changed = app.state::<AppState>().updates.store(status.clone());
    let _ = app.emit(EVENT, status);
    if tray_changed {
        crate::tray::refresh_tray(app);
    }
}

/// The status a build starts with: `disabled` when the `updater` feature is
/// off, `idle` when it is on. Called once from setup.
pub fn prepare(app: &AppHandle) {
    let current_version = app.package_info().version.to_string();
    let state = if cfg!(feature = "updater") {
        UpdateState::Idle
    } else {
        UpdateState::Disabled
    };
    publish(
        app,
        UpdateStatus {
            state,
            current_version,
            available_version: None,
            problem: None,
        },
    );
}

/// Start the six-hourly check, with the launch check as its first turn. Only a
/// Release build has anything to check with, so the other build just records
/// that it cannot update.
#[cfg(feature = "updater")]
pub fn start(app: &AppHandle) {
    prepare(app);

    let (sender, receiver) = mpsc::channel();
    app.state::<AppState>().updates.arm(sender);

    let app = app.clone();
    std::thread::spawn(move || loop {
        if auto_check_enabled(&app) {
            tauri::async_runtime::block_on(check_now(&app));
        }
        // Six hours, unless the preference changes first.
        match receiver.recv_timeout(CHECK_INTERVAL) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    });
}

#[cfg(not(feature = "updater"))]
pub fn start(app: &AppHandle) {
    prepare(app);
}

/// A settings save changed the auto-check preference. Cutting the sleep short
/// makes the toggle take effect now: off stops the next check, on checks now.
pub fn preference_changed(app: &AppHandle) {
    app.state::<AppState>().updates.wake_now();
}

/// Check now, however the preference is set, and download whatever is found.
/// A check already in flight, or an update already waiting for a restart, is
/// left alone: it would only download the same artifact again.
pub async fn check_now(app: &AppHandle) -> UpdateStatus {
    #[cfg(feature = "updater")]
    {
        run_check(app).await
    }
    #[cfg(not(feature = "updater"))]
    {
        status(app)
    }
}

#[cfg(feature = "updater")]
async fn run_check(app: &AppHandle) -> UpdateStatus {
    use tauri_plugin_updater::UpdaterExt;

    let current = status(app);
    if current.state == UpdateState::Ready || !app.state::<AppState>().updates.claim() {
        return current;
    }

    publish(
        app,
        UpdateStatus {
            state: UpdateState::Checking,
            available_version: None,
            problem: None,
            ..current.clone()
        },
    );

    let updater = match app.updater() {
        Ok(updater) => updater,
        Err(error) => {
            return finish(
                app,
                UpdateState::Failed,
                None,
                Some(format!("the update check could not start: {error}")),
            );
        }
    };

    let update = match updater.check().await {
        Ok(Some(update)) => update,
        Ok(None) => return finish(app, UpdateState::UpToDate, None, None),
        Err(error) => {
            return finish(
                app,
                UpdateState::Failed,
                None,
                Some(format!("the update check failed: {error}")),
            );
        }
    };

    let version = update.version.clone();
    publish(
        app,
        UpdateStatus {
            state: UpdateState::Downloading,
            available_version: Some(version.clone()),
            problem: None,
            ..current
        },
    );

    // The download is silent on purpose: no progress surface, no interruption.
    // `download_and_install` verifies the artifact against this build's update
    // public key before it writes a byte into the app.
    match update.download_and_install(|_, _| {}, || {}).await {
        Ok(()) => finish(app, UpdateState::Ready, Some(version), None),
        Err(error) => finish(
            app,
            UpdateState::Failed,
            Some(version),
            Some(format!("the update could not be downloaded: {error}")),
        ),
    }
}

/// The one exit from a check: record the outcome, free the slot, answer.
#[cfg(feature = "updater")]
fn finish(
    app: &AppHandle,
    state: UpdateState,
    available_version: Option<String>,
    problem: Option<String>,
) -> UpdateStatus {
    let status = UpdateStatus {
        state,
        available_version,
        problem,
        ..status(app)
    };
    publish(app, status.clone());
    app.state::<AppState>().updates.release();
    status
}

/// Restart into the update that is already installed. Only the menu bar row and
/// the Settings button call this, and both exist only while `state` is `ready`.
pub fn restart(app: &AppHandle) -> ! {
    app.restart()
}

/// The preference as the file holds it now. A file that cannot be read keeps
/// the shipped value, the same way launch does.
fn auto_check_enabled(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .engine
        .settings()
        .map(|settings| settings.auto_check_updates)
        .unwrap_or(true)
}
