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
use translator_core::{
    EngineError, Replacement, Settings, SettingsError, SettingsStore, ShortcutBindings,
};

use crate::panel;
use crate::shortcuts::{
    self, Plan, ResetOutcome, ShortcutAction, ShortcutStatus, INVALID_BINDING, SAVE_FAILED,
};
use crate::AppState;

/// The login item macOS launches Neris from. Reading and changing it are
/// separate so a failed change never has to be inferred from a boolean.
pub trait LoginItem {
    fn is_enabled(&self) -> Result<bool, String>;
    fn set_enabled(&self, enabled: bool) -> Result<(), String>;
}

/// The Quick Translate popup. Hiding and showing are AppKit work; the workflow
/// only states the order.
pub trait PanelPresenter {
    fn hide(&self) -> Result<(), String>;
    fn show(&self) -> Result<(), String>;
}

/// The global shortcut registrations. A refused registration is an error the
/// workflow may report; it changes nothing on its own.
pub trait KeyRegistry {
    fn register(&self, shortcut: Shortcut) -> Result<(), String>;
    fn unregister(&self, shortcut: Shortcut) -> Result<(), String>;
}

/// The action a live shortcut calls. Changing it cannot fail: it is the shell's
/// own routing table. `live` is what a rebind releases and what a reset frees.
pub trait ShortcutRoute {
    fn live(&self, action: ShortcutAction) -> Option<Shortcut>;
    fn set_route(&self, action: ShortcutAction, shortcut: Option<Shortcut>);
}

/// What happened to the popup after a failed Replace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PopupRestore {
    /// The Replace had no popup in its flow, so there was nothing to bring back.
    NoPopup,
    /// The popup was shown again so the user can retry or copy.
    Restored,
    /// The popup could not be brought back; `problem` is the presenter's
    /// sentence, which the command logs rather than swallowing.
    Failed(String),
}

/// What a Replace left behind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplaceOutcome {
    /// The Selection was replaced; the popup stays hidden.
    Replaced(Replacement),
    /// The Replace failed. `restore` says what became of the popup, so a
    /// popup that could not be brought back is never confused with the Replace
    /// failure itself.
    Failed {
        error: EngineError,
        restore: PopupRestore,
    },
}

/// Hide the popup, run the Replace, and show the popup again when it fails.
///
/// The order is the same for every caller that replaces. A caller with no popup
/// in its flow passes `None`: the operation and its failure still go through
/// this one function, but there is nothing to hide or bring back.
pub fn replace(
    panel: Option<&dyn PanelPresenter>,
    replace: impl FnOnce() -> Result<Replacement, EngineError>,
) -> ReplaceOutcome {
    if let Some(panel) = panel {
        if let Err(problem) = panel.hide() {
            // A popup that stays up must not stop the Replace; it is worth a
            // line because the clipboard fallback would paste into it.
            eprintln!("Neris: the popup was not hidden before Replace: {problem}");
        }
    }
    match replace() {
        Ok(replacement) => ReplaceOutcome::Replaced(replacement),
        Err(error) => {
            let restore = match panel {
                None => PopupRestore::NoPopup,
                Some(panel) => match panel.show() {
                    Ok(()) => PopupRestore::Restored,
                    Err(problem) => PopupRestore::Failed(problem),
                },
            };
            ReplaceOutcome::Failed { error, restore }
        }
    }
}

/// What changing the login item left behind. `enabled` is the login item's
/// actual state, `settings` what the file now holds, and `problem` a sentence
/// when the change could not be made or persisted.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
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
            eprintln!("Neris: the login item was not changed: {error}");
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
        eprintln!("Neris: the login item was not changed: {error}");
        return StartAtLoginOutcome {
            enabled: login_item.is_enabled().unwrap_or(settings.start_at_login),
            settings,
            problem: Some(
                "Neris could not change the login item; it was left as it was.".to_owned(),
            ),
        };
    }
    settings.start_at_login = enabled;
    if let Err(error) = settings_store.save(&settings) {
        eprintln!("Neris: the login item change could not be saved: {error}");
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

/// One binding to make live: what to register and the stored binding it
/// replaces. The pure planner in `shortcuts.rs` decides these.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutChange {
    pub action: ShortcutAction,
    pub binding: String,
    pub shortcut: Shortcut,
    pub previous: Option<Shortcut>,
}

