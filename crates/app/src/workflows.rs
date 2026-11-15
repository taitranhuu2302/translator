//! The shell's workflows: the orderings behind the Tauri commands, written as
//! plain functions over a small set of app-local ports.
//!
//! The ports describe the shell, not the product, so they live here rather than
//! in `translator-core` (ADR-0001). A workflow never needs an `AppHandle` to
//! run; the real shell supplies adapters that do the AppKit work, and the tests
//! supply plain fakes.
//!
//! A command performs one `main_thread::on_main_thread` hop and runs the
//! workflow inside it. The real adapters below therefore call AppKit, Carbon
//! and the plugins directly rather than posting another hop: a nested hop from
//! the main thread would wait on itself.

use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use translator_core::{Settings, SettingsStore};

use crate::panel;
use crate::shortcuts::ShortcutAction;
use crate::AppState;

/// The login item macOS launches Translator from. Reading and changing it are
/// separate so a failed change never has to be inferred from a boolean.
pub trait LoginItem {
    fn is_enabled(&self) -> Result<bool, String>;
    fn set_enabled(&self, enabled: bool) -> Result<(), String>;
}

/// The Quick Translate popup. Hiding and showing are AppKit work; the workflow
/// only states the order.
#[allow(dead_code)] // Driven by the Replace workflow in ticket 09.
pub trait PanelPresenter {
    fn hide(&self) -> Result<(), String>;
    fn show(&self) -> Result<(), String>;
}

/// The global shortcut registrations. A refused registration is an error the
/// workflow may report; it changes nothing on its own.
#[allow(dead_code)] // Driven by the shortcut apply workflow in ticket 10.
pub trait KeyRegistry {
    fn register(&self, shortcut: Shortcut) -> Result<(), String>;
    fn unregister(&self, shortcut: Shortcut) -> Result<(), String>;
}

/// The action a live shortcut calls. Changing it cannot fail: it is the shell's
/// own routing table.
#[allow(dead_code)] // Driven by the shortcut apply workflow in ticket 10.
pub trait ShortcutRoute {
    fn set_route(&self, action: ShortcutAction, shortcut: Option<Shortcut>);
}

/// What changing the login item left behind. `enabled` is the login item's
/// actual state, `settings` what the file now holds, and `problem` a sentence
/// when the change could not be made or persisted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartAtLoginOutcome {
    pub enabled: bool,
    pub settings: Settings,
    pub problem: Option<String>,
}

/// Read the preference, change the login item, save the preference, and put
/// both back when either half refuses.
pub fn set_start_at_login(
    login_item: &dyn LoginItem,
    settings_store: &dyn SettingsStore,
    enabled: bool,
) -> StartAtLoginOutcome {
    let mut settings = match settings_store.load() {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("Translator: the login item was not changed: {error}");
            return StartAtLoginOutcome {
                enabled: login_item.is_enabled().unwrap_or(false),
                settings: Settings::default(),
                problem: Some(
                    "The login item was not changed: the settings file could not be read."
                        .to_owned(),
                ),
            };
        }
    };
    let previous = settings.start_at_login;
    if let Err(error) = login_item.set_enabled(enabled) {
        eprintln!("Translator: the login item was not changed: {error}");
        return StartAtLoginOutcome {
            enabled: login_item.is_enabled().unwrap_or(settings.start_at_login),
            settings,
            problem: Some(
                "Translator could not change the login item; it was left as it was.".to_owned(),
            ),
        };
    }
    settings.start_at_login = enabled;
    if let Err(error) = settings_store.save(&settings) {
        eprintln!("Translator: the login item change could not be saved: {error}");
        // Put back whatever the file still says, which is the value loaded
        // above, not simply the opposite of the requested one.
        let problem = match login_item.set_enabled(previous) {
            Ok(()) => {
                "The login item was changed back because the settings file could not be written."
                    .to_owned()
            }
            Err(revert_error) => format!(
                "The settings file could not be written and the login item could not be changed back: {revert_error}"
            ),
        };
        settings.start_at_login = previous;
        return StartAtLoginOutcome {
            enabled: login_item.is_enabled().unwrap_or(previous),
            settings,
            problem: Some(problem),
        };
    }

    StartAtLoginOutcome {
        enabled: login_item.is_enabled().unwrap_or(enabled),
        settings,
        problem: None,
    }
}

