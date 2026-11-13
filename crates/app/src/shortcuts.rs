//! Rebindable global shortcuts: whether a combination can be used, and the one
//! place that turns a binding into a live registration and a saved setting.
//!
//! `main.rs` owns the process-wide state (the live routes and the tray item);
//! the rules live here so they are testable without a window, a Carbon hotkey or
//! a settings file.

use std::sync::mpsc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
use translator_core::{LanguagePair, Settings, ShortcutBindings};

use crate::AppState;

/// What a combination that cannot be used says. It never guesses why.
pub const INVALID_BINDING: &str = "That combination cannot be used as a shortcut.";

/// A modifier-less combination is legal for the plugin's parser but not
/// acceptable as a global shortcut: a bare letter would swallow that key in
/// every app.
pub const MODIFIERS_REQUIRED: &str = "A shortcut needs at least one of Ctrl, Alt, Cmd or Shift.";

/// The settings file could not be written, so the old registration was put back.
pub const SAVE_FAILED: &str =
    "The shortcut was not changed: the settings file could not be written.";

/// The four actions a global shortcut can trigger. The wire form is the field
/// name in `ShortcutBindings`, so the UI and the settings file share one word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    QuickTranslate,
    Replace,
    ReadAloud,
    TranslateWindow,
}

impl ShortcutAction {
    pub const ALL: [ShortcutAction; 4] = [
        ShortcutAction::QuickTranslate,
        ShortcutAction::Replace,
        ShortcutAction::ReadAloud,
        ShortcutAction::TranslateWindow,
    ];

    /// How the action is named in a message the user reads.
    pub fn label(self) -> &'static str {
        match self {
            ShortcutAction::QuickTranslate => "Quick Translate",
            ShortcutAction::Replace => "Replace",
            ShortcutAction::ReadAloud => "Read Aloud",
            ShortcutAction::TranslateWindow => "Translate Window",
        }
    }

    /// The stored binding for this action.
    pub fn binding(self, bindings: &ShortcutBindings) -> &str {
        match self {
            ShortcutAction::QuickTranslate => &bindings.quick_translate,
            ShortcutAction::Replace => &bindings.replace,
            ShortcutAction::ReadAloud => &bindings.read_aloud,
            ShortcutAction::TranslateWindow => &bindings.translate_window,
        }
    }

    fn with_binding(self, bindings: &mut ShortcutBindings, value: String) {
        match self {
            ShortcutAction::QuickTranslate => bindings.quick_translate = value,
            ShortcutAction::Replace => bindings.replace = value,
            ShortcutAction::ReadAloud => bindings.read_aloud = value,
            ShortcutAction::TranslateWindow => bindings.translate_window = value,
        }
    }
}

/// What should happen to one requested binding. Pure: it reads the saved
/// bindings and answers, so every rule is testable without Tauri.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    /// The binding cannot be used at all; `problem` says why.
    Invalid { binding: String, problem: String },
    /// Another action already owns this combination.
    InUse {
        binding: String,
        other: ShortcutAction,
    },
    /// The combination is already this action's binding.
    Unchanged { binding: String },
    /// Register the new combination, replacing `previous` when one was stored.
    Apply {
        binding: String,
        shortcut: Shortcut,
        previous: Option<Shortcut>,
    },
}

/// Decide what a requested binding means for `action`. Combinations are compared
/// as modifiers and key, never as strings, so `Ctrl`/`Control` and `Cmd`/`Super`
/// cannot slip past the duplicate check.
pub fn plan(action: ShortcutAction, requested: &str, current: &ShortcutBindings) -> Plan {
    let requested = requested.trim();
    let Ok(shortcut) = requested.parse::<Shortcut>() else {
        return Plan::Invalid {
            binding: requested.to_owned(),
            problem: INVALID_BINDING.to_owned(),
        };
    };

    if !has_modifier(&shortcut) {
        return Plan::Invalid {
            binding: binding_string(shortcut),
            problem: MODIFIERS_REQUIRED.to_owned(),
        };
    }

    for other in ShortcutAction::ALL {
        if other == action {
            continue;
        }
        if let Ok(theirs) = other.binding(current).parse::<Shortcut>() {
            if is_the_same(&shortcut, &theirs) {
                return Plan::InUse {
                    binding: binding_string(shortcut),
                    other,
                };
            }
        }
    }

    let previous = action.binding(current).parse::<Shortcut>().ok();
    let binding = binding_string(shortcut);
    match previous {
        Some(previous) if is_the_same(&shortcut, &previous) => Plan::Unchanged { binding },
        previous => Plan::Apply {
            binding,
            shortcut,
            previous,
        },
    }
}