/// What a shortcut change left behind. `problem: None` means the combination
/// is live, routed and in the settings object; any other value is the sentence
/// the settings surface shows, and the old binding is where it was.
#[derive(Clone, Debug)]
pub struct ShortcutApplyOutcome {
    pub binding: String,
    pub problem: Option<String>,
    pub settings: Settings,
    /// True when the registration, the route and the settings all took the new
    /// binding.
    pub applied: bool,
}

/// The one apply sequence: register the new binding, release the old one, point
/// the route at the new one, save the settings, and on a refused save undo all
/// three. A registration macOS refuses is reported before anything changes.
pub fn apply_shortcut(
    registry: &dyn KeyRegistry,
    route: &dyn ShortcutRoute,
    settings_store: &dyn SettingsStore,
    settings: Settings,
    change: ShortcutChange,
) -> ShortcutApplyOutcome {
    let ShortcutChange {
        action,
        binding,
        shortcut,
        previous,
    } = change;

    // 1. Register the new combination. A refusal changes nothing.
    if let Err(error) = registry.register(shortcut) {
        eprintln!("Neris: could not register {binding}: {error}");
        return ShortcutApplyOutcome {
            binding: binding.clone(),
            problem: Some(shortcuts::refused_message(&binding)),
            settings,
            applied: false,
        };
    }

    // 2. Release what the new combination replaces.
    let old = route.live(action).or(previous);
    if let Some(old) = old.filter(|old| !shortcuts::is_the_same(old, &shortcut)) {
        if let Err(error) = registry.unregister(old) {
            eprintln!(
                "Neris: {} was not unregistered: {error}",
                shortcuts::binding_string(old)
            );
        }
    }

    // 3. Point the action at the new combination.
    route.set_route(action, Some(shortcut));

    // 4. Save. A refused write puts the old registration and route back.
    let mut next = settings.clone();
    action.with_binding(&mut next.shortcuts, binding.clone());
    if let Err(error) = settings_store.save(&next) {
        eprintln!("Neris: the shortcut change could not be saved: {error}");
        if let Err(error) = registry.unregister(shortcut) {
            eprintln!(
                "Neris: {} was not unregistered: {error}",
                shortcuts::binding_string(shortcut)
            );
        }
        let restored = match previous {
            Some(old) if registry.register(old).is_ok() => Some(old),
            _ => None,
        };
        route.set_route(action, restored);
        return ShortcutApplyOutcome {
            binding,
            problem: Some(SAVE_FAILED.to_owned()),
            settings,
            applied: false,
        };
    }

    ShortcutApplyOutcome {
        binding,
        problem: None,
        settings: next,
        applied: true,
    }
}

/// The whole rebind: decide with the pure planner, then apply. The planner's
/// refusals (unusable, already in use, no change) never touch a port.
pub fn rebind_shortcut(
    registry: &dyn KeyRegistry,
    route: &dyn ShortcutRoute,
    settings_store: &dyn SettingsStore,
    action: ShortcutAction,
    requested: &str,
) -> Result<ShortcutApplyOutcome, ()> {
    let settings = settings_store.load().map_err(|error| {
        eprintln!("Neris: the shortcut change could not read the settings: {error}");
    })?;
    let plan = shortcuts::plan(action, requested, &settings.shortcuts);
    if let Some(problem) = shortcuts::problem_message(&plan) {
        return Ok(ShortcutApplyOutcome {
            binding: plan.binding().to_owned(),
            problem: Some(problem),
            settings,
            applied: false,
        });
    }

    match plan {
        Plan::Unchanged { binding } => Ok(ShortcutApplyOutcome {
            binding,
            problem: None,
            settings,
            applied: false,
        }),
        Plan::Apply {
            binding,
            shortcut,
            previous,
        } => Ok(apply_shortcut(
            registry,
            route,
            settings_store,
            settings,
            ShortcutChange {
                action,
                binding,
                shortcut,
                previous,
            },
        )),
        Plan::Invalid { .. } | Plan::InUse { .. } => {
            unreachable!("a plan with a problem returned above")
        }
    }
}

