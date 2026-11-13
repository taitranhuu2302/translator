//! The menu bar item's contents. The entries are pure data so the words and
//! accelerators can be tested without a window; this module also turns them
//! into the real AppKit menu and swaps it in when the bindings or the
//! Accessibility permission change.

use std::time::Duration;

use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Manager};
use translator_core::ShortcutBindings;

use crate::main_thread::on_main_thread;
use crate::shortcuts::ShortcutAction;
use crate::AppState;

/// A menu swap is an AppKit call from a worker thread, so it is given the same
/// bounded wait as the shortcut registrations.
const MENU_TIMEOUT: Duration = Duration::from_secs(2);

/// The tray icon's id. `refresh_tray` finds the icon by it.
pub const TRAY_ID: &str = "translator";

/// The separator entry's id. It has no label and cannot be clicked.
pub const SEPARATOR_ID: &str = "separator";

/// The accessibility row while the permission is granted. It is informational.
pub const ACCESSIBILITY_GRANTED_ID: &str = "accessibility-granted";

/// The accessibility row while the permission is missing. Clicking it opens
/// System Settings at the pane where it is granted.
pub const ACCESSIBILITY_NOT_GRANTED_ID: &str = "accessibility-not-granted";

/// Quit. It is the last row, as menu-bar apps put it.
pub const QUIT_ID: &str = "quit";

/// One row of the menu. A separator has [`SEPARATOR_ID`] as its id, an empty
/// label and no accelerator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayEntry {
    pub id: String,
    pub label: String,
    pub accelerator: Option<String>,
}

impl TrayEntry {
    pub fn is_separator(&self) -> bool {
        self.id == SEPARATOR_ID
    }

    fn separator() -> Self {
        Self {
            id: SEPARATOR_ID.to_owned(),
            label: String::new(),
            accelerator: None,
        }
    }
}

/// The id the menu event handler receives for an action. It is the same word the
/// settings file and the IPC use.
pub fn action_id(action: ShortcutAction) -> &'static str {
    match action {
        ShortcutAction::QuickTranslate => "quick_translate",
        ShortcutAction::Replace => "replace",
        ShortcutAction::ReadAloud => "read_aloud",
        ShortcutAction::TranslateWindow => "translate_window",
    }
}

/// Which action a clicked menu row fires.
pub fn action_for_id(id: &str) -> Option<ShortcutAction> {
    ShortcutAction::ALL
        .into_iter()
        .find(|action| action_id(*action) == id)
}

/// The row's words. The window action is named by what it does, as the menu
/// reads better as a verb than as a synonym for the window.
fn action_label(action: ShortcutAction) -> &'static str {
    match action {
        ShortcutAction::Replace => "Replace Selection",
        ShortcutAction::TranslateWindow => "Open Translate Window",
        other => other.label(),
    }
}

/// The whole menu, top to bottom: the four actions with their current
/// combinations, the accessibility state, and Quit. `granted` is the permission
/// as the app last observed it.
pub fn tray_entries(bindings: &ShortcutBindings, granted: bool) -> Vec<TrayEntry> {
    let mut entries = Vec::with_capacity(8);
    for action in ShortcutAction::ALL {
        entries.push(TrayEntry {
            id: action_id(action).to_owned(),
            label: action_label(action).to_owned(),
            accelerator: Some(action.binding(bindings).to_owned()),
        });
    }
    entries.push(TrayEntry::separator());
    entries.push(if granted {
        TrayEntry {
            id: ACCESSIBILITY_GRANTED_ID.to_owned(),
            label: "Accessibility: Granted".to_owned(),
            accelerator: None,
        }
    } else {
        TrayEntry {
            id: ACCESSIBILITY_NOT_GRANTED_ID.to_owned(),
            label: "Accessibility: Not granted — Open Settings…".to_owned(),
            accelerator: None,
        }
    });
    entries.push(TrayEntry::separator());
    entries.push(TrayEntry {
        id: QUIT_ID.to_owned(),
        label: "Quit Translator".to_owned(),
        accelerator: None,
    });
    entries
}

/// Build the real menu from [`tray_entries`]. A binding macOS's menu parser
/// rejects still gets a row, just without its key equivalent: a hand-edited
/// settings file must not cost the app its menu bar item.
pub fn menu(
    app: &AppHandle,
    bindings: &ShortcutBindings,
    granted: bool,
) -> tauri::Result<Menu<tauri::Wry>> {
    let mut items: Vec<Box<dyn IsMenuItem<tauri::Wry>>> = Vec::new();
    for entry in tray_entries(bindings, granted) {
        if entry.is_separator() {
            items.push(Box::new(PredefinedMenuItem::separator(app)?));
            continue;
        }
        let item = match MenuItem::with_id(
            app,
            &entry.id,
            &entry.label,
            true,
            entry.accelerator.as_deref(),
        ) {
            Ok(item) => item,
            Err(error) => {
                eprintln!(
                    "Translator: the tray row for {} has no key equivalent: {error}",
                    entry.id
                );
                MenuItem::with_id(app, &entry.id, &entry.label, true, None::<&str>)?
            }
        };
        items.push(Box::new(item));
    }
    let refs: Vec<&dyn IsMenuItem<tauri::Wry>> = items.iter().map(|item| &**item).collect();
    Menu::with_items(app, &refs)
}