/// The real login item, backed by the autostart plugin. Reading it needs no
/// AppKit; changing it is done where the command put the workflow.
pub struct ShellLoginItem {
    app: AppHandle,
}

impl ShellLoginItem {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl LoginItem for ShellLoginItem {
    fn is_enabled(&self) -> Result<bool, String> {
        self.app
            .autolaunch()
            .is_enabled()
            .map_err(|error| error.to_string())
    }

    fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        let autolaunch = self.app.autolaunch();
        let change = if enabled {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        change.map_err(|error| error.to_string())
    }
}

/// The real popup. The command has already put the workflow on the main thread,
/// so this calls AppKit directly.
#[allow(dead_code)] // Driven by the Replace workflow in ticket 09.
pub struct ShellPanelPresenter {
    app: AppHandle,
}

#[allow(dead_code)]
impl ShellPanelPresenter {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[allow(dead_code)]
impl PanelPresenter for ShellPanelPresenter {
    fn hide(&self) -> Result<(), String> {
        panel::hide(&self.app);
        Ok(())
    }

    fn show(&self) -> Result<(), String> {
        panel::show(&self.app).map_err(|error| error.to_string())
    }
}

/// The real global-shortcut registry. Carbon registration belongs to the main
/// thread, where the command put the workflow.
#[allow(dead_code)] // Driven by the shortcut apply workflow in ticket 10.
pub struct ShellKeyRegistry {
    app: AppHandle,
}

#[allow(dead_code)]
impl ShellKeyRegistry {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[allow(dead_code)]
impl KeyRegistry for ShellKeyRegistry {
    fn register(&self, shortcut: Shortcut) -> Result<(), String> {
        self.app
            .global_shortcut()
            .register(shortcut)
            .map_err(|error| error.to_string())
    }