impl Plan {
    /// The combination the plan is about, as it is stored and shown.
    pub fn binding(&self) -> &str {
        match self {
            Plan::Invalid { binding, .. }
            | Plan::InUse { binding, .. }
            | Plan::Unchanged { binding }
            | Plan::Apply { binding, .. } => binding,
        }
    }
}

/// The user-facing problem a plan carries, if any. `Apply` has none: it only
/// fails later, when the platform refuses the registration.
pub fn problem_message(plan: &Plan) -> Option<String> {
    match plan {
        Plan::Invalid { problem, .. } => Some(problem.clone()),
        Plan::InUse { other, .. } => Some(format!(
            "That shortcut is already used by {}.",
            other.label()
        )),
        Plan::Unchanged { .. } | Plan::Apply { .. } => None,
    }
}

/// What macOS refusing a registration says. It never claims to know the owner:
/// Carbon reports the conflict but not which app holds the combination.
pub fn refused_message(binding: &str) -> String {
    format!("macOS would not give Translator {binding}. Another app may already be using it.")
}

/// A binding as it is stored and shown: `Cmd+Ctrl+Alt+Shift+<key>`, with the key
/// in the plugin's short spelling (`D`, `1`, `F5`). Storing one spelling means
/// the settings file and the tray label read the same.
pub fn binding_string(shortcut: Shortcut) -> String {
    let mut parts = Vec::new();
    if shortcut.mods.contains(Modifiers::SUPER) {
        parts.push("Cmd".to_owned());
    }
    if shortcut.mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl".to_owned());
    }
    if shortcut.mods.contains(Modifiers::ALT) {
        parts.push("Alt".to_owned());
    }
    if shortcut.mods.contains(Modifiers::SHIFT) {
        parts.push("Shift".to_owned());
    }
    parts.push(key_token(shortcut.key));
    parts.join("+")
}

/// `Code::KeyD` → `D`, `Code::Digit1` → `1`; anything else keeps the name the
/// parser already accepts (`F5`, `Space`, `ArrowUp`).
fn key_token(code: Code) -> String {
    let name = code.to_string();
    if let Some(letter) = name.strip_prefix("Key") {
        return letter.to_owned();
    }
    if let Some(digit) = name.strip_prefix("Digit") {
        return digit.to_owned();
    }
    name
}

fn has_modifier(shortcut: &Shortcut) -> bool {
    shortcut
        .mods
        .intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::SUPER)
}

/// Compare the combination, not the plugin's registration id, which is derived
/// the same way but is not what the user chose.
fn is_the_same(one: &Shortcut, other: &Shortcut) -> bool {
    one.mods == other.mods && one.key == other.key
}

/// The shortcuts that registered successfully. A binding that could not be
/// registered is absent, so pressing it does nothing instead of firing the wrong
/// action.
#[derive(Clone, Copy, Default)]
pub struct ShortcutRoutes {
    quick_translate: Option<Shortcut>,
    replace: Option<Shortcut>,
    translate_window: Option<Shortcut>,
    read_aloud: Option<Shortcut>,
}

impl ShortcutRoutes {
    pub fn get(&self, action: ShortcutAction) -> Option<Shortcut> {
        match action {
            ShortcutAction::QuickTranslate => self.quick_translate,
            ShortcutAction::Replace => self.replace,
            ShortcutAction::ReadAloud => self.read_aloud,
            ShortcutAction::TranslateWindow => self.translate_window,
        }
    }

    pub fn set(&mut self, action: ShortcutAction, shortcut: Option<Shortcut>) {
        match action {
            ShortcutAction::QuickTranslate => self.quick_translate = shortcut,
            ShortcutAction::Replace => self.replace = shortcut,
            ShortcutAction::ReadAloud => self.read_aloud = shortcut,
            ShortcutAction::TranslateWindow => self.translate_window = shortcut,
        }
    }