/// Replace the tray icon's menu with one built from the live bindings. It runs
/// on the main thread because both the menu and the icon are AppKit objects,
/// and it is called after launch, after a rebind or reset, and when the
/// Accessibility permission is observed to change.
pub fn refresh_tray(app: &AppHandle, bindings: &ShortcutBindings) {
    let bindings = bindings.clone();
    let outcome = on_main_thread(app, MENU_TIMEOUT, move |handle| {
        let Some(icon) = handle.tray_by_id(TRAY_ID) else {
            eprintln!("Translator: the tray menu was not updated: the menu bar item is missing");
            return;
        };
        let granted = handle.state::<AppState>().engine.accessibility_granted();
        let menu = match menu(handle, &bindings, granted) {
            Ok(menu) => menu,
            Err(error) => {
                eprintln!("Translator: the tray menu was not built: {error}");
                return;
            }
        };
        if let Err(error) = icon.set_menu(Some(menu)) {
            eprintln!("Translator: the tray menu was not updated: {error}");
        }
    });
    if outcome.is_none() {
        eprintln!("Translator: the tray menu was not updated: the main thread did not answer");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> ShortcutBindings {
        ShortcutBindings::default()
    }

    #[test]
    fn the_four_actions_carry_the_live_bindings_and_read_as_the_menu_should() {
        let bindings = ShortcutBindings {
            quick_translate: "Cmd+Shift+K".to_owned(),
            replace: "Ctrl+Alt+M".to_owned(),
            read_aloud: "Cmd+Shift+J".to_owned(),
            translate_window: "Cmd+Shift+O".to_owned(),
        };

        let entries = tray_entries(&bindings, true);
        let actions: Vec<&TrayEntry> = entries
            .iter()
            .filter(|entry| action_for_id(&entry.id).is_some())
            .collect();

        assert_eq!(
            actions
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "quick_translate",
                "replace",
                "read_aloud",
                "translate_window"
            ]
        );
        assert_eq!(
            actions
                .iter()
                .map(|entry| entry.label.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Quick Translate",
                "Replace Selection",
                "Read Aloud",
                "Open Translate Window"
            ]
        );
        assert_eq!(
            actions
                .iter()
                .map(|entry| entry.accelerator.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("Cmd+Shift+K"),
                Some("Ctrl+Alt+M"),
                Some("Cmd+Shift+J"),
                Some("Cmd+Shift+O"),
            ]
        );
    }

    #[test]
    fn a_shipped_menu_shows_the_shipped_bindings() {
        let entries = tray_entries(&shipped(), true);

        assert_eq!(entries[0].accelerator.as_deref(), Some("Ctrl+Alt+D"));
        assert_eq!(entries[1].accelerator.as_deref(), Some("Ctrl+Alt+R"));
        assert_eq!(entries[2].accelerator.as_deref(), Some("Ctrl+Alt+S"));
        assert_eq!(entries[3].accelerator.as_deref(), Some("Ctrl+Alt+E"));
    }

    #[test]
    fn the_menu_groups_actions_accessibility_and_quit_with_separators() {
        let entries = tray_entries(&shipped(), true);

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "quick_translate",
                "replace",
                "read_aloud",
                "translate_window",
                SEPARATOR_ID,
                "accessibility-granted",
                SEPARATOR_ID,
                "quit",
            ]
        );
        assert_eq!(entries.last().expect("quit").label, "Quit Translator");
        assert_eq!(
            entries.iter().filter(|entry| entry.is_separator()).count(),
            2
        );
    }

    #[test]
    fn a_granted_permission_reads_as_granted_and_is_not_a_action() {
        let entries = tray_entries(&shipped(), true);
        let row = entries
            .iter()
            .find(|entry| entry.id == ACCESSIBILITY_GRANTED_ID)
            .expect("the accessibility row");

        assert_eq!(row.label, "Accessibility: Granted");
        assert_eq!(row.accelerator, None);
        assert_eq!(action_for_id(&row.id), None);
    }

    #[test]
    fn a_missing_permission_offers_to_open_the_settings_pane() {
        let entries = tray_entries(&shipped(), false);
        let row = entries
            .iter()
            .find(|entry| entry.id == ACCESSIBILITY_NOT_GRANTED_ID)
            .expect("the accessibility row");

        assert_eq!(row.label, "Accessibility: Not granted — Open Settings…");
        assert!(entries
            .iter()
            .all(|entry| entry.id != ACCESSIBILITY_GRANTED_ID));
    }

    #[test]
    fn every_action_id_maps_back_to_its_action() {
        for action in ShortcutAction::ALL {
            assert_eq!(action_for_id(action_id(action)), Some(action));
        }
        assert_eq!(action_for_id(ACCESSIBILITY_NOT_GRANTED_ID), None);
        assert_eq!(action_for_id(QUIT_ID), None);
        assert_eq!(action_for_id(SEPARATOR_ID), None);
    }
}