/// The settings sink reset drives each shipped default through. The apply
/// sequence's save step lands here, and reset writes the collected object once
/// at the end: a reset is one write by contract, so it cannot leave the file
/// holding some defaults but not others.
struct CollectedSave;

impl SettingsStore for CollectedSave {
    fn load(&self) -> Result<Settings, SettingsError> {
        // The apply sequence is handed the settings it saves; reset never reads
        // through this sink.
        Ok(Settings::default())
    }

    fn save(&self, _settings: &Settings) -> Result<(), SettingsError> {
        Ok(())
    }
}

/// Restore the shipped shortcuts and the default Language Pair. Every live
/// combination is released first so a shipped one another action held is free;
/// each shipped binding then goes through the same apply sequence, and the
/// whole object is written once at the end.
pub fn reset_shortcuts(
    registry: &dyn KeyRegistry,
    route: &dyn ShortcutRoute,
    settings_store: &dyn SettingsStore,
) -> Result<ResetOutcome, ()> {
    let current = settings_store.load().map_err(|error| {
        eprintln!(
            "Neris: the shortcuts could not be reset: the settings file could not be read: {error}"
        );
    })?;

    for action in ShortcutAction::ALL {
        if let Some(live) = route.live(action) {
            if let Err(error) = registry.unregister(live) {
                eprintln!(
                    "Neris: {} was not unregistered: {error}",
                    shortcuts::binding_string(live)
                );
            }
        }
        route.set_route(action, None);
    }

    let defaults = ShortcutBindings::default();
    let collected = CollectedSave;
    let mut settings = shortcuts::settings_after_reset(&current);
    let mut bindings = Vec::with_capacity(ShortcutAction::ALL.len());
    for action in ShortcutAction::ALL {
        let binding = action.binding(&defaults).to_owned();
        match binding.parse::<Shortcut>() {
            Ok(shortcut) => {
                let outcome = apply_shortcut(
                    registry,
                    route,
                    &collected,
                    settings,
                    ShortcutChange {
                        action,
                        binding: binding.clone(),
                        shortcut,
                        previous: None,
                    },
                );
                settings = outcome.settings;
                bindings.push(ShortcutStatus {
                    action,
                    binding,
                    problem: outcome.problem,
                });
            }
            Err(_) => bindings.push(ShortcutStatus {
                action,
                binding,
                problem: Some(INVALID_BINDING.to_owned()),
            }),
        }
    }

    let save_problem = settings_store.save(&settings).err().map(|error| {
        format!(
            "The defaults are live, but the settings file could not be written, so relaunching \
             restores the old settings: {error}"
        )
    });

    Ok(ResetOutcome {
        settings,
        bindings,
        save_problem,
    })
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
pub struct ShellPanelPresenter {
    app: AppHandle,
}

impl ShellPanelPresenter {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

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
pub struct ShellKeyRegistry {
    app: AppHandle,
}

impl ShellKeyRegistry {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

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
pub struct ShellShortcutRoute {
    app: AppHandle,
}

impl ShellShortcutRoute {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl ShortcutRoute for ShellShortcutRoute {
    fn live(&self, action: ShortcutAction) -> Option<Shortcut> {
        self.app.state::<AppState>().route_for(action)
    }

    fn set_route(&self, action: ShortcutAction, shortcut: Option<Shortcut>) {
        self.app.state::<AppState>().set_route(action, shortcut);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::sync::Mutex;
    use translator_core::{
        CaptureError, EngineError, Language, ProviderId, Replacement, Settings, SettingsError,
        Translation,
    };

    fn a_replacement() -> Replacement {
        Replacement {
            source_text: "Xin chào".to_owned(),
            translation: Translation {
                text: "Hello".to_owned(),
                provider: ProviderId::MyMemory,
                detected_source: None,
                target: Language::english(),
            },
        }
    }

    struct FakePanelPresenter {
        refuse_show: bool,
        calls: RefCell<Vec<&'static str>>,
    }

    impl FakePanelPresenter {
        fn new() -> Self {
            Self {
                refuse_show: false,
                calls: RefCell::new(Vec::new()),
            }
        }

        fn refusing_to_show() -> Self {
            Self {
                refuse_show: true,
                ..Self::new()
            }
        }

        fn calls(&self) -> Vec<&'static str> {
            self.calls.borrow().clone()
        }
    }

    impl PanelPresenter for FakePanelPresenter {
        fn hide(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("hide");
            Ok(())
        }

        fn show(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("show");
            if self.refuse_show {
                Err("the panel is gone".to_owned())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn a_successful_replace_hides_the_popup_and_never_shows_it_again() {
        let panel = FakePanelPresenter::new();

        let outcome = replace(Some(&panel), || Ok(a_replacement()));

        assert_eq!(outcome, ReplaceOutcome::Replaced(a_replacement()));
        assert_eq!(panel.calls(), vec!["hide"]);
    }

    #[test]
    fn a_failed_replace_hides_once_and_brings_the_popup_back_once() {
        let panel = FakePanelPresenter::new();
        let error = EngineError::Capture(CaptureError::NotWritable);

        let outcome = replace(Some(&panel), || Err(error.clone()));

        assert_eq!(
            outcome,
            ReplaceOutcome::Failed {
                error,
                restore: PopupRestore::Restored,
            }
        );
        assert_eq!(panel.calls(), vec!["hide", "show"]);
    }

    #[test]
    fn a_popup_that_cannot_come_back_is_reported_beside_the_replace_failure() {
        let panel = FakePanelPresenter::refusing_to_show();
        let error = EngineError::Capture(CaptureError::SecureInput);

        let outcome = replace(Some(&panel), || Err(error.clone()));

        assert_eq!(
            outcome,
            ReplaceOutcome::Failed {
                error,
                restore: PopupRestore::Failed("the panel is gone".to_owned()),
            }
        );
        assert_eq!(panel.calls(), vec!["hide", "show"]);
    }

    #[test]
    fn a_replace_with_no_popup_runs_the_operation_and_never_shows_one() {
        let outcome = replace(None, || Ok(a_replacement()));

        assert_eq!(outcome, ReplaceOutcome::Replaced(a_replacement()));
    }

    #[test]
    fn a_replace_with_no_popup_surfaces_its_failure_without_a_restore() {
        let error = EngineError::Capture(CaptureError::NothingToReplace);

        let outcome = replace(None, || Err(error.clone()));

        assert_eq!(
            outcome,
            ReplaceOutcome::Failed {
                error,
                restore: PopupRestore::NoPopup,
            }
        );
    }

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
            Some("Neris could not change the login item; it was left as it was.")
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

    /// One observable port call. Every fake appends to the same log so a test
    /// can assert the sequence across all three ports at once.
    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Call {
        Register(String),
        Unregister(String),
        Route(ShortcutAction, Option<Shortcut>),
        Save(Settings),
    }

    type Calls = std::sync::Arc<Mutex<Vec<Call>>>;

    fn calls() -> Calls {
        std::sync::Arc::new(Mutex::new(Vec::new()))
    }

    fn log(calls: &Calls) -> Vec<Call> {
        calls.lock().expect("calls lock").clone()
    }

    fn parse(binding: &str) -> Shortcut {
        binding.parse().expect("a valid binding")
    }

    /// A binding in the spelling the workflow stores and the log records.
    fn spelled(binding: &str) -> String {
        shortcuts::binding_string(parse(binding))
    }

    fn settings_with_replace(binding: &str) -> Settings {
        let mut settings = Settings::default();
        settings.shortcuts.replace = spelled(binding);
        settings
    }

    /// The slot a route fake uses for an action, so a test can read it back.
    fn slot(action: ShortcutAction) -> usize {
        ShortcutAction::ALL
            .iter()
            .position(|candidate| *candidate == action)
            .expect("a known action")
    }

    struct FakeKeyRegistry {
        calls: Calls,
        live: RefCell<Vec<Shortcut>>,
        refuse: RefCell<Vec<Shortcut>>,
    }

    impl FakeKeyRegistry {
        fn holding(calls: Calls, live: &[&str]) -> Self {
            Self {
                calls,
                live: RefCell::new(live.iter().map(|binding| parse(binding)).collect()),
                refuse: RefCell::new(Vec::new()),
            }
        }

        fn refusing(calls: Calls, live: &[&str], refused: &[&str]) -> Self {
            Self {
                refuse: RefCell::new(refused.iter().map(|binding| parse(binding)).collect()),
                ..Self::holding(calls, live)
            }
        }

        fn live_bindings(&self) -> Vec<String> {
            self.live
                .borrow()
                .iter()
                .map(|shortcut| shortcuts::binding_string(*shortcut))
                .collect()
        }

        fn refuses(&self, shortcut: &Shortcut) -> bool {
            self.refuse
                .borrow()
                .iter()
                .any(|refused| shortcuts::is_the_same(refused, shortcut))
        }
    }

    impl KeyRegistry for FakeKeyRegistry {
        fn register(&self, shortcut: Shortcut) -> Result<(), String> {
            self.calls
                .lock()
                .expect("calls lock")
                .push(Call::Register(shortcuts::binding_string(shortcut)));
            if self.refuses(&shortcut) {
                return Err("the system refused the combination".to_owned());
            }
            if !self
                .live
                .borrow()
                .iter()
                .any(|live| shortcuts::is_the_same(live, &shortcut))
            {
                self.live.borrow_mut().push(shortcut);
            }
            Ok(())
        }

        fn unregister(&self, shortcut: Shortcut) -> Result<(), String> {
            self.calls
                .lock()
                .expect("calls lock")
                .push(Call::Unregister(shortcuts::binding_string(shortcut)));
            self.live
                .borrow_mut()
                .retain(|live| !shortcuts::is_the_same(live, &shortcut));
            Ok(())
        }
    }

    struct FakeShortcutRoute {
        calls: Calls,
        live: RefCell<[Option<Shortcut>; 4]>,
    }

    impl FakeShortcutRoute {
        fn holding(calls: Calls, seeded: &[(ShortcutAction, &str)]) -> Self {
            let mut live = [None; 4];
            for (action, binding) in seeded {
                live[slot(*action)] = Some(parse(binding));
            }
            Self {
                calls,
                live: RefCell::new(live),
            }
        }

        fn routes(&self) -> [Option<Shortcut>; 4] {
            *self.live.borrow()
        }
    }

    impl ShortcutRoute for FakeShortcutRoute {
        fn live(&self, action: ShortcutAction) -> Option<Shortcut> {
            self.live.borrow()[slot(action)]
        }

        fn set_route(&self, action: ShortcutAction, shortcut: Option<Shortcut>) {
            self.calls
                .lock()
                .expect("calls lock")
                .push(Call::Route(action, shortcut));
            self.live.borrow_mut()[slot(action)] = shortcut;
        }
    }

    struct FakeShortcutStore {
        settings: Mutex<Settings>,
        calls: Calls,
        refuse_load: bool,
        refuse_save: bool,
    }

    impl FakeShortcutStore {
        fn holding(calls: Calls, settings: Settings) -> Self {
            Self {
                settings: Mutex::new(settings),
                calls,
                refuse_load: false,
                refuse_save: false,
            }
        }

        fn refusing_save(calls: Calls, settings: Settings) -> Self {
            Self {
                refuse_save: true,
                ..Self::holding(calls, settings)
            }
        }

        fn refusing_load(calls: Calls) -> Self {
            Self {
                refuse_load: true,
                ..Self::holding(calls, Settings::default())
            }
        }

        fn current(&self) -> Settings {
            self.settings.lock().expect("settings lock").clone()
        }
    }

    impl SettingsStore for FakeShortcutStore {
        fn load(&self) -> Result<Settings, SettingsError> {
            if self.refuse_load {
                return Err(SettingsError::Storage {
                    message: "the settings file is unreadable".to_owned(),
                });
            }
            Ok(self.current())
        }

        fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
            self.calls
                .lock()
                .expect("calls lock")
                .push(Call::Save(settings.clone()));
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
    fn a_successful_rebind_registers_releases_routes_then_saves_in_that_order() {
        let calls = calls();
        let registry = FakeKeyRegistry::holding(calls.clone(), &["Ctrl+Alt+R"]);
        let route =
            FakeShortcutRoute::holding(calls.clone(), &[(ShortcutAction::Replace, "Ctrl+Alt+R")]);
        let store = FakeShortcutStore::holding(calls.clone(), Settings::default());

        let outcome = rebind_shortcut(
            &registry,
            &route,
            &store,
            ShortcutAction::Replace,
            "Ctrl+Alt+M",
        )
        .expect("the settings file is readable");

        assert_eq!(
            log(&calls),
            vec![
                Call::Register(spelled("Ctrl+Alt+M")),
                Call::Unregister(spelled("Ctrl+Alt+R")),
                Call::Route(ShortcutAction::Replace, Some(parse("Ctrl+Alt+M"))),
                Call::Save(settings_with_replace("Ctrl+Alt+M")),
            ]
        );
        assert_eq!(outcome.binding, "Ctrl+Alt+M");
        assert_eq!(outcome.problem, None);
        assert!(outcome.applied);
        assert_eq!(
            registry.live_bindings(),
            vec![spelled("Ctrl+Alt+M")],
            "a successful rebind must leave exactly the new binding registered"
        );
        assert_eq!(store.current(), settings_with_replace("Ctrl+Alt+M"));
    }

    #[test]
    fn a_refused_registration_is_reported_with_the_existing_wording_and_changes_nothing() {
        let calls = calls();
        let registry =
            FakeKeyRegistry::refusing(calls.clone(), &["Ctrl+Alt+R"], &["Ctrl+Alt+M"]);
        let route =
            FakeShortcutRoute::holding(calls.clone(), &[(ShortcutAction::Replace, "Ctrl+Alt+R")]);
        let store = FakeShortcutStore::holding(calls.clone(), Settings::default());

        let outcome = rebind_shortcut(
            &registry,
            &route,
            &store,
            ShortcutAction::Replace,
            "Ctrl+Alt+M",
        )
        .expect("the settings file is readable");

        assert_eq!(outcome.binding, "Ctrl+Alt+M");
        assert_eq!(
            outcome.problem.as_deref(),
            Some("macOS would not give Neris Ctrl+Alt+M. Another app may already be using it.")
        );
        assert!(!outcome.applied);
        assert_eq!(outcome.settings, Settings::default());
        assert_eq!(
            log(&calls),
            vec![Call::Register(spelled("Ctrl+Alt+M"))],
            "a refused registration must not release, route or save"
        );
        assert_eq!(registry.live_bindings(), vec![spelled("Ctrl+Alt+R")]);
        assert_eq!(
            route.routes()[slot(ShortcutAction::Replace)],
            Some(parse("Ctrl+Alt+R"))
        );
    }

    #[test]
    fn a_refused_save_puts_the_old_registration_and_route_back() {
        let calls = calls();
        let registry = FakeKeyRegistry::holding(calls.clone(), &["Ctrl+Alt+R"]);
        let route =
            FakeShortcutRoute::holding(calls.clone(), &[(ShortcutAction::Replace, "Ctrl+Alt+R")]);
        let store = FakeShortcutStore::refusing_save(calls.clone(), Settings::default());

        let outcome = rebind_shortcut(
            &registry,
            &route,
            &store,
            ShortcutAction::Replace,
            "Ctrl+Alt+M",
        )
        .expect("the settings file is readable");

        assert_eq!(
            log(&calls),
            vec![
                Call::Register(spelled("Ctrl+Alt+M")),
                Call::Unregister(spelled("Ctrl+Alt+R")),
                Call::Route(ShortcutAction::Replace, Some(parse("Ctrl+Alt+M"))),
                Call::Save(settings_with_replace("Ctrl+Alt+M")),
                Call::Unregister(spelled("Ctrl+Alt+M")),
                Call::Register(spelled("Ctrl+Alt+R")),
                Call::Route(ShortcutAction::Replace, Some(parse("Ctrl+Alt+R"))),
            ]
        );
        assert_eq!(
            outcome.problem.as_deref(),
            Some("The shortcut was not changed: the settings file could not be written.")
        );
        assert!(!outcome.applied);
        assert_eq!(outcome.settings, Settings::default());
        assert_eq!(registry.live_bindings(), vec![spelled("Ctrl+Alt+R")]);
        assert_eq!(
            route.routes()[slot(ShortcutAction::Replace)],
            Some(parse("Ctrl+Alt+R"))
        );
        assert_eq!(store.current(), Settings::default());
    }

    #[test]
    fn a_plan_that_changes_nothing_does_not_touch_a_port() {
        let calls = calls();
        let registry = FakeKeyRegistry::holding(calls.clone(), &["Ctrl+Alt+R"]);
        let route =
            FakeShortcutRoute::holding(calls.clone(), &[(ShortcutAction::Replace, "Ctrl+Alt+R")]);
        let store = FakeShortcutStore::holding(calls.clone(), Settings::default());

        let outcome = rebind_shortcut(
            &registry,
            &route,
            &store,
            ShortcutAction::Replace,
            "Ctrl+Alt+R",
        )
        .expect("the settings file is readable");

        assert_eq!(outcome.binding, "Ctrl+Alt+R");
        assert_eq!(outcome.problem, None);
        assert!(!outcome.applied);
        assert!(log(&calls).is_empty(), "an unchanged plan touched a port");
    }

    #[test]
    fn a_binding_another_action_owns_never_reaches_the_registry() {
        let calls = calls();
        let registry = FakeKeyRegistry::holding(calls.clone(), &["Ctrl+Alt+R"]);
        let route =
            FakeShortcutRoute::holding(calls.clone(), &[(ShortcutAction::Replace, "Ctrl+Alt+R")]);
        let store = FakeShortcutStore::holding(calls.clone(), Settings::default());

        let outcome = rebind_shortcut(
            &registry,
            &route,
            &store,
            ShortcutAction::QuickTranslate,
            "Ctrl+Alt+R",
        )
        .expect("the settings file is readable");

        assert_eq!(
            outcome.problem.as_deref(),
            Some("That shortcut is already used by Replace.")
        );
        assert!(!outcome.applied);
        assert!(log(&calls).is_empty(), "a refused plan touched a port");
    }

    #[test]
    fn an_unreadable_settings_file_stops_the_rebind_before_a_port_is_touched() {
        let calls = calls();
        let registry = FakeKeyRegistry::holding(calls.clone(), &["Ctrl+Alt+R"]);
        let route =
            FakeShortcutRoute::holding(calls.clone(), &[(ShortcutAction::Replace, "Ctrl+Alt+R")]);
        let store = FakeShortcutStore::refusing_load(calls.clone());

        let outcome = rebind_shortcut(
            &registry,
            &route,
            &store,
            ShortcutAction::Replace,
            "Ctrl+Alt+M",
        );

        assert!(outcome.is_err());
        assert!(log(&calls).is_empty(), "an unreadable file touched a port");
    }

    #[test]
    fn resetting_applies_the_shipped_bindings_through_the_same_sequence_and_saves_once() {
        let calls = calls();
        let custom = ShortcutBindings {
            quick_translate: "Cmd+Shift+K".to_owned(),
            replace: "Cmd+Shift+Q".to_owned(),
            read_aloud: "Cmd+Shift+S".to_owned(),
            translate_window: "Cmd+Shift+W".to_owned(),
        };
        let current = Settings {
            shortcuts: custom.clone(),
            ..Settings::default()
        };
        let registry = FakeKeyRegistry::holding(
            calls.clone(),
            &[
                "Cmd+Shift+K",
                "Cmd+Shift+Q",
                "Cmd+Shift+S",
                "Cmd+Shift+W",
            ],
        );
        let route = FakeShortcutRoute::holding(
            calls.clone(),
            &[
                (ShortcutAction::QuickTranslate, "Cmd+Shift+K"),
                (ShortcutAction::Replace, "Cmd+Shift+Q"),
                (ShortcutAction::ReadAloud, "Cmd+Shift+S"),
                (ShortcutAction::TranslateWindow, "Cmd+Shift+W"),
            ],
        );
        let store = FakeShortcutStore::holding(calls.clone(), current.clone());

        let outcome =
            reset_shortcuts(&registry, &route, &store).expect("the settings file is readable");

        let shipped = ShortcutBindings::default();
        let mut expected = Vec::new();
        for action in ShortcutAction::ALL {
            expected.push(Call::Unregister(spelled(action.binding(&custom))));
            expected.push(Call::Route(action, None));
        }
        for action in ShortcutAction::ALL {
            let binding = action.binding(&shipped);
            expected.push(Call::Register(spelled(binding)));
            expected.push(Call::Route(action, Some(parse(binding))));
        }
        expected.push(Call::Save(shortcuts::settings_after_reset(&current)));

        assert_eq!(log(&calls), expected);
        assert_eq!(
            registry.live_bindings(),
            ShortcutAction::ALL
                .iter()
                .map(|action| spelled(action.binding(&shipped)))
                .collect::<Vec<_>>()
        );
        assert_eq!(outcome.settings, shortcuts::settings_after_reset(&current));
        assert_eq!(outcome.save_problem, None);
        assert!(outcome
            .bindings
            .iter()
            .all(|status| status.problem.is_none()));
        assert_eq!(store.current().shortcuts, ShortcutBindings::default());
        assert_eq!(
            log(&calls)
                .into_iter()
                .filter(|call| matches!(call, Call::Save(_)))
                .count(),
            1,
            "a reset must write the settings file once"
        );
    }

    #[test]
    fn resetting_reports_a_shipped_binding_the_system_refuses() {
        let calls = calls();
        let registry =
            FakeKeyRegistry::refusing(calls.clone(), &["Ctrl+Alt+R"], &["Ctrl+Alt+R"]);
        let route =
            FakeShortcutRoute::holding(calls.clone(), &[(ShortcutAction::Replace, "Ctrl+Alt+R")]);
        let store = FakeShortcutStore::holding(calls.clone(), Settings::default());

        let outcome =
            reset_shortcuts(&registry, &route, &store).expect("the settings file is readable");

        let replace = outcome
            .bindings
            .iter()
            .find(|status| status.action == ShortcutAction::Replace)
            .expect("Replace's outcome");
        assert_eq!(
            replace.problem.as_deref(),
            Some(
                "macOS would not give Neris Ctrl+Alt+R. Another app may already be using it."
            )
        );
        assert_eq!(
            route.routes()[slot(ShortcutAction::Replace)],
            None,
            "a shipped binding the system refused must not be routed"
        );
        assert!(outcome
            .bindings
            .iter()
            .filter(|status| status.action != ShortcutAction::Replace)
            .all(|status| status.problem.is_none()));
        assert_eq!(outcome.save_problem, None);
    }

    #[test]
    fn an_unreadable_settings_file_stops_the_reset_before_a_port_is_touched() {
        let calls = calls();
        let registry = FakeKeyRegistry::holding(calls.clone(), &["Ctrl+Alt+R"]);
        let route =
            FakeShortcutRoute::holding(calls.clone(), &[(ShortcutAction::Replace, "Ctrl+Alt+R")]);
        let store = FakeShortcutStore::refusing_load(calls.clone());

        assert!(reset_shortcuts(&registry, &route, &store).is_err());
        assert!(log(&calls).is_empty(), "an unreadable file touched a port");
    }

    #[test]
    fn the_login_item_outcome_serializes_as_the_shell_returns_it() {
        // The command returns this value to the UI directly, so its field names
        // are the wire contract.
        let outcome = StartAtLoginOutcome {
            enabled: true,
            settings: preference(true),
            problem: Some("left as it was".to_owned()),
        };

        let value = serde_json::to_value(&outcome).expect("the outcome serializes");

        assert_eq!(value["enabled"], serde_json::json!(true));
        assert_eq!(value["problem"], serde_json::json!("left as it was"));
        assert_eq!(
            value["settings"],
            serde_json::to_value(preference(true)).expect("the settings serialize")
        );
    }
}