    /// Which action a live combination fires. The one global handler routes on
    /// this because one plugin owns every binding.
    pub fn route(&self, shortcut: &Shortcut) -> Option<ShortcutAction> {
        ShortcutAction::ALL.into_iter().find(|action| {
            self.get(*action)
                .is_some_and(|live| is_the_same(&live, shortcut))
        })
    }
}

/// What happened to a rebind. `problem: None` means the combination is live and
/// saved; any other value is words for the settings surface, and the stored
/// binding is unchanged.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RebindResult {
    pub binding: String,
    pub problem: Option<String>,
}

/// Register one stored binding at launch, answering the live shortcut or `None`
/// when the parser or macOS refuses it.
pub fn register_binding(app: &AppHandle, binding: &str) -> Option<Shortcut> {
    match binding.parse::<Shortcut>() {
        Ok(shortcut) => match register_on_main_thread(app, shortcut) {
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

/// Apply one rebind: decide, register on the main thread, unregister what it
/// replaces, then save. A save that fails puts the old registration back, so a
/// binding never works now and vanishes at relaunch.
pub fn rebind_shortcut(
    app: &AppHandle,
    action: ShortcutAction,
    requested: &str,
) -> Result<RebindResult, ()> {
    let state = app.state::<AppState>();
    let mut settings = state.engine.settings().map_err(|_| ())?;

    let plan = plan(action, requested, &settings.shortcuts);
    if let Some(problem) = problem_message(&plan) {
        return Ok(RebindResult {
            binding: plan.binding().to_owned(),
            problem: Some(problem),
        });
    }

    let (binding, shortcut, previous) = match plan {
        Plan::Unchanged { binding } => {
            return Ok(RebindResult {
                binding,
                problem: None,
            })
        }
        Plan::Apply {
            binding,
            shortcut,
            previous,
        } => (binding, shortcut, previous),
        Plan::Invalid { .. } | Plan::InUse { .. } => {
            unreachable!("a plan with a problem returned above")
        }
    };

    let live = state.route_for(action);
    if let Err(error) = register_on_main_thread(app, shortcut) {
        eprintln!("Translator: could not register {binding}: {error}");
        return Ok(RebindResult {
            binding: binding.clone(),
            problem: Some(refused_message(&binding)),
        });
    }

    if let Some(old) = live.or(previous) {
        if !is_the_same(&old, &shortcut) {
            unregister_on_main_thread(app, old);
        }
    }
    state.set_route(action, Some(shortcut));

    action.with_binding(&mut settings.shortcuts, binding.clone());
    if state.engine.save_settings(settings).is_err() {
        unregister_on_main_thread(app, shortcut);
        let restored = match previous {
            Some(old) if register_on_main_thread(app, old).is_ok() => Some(old),
            _ => None,
        };
        state.set_route(action, restored);
        return Ok(RebindResult {
            binding,
            problem: Some(SAVE_FAILED.to_owned()),
        });
    }

    if action == ShortcutAction::TranslateWindow {
        set_tray_accelerator(app, Some(binding.clone()));
    }
    Ok(RebindResult {
        binding,
        problem: None,
    })
}

/// One action's outcome after a reset.
#[derive(Clone, Debug, Serialize)]
pub struct ShortcutStatus {
    pub action: ShortcutAction,
    pub binding: String,
    pub problem: Option<String>,
}

/// What a reset leaves behind: the settings now on disk (or in memory when the
/// write failed) and what each shipped combination did.
#[derive(Clone, Debug, Serialize)]
pub struct ResetOutcome {
    pub settings: Settings,
    pub bindings: Vec<ShortcutStatus>,
    /// A failed settings write. The live shortcuts changed but relaunching
    /// restores the previous file, so the surface has to say so.
    pub save_problem: Option<String>,
}

/// The settings a reset writes: the shipped pair and shortcuts, with everything
/// else — region, start at login, onboarding — untouched.
pub fn settings_after_reset(current: &Settings) -> Settings {
    Settings {
        language_pair: LanguagePair::default(),
        shortcuts: ShortcutBindings::default(),
        ..current.clone()
    }
}

/// Restore the four shipped shortcuts and the default pair. Every live shortcut
/// is unregistered first, so a shipped combination that a previous binding
/// occupied is free. A binding macOS refuses is reported, not skipped.
pub fn reset_shortcuts_and_language_pair(app: &AppHandle) -> Result<ResetOutcome, ()> {
    let state = app.state::<AppState>();
    let current = state.engine.settings().map_err(|_| ())?;

    for action in ShortcutAction::ALL {
        if let Some(live) = state.route_for(action) {
            unregister_on_main_thread(app, live);
        }
        state.set_route(action, None);
    }

    let defaults = ShortcutBindings::default();
    let mut bindings = Vec::new();
    for action in ShortcutAction::ALL {
        let binding = action.binding(&defaults).to_owned();
        let problem = match binding.parse::<Shortcut>() {
            Ok(shortcut) => match register_on_main_thread(app, shortcut) {
                Ok(()) => {
                    state.set_route(action, Some(shortcut));
                    None
                }
                Err(error) => {
                    eprintln!("Translator: could not register {binding}: {error}");
                    Some(refused_message(&binding))
                }
            },
            Err(_) => Some(INVALID_BINDING.to_owned()),
        };
        bindings.push(ShortcutStatus {
            action,
            binding,
            problem,
        });
    }

    let open = bindings
        .iter()
        .find(|status| status.action == ShortcutAction::TranslateWindow);
    set_tray_accelerator(
        app,
        open.and_then(|status| status.problem.is_none().then(|| status.binding.clone())),
    );

    let settings = settings_after_reset(&current);
    let save_problem = state
        .engine
        .save_settings(settings.clone())
        .err()
        .map(|error| {
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

/// Keep the tray's “Open Translate Window” accelerator in step with the live
/// binding. The tray is built in `main.rs`; this module only updates it. The
/// item is an AppKit object, so the update is posted to the main thread like the
/// registrations are.
pub fn set_tray_accelerator(app: &AppHandle, binding: Option<String>) {
    let outcome = on_main_thread(app, move |handle| {
        let state = handle.state::<AppState>();
        let item = state.tray_open.lock().expect("tray item lock");
        let Some(item) = item.as_ref() else {
            return;
        };
        if let Err(error) = item.set_accelerator(binding.as_deref()) {
            eprintln!("Translator: the tray shortcut label was not updated: {error}");
        }
    });
    if outcome.is_none() {
        eprintln!(
            "Translator: the tray shortcut label was not updated: the main thread did not answer"
        );
    }
}

/// Register on the main thread and wait for the answer: Carbon registration
/// belongs there, and the caller must know whether it succeeded before it
/// unregisters what the new combination replaces.
fn register_on_main_thread(app: &AppHandle, shortcut: Shortcut) -> Result<(), String> {
    match on_main_thread(app, move |handle| {
        handle
            .global_shortcut()
            .register(shortcut)
            .map_err(|error| error.to_string())
    }) {
        Some(result) => result,
        None => Err("the main thread did not answer".to_owned()),
    }
}

fn unregister_on_main_thread(app: &AppHandle, shortcut: Shortcut) {
    let outcome = on_main_thread(app, move |handle| {
        handle
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|error| error.to_string())
    });
    match outcome {
        Some(Ok(())) => {}
        Some(Err(error)) => {
            eprintln!("Translator: {shortcut} was not unregistered: {error}")
        }
        None => {
            eprintln!("Translator: {shortcut} was not unregistered: the main thread did not answer")
        }
    }
}

/// Run `operation` on the main thread and wait for its answer. The wait is
/// bounded so a shell that cannot reach the main thread reports a failure
/// instead of hanging the Settings surface.
fn on_main_thread<R: Send + 'static>(
    app: &AppHandle,
    operation: impl FnOnce(&AppHandle) -> R + Send + 'static,
) -> Option<R> {
    let (finished, wait) = mpsc::channel();
    let handle = app.clone();
    app.run_on_main_thread(move || {
        let _ = finished.send(operation(&handle));
    })
    .ok()?;
    wait.recv_timeout(Duration::from_secs(2)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use translator_core::{Language, ShortcutBindings};

    fn defaults() -> ShortcutBindings {
        ShortcutBindings::default()
    }

    fn parse(binding: &str) -> Shortcut {
        binding.parse().expect("a valid binding")
    }

    #[test]
    fn the_shipped_bindings_are_four_distinct_combinations() {
        let parsed: Vec<Shortcut> = ShortcutAction::ALL
            .iter()
            .map(|action| parse(action.binding(&defaults())))
            .collect();

        for (index, one) in parsed.iter().enumerate() {
            for other in &parsed[index + 1..] {
                assert_ne!(
                    (one.mods, one.key),
                    (other.mods, other.key),
                    "two shipped shortcuts share a combination"
                );
            }
        }
    }

    #[test]
    fn a_combination_another_action_owns_names_that_action() {
        assert_eq!(
            plan(ShortcutAction::QuickTranslate, "Ctrl+Alt+R", &defaults()),
            Plan::InUse {
                binding: "Ctrl+Alt+R".to_owned(),
                other: ShortcutAction::Replace,
            }
        );
        assert_eq!(
            problem_message(&Plan::InUse {
                binding: "Ctrl+Alt+R".to_owned(),
                other: ShortcutAction::Replace,
            })
            .as_deref(),
            Some("That shortcut is already used by Replace.")
        );
    }

    #[test]
    fn synonyms_compare_by_combination_not_by_spelling() {
        let mut bindings = defaults();
        // Control/Option/KeyR is the same combination as Ctrl+Alt+R.
        bindings.replace = "Control+Option+KeyR".to_owned();
        assert_eq!(
            plan(ShortcutAction::QuickTranslate, "Ctrl+Alt+R", &bindings),
            Plan::InUse {
                binding: "Ctrl+Alt+R".to_owned(),
                other: ShortcutAction::Replace,
            }
        );

        // Cmd and Super are also the same modifier.
        bindings.replace = "Super+Shift+K".to_owned();
        assert_eq!(
            plan(ShortcutAction::QuickTranslate, "Cmd+Shift+K", &bindings),
            Plan::InUse {
                binding: "Cmd+Shift+K".to_owned(),
                other: ShortcutAction::Replace,
            }
        );
    }

    #[test]
    fn the_same_combination_in_another_spelling_is_a_no_op() {
        let mut bindings = defaults();
        bindings.quick_translate = "Control+Option+KeyD".to_owned();

        assert_eq!(
            plan(ShortcutAction::QuickTranslate, "Ctrl+Alt+D", &bindings),
            Plan::Unchanged {
                binding: "Ctrl+Alt+D".to_owned(),
            }
        );
        assert_eq!(
            problem_message(&plan(
                ShortcutAction::QuickTranslate,
                "Ctrl+Alt+D",
                &bindings
            )),
            None
        );
    }

    #[test]
    fn a_blank_binding_is_invalid() {
        for blank in ["", "   ", "\n\t"] {
            assert_eq!(
                plan(ShortcutAction::QuickTranslate, blank, &defaults()),
                Plan::Invalid {
                    binding: String::new(),
                    problem: INVALID_BINDING.to_owned(),
                }
            );
        }
    }

    #[test]
    fn an_unparseable_binding_is_invalid() {
        assert_eq!(
            plan(
                ShortcutAction::QuickTranslate,
                "Ctrl+Alt+Banana",
                &defaults()
            ),
            Plan::Invalid {
                binding: "Ctrl+Alt+Banana".to_owned(),
                problem: INVALID_BINDING.to_owned(),
            }
        );
        assert_eq!(
            plan(ShortcutAction::QuickTranslate, "Ctrl+Shift", &defaults()),
            Plan::Invalid {
                binding: "Ctrl+Shift".to_owned(),
                problem: INVALID_BINDING.to_owned(),
            }
        );
    }

    #[test]
    fn a_binding_without_a_modifier_is_rejected_with_a_reason() {
        assert_eq!(
            plan(ShortcutAction::QuickTranslate, "K", &defaults()),
            Plan::Invalid {
                binding: "K".to_owned(),
                problem: MODIFIERS_REQUIRED.to_owned(),
            }
        );
        assert_eq!(
            plan(ShortcutAction::Replace, "F5", &defaults()),
            Plan::Invalid {
                binding: "F5".to_owned(),
                problem: MODIFIERS_REQUIRED.to_owned(),
            }
        );
    }

    #[test]
    fn a_new_combination_plans_registration_and_replaces_the_old_one() {
        assert_eq!(
            plan(ShortcutAction::Replace, "Ctrl+Alt+M", &defaults()),
            Plan::Apply {
                binding: "Ctrl+Alt+M".to_owned(),
                shortcut: parse("Ctrl+Alt+M"),
                previous: Some(parse("Ctrl+Alt+R")),
            }
        );
    }

    #[test]
    fn bindings_the_capture_ui_generates_parse_back_through_the_real_parser() {
        // The capture UI emits these spellings; the plugin must accept them and
        // the stored canonical spelling must parse again.
        for (typed, canonical) in [
            ("Ctrl+Alt+K", "Ctrl+Alt+K"),
            ("Cmd+Shift+F5", "Cmd+Shift+F5"),
            ("Ctrl+Alt+Digit1", "Ctrl+Alt+1"),
            ("Cmd+Ctrl+Alt+Shift+KeyZ", "Cmd+Ctrl+Alt+Shift+Z"),
        ] {
            let parsed = parse(typed);
            assert_eq!(binding_string(parsed), canonical);
            let reparsed = parse(canonical);
            assert_eq!(
                (reparsed.mods, reparsed.key),
                (parsed.mods, parsed.key),
                "{typed} did not survive the canonical round trip"
            );
        }
    }

    #[test]
    fn a_refused_registration_never_names_an_owner() {
        assert_eq!(
            refused_message("Cmd+Shift+K"),
            "macOS would not give Translator Cmd+Shift+K. Another app may already be using it."
        );
    }

    #[test]
    fn each_registered_shortcut_routes_to_its_own_action() {
        let routes = ShortcutRoutes {
            quick_translate: Some(parse("Ctrl+Alt+D")),
            replace: Some(parse("Ctrl+Alt+R")),
            translate_window: Some(parse("Ctrl+Alt+E")),
            read_aloud: Some(parse("Ctrl+Alt+S")),
        };

        assert_eq!(
            routes.route(&parse("Ctrl+Alt+D")),
            Some(ShortcutAction::QuickTranslate)
        );
        assert_eq!(
            routes.route(&parse("Ctrl+Alt+R")),
            Some(ShortcutAction::Replace)
        );
        assert_eq!(
            routes.route(&parse("Ctrl+Alt+E")),
            Some(ShortcutAction::TranslateWindow)
        );
        assert_eq!(
            routes.route(&parse("Ctrl+Alt+S")),
            Some(ShortcutAction::ReadAloud)
        );
        // Synonyms route too: the plugin hands back its own spelling.
        assert_eq!(
            routes.route(&parse("Control+Option+KeyE")),
            Some(ShortcutAction::TranslateWindow)
        );
    }

    #[test]
    fn a_shortcut_that_did_not_register_routes_nowhere() {
        let mut routes = ShortcutRoutes::default();
        routes.set(ShortcutAction::TranslateWindow, Some(parse("Ctrl+Alt+E")));

        assert_eq!(routes.route(&parse("Ctrl+Alt+S")), None);
        assert_eq!(routes.route(&parse("Ctrl+Alt+D")), None);
        assert_eq!(
            routes.route(&parse("Ctrl+Alt+E")),
            Some(ShortcutAction::TranslateWindow)
        );
    }

    #[test]
    fn reset_restores_the_shipped_pair_and_shortcuts_and_keeps_everything_else() {
        let current = Settings {
            language_pair: LanguagePair::new(Language::vietnamese(), Language::english()),
            shortcuts: ShortcutBindings {
                quick_translate: "Cmd+Shift+K".to_owned(),
                ..ShortcutBindings::default()
            },
            start_at_login: true,
            azure_region: "westeurope".to_owned(),
            onboarding_seen: true,
        };

        let reset = settings_after_reset(&current);

        assert_eq!(reset.language_pair, LanguagePair::default());
        assert_eq!(reset.shortcuts, ShortcutBindings::default());
        assert_eq!(reset.azure_region, "westeurope");
        assert!(reset.start_at_login);
        assert!(reset.onboarding_seen);
    }
}