    fn unregister(&self, shortcut: Shortcut) -> Result<(), String> {
        self.app
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|error| error.to_string())
    }
}

/// The real routing table, held by `AppState` beside the live registrations.
#[allow(dead_code)] // Driven by the shortcut apply workflow in ticket 10.
pub struct ShellShortcutRoute {
    app: AppHandle,
}

#[allow(dead_code)]
impl ShellShortcutRoute {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[allow(dead_code)]
impl ShortcutRoute for ShellShortcutRoute {
    fn set_route(&self, action: ShortcutAction, shortcut: Option<Shortcut>) {
        self.app.state::<AppState>().set_route(action, shortcut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::sync::Mutex;
    use translator_core::{Settings, SettingsError};

    fn preference(start_at_login: bool) -> Settings {
        Settings {
            start_at_login,
            ..Settings::default()
        }
    }

    struct FakeLoginItem {
        enabled: Cell<bool>,
        refuse: bool,
        changes: RefCell<Vec<bool>>,
    }

    impl FakeLoginItem {
        fn new(enabled: bool) -> Self {
            Self {
                enabled: Cell::new(enabled),
                refuse: false,
                changes: RefCell::new(Vec::new()),
            }
        }

        fn refusing() -> Self {
            Self {
                refuse: true,
                ..Self::new(false)
            }
        }

        fn changes(&self) -> Vec<bool> {
            self.changes.borrow().clone()
        }
    }

    impl LoginItem for FakeLoginItem {
        fn is_enabled(&self) -> Result<bool, String> {
            Ok(self.enabled.get())
        }

        fn set_enabled(&self, enabled: bool) -> Result<(), String> {
            self.changes.borrow_mut().push(enabled);
            if self.refuse {
                return Err("the login item refused the change".to_owned());
            }
            self.enabled.set(enabled);
            Ok(())
        }
    }

    struct FakeSettingsStore {
        settings: Mutex<Settings>,
        refuse_load: bool,
        refuse_save: bool,
        saves: Mutex<Vec<Settings>>,
    }

    impl FakeSettingsStore {
        fn holding(settings: Settings) -> Self {
            Self {
                settings: Mutex::new(settings),
                refuse_load: false,
                refuse_save: false,
                saves: Mutex::new(Vec::new()),
            }
        }

        fn refusing_save(settings: Settings) -> Self {
            Self {
                refuse_save: true,
                ..Self::holding(settings)
            }
        }

        fn refusing_load() -> Self {
            Self {
                refuse_load: true,
                ..Self::holding(Settings::default())
            }
        }

        fn current(&self) -> Settings {
            self.settings.lock().expect("settings lock").clone()
        }

        fn saves(&self) -> Vec<Settings> {
            self.saves.lock().expect("saves lock").clone()
        }
    }

    impl SettingsStore for FakeSettingsStore {
        fn load(&self) -> Result<Settings, SettingsError> {
            if self.refuse_load {
                return Err(SettingsError::Storage {
                    message: "the settings file is unreadable".to_owned(),
                });
            }
            Ok(self.current())
        }

        fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
            self.saves
                .lock()
                .expect("saves lock")
                .push(settings.clone());
            if self.refuse_save {
                return Err(SettingsError::Storage {
                    message: "the settings file is unwritable".to_owned(),
                });
            }
            *self.settings.lock().expect("settings lock") = settings.clone();
            Ok(())
        }
    }

    #[test]
    fn a_successful_change_turns_the_login_item_on_and_saves_the_preference() {
        let login_item = FakeLoginItem::new(false);
        let settings = FakeSettingsStore::holding(preference(false));

        let outcome = set_start_at_login(&login_item, &settings, true);

        assert_eq!(outcome.enabled, true);
        assert_eq!(outcome.problem, None);
        assert!(outcome.settings.start_at_login);
        assert_eq!(login_item.enabled.get(), true);
        assert_eq!(login_item.changes(), vec![true]);
        assert_eq!(settings.saves(), vec![preference(true)]);
    }

    #[test]
    fn a_login_item_that_refuses_the_change_leaves_both_sides_where_they_started() {
        let login_item = FakeLoginItem::refusing();
        let settings = FakeSettingsStore::holding(preference(false));

        let outcome = set_start_at_login(&login_item, &settings, true);

        assert_eq!(outcome.enabled, false);
        assert_eq!(outcome.settings, preference(false));
        assert_eq!(
            outcome.problem.as_deref(),
            Some("Translator could not change the login item; it was left as it was.")
        );
        assert_eq!(login_item.enabled.get(), false);
        assert_eq!(settings.current(), preference(false));
        assert!(settings.saves().is_empty());
    }

    #[test]
    fn a_refused_save_puts_the_login_item_back_and_leaves_the_preference_unchanged() {
        for (previous, requested) in [(false, true), (true, false)] {
            let login_item = FakeLoginItem::new(previous);
            let settings = FakeSettingsStore::refusing_save(preference(previous));

            let outcome = set_start_at_login(&login_item, &settings, requested);

            assert_eq!(outcome.enabled, previous);
            assert_eq!(outcome.settings, preference(previous));
            assert_eq!(
                outcome.problem.as_deref(),
                Some("The login item was changed back because the settings file could not be written.")
            );
            assert_eq!(
                login_item.enabled.get(),
                previous,
                "the login item was not restored"
            );
            assert_eq!(
                settings.current(),
                preference(previous),
                "the saved preference moved"
            );
            assert_eq!(settings.saves(), vec![preference(requested)]);
        }
    }

    #[test]
    fn an_unreadable_settings_file_refuses_the_change_before_touching_the_login_item() {
        let login_item = FakeLoginItem::new(true);
        let settings = FakeSettingsStore::refusing_load();

        let outcome = set_start_at_login(&login_item, &settings, false);

        assert_eq!(outcome.enabled, true);
        assert_eq!(outcome.settings, Settings::default());
        assert_eq!(
            outcome.problem.as_deref(),
            Some("The login item was not changed: the settings file could not be read.")
        );
        assert!(login_item.changes().is_empty());
        assert!(settings.saves().is_empty());
    }
}
